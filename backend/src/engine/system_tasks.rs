use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::WorkflowEngine;
use super::error::EngineError;
use super::expression::{evaluate_condition_tree, evaluate_loop_condition, resolve_value};
use crate::models::*;

impl WorkflowEngine {
    /// Create a task record in the DB. Returns `(task_id, is_new)`.
    /// `is_new` is `true` when the row was freshly inserted, `false` when a
    /// non-terminal task with the same (workflow_id, reference_task_name) already
    /// existed. The unique partial index `idx_task_unique_ref` is the ultimate
    /// guard; the SELECT-first path is just an optimistic fast check.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn insert_task_record(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
        status: &str,
        output: &Value,
        sub_workflow_id: Option<&str>,
    ) -> Result<(String, bool), EngineError> {
        let db = self.shards.shard_for(workflow_id);

        // Fast-path dedup: check if a non-terminal task already exists
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT task_id FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 AND status NOT IN ('FAILED', 'TIMED_OUT', 'CANCELED') LIMIT 1",
        )
        .bind(workflow_id)
        .bind(&task_def.task_reference_name)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, ref_name = %task_def.task_reference_name, error = %e, "DB error during task dedup check");
            EngineError::Database(e.to_string())
        })?;

        if let Some(existing_id) = existing {
            tracing::debug!(
                task_id = %existing_id,
                ref_name = %task_def.task_reference_name,
                "Task already exists, skipping duplicate creation"
            );
            return Ok((existing_id, false));
        }

        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let task_input = if task_def.input_parameters.is_empty() {
            input.clone()
        } else {
            let raw = serde_json::to_value(&task_def.input_parameters)
                .unwrap_or_else(|_| Value::Object(Default::default()));
            let raw_str = raw.to_string();
            if raw_str.contains("${") {
                let task_output_rows: Vec<(String, Value)> = sqlx::query_as(
                    "SELECT reference_task_name, output_data FROM task \
                     WHERE workflow_instance_id = $1 \
                     AND status IN ('COMPLETED','SKIPPED','COMPLETED_WITH_ERRORS')",
                )
                .bind(workflow_id)
                .fetch_all(db)
                .await
                .map_err(|e| {
                    tracing::error!(workflow_id = %workflow_id, ref_name = %task_def.task_reference_name, error = %e, "DB error fetching task outputs for template resolution");
                    EngineError::Database(e.to_string())
                })?;
                let task_outputs: HashMap<String, Value> = task_output_rows.into_iter().collect();
                resolve_value(&raw, input, &task_outputs, workflow_id)
            } else {
                raw
            }
        };

        let is_terminal = matches!(status, "COMPLETED" | "SKIPPED" | "COMPLETED_WITH_ERRORS");
        let is_scheduled = status == "SCHEDULED";

        let result = sqlx::query(
            "INSERT INTO task (task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status, input_data, output_data, scheduled_time, start_time, end_time, update_time, seq, sub_workflow_id)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, CASE WHEN $13 THEN NULL ELSE $9 END, CASE WHEN $10 THEN $9 ELSE NULL END, $9, $11, $12)",
        )
        .bind(&task_id)
        .bind(workflow_id)
        .bind(&task_def.task_type)
        .bind(&task_def.name)
        .bind(&task_def.task_reference_name)
        .bind(status)
        .bind(&task_input)
        .bind(output)
        .bind(now)
        .bind(is_terminal)
        .bind(seq)
        .bind(sub_workflow_id)
        .bind(is_scheduled)
        .execute(db)
        .await;

        match result {
            Ok(_) => {
                tracing::debug!(
                    task_id = %task_id,
                    task_type = %task_def.task_type,
                    ref_name = %task_def.task_reference_name,
                    status = %status,
                    "Task record created"
                );
                Ok((task_id, true))
            }
            Err(e) => {
                let err_str = e.to_string();
                if err_str.contains("idx_task_unique_ref")
                    || err_str.contains("duplicate key")
                    || err_str.contains("unique constraint")
                {
                    tracing::error!(
                        workflow_id = %workflow_id,
                        ref_name = %task_def.task_reference_name,
                        "Task insert race condition detected, resolving duplicate"
                    );
                    let id: String = sqlx::query_scalar(
                        "SELECT task_id FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 AND status NOT IN ('FAILED', 'TIMED_OUT', 'CANCELED') LIMIT 1",
                    )
                    .bind(workflow_id)
                    .bind(&task_def.task_reference_name)
                    .fetch_one(db)
                    .await
                    .map_err(|e2| {
                        tracing::error!(workflow_id = %workflow_id, ref_name = %task_def.task_reference_name, error = %e2, "Failed to fetch winner task_id after insert race");
                        EngineError::Database(e2.to_string())
                    })?;

                    tracing::debug!(
                        task_id = %id,
                        ref_name = %task_def.task_reference_name,
                        "Task race resolved — returning winner's id"
                    );
                    Ok((id, false))
                } else {
                    tracing::error!(
                        workflow_id = %workflow_id,
                        ref_name = %task_def.task_reference_name,
                        task_type = %task_def.task_type,
                        error = %err_str,
                        "Failed to insert task record"
                    );
                    Err(EngineError::Database(err_str))
                }
            }
        }
    }

    /// FORK_JOIN / FORK — auto-complete the fork task, then schedule the first
    /// task of every parallel branch concurrently.
    pub(crate) async fn handle_fork_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let fork_output = serde_json::json!({ "forkedBranches": task_def.fork_tasks.len() });
        let (_task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                input,
                seq,
                "COMPLETED",
                &fork_output,
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        // Schedule all fork branches concurrently
        let branch_futs: Vec<_> = task_def
            .fork_tasks
            .iter()
            .enumerate()
            .filter(|(_, branch)| !branch.is_empty())
            .map(|(branch_idx, branch)| {
                let branch_seq = seq + 1 + branch_idx as i32;
                Box::pin(self.schedule_tasks(workflow_id, branch, input, branch_seq))
            })
            .collect();

        let results = futures::future::join_all(branch_futs).await;
        for result in results {
            result?;
        }

        tracing::info!(
            workflow_id = %workflow_id,
            branches = task_def.fork_tasks.len(),
            "FORK scheduled – {} parallel branches",
            task_def.fork_tasks.len()
        );
        Ok(())
    }

    /// JOIN — create the task as IN_PROGRESS, then immediately check whether
    /// all prerequisite tasks are already done.
    pub(crate) async fn handle_join_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let (task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                input,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        if task_def.join_on.is_empty() {
            tracing::error!(
                workflow_id = %workflow_id,
                ref_name = %task_def.task_reference_name,
                "JOIN task has empty join_on list, auto-completing"
            );
        }

        if self.check_join_prerequisites(workflow_id, task_def).await? {
            self.complete_task_by_id(workflow_id, &task_id).await?;
            tracing::info!(workflow_id = %workflow_id, "JOIN auto-completed — all branches done");
        }
        Ok(())
    }

    /// Returns true when every reference in `join_on` has a terminal status.
    pub(crate) async fn check_join_prerequisites(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
    ) -> Result<bool, EngineError> {
        if task_def.join_on.is_empty() {
            return Ok(true);
        }
        let db = self.shards.shard_for(workflow_id);
        for ref_name in &task_def.join_on {
            let status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 ORDER BY seq DESC LIMIT 1",
            )
            .bind(workflow_id)
            .bind(ref_name)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                tracing::error!(workflow_id = %workflow_id, join_ref = %ref_name, error = %e, "DB error checking join prerequisite");
                EngineError::Database(e.to_string())
            })?;

            match status.as_deref() {
                Some("COMPLETED") | Some("SKIPPED") | Some("COMPLETED_WITH_ERRORS") => continue,
                _ => return Ok(false),
            }
        }
        Ok(true)
    }

    /// DECISION / SWITCH — auto-complete with the selected branch, then
    /// schedule the first task of that branch.
    pub(crate) async fn handle_decision_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        parent_tasks: &[WorkflowTask],
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let case_value = self.evaluate_case_value(task_def, input);
        let output = serde_json::json!({ "selectedBranch": &case_value });

        let (_task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                input,
                seq,
                "COMPLETED",
                &output,
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        let branch = task_def
            .decision_cases
            .get(&case_value)
            .unwrap_or(&task_def.default_case);

        if !branch.is_empty() {
            Box::pin(self.schedule_tasks(workflow_id, branch, input, seq + 1)).await?;
        } else {
            if parent_tasks.len() > 1 {
                Box::pin(self.schedule_tasks(workflow_id, &parent_tasks[1..], input, seq + 1))
                    .await?;
            }
        }

        tracing::info!(
            workflow_id = %workflow_id,
            selected = %case_value,
            "DECISION evaluated"
        );
        Ok(())
    }

    /// Evaluate case_value_param / case_expression / condition_tree against the task input.
    pub(crate) fn evaluate_case_value(&self, task_def: &WorkflowTask, input: &Value) -> String {
        // 106: Composite condition tree takes precedence when present
        if let Some(ref tree) = task_def.condition_tree {
            return if evaluate_condition_tree(tree, input) {
                "true".to_string()
            } else {
                "false".to_string()
            };
        }

        if let Some(param) = &task_def.case_value_param {
            if let Some(val) = input.get(param) {
                return match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
            }
            if let Some(val) = task_def.input_parameters.get(param) {
                return match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
            }
        }
        if let Some(expr) = &task_def.case_expression
            && let Some(val) = input.get(expr.as_str())
        {
            return match val {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
        }
        String::new()
    }

    /// SUB_WORKFLOW — create the task as IN_PROGRESS first, then start a child
    /// workflow only if we won the creation race.
    /// Enforces max-depth limiting to prevent infinite recursion.
    pub(crate) async fn handle_sub_workflow_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        // Check sub-workflow depth to prevent infinite recursion (max 10 levels)
        const MAX_SUB_WORKFLOW_DEPTH: u32 = 10;
        let depth = self.get_sub_workflow_depth(workflow_id).await?;
        if depth >= MAX_SUB_WORKFLOW_DEPTH {
            tracing::error!(
                workflow_id = %workflow_id,
                depth = depth,
                max_depth = MAX_SUB_WORKFLOW_DEPTH,
                "Sub-workflow max depth exceeded"
            );
            return Err(EngineError::InvalidState(format!(
                "Sub-workflow max depth ({MAX_SUB_WORKFLOW_DEPTH}) exceeded at depth {depth}"
            )));
        }

        let params = task_def.sub_workflow_param.as_ref().ok_or_else(|| {
            tracing::error!(
                workflow_id = %workflow_id,
                ref_name = %task_def.task_reference_name,
                "SUB_WORKFLOW task missing sub_workflow_param"
            );
            EngineError::InvalidState("SUB_WORKFLOW task missing sub_workflow_param".into())
        })?;

        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                input,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        if !is_new {
            tracing::debug!(
                workflow_id = %workflow_id,
                ref_name = %task_def.task_reference_name,
                "SUB_WORKFLOW task already exists, skipping duplicate"
            );
            return Ok(());
        }

        // CRITICAL: parent linkage must be set ATOMICALLY with the child
        // workflow INSERT. If we started the child first and UPDATEd
        // parent_workflow_id afterwards, at high worker concurrency the child
        // could complete before the UPDATE landed, and its completion handler
        // would observe parent_workflow_id=NULL and silently skip notifying
        // the parent — the parent's SUB_WORKFLOW task would then stay
        // IN_PROGRESS until the sweep timed it out.
        let child_req = StartWorkflowRequest {
            name: params.name.clone(),
            version: params.version.unwrap_or(1),
            input: input.clone(),
            correlation_id: None,
            priority: 0,
            task_to_domain: Default::default(),
            workflow_def: None,
            external_input_payload_storage_path: None,
            created_by: None,
            idempotency_key: None,
            idempotency_strategy: None,
            tags: vec![],
            parent_workflow_id: Some(workflow_id.to_string()),
            parent_workflow_task_id: Some(task_id.clone()),
        };
        let child_id = self.start_workflow(&child_req).await?;

        // Update the parent's task with the child's id (on parent's shard).
        let parent_db = self.shards.shard_for(workflow_id);
        if let Err(e) =
            sqlx::query("UPDATE task SET sub_workflow_id = $2, output_data = $3 WHERE task_id = $1")
                .bind(&task_id)
                .bind(&child_id)
                .bind(serde_json::json!({ "subWorkflowId": &child_id }))
                .execute(parent_db)
                .await
        {
            tracing::error!(task_id = %task_id, child_id = %child_id, error = %e, "Failed to link parent task→child, failing sub-workflow task");
            self.fail_task(
                workflow_id,
                &task_id,
                &format!("Failed to set sub_workflow_id on parent task: {e}"),
            )
            .await?;
            return Err(EngineError::Database(e.to_string()));
        }

        tracing::info!(
            workflow_id = %workflow_id,
            child_id = %child_id,
            "SUB_WORKFLOW started"
        );
        Ok(())
    }

    /// Worker / simple task — insert a SCHEDULED record and push to Redis.
    pub(crate) async fn create_and_queue_worker_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                input,
                seq,
                "SCHEDULED",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        // Resolve domain from workflow's task_to_domain map. Netflix Conductor
        // keys this map by task type/name and supports "*"; keep reference-name
        // fallback for older rust-conductor callers.
        let domain = self
            .lookup_task_domain(workflow_id, &task_def.name, &task_def.task_reference_name)
            .await;
        let queue_name = super::queue_name_for(&task_def.name, domain.as_deref());

        // Persist resolved domain on the task row so subsequent re-enqueues
        // (sweeper, resume, poll retry) route to the same queue.
        if is_new && domain.is_some() {
            let db = self.shards.shard_for(workflow_id);
            let _ = sqlx::query("UPDATE task SET domain = $2 WHERE task_id = $1")
                .bind(&task_id)
                .bind(domain.as_deref())
                .execute(db)
                .await;
        }

        // Inject env_vars from task definition into the task row
        if is_new
            && let Ok(td) = self.get_task_def(&task_def.name).await
            && let Some(env) = &td.env_vars
        {
            let db = self.shards.shard_for(workflow_id);
            let _ = sqlx::query("UPDATE task SET env_vars = $2 WHERE task_id = $1")
                .bind(&task_id)
                .bind(env)
                .execute(db)
                .await;
        }

        if is_new {
            self.set_task_routing(&task_id, workflow_id).await?;
            self.queue
                .enqueue(&queue_name, &task_id)
                .await
                .map_err(|e| {
                    tracing::error!(
                        workflow_id = %workflow_id,
                        task_id = %task_id,
                        error = %e,
                        "Kafka enqueue failed for worker task"
                    );
                    EngineError::Redis(e)
                })?;
        } else {
            // Task already exists — if it's still SCHEDULED, re-enqueue it
            // because the original Kafka push may have been lost.
            let db = self.shards.shard_for(workflow_id);
            let status: Option<String> =
                sqlx::query_scalar("SELECT status FROM task WHERE task_id = $1")
                    .bind(&task_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;

            if status.as_deref() == Some("SCHEDULED") {
                tracing::warn!(
                    workflow_id = %workflow_id,
                    task_id = %task_id,
                    "Re-enqueuing existing SCHEDULED task (possible lost enqueue)"
                );
                self.set_task_routing(&task_id, workflow_id).await?;
                self.queue
                    .enqueue(&queue_name, &task_id)
                    .await
                    .map_err(|e| {
                        tracing::error!(
                            workflow_id = %workflow_id,
                            task_id = %task_id,
                            error = %e,
                            "Kafka re-enqueue failed for existing SCHEDULED task"
                        );
                        EngineError::Redis(e)
                    })?;
            }
        }
        Ok(())
    }

    /// Look up the domain mapped for a task in the workflow's `task_to_domain`
    /// map. Netflix Conductor checks task type/name first, supports a global
    /// `*`, and treats comma-separated values as ordered fallback domains.
    pub(crate) async fn lookup_task_domain(
        &self,
        workflow_id: &str,
        task_name: &str,
        task_reference_name: &str,
    ) -> Option<String> {
        let db = self.shards.shard_for(workflow_id);
        let row: Option<(Option<Value>,)> =
            sqlx::query_as("SELECT task_to_domain FROM workflow WHERE workflow_id = $1")
                .bind(workflow_id)
                .fetch_optional(db)
                .await
                .ok()
                .flatten();
        let map = row.and_then(|(v,)| v)?;
        let v = map
            .get(task_name)
            .or_else(|| map.get(task_reference_name))
            .or_else(|| map.get("*"))?;
        select_task_domain(v.as_str()?)
    }

    /// Mark an existing task as COMPLETED by its task_id.
    pub(crate) async fn complete_task_by_id(
        &self,
        workflow_id: &str,
        task_id: &str,
    ) -> Result<(), EngineError> {
        let now = Utc::now();
        let db = self.shards.shard_for(workflow_id);
        sqlx::query(
            "UPDATE task SET status = 'COMPLETED', end_time = $2, update_time = $2 WHERE task_id = $1 AND status NOT IN ('COMPLETED', 'SKIPPED', 'COMPLETED_WITH_ERRORS', 'FAILED', 'TIMED_OUT', 'CANCELED')",
        )
        .bind(task_id)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, task_id = %task_id, error = %e, "DB error while completing task by ID");
            EngineError::Database(e.to_string())
        })?;
        Ok(())
    }

    // ── TERMINATE ────────────────────────────────────────────────────────

    /// TERMINATE — immediately end the workflow with a given status.
    /// Input parameters:
    ///   terminationStatus: "COMPLETED" | "FAILED" (default "FAILED")
    ///   terminationReason: optional string
    ///   workflowOutput: optional JSON value to set as workflow output
    pub(crate) async fn handle_terminate_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let term_status = task_input
            .get("terminationStatus")
            .and_then(|v| v.as_str())
            .unwrap_or("FAILED");

        let reason = task_input
            .get("terminationReason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let wf_output = task_input
            .get("workflowOutput")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        let output = serde_json::json!({
            "terminationStatus": term_status,
            "terminationReason": &reason,
        });

        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "COMPLETED",
                &output,
                None,
            )
            .await?;

        let db = self.shards.shard_for(workflow_id);
        let now = Utc::now();

        if term_status == "COMPLETED" {
            sqlx::query(
                "UPDATE workflow SET status = 'COMPLETED', end_time = $2, update_time = $2, output = $3, reason_for_incompletion = $4 WHERE workflow_id = $1 AND status = 'RUNNING'",
            )
            .bind(workflow_id)
            .bind(now)
            .bind(&wf_output)
            .bind(if reason.is_empty() { None } else { Some(&reason) })
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            tracing::info!(workflow_id = %workflow_id, "TERMINATE task — workflow completed");
        } else {
            self.fail_workflow(workflow_id, Some(&reason), "FAILED")
                .await?;
            tracing::info!(workflow_id = %workflow_id, reason = %reason, "TERMINATE task — workflow failed");
        }

        Ok(())
    }

    // ── SET_VARIABLE ─────────────────────────────────────────────────────

    /// SET_VARIABLE — merge input parameters into the workflow's `variables`
    /// JSON column, then auto-complete.
    pub(crate) async fn handle_set_variable_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;
        let db = self.shards.shard_for(workflow_id);

        // Read current variables
        let current: Value = sqlx::query_scalar(
            "SELECT COALESCE(variables, '{}') FROM workflow WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_one(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        // Merge
        let mut vars = match current {
            Value::Object(m) => m,
            _ => serde_json::Map::new(),
        };
        if let Value::Object(new_vars) = &task_input {
            for (k, v) in new_vars {
                vars.insert(k.clone(), v.clone());
            }
        }
        let merged = Value::Object(vars.clone());

        // Write back
        sqlx::query(
            "UPDATE workflow SET variables = $2, update_time = NOW() WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .bind(&merged)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        // Auto-complete the task with the merged variables as output
        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "COMPLETED",
                &merged,
                None,
            )
            .await?;

        tracing::info!(workflow_id = %workflow_id, keys = ?vars.keys().collect::<Vec<_>>(), "SET_VARIABLE completed");
        Ok(())
    }

    // ── HTTP ─────────────────────────────────────────────────────────────

    /// HTTP — execute an HTTP request inline.
    /// Input parameters:
    ///   http_request: { uri, method, headers, body, connectionTimeOut, readTimeOut }
    pub(crate) async fn handle_http_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        let http_req = task_input
            .get("http_request")
            .cloned()
            .unwrap_or_else(|| task_input.clone());

        let uri = http_req.get("uri").and_then(|v| v.as_str()).unwrap_or("");

        if uri.is_empty() {
            self.fail_task(
                workflow_id,
                &task_id,
                "HTTP task missing 'uri' in http_request",
            )
            .await?;
            return Ok(());
        }

        let method = http_req
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let timeout_ms = http_req
            .get("connectionTimeOut")
            .or_else(|| http_req.get("readTimeOut"))
            .and_then(|v| v.as_u64())
            .unwrap_or(30_000);

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .build()
            .map_err(|e| {
                tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to build HTTP client");
                EngineError::InvalidState(format!("HTTP client build error: {e}"))
            })?;

        let mut req_builder = match method.as_str() {
            "POST" => client.post(uri),
            "PUT" => client.put(uri),
            "DELETE" => client.delete(uri),
            "PATCH" => client.patch(uri),
            "HEAD" => client.head(uri),
            _ => client.get(uri),
        };

        // Apply headers
        if let Some(Value::Object(headers)) = http_req.get("headers") {
            for (k, v) in headers {
                let header_val = match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
                req_builder = req_builder.header(k.as_str(), header_val);
            }
        }

        // Apply body
        if let Some(body) = http_req.get("body") {
            match body {
                Value::String(s) => {
                    req_builder = req_builder.body(s.clone());
                }
                other => {
                    req_builder = req_builder.json(other);
                }
            }
        }

        let now = Utc::now();
        match req_builder.send().await {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let resp_headers: HashMap<String, String> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let resp_body: Value = response.json().await.unwrap_or(Value::Null);

                let output = serde_json::json!({
                    "response": {
                        "statusCode": status_code,
                        "headers": resp_headers,
                        "body": resp_body,
                    }
                });

                let task_status = if (200..400).contains(&status_code) {
                    "COMPLETED"
                } else {
                    "FAILED"
                };

                let db = self.shards.shard_for(workflow_id);
                sqlx::query(
                    "UPDATE task SET status = $2, output_data = $3, end_time = $4, update_time = $4 WHERE task_id = $1",
                )
                .bind(&task_id)
                .bind(task_status)
                .bind(&output)
                .bind(now)
                .execute(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;

                if task_status == "FAILED" {
                    tracing::warn!(workflow_id = %workflow_id, status_code = status_code, uri = %uri, "HTTP task failed with non-2xx/3xx status");
                } else {
                    tracing::info!(workflow_id = %workflow_id, status_code = status_code, uri = %uri, "HTTP task completed");
                }
            }
            Err(e) => {
                let reason = format!("HTTP request failed: {e}");
                self.fail_task(workflow_id, &task_id, &reason).await?;
                tracing::error!(workflow_id = %workflow_id, uri = %uri, error = %e, "HTTP task request failed");
            }
        }

        Ok(())
    }

    // ── WAIT ─────────────────────────────────────────────────────────────

    /// WAIT — create task in IN_PROGRESS. The task stays in-progress until
    /// an external signal completes it (via the complete-task API) or the
    /// sweeper times it out.
    /// Input parameters (optional):
    ///   duration: e.g. "10s", "5m", "1h" — auto-complete after duration (future: sweeper)
    pub(crate) async fn handle_wait_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        tracing::info!(workflow_id = %workflow_id, ref_name = %task_def.task_reference_name, "WAIT task created — awaiting external signal");
        Ok(())
    }

    // ── DO_WHILE ─────────────────────────────────────────────────────────

    /// DO_WHILE — execute loop_over tasks sequentially, re-evaluating the
    /// loop_condition after each iteration. The loop runs at least once.
    /// The loop_condition is evaluated as a simple truthy check on
    /// the last task's output field specified in the condition.
    pub(crate) async fn handle_do_while_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        if task_def.loop_over.is_empty() {
            let output = serde_json::json!({ "iteration": 0 });
            self.insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "COMPLETED",
                &output,
                None,
            )
            .await?;
            tracing::warn!(workflow_id = %workflow_id, "DO_WHILE has empty loop_over, auto-completing");
            return Ok(());
        }

        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        let max_iterations = 100;
        let mut iteration = 0;

        loop {
            iteration += 1;
            if iteration > max_iterations {
                self.fail_task(
                    workflow_id,
                    &task_id,
                    &format!("DO_WHILE exceeded max iterations ({})", max_iterations),
                )
                .await?;
                tracing::error!(workflow_id = %workflow_id, "DO_WHILE hit max iterations");
                return Ok(());
            }

            // Schedule the loop body with unique ref names per iteration
            let iter_tasks: Vec<WorkflowTask> = task_def
                .loop_over
                .iter()
                .map(|t| {
                    let mut clone = t.clone();
                    clone.task_reference_name = format!("{}__{}", t.task_reference_name, iteration);
                    clone
                })
                .collect();

            let body_seq = seq + (iteration as i32 * 1000);
            Box::pin(self.schedule_tasks(workflow_id, &iter_tasks, input, body_seq)).await?;

            // Wait for all body tasks to complete
            let db = self.shards.shard_for(workflow_id);
            let mut all_done = false;
            for attempt in 0..600 {
                let pending: i64 = sqlx::query_scalar(
                    "SELECT COUNT(*) FROM task WHERE workflow_instance_id = $1 AND reference_task_name = ANY($2) AND status NOT IN ('COMPLETED', 'SKIPPED', 'COMPLETED_WITH_ERRORS', 'FAILED', 'TIMED_OUT', 'CANCELED')",
                )
                .bind(workflow_id)
                .bind(iter_tasks.iter().map(|t| t.task_reference_name.clone()).collect::<Vec<_>>())
                .fetch_one(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;

                if pending == 0 {
                    all_done = true;
                    break;
                }
                if attempt < 599 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            }

            if !all_done {
                self.fail_task(workflow_id, &task_id, "DO_WHILE body tasks timed out")
                    .await?;
                return Ok(());
            }

            // Check if any body task failed
            let failed: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM task WHERE workflow_instance_id = $1 AND reference_task_name = ANY($2) AND status IN ('FAILED', 'TIMED_OUT')",
            )
            .bind(workflow_id)
            .bind(iter_tasks.iter().map(|t| t.task_reference_name.clone()).collect::<Vec<_>>())
            .fetch_one(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            if failed > 0 {
                self.fail_task(workflow_id, &task_id, "DO_WHILE body task failed")
                    .await?;
                return Ok(());
            }

            // Evaluate loop condition
            if let Some(condition) = &task_def.loop_condition {
                // Simple condition: if it references a variable that's falsy, stop
                // For now: "false" literal or empty string means stop
                let last_ref = &iter_tasks.last().unwrap().task_reference_name;
                let last_output: Value = sqlx::query_scalar(
                    "SELECT COALESCE(output_data, '{}') FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 LIMIT 1",
                )
                .bind(workflow_id)
                .bind(last_ref)
                .fetch_optional(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?
                .unwrap_or(Value::Object(Default::default()));

                let should_continue = evaluate_loop_condition(condition, &last_output, iteration);
                if !should_continue {
                    break;
                }
            } else {
                // No condition = run once
                break;
            }
        }

        // Complete the DO_WHILE task
        let output = serde_json::json!({ "iteration": iteration });
        let now = Utc::now();
        let db = self.shards.shard_for(workflow_id);
        sqlx::query(
            "UPDATE task SET status = 'COMPLETED', output_data = $2, end_time = $3, update_time = $3 WHERE task_id = $1",
        )
        .bind(&task_id)
        .bind(&output)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        tracing::info!(workflow_id = %workflow_id, iterations = iteration, "DO_WHILE completed");
        Ok(())
    }

    // ── EVENT ────────────────────────────────────────────────────────────

    /// EVENT — publish an event to the configured sink, then auto-complete.
    /// Input parameters:
    ///   The task's `sink` field specifies the Kafka topic.
    ///   All input_parameters become the event payload.
    pub(crate) async fn handle_event_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let sink = task_def.sink.as_deref().unwrap_or("conductor_events");

        let event_payload = serde_json::json!({
            "workflowId": workflow_id,
            "taskRefName": &task_def.task_reference_name,
            "sink": sink,
            "payload": &task_input,
        });

        // Publish to Kafka
        let payload_str =
            serde_json::to_string(&event_payload).unwrap_or_else(|_| "{}".to_string());

        match self.queue.produce(sink, &payload_str).await {
            Ok(()) => {
                let output = serde_json::json!({ "event": { "sink": sink, "published": true } });
                let (_task_id, _is_new) = self
                    .insert_task_record(
                        workflow_id,
                        task_def,
                        &task_input,
                        seq,
                        "COMPLETED",
                        &output,
                        None,
                    )
                    .await?;
                tracing::info!(workflow_id = %workflow_id, sink = %sink, "EVENT task published and completed");
            }
            Err(e) => {
                let output = serde_json::json!({ "event": { "sink": sink, "published": false, "error": e } });
                let (task_id, _) = self
                    .insert_task_record(
                        workflow_id,
                        task_def,
                        &task_input,
                        seq,
                        "FAILED",
                        &output,
                        None,
                    )
                    .await?;
                tracing::error!(workflow_id = %workflow_id, sink = %sink, error = %e, "EVENT task Kafka publish failed");
                let _ = self
                    .fail_task(workflow_id, &task_id, &format!("Event publish failed: {e}"))
                    .await;
            }
        }

        Ok(())
    }

    // ── LAMBDA / INLINE ──────────────────────────────────────────────────

    // LAMBDA — execute an inline expression without an external worker.
    // The task's `script_expression` or `expression` field contains a simple
    // JSON expression that is evaluated using the task input. The result is
    // written as the task output and the task completes immediately.

    // ── DYNAMIC ───────────────────────────────────────────────────────

    /// DYNAMIC — resolve the actual task type at runtime from input parameters.
    /// The `dynamic_task_name_param` field names the input parameter whose
    /// resolved value is the task type to dispatch (e.g. "taskToExecute").
    pub(crate) async fn handle_dynamic_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let param_name = task_def
            .dynamic_task_name_param
            .as_deref()
            .unwrap_or("dynamicTaskName");

        // Resolve inputParameters first so we can read the dynamic task name
        let resolved_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let resolved_task_name = resolved_input
            .get(param_name)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                tracing::error!(
                    workflow_id = %workflow_id,
                    ref_name = %task_def.task_reference_name,
                    param = %param_name,
                    "DYNAMIC task: could not resolve task name from inputParameters"
                );
                EngineError::NotFound(format!(
                    "DYNAMIC task '{}' could not resolve param '{}'",
                    task_def.task_reference_name, param_name
                ))
            })?;

        // Build a cloned WorkflowTask with the resolved name and type SIMPLE
        let mut resolved_def = task_def.clone();
        resolved_def.name = resolved_task_name.clone();
        resolved_def.task_type = "SIMPLE".to_string();

        // Remove the dynamic param from the input so it doesn't get passed to the worker
        let worker_input = if let Value::Object(mut map) = resolved_input {
            map.remove(param_name);
            Value::Object(map)
        } else {
            resolved_input
        };

        // Override inputParameters to empty so insert_task_record uses our resolved worker_input directly
        resolved_def.input_parameters = std::collections::HashMap::new();

        self.create_and_queue_worker_task(workflow_id, &resolved_def, &worker_input, seq)
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            ref_name = %task_def.task_reference_name,
            resolved_task = %resolved_task_name,
            "DYNAMIC task dispatched"
        );
        Ok(())
    }

    // ── DYNAMIC_FORK_JOIN ────────────────────────────────────────────────

    /// DYNAMIC_FORK_JOIN — dynamically fork multiple tasks based on input
    /// parameters resolved at runtime, then auto-create a JOIN that waits
    /// for all forked tasks to complete.
    ///
    /// Supports two input formats:
    ///
    /// **Format 1** (`dynamicForkJoinTasksParam`): A single input parameter
    /// containing an array of `{ "taskRefName", "taskType", "name", "input" }`.
    ///
    /// **Format 2** (`dynamicForkTasksParam` + `dynamicForkTasksInputParamName`):
    /// One param holds an array of WorkflowTask definitions, another holds a
    /// map of `{ refName: inputObject }`.
    pub(crate) async fn handle_dynamic_fork_join_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        parent_tasks: &[WorkflowTask],
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let resolved_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        // Collect dynamic tasks with per-task input parameters attached.
        // We later schedule them with the parent workflow input context so
        // expressions like ${workflow.input.foo} resolve correctly.
        let mut forked_tasks: Vec<WorkflowTask> = Vec::new();

        if let Some(param_name) = &task_def.dynamic_fork_join_tasks_param {
            // Format 1: single param with array of {taskRefName, taskType, name, input}
            let tasks_value = resolved_input.get(param_name.as_str()).ok_or_else(|| {
                tracing::error!(
                    workflow_id = %workflow_id,
                    ref_name = %task_def.task_reference_name,
                    param = %param_name,
                    "DYNAMIC_FORK_JOIN: could not find param in resolved input"
                );
                EngineError::NotFound(format!(
                    "DYNAMIC_FORK_JOIN '{}': param '{}' not found in input",
                    task_def.task_reference_name, param_name
                ))
            })?;

            let tasks_arr = tasks_value.as_array().ok_or_else(|| {
                EngineError::InvalidState(format!(
                    "DYNAMIC_FORK_JOIN '{}': param '{}' is not an array",
                    task_def.task_reference_name, param_name
                ))
            })?;

            for item in tasks_arr {
                let ref_name = item
                    .get("taskReferenceName")
                    .or_else(|| item.get("taskRefName"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let task_type = item
                    .get("type")
                    .or_else(|| item.get("taskType"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("SIMPLE");
                let task_name = item
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(ref_name);
                let task_input = item
                    .get("input")
                    .or_else(|| item.get("inputParameters"))
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));

                let input_parameters = task_input
                    .as_object()
                    .cloned()
                    .unwrap_or_default()
                    .into_iter()
                    .collect();

                let wt = WorkflowTask {
                    name: task_name.to_string(),
                    task_reference_name: ref_name.to_string(),
                    task_type: task_type.to_string(),
                    description: None,
                    input_parameters,
                    optional: false,
                    start_delay: 0,
                    sub_workflow_param: None,
                    join_on: Vec::new(),
                    fork_tasks: Vec::new(),
                    decision_cases: Default::default(),
                    default_case: Vec::new(),
                    case_expression: None,
                    case_value_param: None,
                    loop_condition: None,
                    loop_over: Vec::new(),
                    retry_count: None,
                    evaluator_type: None,
                    expression: None,
                    script_expression: None,
                    dynamic_task_name_param: None,
                    dynamic_fork_join_tasks_param: None,
                    dynamic_fork_tasks_param: None,
                    dynamic_fork_tasks_input_param_name: None,
                    sink: None,
                    task_definition: None,
                    rate_limited: None,
                    default_exclusive_join_task: Vec::new(),
                    async_complete: false,
                    on_state_change: None,
                    join_status: None,
                    cache_config: None,
                    permissive: None,
                    ..Default::default()
                };
                forked_tasks.push(wt);
            }
        } else if let Some(tasks_param) = &task_def.dynamic_fork_tasks_param {
            // Format 2: separate task defs + input map
            let tasks_value = resolved_input.get(tasks_param.as_str()).ok_or_else(|| {
                EngineError::NotFound(format!(
                    "DYNAMIC_FORK_JOIN '{}': param '{}' not found",
                    task_def.task_reference_name, tasks_param
                ))
            })?;

            let input_param_name = task_def
                .dynamic_fork_tasks_input_param_name
                .as_deref()
                .unwrap_or("forkedTasksInputs");

            let inputs_map = resolved_input
                .get(input_param_name)
                .and_then(|v| v.as_object())
                .cloned()
                .unwrap_or_default();

            let tasks_arr = tasks_value.as_array().ok_or_else(|| {
                EngineError::InvalidState(format!(
                    "DYNAMIC_FORK_JOIN '{}': param '{}' is not an array",
                    task_def.task_reference_name, tasks_param
                ))
            })?;

            for item in tasks_arr {
                let mut wt: WorkflowTask = serde_json::from_value(item.clone()).map_err(|e| {
                    EngineError::InvalidState(format!(
                        "DYNAMIC_FORK_JOIN: invalid task definition: {e}"
                    ))
                })?;
                let task_input = inputs_map
                    .get(&wt.task_reference_name)
                    .cloned()
                    .unwrap_or(Value::Object(Default::default()));

                // Merge per-task input map (format 2) into the task definition
                // inputParameters so branch tasks resolve against parent input.
                if let Some(obj) = task_input.as_object() {
                    for (k, v) in obj {
                        wt.input_parameters.insert(k.clone(), v.clone());
                    }
                }

                forked_tasks.push(wt);
            }
        } else {
            return Err(EngineError::InvalidState(format!(
                "DYNAMIC_FORK_JOIN '{}': requires dynamicForkJoinTasksParam or dynamicForkTasksParam",
                task_def.task_reference_name
            )));
        }

        if forked_tasks.is_empty() {
            return Err(EngineError::InvalidState(format!(
                "DYNAMIC_FORK_JOIN '{}': resolved to zero tasks",
                task_def.task_reference_name
            )));
        }

        // Complete the fork task itself
        let fork_output = serde_json::json!({
            "forkedBranches": forked_tasks.len(),
            "forkedTaskRefs": forked_tasks.iter().map(|t| &t.task_reference_name).collect::<Vec<_>>(),
        });
        let (_task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &resolved_input,
                seq,
                "COMPLETED",
                &fork_output,
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        // Schedule all forked tasks in parallel
        let join_on: Vec<String> = forked_tasks
            .iter()
            .map(|t| t.task_reference_name.clone())
            .collect();
        let fork_count = forked_tasks.len();

        // Prepare owned task arrays so they live long enough for async scheduling
        let prepared: Vec<(Vec<WorkflowTask>, i32)> = forked_tasks
            .into_iter()
            .enumerate()
            .map(|(idx, wt)| (vec![wt], seq + 1 + idx as i32))
            .collect();

        let branch_futs: Vec<_> = prepared
            .iter()
            .map(|(tasks, branch_seq)| {
                Box::pin(self.schedule_tasks(workflow_id, tasks, input, *branch_seq))
            })
            .collect();

        let results = futures::future::join_all(branch_futs).await;
        for result in results {
            result?;
        }

        // Auto-create and schedule a JOIN task — check if the next task in the
        // definition is already a JOIN, otherwise synthesize one.
        let join_task = if parent_tasks.len() > 1 && parent_tasks[1].task_type == "JOIN" {
            let mut jt = parent_tasks[1].clone();
            jt.join_on = join_on.clone();
            jt
        } else {
            WorkflowTask {
                name: format!("{}_join", task_def.task_reference_name),
                task_reference_name: format!("{}_join", task_def.task_reference_name),
                task_type: "JOIN".to_string(),
                description: None,
                input_parameters: Default::default(),
                optional: false,
                start_delay: 0,
                sub_workflow_param: None,
                join_on: join_on.clone(),
                fork_tasks: Vec::new(),
                decision_cases: Default::default(),
                default_case: Vec::new(),
                case_expression: None,
                case_value_param: None,
                loop_condition: None,
                loop_over: Vec::new(),
                retry_count: None,
                evaluator_type: None,
                expression: None,
                script_expression: None,
                dynamic_task_name_param: None,
                dynamic_fork_join_tasks_param: None,
                dynamic_fork_tasks_param: None,
                dynamic_fork_tasks_input_param_name: None,
                sink: None,
                task_definition: None,
                rate_limited: None,
                default_exclusive_join_task: Vec::new(),
                async_complete: false,
                on_state_change: None,
                join_status: None,
                cache_config: None,
                permissive: None,
                ..Default::default()
            }
        };

        self.handle_join_task(workflow_id, &join_task, input, seq + 1000)
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            ref_name = %task_def.task_reference_name,
            branches = fork_count,
            "DYNAMIC_FORK_JOIN scheduled – {} parallel tasks",
            fork_count
        );
        Ok(())
    }

    // ── MAP ──────────────────────────────────────────────────────────────

    /// MAP task: fan-out over an array, creating one sub-task per item,
    /// then auto-JOIN to collect results.
    pub(crate) async fn handle_map_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let items_param = task_def.map_items_param.as_deref().unwrap_or("items");
        let items = task_input
            .get(items_param)
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let map_template = task_def.map_task.as_ref().ok_or_else(|| {
            EngineError::InvalidState(format!(
                "MAP task '{}' missing map_task template",
                task_def.task_reference_name
            ))
        })?;

        let parallelism = task_def
            .map_parallelism
            .unwrap_or(items.len() as i32)
            .max(1) as usize;

        // Record the MAP task itself as COMPLETED
        let map_output = serde_json::json!({
            "itemCount": items.len(),
            "parallelism": parallelism,
        });
        let (_map_task_id, is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "COMPLETED",
                &map_output,
                None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        // Create sub-tasks for each item
        let mut fork_refs = Vec::new();
        for (idx, item) in items.iter().enumerate() {
            let mut sub_task = map_template.as_ref().clone();
            sub_task.task_reference_name = format!("{}__map_{}", task_def.task_reference_name, idx);
            sub_task.name = format!("{}_map_{}", map_template.name, idx);

            // Inject the item and index into sub-task input
            let mut sub_input = task_input.clone();
            if let Some(obj) = sub_input.as_object_mut() {
                obj.insert("mapItem".to_string(), item.clone());
                obj.insert("mapIndex".to_string(), Value::Number(idx.into()));
            }

            let sub_seq = seq + 1 + idx as i32;
            fork_refs.push(sub_task.task_reference_name.clone());

            // Schedule sub-task (respect parallelism: schedule up to `parallelism` at once)
            if idx < parallelism {
                self.create_and_queue_worker_task(workflow_id, &sub_task, &sub_input, sub_seq)
                    .await?;
            } else {
                // Create as SCHEDULED but don't queue — will be picked up by advance
                self.insert_task_record(
                    workflow_id,
                    &sub_task,
                    &sub_input,
                    sub_seq,
                    "SCHEDULED",
                    &Value::Object(Default::default()),
                    None,
                )
                .await?;
            }
        }

        // Create an auto-JOIN task that waits for all sub-tasks
        let join_task = WorkflowTask {
            name: format!("{}_join", task_def.name),
            task_reference_name: format!("{}__map_join", task_def.task_reference_name),
            task_type: "JOIN".into(),
            join_on: fork_refs,
            ..Default::default()
        };
        self.handle_join_task(
            workflow_id,
            &join_task,
            input,
            seq + items.len() as i32 + 100,
        )
        .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            ref_name = %task_def.task_reference_name,
            items = items.len(),
            parallelism = parallelism,
            "MAP task scheduled"
        );
        Ok(())
    }

    // ── WAIT_FOR_SIGNAL ──────────────────────────────────────────────────

    /// Create an IN_PROGRESS task that waits for an external signal.
    /// Completed by `send_signal()` in advanced.rs.
    pub(crate) async fn handle_wait_for_signal_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        // Extract the signal name from input parameters
        let signal_name = task_input
            .get("signalName")
            .and_then(|v| v.as_str())
            .unwrap_or(&task_def.task_reference_name);

        let input_with_signal = serde_json::json!({
            "signalName": signal_name,
            "waitingSince": Utc::now().timestamp_millis(),
            "originalInput": task_input,
        });

        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &input_with_signal,
                seq,
                "IN_PROGRESS",
                &Value::Object(Default::default()),
                None,
            )
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            ref_name = %task_def.task_reference_name,
            signal_name = %signal_name,
            "WAIT_FOR_SIGNAL task created, awaiting signal"
        );
        Ok(())
    }

    // ── LAMBDA / INLINE ──────────────────────────────────────────────────

    pub(crate) async fn handle_lambda_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let task_input = self
            .resolve_task_input(workflow_id, task_def, input)
            .await?;

        let output = if let Some(expr) = &task_def.script_expression {
            // Try to parse the expression as a JSON value, with variable substitution
            match serde_json::from_str::<Value>(expr) {
                Ok(v) => v,
                Err(_) => {
                    // If it's not valid JSON, treat it as a simple key lookup from input
                    task_input
                        .get(expr)
                        .cloned()
                        .unwrap_or(Value::String(expr.clone()))
                }
            }
        } else if let Some(expr) = &task_def.expression {
            match serde_json::from_str::<Value>(expr) {
                Ok(v) => v,
                Err(_) => task_input
                    .get(expr)
                    .cloned()
                    .unwrap_or(Value::String(expr.clone())),
            }
        } else {
            // If no expression, pass through input as output
            task_input.clone()
        };

        let result_output = serde_json::json!({
            "result": output,
        });

        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id,
                task_def,
                &task_input,
                seq,
                "COMPLETED",
                &result_output,
                None,
            )
            .await?;

        tracing::info!(
            workflow_id = %workflow_id,
            ref_name = %task_def.task_reference_name,
            "LAMBDA task evaluated"
        );
        Ok(())
    }

    // ── Shared helper ────────────────────────────────────────────────────

    /// Resolve task input parameters using template expressions.
    async fn resolve_task_input(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
    ) -> Result<Value, EngineError> {
        if task_def.input_parameters.is_empty() {
            return Ok(input.clone());
        }
        let raw = serde_json::to_value(&task_def.input_parameters)
            .unwrap_or_else(|_| Value::Object(Default::default()));
        let raw_str = raw.to_string();
        if !raw_str.contains("${") {
            return Ok(raw);
        }
        let db = self.shards.shard_for(workflow_id);
        let task_output_rows: Vec<(String, Value)> = sqlx::query_as(
            "SELECT reference_task_name, output_data FROM task \
             WHERE workflow_instance_id = $1 \
             AND status IN ('COMPLETED','SKIPPED','COMPLETED_WITH_ERRORS')",
        )
        .bind(workflow_id)
        .fetch_all(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        let task_outputs: HashMap<String, Value> = task_output_rows.into_iter().collect();
        Ok(resolve_value(&raw, input, &task_outputs, workflow_id))
    }

    /// Walk the parent_workflow_id chain to compute sub-workflow nesting depth.
    async fn get_sub_workflow_depth(&self, workflow_id: &str) -> Result<u32, EngineError> {
        let mut depth = 0u32;
        let mut current_id = workflow_id.to_string();
        loop {
            let db = self.shards.shard_for(&current_id);
            let parent: Option<Option<String>> = sqlx::query_scalar(
                "SELECT parent_workflow_id FROM workflow WHERE workflow_id = $1",
            )
            .bind(&current_id)
            .fetch_optional(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            match parent {
                Some(Some(pid)) if !pid.is_empty() => {
                    depth += 1;
                    current_id = pid;
                    if depth > 100 {
                        // Safety: break infinite loops
                        break;
                    }
                }
                _ => break,
            }
        }
        Ok(depth)
    }
}

fn select_task_domain(domain: &str) -> Option<String> {
    let selected = domain.split(',').map(str::trim).rfind(|d| !d.is_empty())?;
    if selected.eq_ignore_ascii_case("NO_DOMAIN") {
        None
    } else {
        Some(selected.to_string())
    }
}

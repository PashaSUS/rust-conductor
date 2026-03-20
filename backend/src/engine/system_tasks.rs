use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::error::EngineError;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    /// Create a task record in the DB. Returns `(task_id, is_new)`.
    /// `is_new` is `true` when the row was freshly inserted, `false` when a
    /// non-terminal task with the same (workflow_id, reference_task_name) already
    /// existed. The unique partial index `idx_task_unique_ref` is the ultimate
    /// guard; the SELECT-first path is just an optimistic fast check.
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
    /// task of every parallel branch.
    pub(crate) async fn handle_fork_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let fork_output = serde_json::json!({ "forkedBranches": task_def.fork_tasks.len() });
        let (_task_id, is_new) = self.insert_task_record(
            workflow_id, task_def, input, seq, "COMPLETED", &fork_output, None,
        )
        .await?;

        if !is_new {
            return Ok(());
        }

        for (branch_idx, branch) in task_def.fork_tasks.iter().enumerate() {
            if !branch.is_empty() {
                let branch_seq = seq + 1 + branch_idx as i32;
                Box::pin(self.schedule_tasks(workflow_id, branch, input, branch_seq))
                    .await?;
            }
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
            .insert_task_record(workflow_id, task_def, input, seq, "IN_PROGRESS", &Value::Object(Default::default()), None)
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

        let (_task_id, is_new) = self.insert_task_record(
            workflow_id, task_def, input, seq, "COMPLETED", &output, None,
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
            Box::pin(self.schedule_tasks(workflow_id, branch, input, seq + 1))
                .await?;
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

    /// Evaluate case_value_param / case_expression against the task input.
    pub(crate) fn evaluate_case_value(&self, task_def: &WorkflowTask, input: &Value) -> String {
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
        if let Some(expr) = &task_def.case_expression {
            if let Some(val) = input.get(expr.as_str()) {
                return match val {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                };
            }
        }
        String::new()
    }

    /// SUB_WORKFLOW — create the task as IN_PROGRESS first, then start a child
    /// workflow only if we won the creation race.
    pub(crate) async fn handle_sub_workflow_task(
        &self,
        workflow_id: &str,
        task_def: &WorkflowTask,
        input: &Value,
        seq: i32,
    ) -> Result<(), EngineError> {
        let params = task_def.sub_workflow_param.as_ref().ok_or_else(|| {
            tracing::error!(
                workflow_id = %workflow_id,
                ref_name = %task_def.task_reference_name,
                "SUB_WORKFLOW task missing sub_workflow_param"
            );
            EngineError::InvalidState(
                "SUB_WORKFLOW task missing sub_workflow_param".into(),
            )
        })?;

        let (task_id, is_new) = self.insert_task_record(
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
        };
        let child_id = self.start_workflow(&child_req).await?;

        // The child workflow lives on its own shard (determined by child_id).
        // Link child → parent (on child's shard).
        let child_db = self.shards.shard_for(&child_id);
        if let Err(e) = sqlx::query(
            "UPDATE workflow SET parent_workflow_id = $2, parent_workflow_task_id = $3 WHERE workflow_id = $1",
        )
        .bind(&child_id)
        .bind(workflow_id)
        .bind(&task_id)
        .execute(child_db)
        .await
        {
            tracing::error!(child_id = %child_id, error = %e, "Failed to link child→parent, failing sub-workflow task");
            self.fail_task(workflow_id, &task_id, &format!("Failed to link child workflow: {e}")).await?;
            return Err(EngineError::Database(e.to_string()));
        }

        // Update the parent's task with the child's id (on parent's shard).
        let parent_db = self.shards.shard_for(workflow_id);
        if let Err(e) = sqlx::query(
            "UPDATE task SET sub_workflow_id = $2, output_data = $3 WHERE task_id = $1",
        )
        .bind(&task_id)
        .bind(&child_id)
        .bind(&serde_json::json!({ "subWorkflowId": &child_id }))
        .execute(parent_db)
        .await
        {
            tracing::error!(task_id = %task_id, child_id = %child_id, error = %e, "Failed to link parent task→child, failing sub-workflow task");
            self.fail_task(workflow_id, &task_id, &format!("Failed to set sub_workflow_id on parent task: {e}")).await?;
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
            .insert_task_record(workflow_id, task_def, input, seq, "SCHEDULED", &Value::Object(Default::default()), None)
            .await?;

        if is_new {
            self.set_task_routing(&task_id, workflow_id).await?;
            self.queue
                .enqueue(&task_def.name, &task_id)
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
            let status: Option<String> = sqlx::query_scalar(
                "SELECT status FROM task WHERE task_id = $1",
            )
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
                    .enqueue(&task_def.name, &task_id)
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

    /// Mark an existing task as COMPLETED by its task_id.
    pub(crate) async fn complete_task_by_id(&self, workflow_id: &str, task_id: &str) -> Result<(), EngineError> {
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;

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
            .insert_task_record(workflow_id, task_def, &task_input, seq, "COMPLETED", &output, None)
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
            self.fail_workflow(workflow_id, Some(&reason), "FAILED").await?;
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;
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
        sqlx::query("UPDATE workflow SET variables = $2, update_time = NOW() WHERE workflow_id = $1")
            .bind(workflow_id)
            .bind(&merged)
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        // Auto-complete the task with the merged variables as output
        let (_task_id, _is_new) = self
            .insert_task_record(workflow_id, task_def, &task_input, seq, "COMPLETED", &merged, None)
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;

        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id, task_def, &task_input, seq, "IN_PROGRESS",
                &Value::Object(Default::default()), None,
            )
            .await?;

        if !is_new {
            return Ok(());
        }

        let http_req = task_input
            .get("http_request")
            .cloned()
            .unwrap_or_else(|| task_input.clone());

        let uri = http_req
            .get("uri")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if uri.is_empty() {
            self.fail_task(workflow_id, &task_id, "HTTP task missing 'uri' in http_request").await?;
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
                Value::String(s) => { req_builder = req_builder.body(s.clone()); }
                other => { req_builder = req_builder.json(other); }
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

                let task_status = if status_code >= 200 && status_code < 400 {
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;

        let (_task_id, _is_new) = self
            .insert_task_record(
                workflow_id, task_def, &task_input, seq, "IN_PROGRESS",
                &Value::Object(Default::default()), None,
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;

        if task_def.loop_over.is_empty() {
            let output = serde_json::json!({ "iteration": 0 });
            self.insert_task_record(workflow_id, task_def, &task_input, seq, "COMPLETED", &output, None)
                .await?;
            tracing::warn!(workflow_id = %workflow_id, "DO_WHILE has empty loop_over, auto-completing");
            return Ok(());
        }

        let (task_id, is_new) = self
            .insert_task_record(
                workflow_id, task_def, &task_input, seq, "IN_PROGRESS",
                &Value::Object(Default::default()), None,
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
                self.fail_task(workflow_id, &task_id, &format!("DO_WHILE exceeded max iterations ({})", max_iterations)).await?;
                tracing::error!(workflow_id = %workflow_id, "DO_WHILE hit max iterations");
                return Ok(());
            }

            // Schedule the loop body with unique ref names per iteration
            let iter_tasks: Vec<WorkflowTask> = task_def.loop_over.iter().map(|t| {
                let mut clone = t.clone();
                clone.task_reference_name = format!("{}__{}", t.task_reference_name, iteration);
                clone
            }).collect();

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
                .bind(&iter_tasks.iter().map(|t| t.task_reference_name.clone()).collect::<Vec<_>>())
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
                self.fail_task(workflow_id, &task_id, "DO_WHILE body tasks timed out").await?;
                return Ok(());
            }

            // Check if any body task failed
            let failed: i64 = sqlx::query_scalar(
                "SELECT COUNT(*) FROM task WHERE workflow_instance_id = $1 AND reference_task_name = ANY($2) AND status IN ('FAILED', 'TIMED_OUT')",
            )
            .bind(workflow_id)
            .bind(&iter_tasks.iter().map(|t| t.task_reference_name.clone()).collect::<Vec<_>>())
            .fetch_one(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            if failed > 0 {
                self.fail_task(workflow_id, &task_id, "DO_WHILE body task failed").await?;
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
        let task_input = self.resolve_task_input(workflow_id, task_def, input).await?;

        let sink = task_def.sink.as_deref().unwrap_or("conductor_events");

        let event_payload = serde_json::json!({
            "workflowId": workflow_id,
            "taskRefName": &task_def.task_reference_name,
            "sink": sink,
            "payload": &task_input,
        });

        // Publish to Kafka
        let payload_str = serde_json::to_string(&event_payload)
            .unwrap_or_else(|_| "{}".to_string());

        match self.queue.produce(sink, &payload_str).await {
            Ok(()) => {
                let output = serde_json::json!({ "event": { "sink": sink, "published": true } });
                let (_task_id, _is_new) = self
                    .insert_task_record(workflow_id, task_def, &task_input, seq, "COMPLETED", &output, None)
                    .await?;
                tracing::info!(workflow_id = %workflow_id, sink = %sink, "EVENT task published and completed");
            }
            Err(e) => {
                let output = serde_json::json!({ "event": { "sink": sink, "published": false, "error": e } });
                let (task_id, _) = self
                    .insert_task_record(workflow_id, task_def, &task_input, seq, "FAILED", &output, None)
                    .await?;
                tracing::error!(workflow_id = %workflow_id, sink = %sink, error = %e, "EVENT task Kafka publish failed");
                let _ = self.fail_task(workflow_id, &task_id, &format!("Event publish failed: {e}")).await;
            }
        }

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
}

// ── Template expression resolution ──────────────────────────────────────────

/// Recursively resolve `${...}` template expressions in a JSON value.
///
/// Supported expressions:
///   ${workflow.input.field.path}  — value from the workflow input
///   ${workflow.workflowId}        — the workflow's ID
///   ${refName.output.field.path}  — output of a completed task by reference name
pub(crate) fn resolve_value(
    val: &Value,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    workflow_id: &str,
) -> Value {
    match val {
        Value::String(s) => resolve_string_value(s, workflow_input, task_outputs, workflow_id),
        Value::Object(map) => {
            let resolved: serde_json::Map<String, Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), resolve_value(v, workflow_input, task_outputs, workflow_id)))
                .collect();
            Value::Object(resolved)
        }
        Value::Array(arr) => {
            Value::Array(
                arr.iter()
                    .map(|v| resolve_value(v, workflow_input, task_outputs, workflow_id))
                    .collect(),
            )
        }
        other => other.clone(),
    }
}

pub(crate) fn resolve_string_value(
    s: &str,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    workflow_id: &str,
) -> Value {
    // If the entire string is a single ${...} expression, preserve the resolved type
    let trimmed = s.trim();
    if trimmed.starts_with("${") && trimmed.ends_with('}') {
        let inner = &trimmed[2..trimmed.len() - 1];
        if !inner.contains("${") {
            if let Some(resolved) = resolve_expression(inner, workflow_input, task_outputs, workflow_id) {
                return resolved;
            }
        }
    }

    if !s.contains("${") {
        return Value::String(s.to_string());
    }

    // String interpolation for mixed content like "prefix-${workflow.input.id}-suffix"
    let mut result = String::new();
    let mut remaining = s;

    while let Some(start) = remaining.find("${") {
        result.push_str(&remaining[..start]);
        let after = &remaining[start + 2..];
        if let Some(end) = after.find('}') {
            let expr = &after[..end];
            match resolve_expression(expr, workflow_input, task_outputs, workflow_id) {
                Some(Value::String(v)) => result.push_str(&v),
                Some(Value::Null) => result.push_str("null"),
                Some(v) => result.push_str(&v.to_string()),
                None => {
                    result.push_str("${");
                    result.push_str(expr);
                    result.push('}');
                }
            }
            remaining = &after[end + 1..];
        } else {
            result.push_str(&remaining[start..]);
            remaining = "";
        }
    }
    result.push_str(remaining);
    Value::String(result)
}

pub(crate) fn resolve_expression(
    expr: &str,
    workflow_input: &Value,
    task_outputs: &HashMap<String, Value>,
    _workflow_id: &str,
) -> Option<Value> {
    let parts: Vec<&str> = expr.split('.').collect();
    if parts.len() < 2 {
        return None;
    }

    if parts[0] == "workflow" {
        return match parts[1] {
            "workflowId" => Some(Value::String(_workflow_id.to_string())),
            "input" => {
                if parts.len() == 2 {
                    Some(workflow_input.clone())
                } else {
                    navigate_json(workflow_input, &parts[2..])
                }
            }
            _ => None,
        };
    }

    // Task reference: refName.output.field.path
    if parts[1] == "output" {
        if let Some(output) = task_outputs.get(parts[0]) {
            if parts.len() == 2 {
                return Some(output.clone());
            }
            return navigate_json(output, &parts[2..]);
        }
    }

    None
}

pub(crate) fn navigate_json(val: &Value, path: &[&str]) -> Option<Value> {
    let mut current = val;
    for &segment in path {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

/// Evaluate a DO_WHILE loop condition. Supports:
///   - `"iteration < N"` — continue while iteration count is below N
///   - `"true"` / `"false"` — literal
///   - Otherwise: check if the last task output's `result` field is truthy
pub(crate) fn evaluate_loop_condition(condition: &str, last_output: &Value, iteration: usize) -> bool {
    let trimmed = condition.trim();

    if trimmed == "false" || trimmed.is_empty() {
        return false;
    }
    if trimmed == "true" {
        return true;
    }

    // Simple "iteration < N" pattern
    if let Some(rest) = trimmed.strip_prefix("iteration") {
        let rest = rest.trim();
        if let Some(n_str) = rest.strip_prefix('<') {
            if let Ok(n) = n_str.trim().parse::<usize>() {
                return iteration < n;
            }
        }
        if let Some(n_str) = rest.strip_prefix("<=") {
            if let Ok(n) = n_str.trim().parse::<usize>() {
                return iteration <= n;
            }
        }
    }

    // Check last task output for a truthy "result" field
    match last_output.get("shouldContinue").or_else(|| last_output.get("result")) {
        Some(Value::Bool(b)) => *b,
        Some(Value::String(s)) => s != "false" && !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0) != 0.0,
        Some(Value::Null) => false,
        _ => false,
    }
}

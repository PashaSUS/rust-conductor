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
        .map_err(|e| EngineError::Database(e.to_string()))?;

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
                .map_err(|e| EngineError::Database(e.to_string()))?;
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
                    let id: String = sqlx::query_scalar(
                        "SELECT task_id FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 AND status NOT IN ('FAILED', 'TIMED_OUT', 'CANCELED') LIMIT 1",
                    )
                    .bind(workflow_id)
                    .bind(&task_def.task_reference_name)
                    .fetch_one(db)
                    .await
                    .map_err(|e2| EngineError::Database(e2.to_string()))?;

                    tracing::debug!(
                        task_id = %id,
                        ref_name = %task_def.task_reference_name,
                        "Task race resolved — returning winner's id"
                    );
                    Ok((id, false))
                } else {
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
            .map_err(|e| EngineError::Database(e.to_string()))?;

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
        sqlx::query(
            "UPDATE workflow SET parent_workflow_id = $2, parent_workflow_task_id = $3 WHERE workflow_id = $1",
        )
        .bind(&child_id)
        .bind(workflow_id)
        .bind(&task_def.task_reference_name)
        .execute(child_db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        // Update the parent's task with the child's id (on parent's shard).
        let parent_db = self.shards.shard_for(workflow_id);
        sqlx::query(
            "UPDATE task SET sub_workflow_id = $2, output_data = $3 WHERE task_id = $1",
        )
        .bind(&task_id)
        .bind(&child_id)
        .bind(&serde_json::json!({ "subWorkflowId": &child_id }))
        .execute(parent_db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

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
            let queue_key = format!("conductor:queue:{}", task_def.name);
            let pool = self.redis.random_pool();
            let mut conn: deadpool_redis::Connection = pool.get().await.map_err(|e| EngineError::Redis(e.to_string()))?;
            let _: () = deadpool_redis::redis::AsyncCommands::lpush(&mut conn, &queue_key, &task_id)
                .await
                .map_err(|e| EngineError::Redis(e.to_string()))?;
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
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }
}

// ── Template expression resolution ──────────────────────────────────────────

/// Recursively resolve `${...}` template expressions in a JSON value.
///
/// Supported expressions:
///   ${workflow.input.field.path}  — value from the workflow input
///   ${workflow.workflowId}        — the workflow's ID
///   ${refName.output.field.path}  — output of a completed task by reference name
fn resolve_value(
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

fn resolve_string_value(
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

fn resolve_expression(
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

fn navigate_json(val: &Value, path: &[&str]) -> Option<Value> {
    let mut current = val;
    for &segment in path {
        current = current.get(segment)?;
    }
    Some(current.clone())
}

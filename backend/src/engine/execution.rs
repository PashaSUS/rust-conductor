use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::WorkflowEngine;
use super::error::EngineError;
use crate::models::*;

impl WorkflowEngine {
    pub async fn start_workflow(&self, req: &StartWorkflowRequest) -> Result<String, EngineError> {
        let mut def = self.get_workflow_def(&req.name, Some(req.version)).await?;

        // 108. Resolve workflow inheritance before scheduling
        if def.base_workflow.is_some() {
            def = self.resolve_inheritance(&def).await?;
        }

        // Apply per-definition input parameter descriptions:
        //  - fill in default_value for missing inputs
        //  - reject request if a required parameter is missing and has no default
        // This is skipped entirely when `input_parameter_definitions` is empty, so
        // definitions using only the legacy Netflix `inputParameters: [names]`
        // behave exactly as before.
        let effective_input =
            apply_input_parameter_definitions(&def.input_parameter_definitions, &req.input)?;

        let workflow_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let db = self.shards.shard_for(&workflow_id);

        let tags_json = serde_json::to_value(&req.tags).unwrap_or(Value::Array(vec![]));
        let sla_deadline = def
            .sla_deadline_seconds
            .map(|secs| now + chrono::Duration::seconds(secs));

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def, tags, sla_deadline, parent_workflow_id, parent_workflow_task_id)
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8, $9, $10, $11, $12)",
        )
        .bind(&workflow_id)
        .bind(&req.name)
        .bind(req.version)
        .bind(&effective_input)
        .bind(&req.correlation_id)
        .bind(now)
        .bind(req.priority)
        .bind(serde_json::to_value(&def).ok())
        .bind(&tags_json)
        .bind(sla_deadline)
        .bind(&req.parent_workflow_id)
        .bind(&req.parent_workflow_task_id)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, name = %req.name, error = %e, "Failed to INSERT workflow row");
            EngineError::Database(e.to_string())
        })?;

        crate::metrics::record_workflow_started();

        if let Err(e) =
            Box::pin(self.schedule_tasks(&workflow_id, &def.tasks, &effective_input, 0)).await
        {
            tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to schedule initial tasks, marking workflow FAILED");
            let _ = sqlx::query(
                "UPDATE workflow SET status = 'FAILED', end_time = NOW(), update_time = NOW(), reason_for_incompletion = $2 WHERE workflow_id = $1",
            )
            .bind(&workflow_id)
            .bind(format!("Failed to schedule initial tasks: {e}"))
            .execute(db)
            .await;
            return Err(e);
        }

        tracing::info!(workflow_id = %workflow_id, name = %req.name, "Workflow started");
        Ok(workflow_id)
    }

    pub(crate) async fn schedule_tasks(
        &self,
        workflow_id: &str,
        tasks: &[WorkflowTask],
        input: &Value,
        start_seq: i32,
    ) -> Result<(), EngineError> {
        if tasks.is_empty() {
            return Ok(());
        }

        let task_def = &tasks[0];
        let task_type = task_def.task_type.as_str();

        match task_type {
            "FORK_JOIN" | "FORK" => {
                self.handle_fork_task(workflow_id, task_def, input, start_seq)
                    .await?;
                if tasks.len() > 1 {
                    let next = &tasks[1];
                    if next.task_type == "JOIN" {
                        self.handle_join_task(workflow_id, next, input, start_seq + 1000)
                            .await?;
                    }
                }
            }
            "JOIN" => {
                self.handle_join_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "DECISION" | "SWITCH" => {
                self.handle_decision_task(workflow_id, task_def, tasks, input, start_seq)
                    .await?;
            }
            "SUB_WORKFLOW" => {
                self.handle_sub_workflow_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "TERMINATE" => {
                self.handle_terminate_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "SET_VARIABLE" => {
                self.handle_set_variable_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "HTTP" => {
                self.handle_http_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "WAIT" => {
                self.handle_wait_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "DO_WHILE" => {
                self.handle_do_while_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "EVENT" => {
                self.handle_event_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "LAMBDA" | "INLINE" => {
                self.handle_lambda_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "DYNAMIC" => {
                self.handle_dynamic_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            // Both spellings are accepted: Netflix Conductor uses
            // FORK_JOIN_DYNAMIC; some clients/UIs use DYNAMIC_FORK_JOIN.
            "DYNAMIC_FORK_JOIN" | "FORK_JOIN_DYNAMIC" => {
                self.handle_dynamic_fork_join_task(workflow_id, task_def, tasks, input, start_seq)
                    .await?;
            }
            "MAP" => {
                self.handle_map_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            "WAIT_FOR_SIGNAL" => {
                self.handle_wait_for_signal_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
            _ => {
                if task_type != "SIMPLE" {
                    tracing::warn!(
                        workflow_id = %workflow_id,
                        task_type = %task_type,
                        ref_name = %task_def.task_reference_name,
                        "Unknown task type, treating as worker task"
                    );
                }
                self.create_and_queue_worker_task(workflow_id, task_def, input, start_seq)
                    .await?;
            }
        }

        Ok(())
    }
}

/// Apply `WorkflowInputParameterDef` rules to the request input.
///
/// * Returns the original `input` unchanged when `defs` is empty (legacy
///   Netflix-Conductor behaviour preserved exactly).
/// * Otherwise, fills in `default_value` for any parameter missing from the
///   request, and returns `EngineError::InvalidInput` (HTTP 400) if a
///   parameter marked `required = true` is absent and has no default.
/// * Never mutates request fields the caller didn't declare — extra keys
///   pass through untouched, matching Conductor's permissive input model.
pub(crate) fn apply_input_parameter_definitions(
    defs: &[crate::models::WorkflowInputParameterDef],
    input: &Value,
) -> Result<Value, EngineError> {
    if defs.is_empty() {
        return Ok(input.clone());
    }

    // Normalise: if caller passed null / non-object, treat as empty object so
    // that we can apply defaults safely without rejecting the request.
    let mut obj = match input {
        Value::Object(map) => map.clone(),
        Value::Null => serde_json::Map::new(),
        other => {
            return Err(EngineError::InvalidInput(format!(
                "workflow input must be a JSON object, got {}",
                match other {
                    Value::Bool(_) => "boolean",
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Array(_) => "array",
                    _ => "value",
                }
            )));
        }
    };

    let mut missing: Vec<String> = Vec::new();
    for def in defs {
        if obj.contains_key(&def.name) {
            continue;
        }
        if let Some(default) = &def.default_value {
            obj.insert(def.name.clone(), default.clone());
        } else if def.required {
            missing.push(def.name.clone());
        }
    }

    if !missing.is_empty() {
        return Err(EngineError::InvalidInput(format!(
            "missing required workflow input parameter(s): {}",
            missing.join(", ")
        )));
    }

    Ok(Value::Object(obj))
}

#[cfg(test)]
mod input_param_tests {
    use super::*;
    use crate::models::WorkflowInputParameterDef;
    use serde_json::json;

    fn def(name: &str, required: bool, default: Option<Value>) -> WorkflowInputParameterDef {
        WorkflowInputParameterDef {
            name: name.to_string(),
            description: None,
            param_type: None,
            required,
            default_value: default,
            example: None,
        }
    }

    #[test]
    fn empty_defs_returns_input_unchanged() {
        let input = json!({"a": 1});
        let out = apply_input_parameter_definitions(&[], &input).unwrap();
        assert_eq!(out, input);
    }

    #[test]
    fn applies_default_for_missing() {
        let defs = vec![def("retries", false, Some(json!(3)))];
        let out = apply_input_parameter_definitions(&defs, &json!({})).unwrap();
        assert_eq!(out, json!({"retries": 3}));
    }

    #[test]
    fn caller_value_overrides_default() {
        let defs = vec![def("retries", false, Some(json!(3)))];
        let out = apply_input_parameter_definitions(&defs, &json!({"retries": 7})).unwrap();
        assert_eq!(out, json!({"retries": 7}));
    }

    #[test]
    fn missing_required_returns_invalid_input() {
        let defs = vec![def("user_id", true, None)];
        let err = apply_input_parameter_definitions(&defs, &json!({})).unwrap_err();
        match err {
            EngineError::InvalidInput(msg) => assert!(msg.contains("user_id")),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn null_input_treated_as_empty_object() {
        let defs = vec![def("x", false, Some(json!("y")))];
        let out = apply_input_parameter_definitions(&defs, &Value::Null).unwrap();
        assert_eq!(out, json!({"x": "y"}));
    }

    #[test]
    fn non_object_input_rejected() {
        let defs = vec![def("x", false, None)];
        let err = apply_input_parameter_definitions(&defs, &json!([1, 2])).unwrap_err();
        assert!(matches!(err, EngineError::InvalidInput(_)));
    }
}

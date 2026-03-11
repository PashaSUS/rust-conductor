use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;

use super::error::EngineError;
use super::{is_task_failed, is_task_terminal, WorkflowEngine};
use crate::models::*;

impl WorkflowEngine {
    pub(crate) async fn advance_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        self.advance_workflow_inner(workflow_id).await
    }

    async fn advance_workflow_inner(&self, workflow_id: &str) -> Result<(), EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        if wf.status != WorkflowStatus::Running {
            return Ok(());
        }
        let Some(def) = &wf.workflow_definition else {
            return Ok(());
        };

        let task_map: HashMap<String, &TaskResult> = wf
            .tasks
            .iter()
            .map(|t| (t.reference_task_name.clone(), t))
            .collect();

        let next_seq = wf.tasks.iter().map(|t| t.seq).max().unwrap_or(-1) + 1;

        let all_done = self
            .evaluate_and_schedule(workflow_id, &def.tasks, &task_map, &wf.input, next_seq)
            .await?;

        if all_done {
            let db = self.shards.shard_for(workflow_id);

            let wf_status: String = sqlx::query_scalar(
                "SELECT status FROM workflow WHERE workflow_id = $1",
            )
            .bind(workflow_id)
            .fetch_one(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            if wf_status != "RUNNING" {
                return Ok(());
            }

            let output: Value = sqlx::query_scalar(
                "SELECT COALESCE(output_data, '{}') FROM task WHERE workflow_instance_id = $1 ORDER BY seq DESC LIMIT 1",
            )
            .bind(workflow_id)
            .fetch_optional(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?
            .unwrap_or(Value::Object(Default::default()));

            let now = Utc::now();
            sqlx::query(
                "UPDATE workflow SET status = 'COMPLETED', end_time = $2, update_time = $2, output = $3 WHERE workflow_id = $1",
            )
            .bind(workflow_id)
            .bind(now)
            .bind(&output)
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            tracing::info!(workflow_id = %workflow_id, "Workflow completed");

            self.try_complete_parent_sub_workflow(workflow_id, &output)
                .await?;
        }

        Ok(())
    }

    /// Recursively walk a sequence of definition tasks and schedule whatever
    /// comes next. Returns `true` when every task in the sequence is terminal.
    async fn evaluate_and_schedule(
        &self,
        workflow_id: &str,
        def_tasks: &[WorkflowTask],
        task_map: &HashMap<String, &TaskResult>,
        input: &Value,
        mut seq: i32,
    ) -> Result<bool, EngineError> {
        for (idx, task_def) in def_tasks.iter().enumerate() {
            let ref_name = &task_def.task_reference_name;

            match task_map.get(ref_name.as_str()) {
                Some(task) if is_task_terminal(&task.status) => {
                    if is_task_failed(&task.status) && !task_def.optional {
                        self.fail_workflow(
                            workflow_id,
                            Some(&format!("Task {} failed: {}", ref_name, task.reason_for_incompletion.as_deref().unwrap_or("unknown"))),
                        )
                        .await?;
                        return Ok(true);
                    }

                    match task_def.task_type.as_str() {
                        "FORK_JOIN" | "FORK" => {
                            for branch in &task_def.fork_tasks {
                                let branch_done = Box::pin(self.evaluate_and_schedule(
                                    workflow_id, branch, task_map, input, seq,
                                ))
                                .await?;

                                if !branch_done {
                                    return Ok(false);
                                }
                            }
                            continue;
                        }
                        "DECISION" | "SWITCH" => {
                            let selected = task
                                .output_data
                                .get("selectedBranch")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let branch = task_def
                                .decision_cases
                                .get(selected)
                                .unwrap_or(&task_def.default_case);
                            let branch_done = Box::pin(self.evaluate_and_schedule(
                                workflow_id, branch, task_map, input, seq,
                            ))
                            .await?;
                            if !branch_done {
                                return Ok(false);
                            }
                            continue;
                        }
                        _ => continue,
                    }
                }

                Some(task) => {
                    match task_def.task_type.as_str() {
                        "JOIN" => {
                            if self.check_join_prerequisites(workflow_id, task_def).await? {
                                self.complete_task_by_id(workflow_id, &task.task_id).await?;
                                continue;
                            }
                        }
                        "SUB_WORKFLOW" => {
                            if let Some(sub_id) = &task.sub_workflow_id {
                                if let Ok(sub_wf) = self.get_workflow(sub_id).await {
                                    match sub_wf.status {
                                        WorkflowStatus::Completed => {
                                            self.complete_sub_workflow_task(
                                                workflow_id,
                                                &task.task_id,
                                                &sub_wf.output,
                                            )
                                            .await?;
                                            continue;
                                        }
                                        WorkflowStatus::Failed
                                        | WorkflowStatus::Terminated
                                        | WorkflowStatus::TimedOut => {
                                            self.fail_task(
                                                workflow_id,
                                                &task.task_id,
                                                "Sub-workflow ended with non-success status",
                                            )
                                            .await?;
                                            return Ok(false);
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    return Ok(false);
                }

                None => {
                    seq += 1;
                    self.schedule_tasks(
                        workflow_id,
                        &def_tasks[idx..],
                        input,
                        seq,
                    )
                    .await?;

                    match task_def.task_type.as_str() {
                        "FORK_JOIN" | "FORK" | "DECISION" | "SWITCH" => {
                            return Ok(false);
                        }
                        _ => return Ok(false),
                    }
                }
            }
        }

        Ok(true)
    }

    /// Complete a SUB_WORKFLOW task with the child workflow's output.
    pub(crate) async fn complete_sub_workflow_task(
        &self,
        workflow_id: &str,
        task_id: &str,
        output: &Value,
    ) -> Result<(), EngineError> {
        let now = Utc::now();
        let db = self.shards.shard_for(workflow_id);
        sqlx::query(
            "UPDATE task SET status = 'COMPLETED', output_data = $2, end_time = $3, update_time = $3 WHERE task_id = $1",
        )
        .bind(task_id)
        .bind(output)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }

    /// Mark a task as FAILED.
    pub(crate) async fn fail_task(&self, workflow_id: &str, task_id: &str, reason: &str) -> Result<(), EngineError> {
        let now = Utc::now();
        let db = self.shards.shard_for(workflow_id);
        sqlx::query(
            "UPDATE task SET status = 'FAILED', reason_for_incompletion = $2, end_time = $3, update_time = $3 WHERE task_id = $1",
        )
        .bind(task_id)
        .bind(reason)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }

    /// When a child workflow completes, find and complete the parent's
    /// SUB_WORKFLOW task so the parent can advance.
    pub(crate) async fn try_complete_parent_sub_workflow(
        &self,
        child_workflow_id: &str,
        child_output: &Value,
    ) -> Result<(), EngineError> {
        // Use the child workflow's parent_workflow_id to route directly
        let child_db = self.shards.shard_for(child_workflow_id);
        let parent_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT parent_workflow_id, parent_workflow_task_id FROM workflow WHERE workflow_id = $1",
        )
        .bind(child_workflow_id)
        .fetch_optional(child_db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if let Some((Some(parent_wf_id), Some(parent_task_id))) = parent_info {
            self.complete_sub_workflow_task(&parent_wf_id, &parent_task_id, child_output)
                .await?;
            Box::pin(self.advance_workflow(&parent_wf_id)).await?;
        }
        Ok(())
    }

    pub(crate) async fn fail_workflow(&self, workflow_id: &str, reason: Option<&str>) -> Result<(), EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let now = Utc::now();

        sqlx::query(
            "UPDATE workflow SET status = 'FAILED', end_time = $2, update_time = $2, reason_for_incompletion = $3 WHERE workflow_id = $1 AND status = 'RUNNING'",
        )
        .bind(workflow_id)
        .bind(now)
        .bind(reason)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        sqlx::query(
            "UPDATE task SET status = 'CANCELED', end_time = $2, update_time = $2 WHERE workflow_instance_id = $1 AND status IN ('SCHEDULED', 'IN_PROGRESS')",
        )
        .bind(workflow_id)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        // If this is a child of a SUB_WORKFLOW, propagate failure to parent
        let parent_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT parent_workflow_id, parent_workflow_task_id FROM workflow WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if let Some((Some(parent_wf_id), Some(parent_task_id))) = parent_info {
            self.fail_task(&parent_wf_id, &parent_task_id, reason.unwrap_or("Sub-workflow failed")).await?;
            Box::pin(self.fail_workflow(&parent_wf_id, reason)).await?;
        }

        self.trigger_failure_workflow(workflow_id, reason).await?;

        tracing::warn!(workflow_id = %workflow_id, "Workflow failed");
        Ok(())
    }

    /// If the workflow definition specifies a `failure_workflow`, start it.
    async fn trigger_failure_workflow(
        &self,
        failed_workflow_id: &str,
        reason: Option<&str>,
    ) -> Result<(), EngineError> {
        let db = self.shards.shard_for(failed_workflow_id);

        let def_json: Option<Value> = sqlx::query_scalar(
            "SELECT workflow_def FROM workflow WHERE workflow_id = $1",
        )
        .bind(failed_workflow_id)
        .fetch_optional(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?
        .flatten();

        let Some(def_json) = def_json else { return Ok(()) };
        let def: WorkflowDef = match serde_json::from_value(def_json) {
            Ok(d) => d,
            Err(_) => return Ok(()),
        };

        let Some(failure_wf_name) = &def.failure_workflow else { return Ok(()) };
        if failure_wf_name.is_empty() {
            return Ok(());
        }

        let (wf_input, wf_tasks): (Value, Value) = {
            let wf = self.get_workflow(failed_workflow_id).await?;
            let tasks_json = serde_json::to_value(&wf.tasks).unwrap_or(Value::Array(vec![]));
            (wf.input, tasks_json)
        };

        let compensation_input = serde_json::json!({
            "failedWorkflowId": failed_workflow_id,
            "failureReason": reason.unwrap_or(""),
            "failedWorkflowInput": wf_input,
            "failedWorkflowTasks": wf_tasks,
        });

        let req = StartWorkflowRequest {
            name: failure_wf_name.clone(),
            version: 1,
            input: compensation_input,
            correlation_id: Some(failed_workflow_id.to_string()),
            priority: 0,
            task_to_domain: Default::default(),
            workflow_def: None,
            external_input_payload_storage_path: None,
            created_by: Some("system:failure-handler".to_string()),
            idempotency_key: None,
            idempotency_strategy: None,
        };

        match self.start_workflow(&req).await {
            Ok(comp_id) => {
                tracing::info!(
                    failed_id = %failed_workflow_id,
                    compensation_id = %comp_id,
                    failure_workflow = %failure_wf_name,
                    "Failure/compensation workflow started"
                );
            }
            Err(e) => {
                tracing::error!(
                    failed_id = %failed_workflow_id,
                    failure_workflow = %failure_wf_name,
                    error = %e,
                    "Failed to start failure/compensation workflow"
                );
            }
        }

        Ok(())
    }
}

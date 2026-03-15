use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::error::EngineError;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    pub async fn start_workflow(&self, req: &StartWorkflowRequest) -> Result<String, EngineError> {
        let def = self.get_workflow_def(&req.name, Some(req.version)).await?;
        let workflow_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let db = self.shards.shard_for(&workflow_id);

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def)
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8)",
        )
        .bind(&workflow_id)
        .bind(&req.name)
        .bind(req.version)
        .bind(&req.input)
        .bind(&req.correlation_id)
        .bind(now)
        .bind(req.priority)
        .bind(serde_json::to_value(&def).ok())
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, name = %req.name, error = %e, "Failed to INSERT workflow row");
            EngineError::Database(e.to_string())
        })?;

        if let Err(e) = Box::pin(self.schedule_tasks(&workflow_id, &def.tasks, &req.input, 0)).await {
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
            _ => {
                if task_type != "SIMPLE" {
                    tracing::error!(
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

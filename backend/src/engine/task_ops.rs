use chrono::Utc;
use futures::future::join_all;
use serde_json::Value;

use super::error::EngineError;
use super::rows::{TaskLogRow, TaskRow, TaskSummaryRow};
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    pub async fn get_task(&self, task_id: &str) -> Result<TaskResult, EngineError> {
        if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await? {
            if let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
                .bind(task_id)
                .fetch_optional(db)
                .await
                .map_err(|e| {
                    tracing::error!(task_id = %task_id, error = %e, "DB error fetching task");
                    EngineError::Database(e.to_string())
                })?
            {
                return Ok(r.into());
            }
        }
        tracing::error!(task_id = %task_id, "Task not found on any shard");
        Err(EngineError::NotFound(format!("Task not found: {task_id}")))
    }

    pub async fn poll_task(&self, task_type: &str, worker_id: Option<&str>) -> Result<Option<PollTask>, EngineError> {
        let task_id = match self.kafka.dequeue(task_type).await {
            Some(id) => id,
            None => return Ok(None),
        };

        // Resolve shard via routing hash (O(1) Redis lookup, no fan-out)
        let result = async {
            if let Some((wf_id, db)) = self.resolve_task_shard(&task_id).await? {
                if let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
                    .bind(&task_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|e| {
                        tracing::error!(task_id = %task_id, error = %e, "DB error fetching task during poll");
                        EngineError::Database(e.to_string())
                    })?
                {
                    let db = self.shards.shard_for(&wf_id);
                    let now = Utc::now();
                    sqlx::query(
                        "UPDATE task SET status = 'IN_PROGRESS', start_time = $2, update_time = $2, poll_count = poll_count + 1, worker_id = $3 WHERE task_id = $1 AND status = 'SCHEDULED'",
                    )
                    .bind(&task_id)
                    .bind(now)
                    .bind(worker_id)
                    .execute(db)
                    .await
                    .map_err(|e| {
                        tracing::error!(task_id = %task_id, workflow_id = %wf_id, error = %e, "DB error transitioning polled task to IN_PROGRESS");
                        EngineError::Database(e.to_string())
                    })?;

                    return Ok(Some(PollTask {
                        task_id: r.task_id,
                        workflow_instance_id: r.workflow_instance_id,
                        task_type: r.task_type,
                        task_def_name: r.task_def_name,
                        reference_task_name: r.reference_task_name,
                        status: TaskStatus::InProgress,
                        input_data: r.input_data,
                        scheduled_time: Some(r.scheduled_time.timestamp_millis()),
                        start_time: Some(now.timestamp_millis()),
                        callback_after_seconds: r.callback_after_seconds,
                        poll_count: r.poll_count,
                        retry_count: r.retry_count,
                    }));
                }
            }
            Ok::<Option<PollTask>, EngineError>(None)
        }
        .await;

        match result {
            Ok(poll_task) => Ok(poll_task),
            Err(e) => {
                // DB lookup failed after dequeue — re-enqueue the task via Kafka so it is not lost
                tracing::warn!(task_id = %task_id, error = %e, "Re-enqueuing task after poll DB failure");
                let _ = self.kafka.enqueue(task_type, &task_id).await;
                Err(e)
            }
        }
    }

    pub async fn update_task(&self, update: &TaskUpdateRequest) -> Result<String, EngineError> {
        let now = Utc::now();
        let status_str = update.status.to_string();
        let is_terminal = matches!(
            update.status,
            TaskStatus::Completed
                | TaskStatus::CompletedWithErrors
                | TaskStatus::Failed
                | TaskStatus::FailedWithTerminalError
                | TaskStatus::TimedOut
        );

        if is_terminal && matches!(update.status, TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut) {
            tracing::error!(
                task_id = %update.task_id,
                workflow_id = %update.workflow_instance_id,
                status = %status_str,
                reason = ?update.reason_for_incompletion,
                worker_id = ?update.worker_id,
                "Task update with error status"
            );
        }

        let db = self.shards.shard_for(&update.workflow_instance_id);

        sqlx::query(
            "UPDATE task SET status = $2, output_data = $3, update_time = $4, end_time = CASE WHEN $5 THEN $4 ELSE end_time END, worker_id = COALESCE($6, worker_id), reason_for_incompletion = $7 WHERE task_id = $1",
        )
        .bind(&update.task_id)
        .bind(&status_str)
        .bind(&update.output_data)
        .bind(now)
        .bind(is_terminal)
        .bind(&update.worker_id)
        .bind(&update.reason_for_incompletion)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(task_id = %update.task_id, workflow_id = %update.workflow_instance_id, error = %e, "DB error updating task status");
            EngineError::Database(e.to_string())
        })?;

        // Clean up routing entry when task reaches terminal status
        if is_terminal {
            let _ = self.delete_task_routing(&update.task_id).await;
        }

        if update.status == TaskStatus::Completed
            || update.status == TaskStatus::CompletedWithErrors
        {
            self.advance_workflow(&update.workflow_instance_id).await?;
        } else if update.status == TaskStatus::Failed
            || update.status == TaskStatus::FailedWithTerminalError
            || update.status == TaskStatus::TimedOut
        {
            let wf_status = if update.status == TaskStatus::TimedOut {
                "TIMED_OUT"
            } else {
                "FAILED"
            };
            self.fail_workflow(&update.workflow_instance_id, update.reason_for_incompletion.as_deref(), wf_status)
                .await?;
        }

        Ok(update.task_id.clone())
    }

    pub async fn ack_task(&self, task_id: &str, worker_id: Option<&str>) -> Result<bool, EngineError> {
        if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await? {
            let r = sqlx::query(
                "UPDATE task SET worker_id = COALESCE($2, worker_id), update_time = NOW() WHERE task_id = $1 AND status = 'IN_PROGRESS'",
            )
            .bind(task_id)
            .bind(worker_id)
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;
            return Ok(r.rows_affected() > 0);
        }
        Ok(false)
    }

    pub async fn batch_poll_tasks(
        &self,
        task_type: &str,
        worker_id: Option<&str>,
        count: usize,
        _timeout: u64,
    ) -> Result<Vec<PollTask>, EngineError> {
        if count == 0 {
            return Ok(vec![]);
        }

        let task_ids = self
            .kafka
            .batch_dequeue(task_type, count, std::time::Duration::from_millis(500))
            .await;

        if task_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut tasks = Vec::with_capacity(task_ids.len());
        for task_id in task_ids {
            if let Some((_wf_id, db)) = self.resolve_task_shard(&task_id).await? {
                if let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
                    .bind(&task_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?
                {
                    let db = self.shards.shard_for(&r.workflow_instance_id);
                    let now = Utc::now();
                    sqlx::query(
                        "UPDATE task SET status = 'IN_PROGRESS', start_time = $2, update_time = $2, poll_count = poll_count + 1, worker_id = $3 WHERE task_id = $1 AND status = 'SCHEDULED'",
                    )
                    .bind(&task_id)
                    .bind(now)
                    .bind(worker_id)
                    .execute(db)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;

                    tasks.push(PollTask {
                        task_id: r.task_id,
                        workflow_instance_id: r.workflow_instance_id,
                        task_type: r.task_type,
                        task_def_name: r.task_def_name,
                        reference_task_name: r.reference_task_name,
                        status: TaskStatus::InProgress,
                        input_data: r.input_data,
                        scheduled_time: Some(r.scheduled_time.timestamp_millis()),
                        start_time: Some(now.timestamp_millis()),
                        callback_after_seconds: r.callback_after_seconds,
                        poll_count: r.poll_count,
                        retry_count: r.retry_count,
                    });
                }
            }
        }
        Ok(tasks)
    }

    pub async fn get_task_logs(&self, task_id: &str) -> Result<Vec<TaskExecLog>, EngineError> {
        if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await? {
            let rows = sqlx::query_as::<_, TaskLogRow>(
                "SELECT log_message, task_id, created_time FROM task_log WHERE task_id = $1 ORDER BY created_time",
            )
            .bind(task_id)
            .fetch_all(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;
            return Ok(rows
                .into_iter()
                .map(|r| TaskExecLog {
                    log: r.log_message,
                    task_id: r.task_id,
                    created_time: Some(r.created_time.timestamp_millis()),
                })
                .collect());
        }
        Ok(vec![])
    }

    pub async fn add_task_log(&self, task_id: &str, log: &str) -> Result<(), EngineError> {
        if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await? {
            sqlx::query("INSERT INTO task_log (task_id, log_message) VALUES ($1, $2)")
                .bind(task_id)
                .bind(log)
                .execute(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
            return Ok(());
        }
        Err(EngineError::NotFound(format!("Task not found: {task_id}")))
    }

    pub async fn search_tasks(
        &self,
        workflow_id: Option<&str>,
        task_type: Option<&str>,
        status: Option<&str>,
        free_text: Option<&str>,
        start: i64,
        size: i64,
    ) -> Result<SearchResult<TaskSummary>, EngineError> {
        let mut where_clause = String::from(" WHERE 1=1");

        if let Some(wid) = workflow_id {
            where_clause.push_str(&format!(" AND workflow_instance_id = '{}'", wid.replace('\'', "")));
        }
        if let Some(tt) = task_type {
            where_clause.push_str(&format!(" AND task_type = '{}'", tt.replace('\'', "")));
        }
        if let Some(s) = status {
            where_clause.push_str(&format!(" AND status = '{}'", s.replace('\'', "")));
        }
        if let Some(ft) = free_text {
            let safe = ft.replace('\'', "");
            where_clause.push_str(&format!(
                " AND (task_def_name ILIKE '%{safe}%' OR reference_task_name ILIKE '%{safe}%' OR task_id ILIKE '%{safe}%')"
            ));
        }

        // If we know the workflow_id, route to the specific shard
        if let Some(wid) = workflow_id {
            let db = self.shards.shard_for(wid);
            return self.search_tasks_on_shard(db, &where_clause, start, size).await;
        }

        // Parallel fan-out across all shards
        let fetch_size = size + start;
        let count_query = format!("SELECT COUNT(*) FROM task{where_clause}");
        let data_query = format!(
            "SELECT task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status, scheduled_time, start_time, end_time, update_time FROM task{where_clause} ORDER BY scheduled_time DESC LIMIT {fetch_size} OFFSET 0"
        );

        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            let cq = count_query.clone();
            let dq = data_query.clone();
            async move {
                let count: i64 = sqlx::query_scalar(&cq)
                    .fetch_one(shard)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
                let rows = sqlx::query_as::<_, TaskSummaryRow>(&dq)
                    .fetch_all(shard)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>((count, rows))
            }
        }).collect();

        let results = join_all(futs).await;
        let mut total: i64 = 0;
        let mut all_results = Vec::new();
        for result in results {
            let (count, rows) = result?;
            total += count;
            all_results.extend(rows);
        }

        // Sort merged results and apply pagination
        all_results.sort_by(|a, b| b.scheduled_time.cmp(&a.scheduled_time));
        let paged: Vec<TaskSummary> = all_results
            .into_iter()
            .skip(start as usize)
            .take(size as usize)
            .map(|r| self.task_summary_from_row(r))
            .collect();

        Ok(SearchResult {
            total_hits: total,
            results: paged,
        })
    }

    async fn search_tasks_on_shard(
        &self,
        db: &crate::store::postgres::DbPool,
        where_clause: &str,
        start: i64,
        size: i64,
    ) -> Result<SearchResult<TaskSummary>, EngineError> {
        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM task{where_clause}"))
            .fetch_one(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        let rows = sqlx::query_as::<_, TaskSummaryRow>(
            &format!(
                "SELECT task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status, scheduled_time, start_time, end_time, update_time FROM task{where_clause} ORDER BY scheduled_time DESC LIMIT {size} OFFSET {start}"
            ),
        )
        .fetch_all(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        Ok(SearchResult {
            total_hits: total,
            results: rows.into_iter().map(|r| self.task_summary_from_row(r)).collect(),
        })
    }

    fn task_summary_from_row(&self, r: TaskSummaryRow) -> TaskSummary {
        TaskSummary {
            task_id: Some(r.task_id),
            workflow_id: Some(r.workflow_instance_id),
            task_type: Some(r.task_type),
            task_def_name: Some(r.task_def_name),
            status: serde_json::from_value(Value::String(r.status)).ok(),
            scheduled_time: Some(r.scheduled_time.timestamp_millis().to_string()),
            start_time: r.start_time.map(|t| t.timestamp_millis().to_string()),
            end_time: r.end_time.map(|t| t.timestamp_millis().to_string()),
            update_time: Some(r.update_time.timestamp_millis().to_string()),
            input: None,
            output: None,
            reason_for_incompletion: None,
            correlation_id: None,
            execution_time: None,
            queue_wait_time: None,
            domain: None,
            workflow_type: None,
            workflow_priority: None,
            external_input_payload_storage_path: None,
            external_output_payload_storage_path: None,
        }
    }

    pub async fn get_queue_sizes(&self) -> Result<std::collections::HashMap<String, i64>, EngineError> {
        // Query all shards for SCHEDULED task counts grouped by task type
        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            async move {
                let rows: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT task_def_name, COUNT(*) FROM task WHERE status = 'SCHEDULED' GROUP BY task_def_name",
                )
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>(rows)
            }
        }).collect();

        let results = join_all(futs).await;
        let mut sizes = std::collections::HashMap::new();
        for result in results {
            for (name, count) in result? {
                *sizes.entry(name).or_insert(0i64) += count;
            }
        }
        Ok(sizes)
    }
}

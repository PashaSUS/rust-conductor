use chrono::Utc;
use futures::future::join_all;
use serde_json::Value;

use super::WorkflowEngine;
use super::error::EngineError;
use super::rows::{TaskLogRow, TaskRow, TaskSummaryRow};
use crate::models::*;

impl WorkflowEngine {
    pub async fn get_task(&self, task_id: &str) -> Result<TaskResult, EngineError> {
        if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await?
            && let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
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
        tracing::error!(task_id = %task_id, "Task not found on any shard");
        Err(EngineError::NotFound(format!("Task not found: {task_id}")))
    }

    pub async fn poll_task(
        &self,
        task_type: &str,
        worker_id: Option<&str>,
    ) -> Result<Option<PollTask>, EngineError> {
        crate::metrics::record_task_poll(task_type);
        let task_id = match self.queue.dequeue(task_type).await {
            Some(id) => id,
            None => return Ok(None),
        };

        // Resolve shard via routing hash (O(1) Redis lookup, no fan-out)
        let result = async {
            if let Some((wf_id, db)) = self.resolve_task_shard(&task_id).await?
                && let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
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
                        priority: r.priority,
                        env_vars: r.env_vars,
                    }));
                }
            Ok::<Option<PollTask>, EngineError>(None)
        }
        .await;

        match result {
            Ok(poll_task) => Ok(poll_task),
            Err(e) => {
                // DB lookup failed after dequeue — re-enqueue the task via Kafka so it is not lost
                tracing::warn!(task_id = %task_id, error = %e, "Re-enqueuing task after poll DB failure");
                let _ = self.queue.enqueue(task_type, &task_id).await;
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

        if is_terminal
            && matches!(
                update.status,
                TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut
            )
        {
            tracing::error!(
                task_id = %update.task_id,
                workflow_id = %update.workflow_instance_id,
                status = %status_str,
                reason = ?update.reason_for_incompletion,
                worker_id = ?update.worker_id,
                "Task update with error status"
            );
        }

        // Validate output data against task definition's output schema
        if matches!(
            update.status,
            TaskStatus::Completed | TaskStatus::CompletedWithErrors
        ) && let Ok(task) = self.get_task(&update.task_id).await
            && let Ok(task_def) = self.get_task_def(&task.task_def_name).await
            && task_def.enforce_schema
            && let Some(schema) = &task_def.output_schema
            && let Some(expected_keys) = &schema.data
        {
            let output = &update.output_data;
            for key in expected_keys.keys() {
                if output.get(key).is_none() {
                    tracing::error!(
                        task_id = %update.task_id,
                        missing_key = %key,
                        "Output schema validation failed: missing required key"
                    );
                    return Err(EngineError::InvalidState(format!(
                        "Output schema validation failed: missing key '{key}'"
                    )));
                }
            }
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
            // Conditional retry: if the task_def specifies retry_on_errors,
            // only retry when the failure reason matches one of those patterns.
            // FailedWithTerminalError is never retried.
            if update.status == TaskStatus::Failed
                && let Ok(task) = self.get_task(&update.task_id).await
                && let Ok(task_def) = self.get_task_def(&task.task_def_name).await
                && !task_def.retry_on_errors.is_empty()
            {
                let reason = update.reason_for_incompletion.as_deref().unwrap_or("");
                let matches = task_def
                    .retry_on_errors
                    .iter()
                    .any(|pattern| reason.contains(pattern.as_str()));
                if !matches {
                    tracing::info!(
                        task_id = %update.task_id,
                        reason = %reason,
                        patterns = ?task_def.retry_on_errors,
                        "Task failure does not match retry_on_errors patterns, skipping retry"
                    );
                    self.fail_workflow(
                        &update.workflow_instance_id,
                        update.reason_for_incompletion.as_deref(),
                        "FAILED",
                    )
                    .await?;
                    return Ok(update.task_id.clone());
                }
            }

            let wf_status = if update.status == TaskStatus::TimedOut {
                "TIMED_OUT"
            } else {
                "FAILED"
            };
            self.fail_workflow(
                &update.workflow_instance_id,
                update.reason_for_incompletion.as_deref(),
                wf_status,
            )
            .await?;
        }

        Ok(update.task_id.clone())
    }

    pub async fn ack_task(
        &self,
        task_id: &str,
        worker_id: Option<&str>,
    ) -> Result<bool, EngineError> {
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
            .queue
            .batch_dequeue(task_type, count, std::time::Duration::from_millis(500))
            .await;

        if task_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut tasks = Vec::with_capacity(task_ids.len());
        for task_id in &task_ids {
            let result = async {
                if let Some((_wf_id, db)) = self.resolve_task_shard(task_id).await?
                    && let Some(r) = sqlx::query_as::<_, TaskRow>("SELECT * FROM task WHERE task_id = $1")
                        .bind(task_id)
                        .fetch_optional(db)
                        .await
                        .map_err(|e| EngineError::Database(e.to_string()))?
                    {
                        let db = self.shards.shard_for(&r.workflow_instance_id);
                        let now = Utc::now();
                        sqlx::query(
                            "UPDATE task SET status = 'IN_PROGRESS', start_time = $2, update_time = $2, poll_count = poll_count + 1, worker_id = $3 WHERE task_id = $1 AND status = 'SCHEDULED'",
                        )
                        .bind(task_id)
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
                        priority: r.priority,
                        env_vars: r.env_vars,
                    });
                }
                Ok::<(), EngineError>(())
            }
            .await;

            if let Err(e) = result {
                // Re-enqueue on failure so the task is not lost from Kafka
                tracing::warn!(task_id = %task_id, error = %e, "Re-enqueuing task after batch poll DB failure");
                let _ = self.queue.enqueue(task_type, task_id).await;
            }
        }
        // Sort by priority descending so higher priority tasks are returned first
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
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
            where_clause.push_str(&format!(
                " AND workflow_instance_id = '{}'",
                wid.replace('\'', "")
            ));
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

        // If we know the workflow_id, route to the specific read shard
        if let Some(wid) = workflow_id {
            let db = self.shards.read_shard_for(wid);
            return self
                .search_tasks_on_shard(db, &where_clause, start, size)
                .await;
        }

        // Parallel fan-out across all read shards
        let fetch_size = size + start;
        let count_query = format!("SELECT COUNT(*) FROM task{where_clause}");
        let data_query = format!(
            "SELECT task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status, scheduled_time, start_time, end_time, update_time FROM task{where_clause} ORDER BY scheduled_time DESC LIMIT {fetch_size} OFFSET 0"
        );

        let futs: Vec<_> = self
            .shards
            .read_shards()
            .iter()
            .map(|shard| {
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
            })
            .collect();

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
            next_cursor: None,
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
            results: rows
                .into_iter()
                .map(|r| self.task_summary_from_row(r))
                .collect(),
            next_cursor: None,
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

    pub async fn get_queue_sizes(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>, EngineError> {
        // Query all read shards for SCHEDULED task counts grouped by task type
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
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

    // ── Update task by reference name ──

    pub async fn update_task_by_ref_name(
        &self,
        workflow_id: &str,
        task_ref_name: &str,
        status: &str,
        output: &Value,
    ) -> Result<String, EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let now = Utc::now();

        let task_id: Option<String> = sqlx::query_scalar(
            "SELECT task_id FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 ORDER BY seq DESC LIMIT 1",
        )
        .bind(workflow_id)
        .bind(task_ref_name)
        .fetch_optional(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let task_id = task_id.ok_or_else(|| {
            EngineError::NotFound(format!(
                "Task with ref name {task_ref_name} not found in workflow {workflow_id}"
            ))
        })?;

        let parsed_status: TaskStatus =
            serde_json::from_value(Value::String(status.to_string()))
                .map_err(|_| EngineError::InvalidState(format!("Invalid task status: {status}")))?;

        let is_terminal = super::is_task_terminal(&parsed_status);

        sqlx::query(
            "UPDATE task SET status = $2, output_data = $3, update_time = $4, end_time = CASE WHEN $5 THEN $4 ELSE end_time END WHERE task_id = $1",
        )
        .bind(&task_id)
        .bind(status)
        .bind(output)
        .bind(now)
        .bind(is_terminal)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if is_terminal {
            let _ = self.delete_task_routing(&task_id).await;
        }

        if parsed_status == TaskStatus::Completed
            || parsed_status == TaskStatus::CompletedWithErrors
        {
            self.advance_workflow(workflow_id).await?;
        } else if super::is_task_failed(&parsed_status) {
            let wf_status = if parsed_status == TaskStatus::TimedOut {
                "TIMED_OUT"
            } else {
                "FAILED"
            };
            self.fail_workflow(workflow_id, None, wf_status).await?;
        }

        Ok(task_id)
    }

    // ── In-progress tasks ──

    pub async fn get_in_progress_tasks(
        &self,
        task_type: &str,
    ) -> Result<Vec<TaskResult>, EngineError> {
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
            async move {
                sqlx::query_as::<_, TaskRow>(
                    "SELECT * FROM task WHERE task_def_name = $1 AND status = 'IN_PROGRESS' ORDER BY start_time DESC",
                )
                .bind(task_type)
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))
            }
        }).collect();

        let results = join_all(futs).await;
        let mut all_tasks = Vec::new();
        for result in results {
            all_tasks.extend(result?.into_iter().map(|r| r.into()));
        }
        Ok(all_tasks)
    }

    pub async fn get_in_progress_task_for_workflow(
        &self,
        workflow_id: &str,
        task_ref_name: &str,
    ) -> Result<Option<TaskResult>, EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let row = sqlx::query_as::<_, TaskRow>(
            "SELECT * FROM task WHERE workflow_instance_id = $1 AND reference_task_name = $2 AND status = 'IN_PROGRESS' LIMIT 1",
        )
        .bind(workflow_id)
        .bind(task_ref_name)
        .fetch_optional(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        Ok(row.map(|r| r.into()))
    }

    // ── Queue details ──

    pub async fn get_all_queue_details(
        &self,
    ) -> Result<std::collections::HashMap<String, i64>, EngineError> {
        self.get_queue_sizes().await
    }

    pub async fn get_all_queue_details_verbose(
        &self,
    ) -> Result<
        std::collections::HashMap<String, std::collections::HashMap<String, i64>>,
        EngineError,
    > {
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
            async move {
                let rows: Vec<(String, String, i64)> = sqlx::query_as(
                    "SELECT task_def_name, status, COUNT(*) FROM task WHERE status IN ('SCHEDULED', 'IN_PROGRESS') GROUP BY task_def_name, status",
                )
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>(rows)
            }
        }).collect();

        let results = join_all(futs).await;
        let mut details: std::collections::HashMap<String, std::collections::HashMap<String, i64>> =
            std::collections::HashMap::new();
        for result in results {
            for (name, status, count) in result? {
                *details.entry(name).or_default().entry(status).or_insert(0) += count;
            }
        }
        Ok(details)
    }

    pub async fn get_poll_data(&self, task_type: &str) -> Result<Vec<PollData>, EngineError> {
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
            async move {
                let rows: Vec<(Option<String>, Option<i64>)> = sqlx::query_as(
                    "SELECT worker_id, MAX(EXTRACT(EPOCH FROM update_time)::bigint * 1000) FROM task WHERE task_def_name = $1 AND status = 'IN_PROGRESS' GROUP BY worker_id",
                )
                .bind(task_type)
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>(rows)
            }
        }).collect();

        let results = join_all(futs).await;
        let mut poll_data = Vec::new();
        for result in results {
            for (worker_id, last_poll) in result? {
                poll_data.push(PollData {
                    queue_name: Some(task_type.to_string()),
                    domain: None,
                    worker_id,
                    last_poll_time: last_poll,
                });
            }
        }
        Ok(poll_data)
    }

    pub async fn get_all_poll_data(&self) -> Result<Vec<PollData>, EngineError> {
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
            async move {
                let rows: Vec<(String, Option<String>, Option<i64>)> = sqlx::query_as(
                    "SELECT task_def_name, worker_id, MAX(EXTRACT(EPOCH FROM update_time)::bigint * 1000) FROM task WHERE status = 'IN_PROGRESS' GROUP BY task_def_name, worker_id",
                )
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>(rows)
            }
        }).collect();

        let results = join_all(futs).await;
        let mut poll_data = Vec::new();
        for result in results {
            for (queue_name, worker_id, last_poll) in result? {
                poll_data.push(PollData {
                    queue_name: Some(queue_name),
                    domain: None,
                    worker_id,
                    last_poll_time: last_poll,
                });
            }
        }
        Ok(poll_data)
    }
}

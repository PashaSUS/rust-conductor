use std::collections::HashMap;

use chrono::Utc;
use futures::future::join_all;
use serde_json::Value;
use uuid::Uuid;

use super::error::EngineError;
use super::rows::{TaskRow, WorkflowRow, WorkflowSummaryRow};
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    pub async fn get_workflow(&self, workflow_id: &str) -> Result<Workflow, EngineError> {
        let db = self.shards.shard_for(workflow_id);

        let wf_row = sqlx::query_as::<_, WorkflowRow>(
            "SELECT * FROM workflow WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "DB error fetching workflow");
            EngineError::Database(e.to_string())
        })?
        .ok_or_else(|| {
            tracing::error!(workflow_id = %workflow_id, "Workflow not found");
            EngineError::NotFound(format!("Workflow not found: {workflow_id}"))
        })?;

        let tasks = sqlx::query_as::<_, TaskRow>(
            "SELECT * FROM task WHERE workflow_instance_id = $1 ORDER BY seq",
        )
        .bind(workflow_id)
        .fetch_all(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "DB error fetching workflow tasks");
            EngineError::Database(e.to_string())
        })?;

        let tasks: Vec<TaskResult> = tasks.into_iter().map(|r| r.into()).collect();

        Ok(Workflow {
            workflow_id: wf_row.workflow_id,
            workflow_name: wf_row.workflow_name,
            workflow_version: wf_row.workflow_version,
            status: serde_json::from_value(Value::String(wf_row.status))
                .unwrap_or(WorkflowStatus::Running),
            input: wf_row.input,
            output: wf_row.output,
            tasks,
            correlation_id: wf_row.correlation_id,
            start_time: wf_row.start_time.timestamp_millis(),
            end_time: wf_row.end_time.map(|t| t.timestamp_millis()),
            update_time: wf_row.update_time.timestamp_millis(),
            created_by: wf_row.created_by,
            updated_by: None,
            reason_for_incompletion: wf_row.reason_for_incompletion,
            workflow_definition: wf_row
                .workflow_def
                .and_then(|v| serde_json::from_value(v).ok()),
            priority: wf_row.priority,
            variables: wf_row
                .variables
                .as_object()
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .collect(),
            failed_reference_task_names: vec![],
            owner_app: None,
            parent_workflow_id: None,
            parent_workflow_task_id: None,
            re_run_from_workflow_id: None,
            event: None,
            task_to_domain: HashMap::new(),
            failed_task_names: vec![],
            external_input_payload_storage_path: None,
            external_output_payload_storage_path: None,
            last_retried_time: None,
            history: vec![],
            idempotency_key: None,
            rate_limit_key: None,
            rate_limited: None,
        })
    }

    pub async fn terminate_workflow(
        &self,
        workflow_id: &str,
        reason: Option<&str>,
    ) -> Result<(), EngineError> {
        let db = self.shards.shard_for(workflow_id);

        let result = sqlx::query(
            "UPDATE workflow SET status = 'TERMINATED', end_time = NOW(), update_time = NOW(), reason_for_incompletion = $2 WHERE workflow_id = $1 AND status = 'RUNNING'",
        )
        .bind(workflow_id)
        .bind(reason)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "DB error terminating workflow");
            EngineError::Database(e.to_string())
        })?;

        if result.rows_affected() == 0 {
            tracing::error!(workflow_id = %workflow_id, "Terminate called but no RUNNING workflow found");
            return Err(EngineError::NotFound(format!(
                "Running workflow not found: {workflow_id}"
            )));
        }

        sqlx::query(
            "UPDATE task SET status = 'CANCELED', end_time = NOW(), update_time = NOW() WHERE workflow_instance_id = $1 AND status IN ('SCHEDULED', 'IN_PROGRESS')",
        )
        .bind(workflow_id)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        tracing::info!(workflow_id = %workflow_id, "Workflow terminated");
        Ok(())
    }

    pub async fn pause_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let result = sqlx::query(
            "UPDATE workflow SET status = 'PAUSED', update_time = NOW() WHERE workflow_id = $1 AND status = 'RUNNING'",
        )
        .bind(workflow_id)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "Running workflow not found: {workflow_id}"
            )));
        }
        Ok(())
    }

    pub async fn resume_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let result = sqlx::query(
            "UPDATE workflow SET status = 'RUNNING', update_time = NOW() WHERE workflow_id = $1 AND status = 'PAUSED'",
        )
        .bind(workflow_id)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "Paused workflow not found: {workflow_id}"
            )));
        }
        Ok(())
    }

    pub async fn restart_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        let def = self
            .get_workflow_def(&wf.workflow_name, Some(wf.workflow_version))
            .await?;
        let db = self.shards.shard_for(workflow_id);

        sqlx::query(
            "UPDATE workflow SET status = 'RUNNING', end_time = NULL, update_time = NOW(), output = '{}', reason_for_incompletion = NULL WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        sqlx::query("DELETE FROM task WHERE workflow_instance_id = $1")
            .bind(workflow_id)
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        self.schedule_tasks(workflow_id, &def.tasks, &wf.input, 0)
            .await?;

        tracing::info!(workflow_id = %workflow_id, "Workflow restarted");
        Ok(())
    }

    pub async fn retry_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        if wf.status != WorkflowStatus::Failed {
            tracing::error!(workflow_id = %workflow_id, status = ?wf.status, "Retry called on non-FAILED workflow");
            return Err(EngineError::InvalidState(
                "Can only retry FAILED workflows".into(),
            ));
        }
        let db = self.shards.shard_for(workflow_id);

        sqlx::query(
            "UPDATE workflow SET status = 'RUNNING', end_time = NULL, update_time = NOW(), reason_for_incompletion = NULL WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let failed_tasks: Vec<(String, String)> = sqlx::query_as(
            "SELECT task_id, task_def_name FROM task WHERE workflow_instance_id = $1 AND status = 'FAILED'",
        )
        .bind(workflow_id)
        .fetch_all(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        for (task_id, task_def_name) in &failed_tasks {
            sqlx::query(
                "UPDATE task SET status = 'SCHEDULED', start_time = NULL, end_time = NULL, update_time = NOW(), retry_count = retry_count + 1 WHERE task_id = $1",
            )
            .bind(task_id)
            .execute(db)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            self.set_task_routing(task_id, workflow_id).await?;
            self.kafka
                .enqueue(task_def_name, task_id)
                .await
                .map_err(|e| EngineError::Redis(e))?;
        }

        Ok(())
    }

    pub async fn search_workflows(
        &self,
        status: Option<&str>,
        name: Option<&str>,
        free_text: Option<&str>,
        start: i64,
        size: i64,
    ) -> Result<SearchResult<WorkflowSummary>, EngineError> {
        let mut where_clause = String::from(" WHERE 1=1");
        if let Some(s) = status {
            where_clause.push_str(&format!(" AND status = '{}'", s.replace('\'', "")));
        }
        if let Some(n) = name {
            where_clause.push_str(&format!(" AND workflow_name = '{}'", n.replace('\'', "")));
        }
        if let Some(ft) = free_text {
            let safe = ft.replace('\'', "");
            where_clause.push_str(&format!(
                " AND (workflow_name ILIKE '%{safe}%' OR correlation_id ILIKE '%{safe}%' OR workflow_id ILIKE '%{safe}%')"
            ));
        }

        let select_cols = "SELECT workflow_id, workflow_name, workflow_version, status, start_time, end_time, input::text, output::text, correlation_id, priority FROM workflow";

        // Parallel fan-out across all shards
        let fetch_size = size + start;
        let count_query = format!("SELECT COUNT(*) FROM workflow{where_clause}");
        let data_query = format!("{select_cols}{where_clause} ORDER BY start_time DESC LIMIT {fetch_size} OFFSET 0");

        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            let cq = count_query.clone();
            let dq = data_query.clone();
            async move {
                let count: i64 = sqlx::query_scalar(&cq)
                    .fetch_one(shard)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
                let rows = sqlx::query_as::<_, WorkflowSummaryRow>(&dq)
                    .fetch_all(shard)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>((count, rows))
            }
        }).collect();

        let results = join_all(futs).await;
        let mut total: i64 = 0;
        let mut all_results: Vec<WorkflowSummary> = Vec::new();
        for result in results {
            let (count, rows) = result?;
            total += count;
            all_results.extend(rows.into_iter().map(|r| r.into()));
        }

        // Sort merged results and apply pagination
        all_results.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        let paged: Vec<WorkflowSummary> = all_results
            .into_iter()
            .skip(start as usize)
            .take(size as usize)
            .collect();

        Ok(SearchResult {
            total_hits: total,
            results: paged,
        })
    }

    pub async fn workflow_stats(&self) -> Result<HashMap<String, i64>, EngineError> {
        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            sqlx::query_as::<_, (String, i64)>(
                "SELECT status, COUNT(*) FROM workflow GROUP BY status",
            )
            .fetch_all(shard)
        }).collect();

        let results = join_all(futs).await;
        let mut stats: HashMap<String, i64> = HashMap::new();
        for result in results {
            let rows = result.map_err(|e| EngineError::Database(e.to_string()))?;
            for (status, count) in rows {
                *stats.entry(status).or_insert(0) += count;
            }
        }

        Ok(stats)
    }

    pub async fn decide_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        self.advance_workflow(workflow_id).await
    }

    pub async fn rerun_workflow(&self, workflow_id: &str, req: &RerunWorkflowRequest) -> Result<String, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        let new_id = req.re_run_from_workflow_id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());

        let input = match &req.workflow_input {
            Some(m) => serde_json::to_value(m).unwrap_or(wf.input.clone()),
            None => wf.input.clone(),
        };
        let def = self.get_workflow_def(&wf.workflow_name, Some(wf.workflow_version)).await?;
        let now = Utc::now();
        let db = self.shards.shard_for(&new_id);

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def)
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8)",
        )
        .bind(&new_id)
        .bind(&wf.workflow_name)
        .bind(wf.workflow_version)
        .bind(&input)
        .bind(&req.correlation_id)
        .bind(now)
        .bind(wf.priority)
        .bind(serde_json::to_value(&def).ok())
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let from_task = req.re_run_from_task_id.as_deref();
        let start_seq = if let Some(task_ref) = from_task {
            def.tasks.iter().position(|t| t.task_reference_name == task_ref).unwrap_or(0) as i32
        } else {
            0
        };

        self.schedule_tasks(&new_id, &def.tasks[start_seq as usize..], &input, start_seq).await?;

        Ok(new_id)
    }

    pub async fn skip_task(&self, workflow_id: &str, task_reference_name: &str, req: &SkipTaskRequest) -> Result<(), EngineError> {
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let output = match &req.task_output {
            Some(m) => serde_json::to_value(m).unwrap_or_else(|_| Value::Object(Default::default())),
            None => Value::Object(Default::default()),
        };
        let db = self.shards.shard_for(workflow_id);

        sqlx::query(
            "INSERT INTO task (task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status, output_data, scheduled_time, start_time, end_time, update_time, seq)
             VALUES ($1, $2, 'SKIPPED', $3, $3, 'SKIPPED', $4, $5, $5, $5, $5, 0)"
        )
        .bind(&task_id)
        .bind(workflow_id)
        .bind(task_reference_name)
        .bind(&output)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        self.advance_workflow(workflow_id).await
    }

    pub async fn get_running_workflows(
        &self,
        name: &str,
        version: Option<i32>,
        start_time: Option<i64>,
        end_time: Option<i64>,
    ) -> Result<Vec<String>, EngineError> {
        let mut base_query = String::from(
            "SELECT workflow_id FROM workflow WHERE workflow_name = $1 AND status = 'RUNNING'",
        );
        if let Some(v) = version {
            base_query.push_str(&format!(" AND workflow_version = {v}"));
        }
        if let Some(st) = start_time {
            base_query.push_str(&format!(" AND start_time >= to_timestamp({st})"));
        }
        if let Some(et) = end_time {
            base_query.push_str(&format!(" AND start_time <= to_timestamp({et})"));
        }
        base_query.push_str(" ORDER BY start_time DESC");

        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            let q = base_query.clone();
            async move {
                sqlx::query_scalar::<_, String>(&q)
                    .bind(name)
                    .fetch_all(shard)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))
            }
        }).collect();

        let results = join_all(futs).await;
        let mut all_ids = Vec::new();
        for result in results {
            all_ids.extend(result?);
        }
        Ok(all_ids)
    }

    pub async fn delete_workflow(&self, workflow_id: &str, archive: bool) -> Result<(), EngineError> {
        let db = self.shards.shard_for(workflow_id);

        if archive {
            sqlx::query("UPDATE workflow SET status = 'TERMINATED' WHERE workflow_id = $1")
                .bind(workflow_id)
                .execute(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
        } else {
            let result = sqlx::query("DELETE FROM workflow WHERE workflow_id = $1")
                .bind(workflow_id)
                .execute(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
            if result.rows_affected() == 0 {
                return Err(EngineError::NotFound(format!("Workflow not found: {workflow_id}")));
            }
        }
        Ok(())
    }

    // ── Bulk Operations ──

    pub async fn bulk_pause(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids.iter().map(|id| {
            let id = id.clone();
            async move { (id.clone(), self.pause_workflow(&id).await) }
        }).collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_resume(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids.iter().map(|id| {
            let id = id.clone();
            async move { (id.clone(), self.resume_workflow(&id).await) }
        }).collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_retry(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids.iter().map(|id| {
            let id = id.clone();
            async move { (id.clone(), self.retry_workflow(&id).await) }
        }).collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_restart(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids.iter().map(|id| {
            let id = id.clone();
            async move { (id.clone(), self.restart_workflow(&id).await) }
        }).collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_terminate(&self, workflow_ids: &[String], reason: Option<&str>) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids.iter().map(|id| {
            let id = id.clone();
            async move { (id.clone(), self.terminate_workflow(&id, reason).await) }
        }).collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }
}

fn collect_bulk_results(results: Vec<(String, Result<(), EngineError>)>) -> BulkResponse {
    let mut successful = vec![];
    let mut errors = HashMap::new();
    for (id, result) in results {
        match result {
            Ok(()) => successful.push(id),
            Err(e) => { errors.insert(id, e.to_string()); }
        }
    }
    BulkResponse { bulk_successful_results: successful, bulk_error_results: errors }
}

use std::collections::HashMap;

use chrono::Utc;
use futures::future::join_all;
use serde_json::Value;
use uuid::Uuid;

use super::WorkflowEngine;
use super::error::EngineError;
use super::rows::{TaskRow, WorkflowRow, WorkflowSummaryRow};
use crate::models::*;

/// Simple base64 encode (no padding, URL-safe).
fn base64_encode(data: &[u8]) -> String {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(CHARS[((triple >> 18) & 0x3F) as usize] as char);
        out.push(CHARS[((triple >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARS[((triple >> 6) & 0x3F) as usize] as char);
        }
        if chunk.len() > 2 {
            out.push(CHARS[(triple & 0x3F) as usize] as char);
        }
    }
    out
}

/// Simple base64 decode (URL-safe, no padding).
fn base64_decode(data: &[u8]) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'-' | b'+' => Some(62),
            b'_' | b'/' => Some(63),
            _ => None,
        }
    }
    let mut out = Vec::new();
    let mut buf: u32 = 0;
    let mut bits: u32 = 0;
    for &b in data {
        if b == b'=' {
            continue;
        }
        buf = (buf << 6) | val(b)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    Some(out)
}

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

    pub async fn restart_workflow(&self, workflow_id: &str) -> Result<String, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        let def = self
            .get_workflow_def(&wf.workflow_name, Some(wf.workflow_version))
            .await?;
        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let db = self.shards.shard_for(&new_id);

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def)
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8)",
        )
        .bind(&new_id)
        .bind(&wf.workflow_name)
        .bind(wf.workflow_version)
        .bind(&wf.input)
        .bind(&wf.correlation_id)
        .bind(now)
        .bind(wf.priority)
        .bind(serde_json::to_value(&def).ok())
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        self.schedule_tasks(&new_id, &def.tasks, &wf.input, 0)
            .await?;

        tracing::info!(workflow_id = %new_id, original_id = %workflow_id, "Workflow restarted as new execution");
        Ok(new_id)
    }

    pub async fn retry_workflow(&self, workflow_id: &str) -> Result<String, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        if wf.status != WorkflowStatus::Failed {
            tracing::error!(workflow_id = %workflow_id, status = ?wf.status, "Retry called on non-FAILED workflow");
            return Err(EngineError::InvalidState(
                "Can only retry FAILED workflows".into(),
            ));
        }
        let def = self
            .get_workflow_def(&wf.workflow_name, Some(wf.workflow_version))
            .await?;
        let new_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let db = self.shards.shard_for(&new_id);

        sqlx::query(
            "INSERT INTO workflow (workflow_id, workflow_name, workflow_version, status, input, correlation_id, start_time, update_time, priority, workflow_def)
             VALUES ($1, $2, $3, 'RUNNING', $4, $5, $6, $6, $7, $8)",
        )
        .bind(&new_id)
        .bind(&wf.workflow_name)
        .bind(wf.workflow_version)
        .bind(&wf.input)
        .bind(&wf.correlation_id)
        .bind(now)
        .bind(wf.priority)
        .bind(serde_json::to_value(&def).ok())
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        self.schedule_tasks(&new_id, &def.tasks, &wf.input, 0)
            .await?;

        tracing::info!(workflow_id = %new_id, original_id = %workflow_id, "Workflow retried as new execution");
        Ok(new_id)
    }

    pub async fn search_workflows(
        &self,
        status: Option<&str>,
        name: Option<&str>,
        free_text: Option<&str>,
        start: i64,
        size: i64,
        tags: Option<&[String]>,
    ) -> Result<SearchResult<WorkflowSummary>, EngineError> {
        self.search_workflows_with_cursor(status, name, free_text, start, size, tags, None)
            .await
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn search_workflows_with_cursor(
        &self,
        status: Option<&str>,
        name: Option<&str>,
        free_text: Option<&str>,
        start: i64,
        size: i64,
        tags: Option<&[String]>,
        cursor: Option<&str>,
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
        if let Some(tag_list) = tags {
            for tag in tag_list {
                let safe = tag.replace(['\'', '"'], "");
                where_clause.push_str(&format!(" AND tags @> '[\"{safe}\"]'::jsonb"));
            }
        }

        // Cursor-based keyset pagination: decode (start_time_ms, workflow_id) from cursor
        let cursor_decoded = cursor.and_then(|c| {
            let decoded = String::from_utf8(base64_decode(c.as_bytes())?).ok()?;
            let v: serde_json::Value = serde_json::from_str(&decoded).ok()?;
            let t = v.get("t")?.as_i64()?;
            let id = v.get("id")?.as_str()?.to_string();
            Some((t, id))
        });

        if let Some((cursor_time_ms, cursor_id)) = &cursor_decoded {
            // Convert milliseconds to a safe timestamp string for keyset pagination
            let ts = chrono::DateTime::from_timestamp_millis(*cursor_time_ms);
            if let Some(ts) = ts {
                let ts_str = ts.format("%Y-%m-%d %H:%M:%S%.6f%z").to_string();
                let safe_id = cursor_id.replace('\'', "");
                where_clause.push_str(&format!(
                    " AND (start_time, workflow_id) < ('{ts_str}'::timestamptz, '{safe_id}')"
                ));
            }
        }

        let select_cols = "SELECT workflow_id, workflow_name, workflow_version, status, start_time, end_time, input::text, output::text, correlation_id, priority FROM workflow";

        // Use read replicas for search queries
        let read_shards = self.shards.read_shards();

        // For cursor-based pagination, we don't use OFFSET
        let effective_offset = if cursor_decoded.is_some() { 0 } else { start };
        let fetch_size = size + effective_offset;
        let count_query = format!("SELECT COUNT(*) FROM workflow{where_clause}");
        let data_query = format!(
            "{select_cols}{where_clause} ORDER BY start_time DESC, workflow_id DESC LIMIT {fetch_size} OFFSET 0"
        );

        let futs: Vec<_> = read_shards
            .iter()
            .map(|shard| {
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
            })
            .collect();

        let results = join_all(futs).await;
        let mut total: i64 = 0;
        let mut all_results: Vec<WorkflowSummary> = Vec::new();
        for result in results {
            let (count, rows) = result?;
            total += count;
            all_results.extend(rows.into_iter().map(|r| r.into()));
        }

        // Sort merged results and apply pagination
        all_results.sort_by(|a, b| {
            b.start_time
                .cmp(&a.start_time)
                .then_with(|| b.workflow_id.cmp(&a.workflow_id))
        });
        let skip = if cursor_decoded.is_some() {
            0
        } else {
            start as usize
        };
        let paged: Vec<WorkflowSummary> = all_results
            .into_iter()
            .skip(skip)
            .take(size as usize)
            .collect();

        // Build next cursor from the last result
        let next_cursor = paged.last().and_then(|last| {
            let t = last.start_time.as_ref()?.parse::<i64>().ok()?;
            let cursor_json = serde_json::json!({"t": t, "id": &last.workflow_id});
            Some(base64_encode(cursor_json.to_string().as_bytes()))
        });

        Ok(SearchResult {
            total_hits: total,
            results: paged,
            next_cursor,
        })
    }

    pub async fn workflow_stats(&self) -> Result<HashMap<String, i64>, EngineError> {
        let futs: Vec<_> = self
            .shards
            .read_shards()
            .iter()
            .map(|shard| {
                sqlx::query_as::<_, (String, i64)>(
                    "SELECT status, COUNT(*) FROM workflow GROUP BY status",
                )
                .fetch_all(shard)
            })
            .collect();

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

    pub async fn workflow_metrics(&self, name: &str) -> Result<WorkflowMetrics, EngineError> {
        let query = r#"
            SELECT status, start_time, end_time
            FROM workflow
            WHERE workflow_name = $1
            ORDER BY start_time DESC
            LIMIT 100
        "#;

        let read_shards = self.shards.read_shards();
        let futs: Vec<_> = read_shards
            .iter()
            .map(|shard| async move {
                sqlx::query_as::<_, (String, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)>(
                    query,
                )
                .bind(name)
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))
            })
            .collect();

        let results = join_all(futs).await;
        let mut all_rows: Vec<(String, chrono::DateTime<Utc>, Option<chrono::DateTime<Utc>>)> =
            Vec::new();
        for result in results {
            all_rows.extend(result?);
        }

        // Sort by start_time desc and take top 100 across all shards
        all_rows.sort_by(|a, b| b.1.cmp(&a.1));
        all_rows.truncate(100);

        let total = all_rows.len() as i64;
        if total == 0 {
            return Ok(WorkflowMetrics {
                workflow_name: name.to_string(),
                sample_size: 0,
                status_distribution: HashMap::new(),
                success_rate: 0.0,
                failure_rate: 0.0,
                avg_duration_ms: None,
                min_duration_ms: None,
                max_duration_ms: None,
                p50_duration_ms: None,
                p95_duration_ms: None,
            });
        }

        let mut status_dist: HashMap<String, i64> = HashMap::new();
        let mut durations: Vec<i64> = Vec::new();
        let mut completed = 0i64;
        let mut failed = 0i64;

        for (status, start, end) in &all_rows {
            *status_dist.entry(status.clone()).or_insert(0) += 1;
            match status.as_str() {
                "COMPLETED" => completed += 1,
                "FAILED" | "TIMED_OUT" => failed += 1,
                _ => {}
            }
            if let Some(e) = end {
                let dur = (*e - *start).num_milliseconds();
                if dur >= 0 {
                    durations.push(dur);
                }
            }
        }

        durations.sort();

        let avg = if durations.is_empty() {
            None
        } else {
            Some(durations.iter().sum::<i64>() / durations.len() as i64)
        };
        let min_d = durations.first().copied();
        let max_d = durations.last().copied();
        let p50 = if durations.is_empty() {
            None
        } else {
            Some(durations[durations.len() / 2])
        };
        let p95 = if durations.is_empty() {
            None
        } else {
            Some(
                durations[(durations.len() as f64 * 0.95) as usize].min(*durations.last().unwrap()),
            )
        };

        Ok(WorkflowMetrics {
            workflow_name: name.to_string(),
            sample_size: total,
            status_distribution: status_dist,
            success_rate: completed as f64 / total as f64 * 100.0,
            failure_rate: failed as f64 / total as f64 * 100.0,
            avg_duration_ms: avg,
            min_duration_ms: min_d,
            max_duration_ms: max_d,
            p50_duration_ms: p50,
            p95_duration_ms: p95,
        })
    }

    pub async fn decide_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        self.advance_workflow(workflow_id).await
    }

    /// Check for RUNNING workflows whose SLA deadline has passed.
    /// Breached workflows are failed with status TIMED_OUT.
    pub async fn check_sla_breaches(&self) -> Result<u64, EngineError> {
        let mut breached: u64 = 0;

        for shard in self.shards.all_shards() {
            let rows: Vec<(String,)> = sqlx::query_as(
                "SELECT workflow_id FROM workflow \
                 WHERE status = 'RUNNING' \
                   AND sla_deadline IS NOT NULL \
                   AND sla_deadline < NOW()",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error checking SLA breaches");
                EngineError::Database(e.to_string())
            })?;

            for (wf_id,) in &rows {
                tracing::warn!(workflow_id = %wf_id, "SLA breach detected, failing workflow");
                if let Err(e) = self
                    .fail_workflow(wf_id, Some("SLA deadline breached"), "TIMED_OUT")
                    .await
                {
                    tracing::error!(workflow_id = %wf_id, error = %e, "Failed to fail SLA-breached workflow");
                } else {
                    breached += 1;
                }
            }
        }

        Ok(breached)
    }

    pub async fn rerun_workflow(
        &self,
        workflow_id: &str,
        req: &RerunWorkflowRequest,
    ) -> Result<String, EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        let new_id = req
            .re_run_from_workflow_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let input = match &req.workflow_input {
            Some(m) => serde_json::to_value(m).unwrap_or(wf.input.clone()),
            None => wf.input.clone(),
        };
        let def = self
            .get_workflow_def(&wf.workflow_name, Some(wf.workflow_version))
            .await?;
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
            def.tasks
                .iter()
                .position(|t| t.task_reference_name == task_ref)
                .unwrap_or(0) as i32
        } else {
            0
        };

        self.schedule_tasks(&new_id, &def.tasks[start_seq as usize..], &input, start_seq)
            .await?;

        Ok(new_id)
    }

    pub async fn skip_task(
        &self,
        workflow_id: &str,
        task_reference_name: &str,
        req: &SkipTaskRequest,
    ) -> Result<(), EngineError> {
        let task_id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let output = match &req.task_output {
            Some(m) => {
                serde_json::to_value(m).unwrap_or_else(|_| Value::Object(Default::default()))
            }
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

        let futs: Vec<_> = self
            .shards
            .read_shards()
            .iter()
            .map(|shard| {
                let q = base_query.clone();
                async move {
                    sqlx::query_scalar::<_, String>(&q)
                        .bind(name)
                        .fetch_all(shard)
                        .await
                        .map_err(|e| EngineError::Database(e.to_string()))
                }
            })
            .collect();

        let results = join_all(futs).await;
        let mut all_ids = Vec::new();
        for result in results {
            all_ids.extend(result?);
        }
        Ok(all_ids)
    }

    pub async fn delete_workflow(
        &self,
        workflow_id: &str,
        archive: bool,
    ) -> Result<(), EngineError> {
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
                return Err(EngineError::NotFound(format!(
                    "Workflow not found: {workflow_id}"
                )));
            }
        }
        Ok(())
    }

    // ── Bulk Operations ──

    pub async fn bulk_pause(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.pause_workflow(&id).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_resume(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.resume_workflow(&id).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_retry(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move {
                    let result = self.retry_workflow(&id).await.map(|_| ());
                    (id, result)
                }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_restart(&self, workflow_ids: &[String]) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move {
                    let result = self.restart_workflow(&id).await.map(|_| ());
                    (id, result)
                }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    pub async fn bulk_terminate(
        &self,
        workflow_ids: &[String],
        reason: Option<&str>,
    ) -> Result<BulkResponse, EngineError> {
        let futs: Vec<_> = workflow_ids
            .iter()
            .map(|id| {
                let id = id.clone();
                async move { (id.clone(), self.terminate_workflow(&id, reason).await) }
            })
            .collect();
        Ok(collect_bulk_results(join_all(futs).await))
    }

    // ── Workflow Status ──

    pub async fn get_workflow_status(
        &self,
        workflow_id: &str,
        include_tasks: bool,
    ) -> Result<Workflow, EngineError> {
        if include_tasks {
            self.get_workflow(workflow_id).await
        } else {
            let db = self.shards.shard_for(workflow_id);
            let wf_row =
                sqlx::query_as::<_, WorkflowRow>("SELECT * FROM workflow WHERE workflow_id = $1")
                    .bind(workflow_id)
                    .fetch_optional(db)
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?
                    .ok_or_else(|| {
                        EngineError::NotFound(format!("Workflow not found: {workflow_id}"))
                    })?;

            Ok(Workflow {
                workflow_id: wf_row.workflow_id,
                workflow_name: wf_row.workflow_name,
                workflow_version: wf_row.workflow_version,
                status: serde_json::from_value(Value::String(wf_row.status))
                    .unwrap_or(WorkflowStatus::Running),
                input: wf_row.input,
                output: wf_row.output,
                tasks: vec![],
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
    }

    // ── Update Workflow Variables ──

    pub async fn update_workflow_variables(
        &self,
        workflow_id: &str,
        variables: &HashMap<String, Value>,
    ) -> Result<Workflow, EngineError> {
        let db = self.shards.shard_for(workflow_id);
        let vars_json =
            serde_json::to_value(variables).unwrap_or(Value::Object(Default::default()));

        sqlx::query(
            "UPDATE workflow SET variables = variables || $2, update_time = NOW() WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .bind(&vars_json)
        .execute(db)
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        self.get_workflow(workflow_id).await
    }
}

fn collect_bulk_results(results: Vec<(String, Result<(), EngineError>)>) -> BulkResponse {
    let mut successful = vec![];
    let mut errors = HashMap::new();
    for (id, result) in results {
        match result {
            Ok(()) => successful.push(id),
            Err(e) => {
                errors.insert(id, e.to_string());
            }
        }
    }
    BulkResponse {
        bulk_successful_results: successful,
        bulk_error_results: errors,
    }
}

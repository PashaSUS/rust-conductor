use chrono::Utc;
use serde_json::Value;
use uuid::Uuid;

use super::WorkflowEngine;
use super::error::EngineError;
use crate::models::*;

/// Row type for scheduled_workflow queries.
#[derive(sqlx::FromRow)]
struct ScheduledRow {
    schedule_id: String,
    name: String,
    cron_expression: String,
    timezone: String,
    workflow_name: String,
    workflow_version: i32,
    workflow_input: Value,
    enabled: bool,
    last_run_at: Option<chrono::DateTime<Utc>>,
    next_run_at: Option<chrono::DateTime<Utc>>,
    #[allow(dead_code)]
    last_error: Option<String>,
}

impl From<ScheduledRow> for ScheduledWorkflow {
    fn from(r: ScheduledRow) -> Self {
        Self {
            schedule_id: r.schedule_id,
            name: r.name,
            cron_expression: r.cron_expression,
            timezone: r.timezone,
            workflow_name: r.workflow_name,
            workflow_version: r.workflow_version,
            workflow_input: r.workflow_input,
            enabled: r.enabled,
            last_run_at: r.last_run_at.map(|t| t.timestamp_millis()),
            next_run_at: r.next_run_at.map(|t| t.timestamp_millis()),
            last_error: r.last_error,
        }
    }
}

impl WorkflowEngine {
    pub async fn create_schedule(
        &self,
        sched: &ScheduledWorkflow,
    ) -> Result<ScheduledWorkflow, EngineError> {
        let id = if sched.schedule_id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            sched.schedule_id.clone()
        };

        let next_run = Self::compute_next_run(&sched.cron_expression)?;

        sqlx::query(
            "INSERT INTO scheduled_workflow (schedule_id, name, cron_expression, timezone, workflow_name, workflow_version, workflow_input, enabled, next_run_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (schedule_id) DO UPDATE SET
                cron_expression = $3, timezone = $4, workflow_name = $5,
                workflow_version = $6, workflow_input = $7, enabled = $8,
                next_run_at = $9, updated_on = NOW()",
        )
        .bind(&id)
        .bind(&sched.name)
        .bind(&sched.cron_expression)
        .bind(&sched.timezone)
        .bind(&sched.workflow_name)
        .bind(sched.workflow_version)
        .bind(&sched.workflow_input)
        .bind(sched.enabled)
        .bind(next_run)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let mut result = sched.clone();
        result.schedule_id = id;
        result.next_run_at = next_run.map(|t| t.timestamp_millis());
        Ok(result)
    }

    pub async fn list_schedules(&self) -> Result<Vec<ScheduledWorkflow>, EngineError> {
        let rows =
            sqlx::query_as::<_, ScheduledRow>("SELECT * FROM scheduled_workflow ORDER BY name")
                .fetch_all(self.shards.primary())
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(Into::into).collect())
    }

    pub async fn get_schedule(&self, schedule_id: &str) -> Result<ScheduledWorkflow, EngineError> {
        let row = sqlx::query_as::<_, ScheduledRow>(
            "SELECT * FROM scheduled_workflow WHERE schedule_id = $1",
        )
        .bind(schedule_id)
        .fetch_optional(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?
        .ok_or_else(|| EngineError::NotFound(format!("Schedule not found: {schedule_id}")))?;

        Ok(row.into())
    }

    pub async fn delete_schedule(&self, schedule_id: &str) -> Result<(), EngineError> {
        let result = sqlx::query("DELETE FROM scheduled_workflow WHERE schedule_id = $1")
            .bind(schedule_id)
            .execute(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "Schedule not found: {schedule_id}"
            )));
        }
        Ok(())
    }

    pub async fn toggle_schedule(
        &self,
        schedule_id: &str,
        enabled: bool,
    ) -> Result<(), EngineError> {
        let result = sqlx::query(
            "UPDATE scheduled_workflow SET enabled = $2, updated_on = NOW() WHERE schedule_id = $1",
        )
        .bind(schedule_id)
        .bind(enabled)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "Schedule not found: {schedule_id}"
            )));
        }
        Ok(())
    }

    /// Run all due schedules. Called by the background sweeper loop.
    pub async fn run_due_schedules(&self) -> Result<u64, EngineError> {
        let now = Utc::now();
        let due: Vec<ScheduledRow> = sqlx::query_as(
            "SELECT * FROM scheduled_workflow WHERE enabled = TRUE AND next_run_at <= $1",
        )
        .bind(now)
        .fetch_all(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        let mut executed = 0u64;
        for sched in &due {
            let req = StartWorkflowRequest {
                name: sched.workflow_name.clone(),
                version: sched.workflow_version,
                input: sched.workflow_input.clone(),
                correlation_id: Some(format!("schedule:{}", sched.schedule_id)),
                priority: 0,
                task_to_domain: Default::default(),
                workflow_def: None,
                external_input_payload_storage_path: None,
                created_by: Some(format!("scheduler:{}", sched.name)),
                idempotency_key: None,
                idempotency_strategy: None,
                tags: vec![],
            };

            match self.start_workflow(&req).await {
                Ok(wf_id) => {
                    tracing::info!(
                        schedule_id = %sched.schedule_id,
                        workflow_id = %wf_id,
                        "Scheduled workflow triggered"
                    );
                    executed += 1;
                    // Clear any previous error
                    let _ = sqlx::query(
                        "UPDATE scheduled_workflow SET last_error = NULL WHERE schedule_id = $1",
                    )
                    .bind(&sched.schedule_id)
                    .execute(self.shards.primary())
                    .await;
                }
                Err(e) => {
                    tracing::error!(
                        schedule_id = %sched.schedule_id,
                        error = %e,
                        "Failed to trigger scheduled workflow"
                    );
                    // Store the error so it's visible in the UI
                    let _ = sqlx::query(
                        "UPDATE scheduled_workflow SET last_error = $2 WHERE schedule_id = $1",
                    )
                    .bind(&sched.schedule_id)
                    .bind(e.to_string())
                    .execute(self.shards.primary())
                    .await;
                }
            }

            // Update last_run_at and compute next_run_at
            let next_run = Self::compute_next_run(&sched.cron_expression).unwrap_or(None);
            let _ = sqlx::query(
                "UPDATE scheduled_workflow SET last_run_at = $2, next_run_at = $3, updated_on = NOW() WHERE schedule_id = $1",
            )
            .bind(&sched.schedule_id)
            .bind(now)
            .bind(next_run)
            .execute(self.shards.primary())
            .await;
        }

        Ok(executed)
    }

    /// Parse a simple cron expression and compute the next run time.
    /// Supports standard 5-field cron: minute hour day-of-month month day-of-week.
    /// For simplicity, computes next aligned minute boundary.
    fn compute_next_run(cron_expr: &str) -> Result<Option<chrono::DateTime<Utc>>, EngineError> {
        let parts: Vec<&str> = cron_expr.split_whitespace().collect();
        if parts.len() < 5 {
            return Err(EngineError::InvalidState(format!(
                "Invalid cron expression (need 5 fields): {cron_expr}"
            )));
        }

        let now = Utc::now();
        // Simple approach: check each minute for the next 48 hours
        let mut candidate = now + chrono::Duration::minutes(1);
        // Truncate seconds
        candidate = candidate
            .date_naive()
            .and_hms_opt(candidate.time().hour(), candidate.time().minute(), 0)
            .map(|dt| dt.and_utc())
            .unwrap_or(candidate);

        use chrono::Datelike;
        use chrono::Timelike;

        for _ in 0..(48 * 60) {
            let minute = candidate.minute();
            let hour = candidate.hour();
            let dom = candidate.day();
            let month = candidate.month();
            let dow = candidate.weekday().num_days_from_sunday(); // 0=Sun

            if Self::cron_field_matches(parts[0], minute)
                && Self::cron_field_matches(parts[1], hour)
                && Self::cron_field_matches(parts[2], dom)
                && Self::cron_field_matches(parts[3], month)
                && Self::cron_field_matches(parts[4], dow)
            {
                return Ok(Some(candidate));
            }
            candidate += chrono::Duration::minutes(1);
        }

        Ok(None)
    }

    /// Check if a cron field matches a given value. Supports *, specific numbers, and */N.
    fn cron_field_matches(field: &str, value: u32) -> bool {
        if field == "*" {
            return true;
        }
        // */N step
        if let Some(step_str) = field.strip_prefix("*/")
            && let Ok(step) = step_str.parse::<u32>()
        {
            return step > 0 && value.is_multiple_of(step);
        }
        // Comma-separated values
        for part in field.split(',') {
            // Range: N-M
            if let Some((start_str, end_str)) = part.split_once('-') {
                if let (Ok(start), Ok(end)) = (start_str.parse::<u32>(), end_str.parse::<u32>())
                    && value >= start
                    && value <= end
                {
                    return true;
                }
                continue;
            }
            // Exact match
            if let Ok(v) = part.trim().parse::<u32>()
                && v == value
            {
                return true;
            }
        }
        false
    }
}

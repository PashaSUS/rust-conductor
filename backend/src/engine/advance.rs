use std::collections::HashMap;

use chrono::Utc;
use serde_json::Value;

use super::error::EngineError;
use super::{WorkflowEngine, is_task_failed, is_task_terminal};
use crate::models::*;
use deadpool_redis::redis;

impl WorkflowEngine {
    pub(crate) async fn advance_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        // Per-workflow distributed lock to prevent concurrent advance_workflow
        // calls from racing to schedule the same next task.
        //
        // IMPORTANT: We must NOT hold a Redis connection across the full
        // `advance_workflow_inner` call. That function performs many SQL
        // round-trips and other Redis ops, so holding the connection here
        // pegs every concurrent advance to one slot in the deadpool, which
        // exhausts the pool under high load (observed as
        // "Timeout occurred while waiting for a slot to become available").
        //
        // Instead: acquire → SET NX → drop. Do the work. Acquire again → DEL.
        //
        // CONCURRENCY CORRECTNESS: a naive "skip if held" misses updates
        // committed by other callers between our snapshot read and lock
        // release. Concretely: branch A completes → acquires lock → reads
        // workflow → B is still IN_PROGRESS → JOIN check fails → returns.
        // Meanwhile B completes and tries to advance, but the lock is still
        // held → skipped → JOIN is never re-evaluated and the workflow
        // hangs until the sweeper rescues it.
        //
        // Fix: skipped callers SET a "dirty" flag. The lock holder clears
        // the flag before running inner; after inner finishes, if dirty was
        // set during the run we loop and run inner again. This is the
        // canonical "leader-with-recheck" pattern.
        let lock_key = format!("conductor:advance_lock:{}", workflow_id);
        let dirty_key = format!("conductor:advance_dirty:{}", workflow_id);
        let pool = self.redis.pool_for_key(workflow_id);

        let acquired = {
            let mut conn = pool.get().await.map_err(|e| {
                tracing::warn!(workflow_id = %workflow_id, error = %e, "Redis conn failed for advance lock; skipping this advance round");
                EngineError::Redis(e.to_string())
            })?;
            let acquired: Option<String> = redis::cmd("SET")
                .arg(&lock_key)
                .arg("1")
                .arg("NX")
                .arg("EX")
                .arg(30_u32)
                .query_async(&mut *conn)
                .await
                .unwrap_or(None);
            if acquired.is_none() {
                // Could not acquire — flag the workflow dirty so the
                // current holder will re-run inner before releasing.
                let _: Result<(), _> = redis::cmd("SET")
                    .arg(&dirty_key)
                    .arg("1")
                    .arg("EX")
                    .arg(60_u32)
                    .query_async(&mut *conn)
                    .await;
            }
            acquired
        };

        if acquired.is_none() {
            tracing::debug!(workflow_id = %workflow_id, "advance_workflow skipped — lock held by another caller; flagged dirty");
            return Ok(());
        }

        // Loop: run inner; if any caller flagged dirty during the run,
        // clear and run again. Bound the number of iterations to avoid
        // pathological infinite loops under sustained contention.
        let mut result: Result<(), EngineError> = Ok(());
        for _ in 0..8 {
            // Clear dirty flag *before* running inner so any flag set
            // during this run causes a re-loop.
            if let Ok(mut conn) = pool.get().await {
                let _: Result<(), _> = redis::cmd("DEL")
                    .arg(&dirty_key)
                    .query_async(&mut *conn)
                    .await;
            }

            result = self.advance_workflow_inner(workflow_id).await;
            if result.is_err() {
                break;
            }

            // Was anything flagged during the inner run?
            let dirty: Option<String> = if let Ok(mut conn) = pool.get().await {
                redis::cmd("GET")
                    .arg(&dirty_key)
                    .query_async(&mut *conn)
                    .await
                    .unwrap_or(None)
            } else {
                None
            };
            if dirty.is_none() {
                break;
            }
            tracing::debug!(workflow_id = %workflow_id, "advance_workflow re-running due to dirty flag");
        }

        // Release lock with a fresh, short-lived connection. Failure to release
        // is non-fatal — the 30s TTL guarantees forward progress.
        if let Ok(mut conn) = pool.get().await {
            let _: Result<(), _> = redis::cmd("DEL")
                .arg(&lock_key)
                .query_async(&mut *conn)
                .await;
        } else {
            tracing::debug!(workflow_id = %workflow_id, "Could not acquire Redis to DEL advance lock; will expire via TTL");
        }

        result
    }

    async fn advance_workflow_inner(&self, workflow_id: &str) -> Result<(), EngineError> {
        let wf = self.get_workflow(workflow_id).await?;
        if wf.status != WorkflowStatus::Running {
            return Ok(());
        }
        let Some(def) = &wf.workflow_definition else {
            tracing::error!(workflow_id = %workflow_id, "advance_workflow called but workflow has no embedded definition");
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
            .map_err(|e| {
                tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to read workflow status during completion check");
                EngineError::Database(e.to_string())
            })?;

            if wf_status != "RUNNING" {
                return Ok(());
            }

            let output: Value = sqlx::query_scalar(
                "SELECT COALESCE(output_data, '{}') FROM task WHERE workflow_instance_id = $1 ORDER BY seq DESC LIMIT 1",
            )
            .bind(workflow_id)
            .fetch_optional(db)
            .await
            .map_err(|e| {
                tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to read final task output for workflow completion");
                EngineError::Database(e.to_string())
            })?
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
            .map_err(|e| {
                tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to mark workflow COMPLETED");
                EngineError::Database(e.to_string())
            })?;

            tracing::info!(workflow_id = %workflow_id, "Workflow completed");

            crate::metrics::record_workflow_completed();

            // Fire completion webhook if configured
            self.notify_webhooks(def, workflow_id, "COMPLETED", &output);

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
        seq: i32,
    ) -> Result<bool, EngineError> {
        for (idx, task_def) in def_tasks.iter().enumerate() {
            let ref_name = &task_def.task_reference_name;

            match task_map.get(ref_name.as_str()) {
                Some(task) if is_task_terminal(&task.status) => {
                    if is_task_failed(&task.status) && !task_def.optional {
                        let wf_status = if task.status == TaskStatus::TimedOut {
                            "TIMED_OUT"
                        } else {
                            "FAILED"
                        };
                        tracing::error!(
                            workflow_id = %workflow_id,
                            task_ref = %ref_name,
                            task_status = ?task.status,
                            reason = ?task.reason_for_incompletion,
                            "Non-optional task failed, failing workflow"
                        );
                        self.fail_workflow(
                            workflow_id,
                            Some(&format!(
                                "Task {} failed: {}",
                                ref_name,
                                task.reason_for_incompletion.as_deref().unwrap_or("unknown")
                            )),
                            wf_status,
                        )
                        .await?;
                        return Ok(true);
                    }

                    match task_def.task_type.as_str() {
                        "FORK_JOIN" | "FORK" => {
                            for branch in &task_def.fork_tasks {
                                let branch_done = Box::pin(self.evaluate_and_schedule(
                                    workflow_id,
                                    branch,
                                    task_map,
                                    input,
                                    seq,
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
                                workflow_id,
                                branch,
                                task_map,
                                input,
                                seq,
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
                                match self.get_workflow(sub_id).await {
                                    Ok(sub_wf) => match sub_wf.status {
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
                                            tracing::error!(
                                                workflow_id = %workflow_id,
                                                sub_workflow_id = %sub_id,
                                                sub_status = ?sub_wf.status,
                                                task_id = %task.task_id,
                                                "Sub-workflow ended with non-success status, failing parent task and workflow"
                                            );
                                            self.fail_task(
                                                workflow_id,
                                                &task.task_id,
                                                "Sub-workflow ended with non-success status",
                                            )
                                            .await?;
                                            if !task_def.optional {
                                                let wf_status = match sub_wf.status {
                                                    WorkflowStatus::TimedOut => "TIMED_OUT",
                                                    _ => "FAILED",
                                                };
                                                self.fail_workflow(
                                                    workflow_id,
                                                    Some(&format!(
                                                        "Sub-workflow {} ended with status {:?}",
                                                        sub_id, sub_wf.status
                                                    )),
                                                    wf_status,
                                                )
                                                .await?;
                                            }
                                            return Ok(true);
                                        }
                                        _ => {}
                                    },
                                    Err(e) => {
                                        tracing::error!(
                                            workflow_id = %workflow_id,
                                            sub_workflow_id = %sub_id,
                                            error = %e,
                                            "Failed to fetch sub-workflow status"
                                        );
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                    return Ok(false);
                }

                None => {
                    self.schedule_tasks(workflow_id, &def_tasks[idx..], input, seq)
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
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, task_id = %task_id, error = %e, "Failed to complete sub-workflow task");
            EngineError::Database(e.to_string())
        })?;
        Ok(())
    }

    /// Mark a task as FAILED.
    pub(crate) async fn fail_task(
        &self,
        workflow_id: &str,
        task_id: &str,
        reason: &str,
    ) -> Result<(), EngineError> {
        let now = Utc::now();
        let db = self.shards.shard_for(workflow_id);
        tracing::error!(workflow_id = %workflow_id, task_id = %task_id, reason = %reason, "Marking task as FAILED");
        sqlx::query(
            "UPDATE task SET status = 'FAILED', reason_for_incompletion = $2, end_time = $3, update_time = $3 WHERE task_id = $1",
        )
        .bind(task_id)
        .bind(reason)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, task_id = %task_id, error = %e, "DB error while marking task FAILED");
            EngineError::Database(e.to_string())
        })?;
        Ok(())
    }

    /// When a child workflow completes, find and complete the parent's
    /// SUB_WORKFLOW task so the parent can advance.
    ///
    /// Each step is retried a few times on transient DB errors (e.g. PgBouncer
    /// `query_wait_timeout` under burst load). If retries are exhausted we fall
    /// through to the sweeper, which picks the parent up via the
    /// "stale SUB_WORKFLOW" sweep after 30s — but at high throughput we want
    /// the hot path to succeed nearly always so the sweeper stays idle.
    pub(crate) async fn try_complete_parent_sub_workflow(
        &self,
        child_workflow_id: &str,
        child_output: &Value,
    ) -> Result<(), EngineError> {
        let child_db = self.shards.shard_for(child_workflow_id);

        let mut parent_info: Option<(Option<String>, Option<String>)> = None;
        let mut last_err: Option<sqlx::Error> = None;
        for attempt in 0..3u32 {
            match sqlx::query_as(
                "SELECT parent_workflow_id, parent_workflow_task_id FROM workflow WHERE workflow_id = $1",
            )
            .bind(child_workflow_id)
            .fetch_optional(child_db)
            .await
            {
                Ok(row) => {
                    parent_info = row;
                    last_err = None;
                    break;
                }
                Err(e) => {
                    last_err = Some(e);
                    // Tiny back-off before retry (10ms, 30ms)
                    tokio::time::sleep(std::time::Duration::from_millis(10 * (1 + attempt as u64) * (1 + attempt as u64))).await;
                }
            }
        }
        if let Some(e) = last_err {
            tracing::error!(child_workflow_id = %child_workflow_id, error = %e, "Failed to look up parent_workflow_id for child after retries");
            return Err(EngineError::Database(e.to_string()));
        }

        if let Some((Some(parent_wf_id), Some(parent_task_id))) = parent_info {
            // Retry the parent task UPDATE — this is the most contention-prone step.
            let mut last_err: Option<EngineError> = None;
            for attempt in 0..3u32 {
                match self
                    .complete_sub_workflow_task(&parent_wf_id, &parent_task_id, child_output)
                    .await
                {
                    Ok(()) => {
                        last_err = None;
                        break;
                    }
                    Err(e) => {
                        last_err = Some(e);
                        tokio::time::sleep(std::time::Duration::from_millis(10 * (1 + attempt as u64) * (1 + attempt as u64))).await;
                    }
                }
            }
            if let Some(e) = last_err {
                return Err(e);
            }
            Box::pin(self.advance_workflow(&parent_wf_id)).await?;
        }
        Ok(())
    }

    pub(crate) async fn fail_workflow(
        &self,
        workflow_id: &str,
        reason: Option<&str>,
        terminal_status: &str,
    ) -> Result<(), EngineError> {
        tracing::error!(
            workflow_id = %workflow_id,
            status = %terminal_status,
            reason = ?reason,
            "Failing workflow"
        );

        // 104. Saga compensation — if the workflow definition has saga_enabled,
        // run compensation tasks before marking the workflow as failed.
        {
            let db = self.shards.shard_for(workflow_id);
            if let Some(def_json) = sqlx::query_scalar::<_, Option<Value>>(
                "SELECT workflow_def FROM workflow WHERE workflow_id = $1",
            )
            .bind(workflow_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten()
            {
                if let Ok(def) = serde_json::from_value::<WorkflowDef>(def_json) {
                    if def.saga_enabled {
                        // Fetch completed tasks for compensation
                        let completed_rows: Vec<TaskResult> = self
                            .get_workflow(workflow_id)
                            .await
                            .map(|wf| {
                                wf.tasks
                                    .into_iter()
                                    .filter(|t| t.status == TaskStatus::Completed)
                                    .collect()
                            })
                            .unwrap_or_default();
                        if let Err(e) = self
                            .run_saga_compensation(workflow_id, &def, &completed_rows)
                            .await
                        {
                            tracing::error!(
                                workflow_id = %workflow_id,
                                error = %e,
                                "Saga compensation failed"
                            );
                        }
                    }
                }
            }
        }

        // Only allow known terminal statuses to prevent SQL injection
        let safe_status = match terminal_status {
            "TIMED_OUT" => "TIMED_OUT",
            _ => "FAILED",
        };
        let db = self.shards.shard_for(workflow_id);
        let now = Utc::now();

        sqlx::query(
            &format!("UPDATE workflow SET status = '{safe_status}', end_time = $2, update_time = $2, reason_for_incompletion = $3 WHERE workflow_id = $1 AND status = 'RUNNING'"),
        )
        .bind(workflow_id)
        .bind(now)
        .bind(reason)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "DB error while setting workflow to {}", safe_status);
            EngineError::Database(e.to_string())
        })?;

        crate::metrics::record_workflow_failed();

        sqlx::query(
            "UPDATE task SET status = 'CANCELED', end_time = $2, update_time = $2 WHERE workflow_instance_id = $1 AND status IN ('SCHEDULED', 'IN_PROGRESS')",
        )
        .bind(workflow_id)
        .bind(now)
        .execute(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "DB error while canceling active tasks on workflow failure");
            EngineError::Database(e.to_string())
        })?;

        // If this is a child of a SUB_WORKFLOW, propagate failure to parent
        let parent_info: Option<(Option<String>, Option<String>)> = sqlx::query_as(
            "SELECT parent_workflow_id, parent_workflow_task_id FROM workflow WHERE workflow_id = $1",
        )
        .bind(workflow_id)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            tracing::error!(workflow_id = %workflow_id, error = %e, "Failed to look up parent for failed child workflow");
            EngineError::Database(e.to_string())
        })?;

        if let Some((Some(parent_wf_id), Some(parent_task_id))) = parent_info {
            tracing::error!(
                child_workflow_id = %workflow_id,
                parent_workflow_id = %parent_wf_id,
                parent_task_id = %parent_task_id,
                "Propagating failure from child sub-workflow to parent"
            );
            self.fail_task(
                &parent_wf_id,
                &parent_task_id,
                reason.unwrap_or("Sub-workflow failed"),
            )
            .await?;
            Box::pin(self.fail_workflow(&parent_wf_id, reason, terminal_status)).await?;
        }

        self.trigger_failure_workflow(workflow_id, reason).await?;

        // Fire failure webhook if configured
        {
            let db = self.shards.shard_for(workflow_id);
            if let Some(def_json) = sqlx::query_scalar::<_, Option<Value>>(
                "SELECT workflow_def FROM workflow WHERE workflow_id = $1",
            )
            .bind(workflow_id)
            .fetch_optional(db)
            .await
            .ok()
            .flatten()
            .flatten()
                && let Ok(def) = serde_json::from_value::<WorkflowDef>(def_json)
            {
                self.notify_webhooks(&def, workflow_id, terminal_status, &Value::Null);
            }
        }

        tracing::warn!(workflow_id = %workflow_id, status = %terminal_status, "Workflow terminated");
        Ok(())
    }

    /// If the workflow definition specifies a `failure_workflow`, start it.
    async fn trigger_failure_workflow(
        &self,
        failed_workflow_id: &str,
        reason: Option<&str>,
    ) -> Result<(), EngineError> {
        let db = self.shards.shard_for(failed_workflow_id);

        let def_json: Option<Value> =
            sqlx::query_scalar("SELECT workflow_def FROM workflow WHERE workflow_id = $1")
                .bind(failed_workflow_id)
                .fetch_optional(db)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?
                .flatten();

        let Some(def_json) = def_json else {
            return Ok(());
        };
        let def: WorkflowDef = match serde_json::from_value(def_json) {
            Ok(d) => d,
            Err(e) => {
                tracing::error!(workflow_id = %failed_workflow_id, error = %e, "Failed to deserialize workflow_def for failure workflow trigger");
                return Ok(());
            }
        };

        let Some(failure_wf_name) = &def.failure_workflow else {
            return Ok(());
        };
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
            tags: vec![],
            parent_workflow_id: None,
            parent_workflow_task_id: None,
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

    /// Fire a webhook (POST) for workflow lifecycle events.
    /// Runs in a detached task so it never blocks the main workflow path.
    pub(crate) fn fire_webhook(url: String, payload: Value) {
        tokio::spawn(async move {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .unwrap_or_default();
            match client.post(&url).json(&payload).send().await {
                Ok(resp) => {
                    tracing::info!(url = %url, status = %resp.status(), "Webhook delivered");
                }
                Err(e) => {
                    tracing::error!(url = %url, error = %e, "Webhook delivery failed");
                }
            }
        });
    }

    /// Notify configured webhooks on workflow completion or failure.
    pub(crate) fn notify_webhooks(
        &self,
        def: &WorkflowDef,
        workflow_id: &str,
        status: &str,
        output: &Value,
    ) {
        let payload = serde_json::json!({
            "workflowId": workflow_id,
            "workflowName": def.name,
            "status": status,
            "output": output,
        });

        match status {
            "COMPLETED" => {
                if let Some(url) = &def.on_complete_webhook
                    && !url.is_empty()
                {
                    Self::fire_webhook(url.clone(), payload);
                }
            }
            "FAILED" | "TIMED_OUT" => {
                if let Some(url) = &def.on_failure_webhook
                    && !url.is_empty()
                {
                    Self::fire_webhook(url.clone(), payload);
                }
            }
            _ => {}
        }
    }
}

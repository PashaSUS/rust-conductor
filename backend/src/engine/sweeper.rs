use chrono::Utc;

use super::WorkflowEngine;
use super::error::EngineError;
use super::rows::OrphanedTaskRow;

impl WorkflowEngine {
    /// Scans for orphaned tasks across all shards: SCHEDULED worker tasks in
    /// DB but missing from Redis queues. Also times out stale IN_PROGRESS
    /// **worker** tasks. System tasks are never touched.
    pub async fn sweep_orphaned_tasks(&self) -> Result<u64, EngineError> {
        let mut recovered: u64 = 0;

        for shard in self.shards.all_shards() {
            // 1. Re-queue orphaned SCHEDULED *worker* tasks.
            //    SELECT candidates first, then UPDATE + enqueue each one
            //    individually so a single Kafka failure doesn't block the rest.
            let orphans = sqlx::query_as::<_, OrphanedTaskRow>(
                "SELECT task_id, task_def_name, workflow_instance_id FROM task \
                 WHERE status = 'SCHEDULED' \
                   AND task_type NOT IN ('FORK','FORK_JOIN','JOIN','DECISION','SWITCH','SUB_WORKFLOW','DO_WHILE','TERMINATE','SET_VARIABLE','WAIT','LAMBDA','INLINE','EVENT','MAP','WAIT_FOR_SIGNAL') \
                   AND scheduled_time < NOW() - INTERVAL '30 seconds'",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error scanning for orphaned SCHEDULED tasks");
                EngineError::Database(e.to_string())
            })?;

            if !orphans.is_empty() {
                tracing::error!(
                    count = orphans.len(),
                    "Found orphaned SCHEDULED tasks, re-queuing"
                );
                for orphan in &orphans {
                    // Enqueue first — only bump scheduled_time on success
                    self.set_task_routing(&orphan.task_id, &orphan.workflow_instance_id)
                        .await?;
                    match self
                        .queue
                        .enqueue(&orphan.task_def_name, &orphan.task_id)
                        .await
                    {
                        Ok(_) => {
                            // Bump scheduled_time so this orphan isn't re-queued next cycle
                            let _ = sqlx::query(
                                "UPDATE task SET scheduled_time = NOW(), update_time = NOW() WHERE task_id = $1",
                            )
                            .bind(&orphan.task_id)
                            .execute(shard)
                            .await;
                            recovered += 1;
                        }
                        Err(e) => {
                            // Leave scheduled_time unchanged so it's retried next sweep
                            tracing::error!(
                                task_id = %orphan.task_id,
                                error = %e,
                                "Kafka enqueue failed for orphaned task, will retry next sweep"
                            );
                        }
                    }
                }
            }

            // 2. Time out stale IN_PROGRESS *worker* tasks and collect affected workflows
            let now = Utc::now();
            let timed_out_workflows: Vec<(String,)> = sqlx::query_as(
                "UPDATE task SET status = 'TIMED_OUT', end_time = $1, update_time = $1, \
                 reason_for_incompletion = 'Task timed out (sweep)' \
                 WHERE status = 'IN_PROGRESS' \
                   AND task_type NOT IN ('FORK','FORK_JOIN','JOIN','DECISION','SWITCH','SUB_WORKFLOW','DO_WHILE','TERMINATE','SET_VARIABLE','WAIT','LAMBDA','INLINE','EVENT','MAP','WAIT_FOR_SIGNAL') \
                   AND start_time < NOW() - INTERVAL '10 minutes' \
                 RETURNING workflow_instance_id",
            )
            .bind(now)
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error timing out stale IN_PROGRESS tasks");
                EngineError::Database(e.to_string())
            })?;

            if !timed_out_workflows.is_empty() {
                tracing::error!(
                    count = timed_out_workflows.len(),
                    "Timed out stale IN_PROGRESS tasks"
                );

                // Advance each affected workflow so it detects the timed-out task
                // and either fails the workflow or proceeds (if the task was optional).
                let mut seen = std::collections::HashSet::new();
                for (wf_id,) in &timed_out_workflows {
                    if seen.insert(wf_id.clone())
                        && let Err(e) = self.advance_workflow(wf_id).await
                    {
                        tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance after timeout failed");
                    }
                }
            }

            // 3. Sweep running workflows with IN_PROGRESS SUB_WORKFLOW tasks
            let stale_subs: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT t.workflow_instance_id FROM task t \
                 JOIN workflow w ON w.workflow_id = t.workflow_instance_id \
                 WHERE t.task_type = 'SUB_WORKFLOW' AND t.status = 'IN_PROGRESS' \
                   AND w.status = 'RUNNING'",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error scanning for stale SUB_WORKFLOW tasks");
                EngineError::Database(e.to_string())
            })?;

            if !stale_subs.is_empty() {
                tracing::error!(
                    count = stale_subs.len(),
                    "Found stale SUB_WORKFLOW tasks, advancing parent workflows"
                );
            }

            for (wf_id,) in &stale_subs {
                if let Err(e) = self.advance_workflow(wf_id).await {
                    tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance failed");
                }
            }

            // 4. Detect RUNNING workflows with zero non-terminal tasks (orphans).
            //    These got stuck because schedule_tasks failed after the workflow
            //    row was created, or because all tasks completed but
            //    advance_workflow was never triggered.
            let stuck_workflows: Vec<(String,)> = sqlx::query_as(
                "SELECT w.workflow_id FROM workflow w \
                 WHERE w.status = 'RUNNING' \
                   AND w.update_time < NOW() - INTERVAL '30 seconds' \
                   AND NOT EXISTS ( \
                     SELECT 1 FROM task t \
                     WHERE t.workflow_instance_id = w.workflow_id \
                       AND t.status IN ('SCHEDULED','IN_PROGRESS') \
                   )",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error scanning for stuck RUNNING workflows");
                EngineError::Database(e.to_string())
            })?;

            for (wf_id,) in &stuck_workflows {
                tracing::error!(workflow_id = %wf_id, "Detected stuck RUNNING workflow with no active tasks, advancing");
                if let Err(e) = self.advance_workflow(wf_id).await {
                    tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance for stuck workflow failed");
                }
            }

            // 5. Safety net: RUNNING workflows with a non-optional FAILED task
            //    that somehow wasn't caught by advance_workflow (e.g. the
            //    advance returned Ok(false) before the fix).
            let failed_task_workflows: Vec<(String,)> = sqlx::query_as(
                "SELECT DISTINCT t.workflow_instance_id FROM task t \
                 JOIN workflow w ON w.workflow_id = t.workflow_instance_id \
                 WHERE t.status IN ('FAILED','TIMED_OUT') \
                   AND w.status = 'RUNNING' \
                   AND t.update_time < NOW() - INTERVAL '30 seconds'",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error scanning for RUNNING workflows with FAILED tasks");
                EngineError::Database(e.to_string())
            })?;

            if !failed_task_workflows.is_empty() {
                tracing::error!(
                    count = failed_task_workflows.len(),
                    "Found RUNNING workflows with FAILED tasks, advancing"
                );
            }

            for (wf_id,) in &failed_task_workflows {
                if let Err(e) = self.advance_workflow(wf_id).await {
                    tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance for workflow with FAILED task failed");
                }
            }

            // 6. Heartbeat timeout detection (110)
            //    Fail IN_PROGRESS tasks with heartbeat_timeout_seconds set
            //    whose update_time exceeds the timeout.
            let heartbeat_timed_out: Vec<(String, String)> = sqlx::query_as(
                "SELECT t.task_id, t.workflow_instance_id FROM task t \
                 JOIN workflow w ON w.workflow_id = t.workflow_instance_id \
                 WHERE t.status = 'IN_PROGRESS' \
                   AND w.status = 'RUNNING' \
                   AND w.workflow_def IS NOT NULL \
                   AND t.update_time < NOW() - INTERVAL '1 second' * COALESCE( \
                       (SELECT (elem->>'heartbeatTimeoutSeconds')::bigint \
                        FROM jsonb_array_elements(w.workflow_def->'tasks') elem \
                        WHERE elem->>'taskReferenceName' = t.reference_task_name \
                          AND elem->>'heartbeatTimeoutSeconds' IS NOT NULL \
                        LIMIT 1), \
                       999999999)",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| {
                tracing::error!(error = %e, "DB error scanning for heartbeat-timed-out tasks");
                EngineError::Database(e.to_string())
            })?;

            if !heartbeat_timed_out.is_empty() {
                let now = Utc::now();
                tracing::warn!(
                    count = heartbeat_timed_out.len(),
                    "Found tasks with expired heartbeat, timing out"
                );
                let mut hb_seen = std::collections::HashSet::new();
                for (task_id, wf_id) in &heartbeat_timed_out {
                    let _ = sqlx::query(
                        "UPDATE task SET status = 'TIMED_OUT', end_time = $2, update_time = $2, \
                         reason_for_incompletion = 'Heartbeat timeout' \
                         WHERE task_id = $1 AND status = 'IN_PROGRESS'",
                    )
                    .bind(task_id)
                    .bind(now)
                    .execute(shard)
                    .await;

                    if hb_seen.insert(wf_id.clone()) {
                        if let Err(e) = self.advance_workflow(wf_id).await {
                            tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance after heartbeat timeout failed");
                        }
                    }
                }
            }
        }

        if recovered > 0 {
            tracing::info!(count = recovered, "Re-queued orphaned SCHEDULED tasks");
        }

        Ok(recovered)
    }

    /// Starts a background loop that periodically sweeps for orphaned tasks.
    /// Uses adaptive intervals: decreases when work is found, increases when idle.
    pub fn start_background_sweeper(engine: std::sync::Arc<Self>) {
        tokio::spawn(async move {
            let min_interval = engine.sweeper_min_interval_secs;
            let max_interval = engine.sweeper_max_interval_secs;
            let mut current_interval = 30u64.clamp(min_interval, max_interval);

            loop {
                tokio::time::sleep(std::time::Duration::from_secs(current_interval)).await;

                let mut work_done = false;

                match engine.sweep_orphaned_tasks().await {
                    Ok(recovered) if recovered > 0 => {
                        work_done = true;
                    }
                    Ok(_) => {}
                    Err(e) => tracing::error!(error = %e, "Background sweep failed"),
                }
                // Run due CRON schedules
                match engine.run_due_schedules().await {
                    Ok(n) if n > 0 => {
                        tracing::info!(count = n, "CRON schedules triggered");
                        work_done = true;
                    }
                    Err(e) => tracing::error!(error = %e, "CRON scheduler check failed"),
                    _ => {}
                }
                // Check SLA breaches
                match engine.check_sla_breaches().await {
                    Ok(n) if n > 0 => {
                        tracing::warn!(count = n, "SLA breaches detected");
                        work_done = true;
                    }
                    Err(e) => tracing::error!(error = %e, "SLA breach check failed"),
                    _ => {}
                }

                // Adaptive interval: if work was done, speed up; if idle, slow down
                if work_done {
                    current_interval = (current_interval / 2).max(min_interval);
                    tracing::debug!(
                        interval_secs = current_interval,
                        "Sweeper found work, decreasing interval"
                    );
                } else {
                    current_interval = (current_interval + 5).min(max_interval);
                }
            }
        });
    }
}

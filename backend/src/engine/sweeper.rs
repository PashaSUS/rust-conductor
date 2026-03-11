use chrono::Utc;


use super::error::EngineError;
use super::rows::OrphanedTaskRow;
use super::WorkflowEngine;

impl WorkflowEngine {
    /// Scans for orphaned tasks across all shards: SCHEDULED worker tasks in
    /// DB but missing from Redis queues. Also times out stale IN_PROGRESS
    /// **worker** tasks. System tasks are never touched.
    pub async fn sweep_orphaned_tasks(&self) -> Result<u64, EngineError> {
        let mut recovered: u64 = 0;

        for shard in self.shards.all_shards() {
            // 1. Re-queue orphaned SCHEDULED *worker* tasks.
            //    Use UPDATE ... RETURNING to atomically bump scheduled_time
            //    so the next sweep cycle won't re-queue them again.
            let orphans = sqlx::query_as::<_, OrphanedTaskRow>(
                "UPDATE task SET scheduled_time = NOW(), update_time = NOW() \
                 WHERE task_id IN ( \
                   SELECT task_id FROM task \
                   WHERE status = 'SCHEDULED' \
                     AND task_type NOT IN ('FORK','FORK_JOIN','JOIN','DECISION','SWITCH','SUB_WORKFLOW','DO_WHILE','TERMINATE','SET_VARIABLE','WAIT') \
                     AND scheduled_time < NOW() - INTERVAL '30 seconds' \
                 ) RETURNING task_id, task_def_name, workflow_instance_id",
            )
            .fetch_all(shard)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            if !orphans.is_empty() {
                for orphan in &orphans {
                    self.set_task_routing(&orphan.task_id, &orphan.workflow_instance_id).await?;
                    let queue_key = format!("conductor:queue:{}", orphan.task_def_name);
                    let pool = self.redis.random_pool();
                    let mut conn: deadpool_redis::Connection = pool.get().await.map_err(|e| EngineError::Redis(e.to_string()))?;
                    let _: () = deadpool_redis::redis::AsyncCommands::lpush(&mut conn, &queue_key, &orphan.task_id)
                        .await
                        .map_err(|e| EngineError::Redis(e.to_string()))?;
                    recovered += 1;
                }
            }

            // 2. Time out stale IN_PROGRESS *worker* tasks and collect affected workflows
            let now = Utc::now();
            let timed_out_workflows: Vec<(String,)> = sqlx::query_as(
                "UPDATE task SET status = 'TIMED_OUT', end_time = $1, update_time = $1, \
                 reason_for_incompletion = 'Task timed out (sweep)' \
                 WHERE status = 'IN_PROGRESS' \
                   AND task_type NOT IN ('FORK','FORK_JOIN','JOIN','DECISION','SWITCH','SUB_WORKFLOW','DO_WHILE','TERMINATE','SET_VARIABLE','WAIT') \
                   AND start_time < NOW() - INTERVAL '10 minutes' \
                 RETURNING workflow_instance_id",
            )
            .bind(now)
            .fetch_all(shard)
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

            if !timed_out_workflows.is_empty() {
                tracing::warn!(count = timed_out_workflows.len(), "Timed out stale IN_PROGRESS tasks");

                // Advance each affected workflow so it detects the timed-out task
                // and either fails the workflow or proceeds (if the task was optional).
                let mut seen = std::collections::HashSet::new();
                for (wf_id,) in &timed_out_workflows {
                    if seen.insert(wf_id.clone()) {
                        if let Err(e) = self.advance_workflow(wf_id).await {
                            tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance after timeout failed");
                        }
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
            .map_err(|e| EngineError::Database(e.to_string()))?;

            for (wf_id,) in &stale_subs {
                if let Err(e) = self.advance_workflow(wf_id).await {
                    tracing::error!(workflow_id = %wf_id, error = %e, "Sweep advance failed");
                }
            }
        }

        if recovered > 0 {
            tracing::info!(count = recovered, "Re-queued orphaned SCHEDULED tasks");
        }

        Ok(recovered)
    }

    /// Starts a background loop that periodically sweeps for orphaned tasks.
    pub fn start_background_sweeper(engine: std::sync::Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            loop {
                interval.tick().await;
                match engine.sweep_orphaned_tasks().await {
                    Ok(_) => {}
                    Err(e) => tracing::error!(error = %e, "Background sweep failed"),
                }
            }
        });
    }
}

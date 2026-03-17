mod admin;
mod advance;
mod error;
mod events;
mod execution;
mod metadata;
mod rows;
pub mod shard;
mod sweeper;
mod system_tasks;
mod task_ops;
mod workflow_ops;

use crate::store::kafka::KafkaTaskQueue;
use crate::store::redis::ShardedRedis;
pub use error::EngineError;
pub use shard::ShardedPool;

use crate::models::*;

fn is_task_terminal(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed
            | TaskStatus::Skipped
            | TaskStatus::CompletedWithErrors
            | TaskStatus::Failed
            | TaskStatus::FailedWithTerminalError
            | TaskStatus::TimedOut
            | TaskStatus::Canceled
    )
}

#[allow(dead_code)]
fn is_task_successful(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed | TaskStatus::Skipped | TaskStatus::CompletedWithErrors
    )
}

fn is_task_failed(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Failed | TaskStatus::FailedWithTerminalError | TaskStatus::TimedOut
    )
}

/// Redis hash key that maps task_id → workflow_instance_id for O(1) shard
/// routing.  Populated at task-queue time, deleted on terminal status.
const TASK_ROUTING_KEY: &str = "conductor:task_routing";

#[derive(Clone)]
pub struct WorkflowEngine {
    shards: ShardedPool,
    redis: ShardedRedis,
    kafka: KafkaTaskQueue,
}

impl WorkflowEngine {
    pub fn new(shards: ShardedPool, redis: ShardedRedis, kafka: KafkaTaskQueue) -> Self {
        Self { shards, redis, kafka }
    }

    // ── Task-shard routing helpers ─────────────────────────────────────

    /// Record task_id → workflow_instance_id in Redis so future lookups can
    /// route to the correct shard without a broadcast fan-out.
    pub(crate) async fn set_task_routing(
        &self,
        task_id: &str,
        workflow_id: &str,
    ) -> Result<(), EngineError> {
        let mut conn = self
            .redis
            .random_pool()
            .get()
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Redis connection failed while setting task routing");
                EngineError::Redis(e.to_string())
            })?;
        let _: () = deadpool_redis::redis::cmd("HSET")
            .arg(TASK_ROUTING_KEY)
            .arg(task_id)
            .arg(workflow_id)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, workflow_id = %workflow_id, error = %e, "Redis HSET failed for task routing");
                EngineError::Redis(e.to_string())
            })?;
        Ok(())
    }

    /// Look up the workflow_instance_id for a task_id via Redis.
    pub(crate) async fn get_task_routing(
        &self,
        task_id: &str,
    ) -> Result<Option<String>, EngineError> {
        let mut conn = self
            .redis
            .random_pool()
            .get()
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Redis connection failed while getting task routing");
                EngineError::Redis(e.to_string())
            })?;
        let wf_id: Option<String> = deadpool_redis::redis::cmd("HGET")
            .arg(TASK_ROUTING_KEY)
            .arg(task_id)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Redis HGET failed for task routing");
                EngineError::Redis(e.to_string())
            })?;
        Ok(wf_id)
    }

    /// Remove the routing entry for a task (call when task reaches terminal).
    pub(crate) async fn delete_task_routing(&self, task_id: &str) -> Result<(), EngineError> {
        let mut conn = self
            .redis
            .random_pool()
            .get()
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Redis connection failed while deleting task routing");
                EngineError::Redis(e.to_string())
            })?;
        let _: () = deadpool_redis::redis::cmd("HDEL")
            .arg(TASK_ROUTING_KEY)
            .arg(task_id)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(task_id = %task_id, error = %e, "Redis HDEL failed for task routing cleanup");
                EngineError::Redis(e.to_string())
            })?;
        Ok(())
    }

    /// Resolve task_id → shard pool.  Tries the Redis routing hash first;
    /// falls back to a single-shard shortcut (if only 1 shard) or a
    /// broadcast fan-out as last resort.
    pub(crate) async fn resolve_task_shard(
        &self,
        task_id: &str,
    ) -> Result<Option<(String, &crate::store::postgres::DbPool)>, EngineError> {
        // Fast path: Redis routing
        if let Some(wf_id) = self.get_task_routing(task_id).await? {
            return Ok(Some((wf_id.clone(), self.shards.shard_for(&wf_id))));
        }
        // Single-shard shortcut
        if self.shards.all_shards().len() == 1 {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT workflow_instance_id FROM task WHERE task_id = $1")
                    .bind(task_id)
                    .fetch_optional(&self.shards.all_shards()[0])
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
            if let Some((wf_id,)) = row {
                // Back-fill routing for next time
                let _ = self.set_task_routing(task_id, &wf_id).await;
                return Ok(Some((wf_id.clone(), self.shards.shard_for(&wf_id))));
            }
            return Ok(None);
        }
        // Fallback: sequential probe (much cheaper than parallel fan-out)
        for shard in self.shards.all_shards() {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT workflow_instance_id FROM task WHERE task_id = $1")
                    .bind(task_id)
                    .fetch_optional(shard)
                    .await
                    .map_err(|e| {
                        tracing::error!(task_id = %task_id, error = %e, "DB error during shard fan-out for task routing");
                        EngineError::Database(e.to_string())
                    })?;
            if let Some((wf_id,)) = row {
                let _ = self.set_task_routing(task_id, &wf_id).await;
                return Ok(Some((wf_id.clone(), self.shards.shard_for(&wf_id))));
            }
        }
        Ok(None)
    }
}

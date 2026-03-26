mod admin;
mod advance;
mod error;
mod events;
mod execution;
mod metadata;
mod rows;
mod scheduler;
pub mod shard;
mod sweeper;
pub(crate) mod system_tasks;
mod task_ops;
mod workflow_ops;

#[cfg(test)]
mod tests;

#[cfg(feature = "kafka")]
use crate::store::kafka::KafkaTaskQueue;
use crate::store::redis::ShardedRedis;
#[cfg(not(feature = "kafka"))]
use crate::store::redis_queue::RedisTaskQueue;
pub use error::EngineError;
pub use shard::ShardedPool;

use crate::models::*;
use std::num::NonZeroUsize;
use std::sync::Mutex;

pub(crate) fn is_task_terminal(status: &TaskStatus) -> bool {
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

#[cfg(test)]
pub(crate) fn is_task_successful(status: &TaskStatus) -> bool {
    matches!(
        status,
        TaskStatus::Completed | TaskStatus::Skipped | TaskStatus::CompletedWithErrors
    )
}

pub(crate) fn is_task_failed(status: &TaskStatus) -> bool {
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
    #[cfg(feature = "kafka")]
    queue: KafkaTaskQueue,
    #[cfg(not(feature = "kafka"))]
    queue: RedisTaskQueue,
    #[cfg(feature = "external-storage")]
    #[allow(dead_code)]
    pub(crate) external_storage: Option<crate::store::s3::ExternalPayloadStorage>,
    /// In-memory LRU cache for workflow definitions.
    wf_def_cache: std::sync::Arc<Mutex<lru::LruCache<(String, i32), WorkflowDef>>>,
    /// In-memory LRU cache for task definitions.
    task_def_cache: std::sync::Arc<Mutex<lru::LruCache<String, TaskDef>>>,
    /// Sweeper configuration.
    pub(crate) sweeper_min_interval_secs: u64,
    pub(crate) sweeper_max_interval_secs: u64,
}

impl WorkflowEngine {
    #[allow(dead_code)]
    pub fn new(
        shards: ShardedPool,
        redis: ShardedRedis,
        #[cfg(feature = "kafka")] queue: KafkaTaskQueue,
        #[cfg(not(feature = "kafka"))] queue: RedisTaskQueue,
        #[cfg(feature = "external-storage")] external_storage: Option<
            crate::store::s3::ExternalPayloadStorage,
        >,
    ) -> Self {
        Self::with_cache_size(
            shards,
            redis,
            #[cfg(feature = "kafka")]
            queue,
            #[cfg(not(feature = "kafka"))]
            queue,
            #[cfg(feature = "external-storage")]
            external_storage,
            1000,
        )
    }

    pub fn with_cache_size(
        shards: ShardedPool,
        redis: ShardedRedis,
        #[cfg(feature = "kafka")] queue: KafkaTaskQueue,
        #[cfg(not(feature = "kafka"))] queue: RedisTaskQueue,
        #[cfg(feature = "external-storage")] external_storage: Option<
            crate::store::s3::ExternalPayloadStorage,
        >,
        cache_max_entries: usize,
    ) -> Self {
        let cap = NonZeroUsize::new(cache_max_entries.max(1)).unwrap();
        Self {
            shards,
            redis,
            queue,
            #[cfg(feature = "external-storage")]
            external_storage,
            wf_def_cache: std::sync::Arc::new(Mutex::new(lru::LruCache::new(cap))),
            task_def_cache: std::sync::Arc::new(Mutex::new(lru::LruCache::new(cap))),
            sweeper_min_interval_secs: 10,
            sweeper_max_interval_secs: 60,
        }
    }

    /// Configure sweeper interval bounds.
    pub fn with_sweeper_config(mut self, min_secs: u64, max_secs: u64) -> Self {
        self.sweeper_min_interval_secs = min_secs;
        self.sweeper_max_interval_secs = max_secs;
        self
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
            .pool_for_key(task_id)
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
            .pool_for_key(task_id)
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
            .pool_for_key(task_id)
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
        if self.shards.read_shards().len() == 1 {
            let row: Option<(String,)> =
                sqlx::query_as("SELECT workflow_instance_id FROM task WHERE task_id = $1")
                    .bind(task_id)
                    .fetch_optional(&self.shards.read_shards()[0])
                    .await
                    .map_err(|e| EngineError::Database(e.to_string()))?;
            if let Some((wf_id,)) = row {
                // Back-fill routing for next time
                let _ = self.set_task_routing(task_id, &wf_id).await;
                return Ok(Some((wf_id.clone(), self.shards.shard_for(&wf_id))));
            }
            return Ok(None);
        }
        // Fallback: parallel fan-out across all shards
        let futs: Vec<_> = self
            .shards
            .read_shards()
            .iter()
            .map(|shard| {
                sqlx::query_as::<_, (String,)>(
                    "SELECT workflow_instance_id FROM task WHERE task_id = $1",
                )
                .bind(task_id)
                .fetch_optional(shard)
            })
            .collect();
        for result in futures::future::join_all(futs).await {
            if let Ok(Some((wf_id,))) = result {
                let _ = self.set_task_routing(task_id, &wf_id).await;
                return Ok(Some((wf_id.clone(), self.shards.shard_for(&wf_id))));
            }
        }
        Ok(None)
    }

    // ── Redis pipeline batching for bulk routing lookups ──────────────

    /// Batch lookup multiple task_ids → workflow_instance_ids via Redis pipeline.
    #[allow(dead_code)]
    pub(crate) async fn batch_get_task_routing(
        &self,
        task_ids: &[String],
    ) -> Result<Vec<Option<String>>, EngineError> {
        if task_ids.is_empty() {
            return Ok(vec![]);
        }
        // Group task_ids by their Redis shard
        let mut shard_groups: std::collections::HashMap<usize, Vec<(usize, &str)>> =
            std::collections::HashMap::new();
        for (idx, tid) in task_ids.iter().enumerate() {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(tid.as_str(), &mut hasher);
            let shard_idx = (std::hash::Hasher::finish(&hasher) as usize) % self.redis.num_shards();
            shard_groups.entry(shard_idx).or_default().push((idx, tid));
        }

        let mut results = vec![None; task_ids.len()];

        for items in shard_groups.values() {
            let pool = self.redis.pool_for_key(items[0].1);
            let mut conn = pool
                .get()
                .await
                .map_err(|e| EngineError::Redis(e.to_string()))?;

            // Build pipeline
            let mut pipe = deadpool_redis::redis::pipe();
            for (_, tid) in items {
                pipe.cmd("HGET").arg(TASK_ROUTING_KEY).arg(*tid);
            }

            let values: Vec<Option<String>> = pipe
                .query_async(&mut *conn)
                .await
                .map_err(|e| EngineError::Redis(e.to_string()))?;

            for (i, (orig_idx, _)) in items.iter().enumerate() {
                if i < values.len() {
                    results[*orig_idx] = values[i].clone();
                }
            }
        }

        Ok(results)
    }

    /// Batch set multiple task_id → workflow_id routing entries via Redis pipeline.
    #[allow(dead_code)]
    pub(crate) async fn batch_set_task_routing(
        &self,
        mappings: &[(&str, &str)],
    ) -> Result<(), EngineError> {
        if mappings.is_empty() {
            return Ok(());
        }
        // Group by Redis shard
        let mut shard_groups: std::collections::HashMap<usize, Vec<(&str, &str)>> =
            std::collections::HashMap::new();
        for &(tid, wid) in mappings {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            std::hash::Hash::hash(tid, &mut hasher);
            let shard_idx = (std::hash::Hasher::finish(&hasher) as usize) % self.redis.num_shards();
            shard_groups.entry(shard_idx).or_default().push((tid, wid));
        }

        for items in shard_groups.values() {
            let pool = self.redis.pool_for_key(items[0].0);
            let mut conn = pool
                .get()
                .await
                .map_err(|e| EngineError::Redis(e.to_string()))?;

            let mut pipe = deadpool_redis::redis::pipe();
            for &(tid, wid) in items {
                pipe.cmd("HSET").arg(TASK_ROUTING_KEY).arg(tid).arg(wid);
            }

            let _: Vec<i64> = pipe
                .query_async(&mut *conn)
                .await
                .map_err(|e| EngineError::Redis(e.to_string()))?;
        }

        Ok(())
    }

    // ── Pool metrics for /metrics endpoint ────────────────────────────

    /// Collect connection pool metrics for all backends.
    pub fn pool_metrics(&self) -> serde_json::Value {
        let pg_metrics: Vec<serde_json::Value> = self
            .shards
            .all_shards()
            .iter()
            .enumerate()
            .map(|(i, pool)| {
                let m = crate::store::postgres::pool_metrics(pool);
                serde_json::json!({
                    "shard": i,
                    "size": m.size,
                    "idle": m.num_idle,
                    "active": m.active,
                })
            })
            .collect();

        let replica_metrics: Vec<serde_json::Value> = if self.shards.has_replicas() {
            self.shards
                .read_shards()
                .iter()
                .enumerate()
                .map(|(i, pool)| {
                    let m = crate::store::postgres::pool_metrics(pool);
                    serde_json::json!({
                        "shard": i,
                        "size": m.size,
                        "idle": m.num_idle,
                        "active": m.active,
                    })
                })
                .collect()
        } else {
            vec![]
        };

        let redis_metrics: Vec<serde_json::Value> = self
            .redis
            .pool_metrics()
            .into_iter()
            .map(|m| {
                serde_json::json!({
                    "shard": m.shard,
                    "size": m.size,
                    "available": m.available,
                    "max_size": m.max_size,
                })
            })
            .collect();

        serde_json::json!({
            "postgres": {
                "primary": pg_metrics,
                "replicas": replica_metrics,
            },
            "redis": redis_metrics,
        })
    }

    // ── LRU cache helpers ─────────────────────────────────────────────

    pub(crate) fn cache_get_workflow_def(&self, name: &str, version: i32) -> Option<WorkflowDef> {
        self.wf_def_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get(&(name.to_string(), version)).cloned())
    }

    pub(crate) fn cache_put_workflow_def(&self, def: &WorkflowDef) {
        if let Ok(mut cache) = self.wf_def_cache.lock() {
            cache.put((def.name.clone(), def.version), def.clone());
        }
    }

    pub(crate) fn cache_invalidate_workflow_def(&self, name: &str, version: i32) {
        if let Ok(mut cache) = self.wf_def_cache.lock() {
            cache.pop(&(name.to_string(), version));
        }
    }

    pub(crate) fn cache_get_task_def(&self, name: &str) -> Option<TaskDef> {
        self.task_def_cache
            .lock()
            .ok()
            .and_then(|mut cache| cache.get(&name.to_string()).cloned())
    }

    pub(crate) fn cache_put_task_def(&self, def: &TaskDef) {
        if let Ok(mut cache) = self.task_def_cache.lock() {
            cache.put(def.name.clone(), def.clone());
        }
    }

    pub(crate) fn cache_invalidate_task_def(&self, name: &str) {
        if let Ok(mut cache) = self.task_def_cache.lock() {
            cache.pop(&name.to_string());
        }
    }
}

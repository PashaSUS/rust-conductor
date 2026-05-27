use deadpool_redis::redis;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::sync::RwLock;

use super::redis::ShardedRedis;

const CONSUMER_GROUP: &str = "conductor-workers";

/// Per-process consumer name. Multi-replica deployments MUST use distinct
/// consumer names within a Redis Streams consumer group — if every replica
/// uses the same name they all share a single virtual consumer slot and
/// XREADGROUP only delivers a message to one of them at a time, with
/// pending-list contention dragging throughput to a fraction of single-node
/// performance. We derive the name from `HOSTNAME` (set per-container by
/// Docker / Kubernetes) plus the OS PID, so each backend replica has a
/// unique consumer in the group.
fn consumer_name() -> &'static str {
    static NAME: OnceLock<String> = OnceLock::new();
    NAME.get_or_init(|| {
        let host = std::env::var("HOSTNAME")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "worker".to_string());
        format!("{host}-{}", std::process::id())
    })
}

/// Redis Streams-backed task queue.  Drop-in replacement for `KafkaTaskQueue`
/// when the `kafka` feature is disabled.
///
/// Each task type maps to a Redis stream keyed `conductor:queue:{task_type}`.
/// Uses `XADD` / `XREADGROUP` / `XACK` for at-least-once delivery identical
/// to the Kafka implementation.
#[derive(Clone)]
pub struct RedisTaskQueue {
    redis: ShardedRedis,
    /// Tracks which consumer groups we have already created (idempotent, but
    /// avoids a round-trip every enqueue).
    known_groups: Arc<RwLock<HashSet<String>>>,
}

impl RedisTaskQueue {
    pub fn new(redis: ShardedRedis) -> Self {
        Self {
            redis,
            known_groups: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    fn stream_key(task_type: &str) -> String {
        format!("conductor:queue:{task_type}")
    }

    /// Ensure the consumer group exists for a given stream.  No-ops if the
    /// group was already created in this process lifetime.
    async fn ensure_group(&self, task_type: &str) -> Result<(), String> {
        {
            let known = self.known_groups.read().await;
            if known.contains(task_type) {
                return Ok(());
            }
        }

        let key = Self::stream_key(task_type);
        let pool = self.redis.pool_for_key(&key);
        let mut conn = pool.get().await.map_err(|e| e.to_string())?;

        // XGROUP CREATE <key> <group> 0 MKSTREAM — idempotent via BUSYGROUP
        let res: Result<String, redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&key)
            .arg(CONSUMER_GROUP)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut *conn)
            .await;

        match res {
            Ok(_) => {}
            Err(e) if e.to_string().contains("BUSYGROUP") => { /* already exists */ }
            Err(e) => return Err(e.to_string()),
        }

        self.known_groups
            .write()
            .await
            .insert(task_type.to_string());
        Ok(())
    }

    /// Enqueue a task_id onto the Redis stream for the given task type.
    pub async fn enqueue(&self, task_type: &str, task_id: &str) -> Result<(), String> {
        self.ensure_group(task_type).await?;

        let key = Self::stream_key(task_type);
        let pool = self.redis.pool_for_key(&key);
        let mut conn = pool.get().await.map_err(|e| e.to_string())?;

        let _: String = redis::cmd("XADD")
            .arg(&key)
            .arg("*") // auto-generate stream ID
            .arg("task_id")
            .arg(task_id)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(
                    task_type = %task_type,
                    task_id = %task_id,
                    error = %e,
                    "Redis XADD failed"
                );
                e.to_string()
            })?;

        Ok(())
    }

    /// Produce an arbitrary message to a given Redis stream (topic).
    pub async fn produce(&self, topic: &str, payload: &str) -> Result<(), String> {
        // For produce we use the topic directly as a stream key
        let key = format!("conductor:events:{topic}");
        let pool = self.redis.pool_for_key(&key);
        let mut conn = pool.get().await.map_err(|e| e.to_string())?;

        // Ensure consumer group for event streams too
        let res: Result<String, redis::RedisError> = redis::cmd("XGROUP")
            .arg("CREATE")
            .arg(&key)
            .arg(CONSUMER_GROUP)
            .arg("0")
            .arg("MKSTREAM")
            .query_async(&mut *conn)
            .await;
        match res {
            Ok(_) | Err(_) => {} // best-effort for event streams
        }

        let _: String = redis::cmd("XADD")
            .arg(&key)
            .arg("*")
            .arg("payload")
            .arg(payload)
            .query_async(&mut *conn)
            .await
            .map_err(|e| {
                tracing::error!(topic = %topic, error = %e, "Redis XADD (produce) failed");
                e.to_string()
            })?;
        Ok(())
    }

    /// Dequeue a single task_id from the Redis stream for the given task type.
    /// Returns `None` if no message is available within the timeout.
    pub async fn dequeue(&self, task_type: &str) -> Option<String> {
        self.ensure_group(task_type).await.ok()?;

        let key = Self::stream_key(task_type);
        let pool = self.redis.pool_for_key(&key);
        let mut conn = pool.get().await.ok()?;

        // XREADGROUP GROUP <group> <consumer> COUNT 1 BLOCK <ms> STREAMS <key> >
        let result: Result<redis::Value, _> = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(CONSUMER_GROUP)
            .arg(consumer_name())
            .arg("COUNT")
            .arg(1)
            .arg("BLOCK")
            .arg(500) // 500ms timeout, same as Kafka impl
            .arg("STREAMS")
            .arg(&key)
            .arg(">")
            .query_async(&mut *conn)
            .await;

        match result {
            Ok(redis::Value::Array(streams)) => {
                // Response: [[stream_key, [[msg_id, [field, value, ...]]]]]
                self.extract_and_ack(&mut conn, &key, &streams, task_type)
                    .await
            }
            _ => None,
        }
    }

    /// Dequeue up to `count` task_ids from the stream, waiting up to `timeout`.
    pub async fn batch_dequeue(
        &self,
        task_type: &str,
        count: usize,
        timeout: Duration,
    ) -> Vec<String> {
        if self.ensure_group(task_type).await.is_err() {
            return vec![];
        }

        let key = Self::stream_key(task_type);
        let pool = self.redis.pool_for_key(&key);
        let mut conn = match pool.get().await {
            Ok(c) => c,
            Err(_) => return vec![],
        };

        let block_ms = timeout.as_millis().min(u64::MAX as u128) as u64;

        let result: Result<redis::Value, _> = redis::cmd("XREADGROUP")
            .arg("GROUP")
            .arg(CONSUMER_GROUP)
            .arg(consumer_name())
            .arg("COUNT")
            .arg(count)
            .arg("BLOCK")
            .arg(block_ms)
            .arg("STREAMS")
            .arg(&key)
            .arg(">")
            .query_async(&mut *conn)
            .await;

        match result {
            Ok(redis::Value::Array(streams)) => {
                self.extract_batch_and_ack(&mut conn, &key, &streams, task_type)
                    .await
            }
            _ => vec![],
        }
    }

    /// Check that the Redis backend is reachable (always true if we got here,
    /// but we do a PING for parity with the Kafka health check).
    pub async fn health_check(&self) -> bool {
        let pool = self.redis.random_pool();
        match pool.get().await {
            Ok(mut conn) => {
                let res: Result<String, _> = redis::cmd("PING").query_async(&mut *conn).await;
                res.is_ok()
            }
            Err(_) => false,
        }
    }

    // ── Internal helpers ───────────────────────────────────────────────

    /// Extract a single task_id from XREADGROUP response and ACK the message.
    async fn extract_and_ack(
        &self,
        conn: &mut deadpool_redis::Connection,
        key: &str,
        streams: &[redis::Value],
        task_type: &str,
    ) -> Option<String> {
        // streams = [ [key, [ [msg_id, [field, value, ...]] ] ] ]
        let stream_data = match streams.first() {
            Some(redis::Value::Array(inner)) => inner,
            _ => return None,
        };
        let messages = match stream_data.get(1) {
            Some(redis::Value::Array(msgs)) => msgs,
            _ => return None,
        };
        let msg = match messages.first() {
            Some(redis::Value::Array(m)) => m,
            _ => return None,
        };
        let msg_id = match msg.first() {
            Some(redis::Value::BulkString(id)) => String::from_utf8_lossy(id).to_string(),
            _ => return None,
        };
        let task_id = self.extract_task_id_from_fields(msg.get(1))?;

        // ACK
        let _: Result<i64, _> = redis::cmd("XACK")
            .arg(key)
            .arg(CONSUMER_GROUP)
            .arg(&msg_id)
            .query_async(conn)
            .await;

        tracing::debug!(task_type = %task_type, task_id = %task_id, "Redis stream dequeue success");
        Some(task_id)
    }

    /// Extract multiple task_ids from XREADGROUP response and ACK all messages.
    async fn extract_batch_and_ack(
        &self,
        conn: &mut deadpool_redis::Connection,
        key: &str,
        streams: &[redis::Value],
        task_type: &str,
    ) -> Vec<String> {
        let stream_data = match streams.first() {
            Some(redis::Value::Array(inner)) => inner,
            _ => return vec![],
        };
        let messages = match stream_data.get(1) {
            Some(redis::Value::Array(msgs)) => msgs,
            _ => return vec![],
        };

        let mut results = Vec::with_capacity(messages.len());
        let mut msg_ids: Vec<String> = Vec::with_capacity(messages.len());

        for entry in messages {
            if let redis::Value::Array(m) = entry {
                if let Some(redis::Value::BulkString(id)) = m.first() {
                    let msg_id = String::from_utf8_lossy(id).to_string();
                    if let Some(task_id) = self.extract_task_id_from_fields(m.get(1)) {
                        results.push(task_id);
                        msg_ids.push(msg_id);
                    }
                }
            }
        }

        // Batch ACK
        if !msg_ids.is_empty() {
            let mut cmd = redis::cmd("XACK");
            cmd.arg(key).arg(CONSUMER_GROUP);
            for id in &msg_ids {
                cmd.arg(id);
            }
            let _: Result<i64, _> = cmd.query_async(conn).await;
            tracing::debug!(
                task_type = %task_type,
                count = results.len(),
                "Redis stream batch dequeue success"
            );
        }

        results
    }

    /// Parse field-value pairs from a stream message to find the `task_id` value.
    fn extract_task_id_from_fields(&self, fields: Option<&redis::Value>) -> Option<String> {
        match fields {
            Some(redis::Value::Array(pairs)) => {
                // pairs = [field1, value1, field2, value2, ...]
                let mut iter = pairs.iter();
                while let Some(field) = iter.next() {
                    let val = iter.next()?;
                    if let redis::Value::BulkString(f) = field {
                        if f == b"task_id" {
                            if let redis::Value::BulkString(v) = val {
                                return Some(String::from_utf8_lossy(v).to_string());
                            }
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }
}

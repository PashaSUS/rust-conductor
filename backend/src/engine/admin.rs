use std::collections::HashMap;

use futures::future::join_all;
use serde_json::Value;

use super::WorkflowEngine;
use super::error::EngineError;
use super::rows::{ConfigRow, TaskRow};
use crate::models::*;

impl WorkflowEngine {
    // ── Queue Admin ──

    pub async fn pause_queue(&self, queue_name: &str) -> Result<(), EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool
            .get()
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        let _: () = deadpool_redis::redis::AsyncCommands::set(&mut conn, &key, "true")
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(())
    }

    pub async fn resume_queue(&self, queue_name: &str) -> Result<(), EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool
            .get()
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        let _: () = deadpool_redis::redis::AsyncCommands::del(&mut conn, &key)
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(())
    }

    pub async fn is_queue_paused(&self, queue_name: &str) -> Result<bool, EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool
            .get()
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        let exists: bool = deadpool_redis::redis::AsyncCommands::exists(&mut conn, &key)
            .await
            .map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(exists)
    }

    // ── Config ──

    pub async fn get_all_config(&self) -> Result<HashMap<String, Value>, EngineError> {
        let rows = sqlx::query_as::<_, ConfigRow>("SELECT key, value FROM config ORDER BY key")
            .fetch_all(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        Ok(rows.into_iter().map(|r| (r.key, r.value)).collect())
    }

    pub async fn sweep_workflow(&self, workflow_id: &str) -> Result<(), EngineError> {
        self.decide_workflow(workflow_id).await
    }

    // ── Health ──

    pub async fn health_check(&self) -> Result<HealthCheckStatus, EngineError> {
        // Check all shards in parallel
        let shard_futs: Vec<_> = self
            .shards
            .read_shards()
            .iter()
            .enumerate()
            .map(|(i, shard)| async move {
                let ok = sqlx::query("SELECT 1").execute(shard).await.is_ok();

                Health {
                    healthy: ok,
                    error_message: if ok {
                        None
                    } else {
                        Some(format!("Shard {} connection failed", i))
                    },
                    details: {
                        let mut m = HashMap::new();
                        m.insert(
                            "name".into(),
                            Value::String(format!("postgres-shard-{}", i)),
                        );
                        m
                    },
                }
            })
            .collect();

        let mut health_results = join_all(shard_futs).await;

        let pool = self.redis.random_pool();
        let redis_ok = pool.get().await.map(|_| true).unwrap_or(false);

        health_results.push(Health {
            healthy: redis_ok,
            error_message: if redis_ok {
                None
            } else {
                Some("Redis connection failed".into())
            },
            details: {
                let mut m = HashMap::new();
                m.insert("name".into(), Value::String("redis".into()));
                m
            },
        });

        let queue_ok = self.queue.health_check().await;

        health_results.push(Health {
            healthy: queue_ok,
            error_message: if queue_ok {
                None
            } else {
                Some("Task queue connection failed".into())
            },
            details: {
                let mut m = HashMap::new();
                #[cfg(feature = "kafka")]
                m.insert("name".into(), Value::String("kafka".into()));
                #[cfg(not(feature = "kafka"))]
                m.insert("name".into(), Value::String("redis-streams".into()));
                m
            },
        });

        let all_healthy = health_results.iter().all(|h| h.healthy);

        Ok(HealthCheckStatus {
            healthy: all_healthy,
            health_results,
            suppressed_health_results: vec![],
        })
    }

    #[allow(dead_code)]
    pub(crate) fn is_system_task_type(task_type: &str) -> bool {
        matches!(
            task_type,
            "FORK"
                | "FORK_JOIN"
                | "JOIN"
                | "DECISION"
                | "SWITCH"
                | "SUB_WORKFLOW"
                | "DO_WHILE"
                | "TERMINATE"
                | "SET_VARIABLE"
                | "WAIT"
                | "HTTP"
                | "EVENT"
        )
    }

    // ── Admin task operations ──

    pub async fn get_tasks_for_type(
        &self,
        task_type: &str,
    ) -> Result<Vec<TaskResult>, EngineError> {
        let futs: Vec<_> = self.shards.read_shards().iter().map(|shard| {
            async move {
                sqlx::query_as::<_, TaskRow>(
                    "SELECT * FROM task WHERE task_def_name = $1 ORDER BY scheduled_time DESC LIMIT 100",
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

    pub async fn requeue_pending_tasks(&self, task_type: &str) -> Result<i64, EngineError> {
        let futs: Vec<_> = self.shards.all_shards().iter().map(|shard| {
            async move {
                let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
                    "SELECT task_id, workflow_instance_id, domain FROM task WHERE task_def_name = $1 AND status = 'SCHEDULED'",
                )
                .bind(task_type)
                .fetch_all(shard)
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;
                Ok::<_, EngineError>(rows)
            }
        }).collect();

        let results = join_all(futs).await;
        let mut count: i64 = 0;
        for result in results {
            for (task_id, workflow_id, domain) in result? {
                self.set_task_routing(&task_id, &workflow_id).await?;
                let queue_name = super::queue_name_for(task_type, domain.as_deref());
                self.queue
                    .enqueue(&queue_name, &task_id)
                    .await
                    .map_err(EngineError::Redis)?;
                count += 1;
            }
        }
        Ok(count)
    }
}

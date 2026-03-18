use std::collections::HashMap;

use futures::future::join_all;
use serde_json::Value;

use super::error::EngineError;
use super::rows::ConfigRow;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    // ── Queue Admin ──

    pub async fn pause_queue(&self, queue_name: &str) -> Result<(), EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool.get().await.map_err(|e| EngineError::Redis(e.to_string()))?;
        let _: () = deadpool_redis::redis::AsyncCommands::set(&mut conn, &key, "true").await.map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(())
    }

    pub async fn resume_queue(&self, queue_name: &str) -> Result<(), EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool.get().await.map_err(|e| EngineError::Redis(e.to_string()))?;
        let _: () = deadpool_redis::redis::AsyncCommands::del(&mut conn, &key).await.map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(())
    }

    pub async fn is_queue_paused(&self, queue_name: &str) -> Result<bool, EngineError> {
        let key = format!("conductor:queue:paused:{queue_name}");
        let pool = self.redis.random_pool();
        let mut conn = pool.get().await.map_err(|e| EngineError::Redis(e.to_string()))?;
        let exists: bool = deadpool_redis::redis::AsyncCommands::exists(&mut conn, &key).await.map_err(|e| EngineError::Redis(e.to_string()))?;
        Ok(exists)
    }

    // ── Config ──

    pub async fn get_all_config(&self) -> Result<HashMap<String, Value>, EngineError> {
        let rows = sqlx::query_as::<_, ConfigRow>(
            "SELECT key, value FROM config ORDER BY key",
        )
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
        let shard_futs: Vec<_> = self.shards.all_shards().iter().enumerate().map(|(i, shard)| async move {
            let ok = sqlx::query("SELECT 1")
                .execute(shard)
                .await
                .is_ok();

            Health {
                healthy: ok,
                error_message: if ok { None } else { Some(format!("Shard {} connection failed", i)) },
                details: {
                    let mut m = HashMap::new();
                    m.insert("name".into(), Value::String(format!("postgres-shard-{}", i)));
                    m
                },
            }
        }).collect();

        let mut health_results = join_all(shard_futs).await;

        let pool = self.redis.random_pool();
        let redis_ok = pool.get().await.map(|_| true).unwrap_or(false);

        health_results.push(Health {
            healthy: redis_ok,
            error_message: if redis_ok { None } else { Some("Redis connection failed".into()) },
            details: {
                let mut m = HashMap::new();
                m.insert("name".into(), Value::String("redis".into()));
                m
            },
        });

        let kafka_ok = self.kafka.health_check().await;

        health_results.push(Health {
            healthy: kafka_ok,
            error_message: if kafka_ok { None } else { Some("Kafka connection failed".into()) },
            details: {
                let mut m = HashMap::new();
                m.insert("name".into(), Value::String("kafka".into()));
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
            "FORK" | "FORK_JOIN" | "JOIN" | "DECISION" | "SWITCH" | "SUB_WORKFLOW"
                | "DO_WHILE" | "TERMINATE" | "SET_VARIABLE" | "WAIT" | "HTTP" | "EVENT"
        )
    }
}

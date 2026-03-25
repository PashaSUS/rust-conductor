use deadpool_redis::{Config, Pool, Runtime};
use rand::seq::IndexedRandom;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::time::Duration;

pub type RedisPool = Pool;

/// A sharded Redis pool. Each pool is a deadpool_redis::Pool.
#[derive(Clone)]
pub struct ShardedRedis {
    pools: Arc<Vec<RedisPool>>,
}

impl ShardedRedis {
    /// Create a sharded Redis pool from a comma-separated list of URLs (async)
    pub async fn new_async(redis_urls: &[String]) -> anyhow::Result<ShardedRedis> {
        assert!(!redis_urls.is_empty(), "At least one Redis URL required");

        tracing::info!(
            num_shards = redis_urls.len(),
            "Initializing sharded Redis pool"
        );
        tracing::info!(urls = ?redis_urls, "Redis shard URLs");

        let mut pools: Vec<RedisPool> = Vec::with_capacity(redis_urls.len());
        for url in redis_urls {
            let mut cfg: Config = Config::from_url(url);
            cfg.pool = Some(deadpool_redis::PoolConfig {
                max_size: 128,
                timeouts: deadpool_redis::Timeouts {
                    wait: Some(Duration::from_secs(10)),
                    create: Some(Duration::from_secs(20)),
                    recycle: Some(Duration::from_secs(5)),
                },
                ..Default::default()
            });

            // Create the async pool
            let pool: RedisPool = cfg.create_pool(Some(Runtime::Tokio1))?;

            // Optional: test connection immediately
            let mut conn: deadpool_redis::Connection = pool.get().await?;
            let pong: String = redis::cmd("PING").query_async(&mut conn).await?;
            tracing::info!(url, pong, "Redis shard alive");

            pools.push(pool);
        }

        Ok(ShardedRedis {
            pools: Arc::new(pools),
        })
    }

    /// Deterministically route a key to a specific Redis shard.
    /// This ensures HSET and HGET for the same key always hit the same shard.
    pub fn pool_for_key(&self, key: &str) -> &RedisPool {
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let idx = (hasher.finish() as usize) % self.pools.len();
        &self.pools[idx]
    }

    /// Randomly pick a pool for operations that don't need deterministic routing.
    pub fn random_pool(&self) -> &RedisPool {
        let pools = &self.pools;
        let mut rng = rand::rng();
        pools.choose(&mut rng).unwrap_or(&pools[0])
    }

    /// Number of Redis shards.
    pub fn num_shards(&self) -> usize {
        self.pools.len()
    }

    /// Pool metrics for each shard.
    pub fn pool_metrics(&self) -> Vec<RedisPoolMetrics> {
        self.pools
            .iter()
            .enumerate()
            .map(|(i, pool)| {
                let status = pool.status();
                RedisPoolMetrics {
                    shard: i,
                    size: status.size,
                    available: status.available,
                    max_size: status.max_size,
                }
            })
            .collect()
    }
}

/// Pool metrics for a single Redis shard.
pub struct RedisPoolMetrics {
    pub shard: usize,
    pub size: usize,
    pub available: usize,
    pub max_size: usize,
}

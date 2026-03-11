use deadpool_redis::{Config, Pool, Runtime};
use rand::{seq::SliceRandom, thread_rng};
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

    /// Randomly pick a pool for a push (to spread hot queues across all shards).
    pub fn random_pool(&self) -> &RedisPool {
        let pools = &self.pools;
        let mut rng = thread_rng();
        pools.choose(&mut rng).unwrap_or(&pools[0])
    }

    /// For polling: return all pools in random order (to balance pop load).
    pub fn all_pools_shuffled(&self) -> Vec<&RedisPool> {
        let mut idxs: Vec<_> = (0..self.pools.len()).collect();
        let mut rng = thread_rng();
        idxs.shuffle(&mut rng);
        idxs.into_iter().map(|i| &self.pools[i]).collect()
    }
}

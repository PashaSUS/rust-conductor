pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub grpc_port: u16,
    pub redis_urls: String,
    #[cfg(feature = "kafka")]
    pub kafka_brokers: String,
    pub cors_origin: Option<String>,
    /// Resolved list of shard database URLs (one per shard).
    pub shard_database_urls: Vec<String>,
    /// Optional read-replica URLs (one per shard, parallel to shard_database_urls).
    pub replica_database_urls: Vec<String>,
    /// Whether external payload storage (S3/MinIO) is configured.
    #[cfg(feature = "external-storage")]
    pub external_storage_enabled: bool,
    /// Slow query logging threshold in milliseconds (default 500).
    pub slow_query_threshold_ms: u64,
    /// Maximum entries in the in-memory LRU definition cache (default 1000).
    pub def_cache_max_entries: usize,
    /// Minimum sweeper interval in seconds (default 10).
    pub sweeper_min_interval_secs: u64,
    /// Maximum sweeper interval in seconds (default 60).
    pub sweeper_max_interval_secs: u64,
    /// Whether rate limiting is enabled (default true). Set RATE_LIMIT_ENABLED=false to disable.
    pub rate_limit_enabled: bool,
    /// Max requests per window for rate limiting (default 1000).
    pub rate_limit_max_requests: u64,
    /// Rate limit window in seconds (default 60).
    pub rate_limit_window_secs: u64,
}

impl AppConfig {
    pub fn from_env() -> Self {
        dotenvy::dotenv().ok();

        // Resolution order:
        // 1. SHARD_DATABASE_URLS = "url1,url2,..."   (explicit comma-separated list)
        // 2. NUM_SHARDS + SHARD_DB_URL_TEMPLATE      (auto-generate from template)
        //    Template uses `{i}` as the shard index placeholder, e.g.
        //    "postgres://conductor:conductor@pgbouncer-shard-{i}:6432/conductor"
        // 3. DATABASE_URL                             (single-shard fallback)
        let shard_database_urls = if let Ok(urls) = std::env::var("SHARD_DATABASE_URLS") {
            urls.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else if let Ok(n) = std::env::var("NUM_SHARDS") {
            let num: usize = n.parse().expect("NUM_SHARDS must be a positive integer");
            assert!(num > 0, "NUM_SHARDS must be >= 1");
            let template = std::env::var("SHARD_DB_URL_TEMPLATE").unwrap_or_else(|_| {
                "postgres://conductor:conductor@localhost:6432/conductor".into()
            });
            (0..num)
                .map(|i| template.replace("{i}", &i.to_string()))
                .collect()
        } else {
            vec![std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://conductor:conductor@localhost:5432/conductor".into()
            })]
        };

        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "localhost".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8090),
            grpc_port: std::env::var("GRPC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(50055),
            redis_urls: std::env::var("REDIS_URLS")
                .unwrap_or_else(|_| "redis://localhost:6379".into()),
            #[cfg(feature = "kafka")]
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".into()),
            cors_origin: std::env::var("CORS_ORIGIN").ok(),
            shard_database_urls,
            replica_database_urls: std::env::var("SHARD_REPLICA_DB_URLS")
                .map(|urls| {
                    urls.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default(),
            #[cfg(feature = "external-storage")]
            external_storage_enabled: std::env::var("S3_ENDPOINT").is_ok(),
            slow_query_threshold_ms: std::env::var("SLOW_QUERY_THRESHOLD_MS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            def_cache_max_entries: std::env::var("DEF_CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            sweeper_min_interval_secs: std::env::var("SWEEPER_MIN_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            sweeper_max_interval_secs: std::env::var("SWEEPER_MAX_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
            rate_limit_enabled: std::env::var("RATE_LIMIT_ENABLED")
                .map(|v| v != "false" && v != "0")
                .unwrap_or(true),
            rate_limit_max_requests: std::env::var("RATE_LIMIT_MAX_REQUESTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000),
            rate_limit_window_secs: std::env::var("RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(60),
        }
    }
}

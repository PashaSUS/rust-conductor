use serde::Deserialize;

/// Optional file-based configuration. All fields are optional; values not
/// present here fall through to environment variables, then to defaults.
/// Loaded from the path given by `CONFIG_FILE` (default
/// `/etc/rust-conductor/config.json`). When the file is absent, behaviour is
/// unchanged from the pure-env-var setup.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct FileConfig {
    pub host: Option<String>,
    pub port: Option<u16>,
    pub grpc_port: Option<u16>,
    pub redis_urls: Option<String>,
    pub kafka_brokers: Option<String>,
    pub cors_origin: Option<String>,
    pub slow_query_threshold_ms: Option<u64>,
    pub def_cache_max_entries: Option<usize>,
    pub sweeper_min_interval_secs: Option<u64>,
    pub sweeper_max_interval_secs: Option<u64>,
    #[serde(rename = "rateLimit")]
    pub rate_limit: Option<RateLimitConfig>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct RateLimitConfig {
    pub enabled: Option<bool>,
    pub max_requests: Option<u64>,
    pub window_secs: Option<u64>,
}

impl FileConfig {
    /// Load JSON config from `CONFIG_FILE` (default
    /// `/etc/rust-conductor/config.json`). Returns `Default::default()` when
    /// the file is missing or unreadable. Logs a warning on parse error so
    /// the process still starts on a typo.
    fn load() -> Self {
        let path = std::env::var("CONFIG_FILE")
            .unwrap_or_else(|_| "/etc/rust-conductor/config.json".to_string());
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<FileConfig>(&s) {
                Ok(c) => {
                    eprintln!("config: loaded {path}");
                    c
                }
                Err(e) => {
                    eprintln!("config: failed to parse {path}: {e}; using env/defaults");
                    Self::default()
                }
            },
            Err(_) => Self::default(),
        }
    }
}

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
        let file = FileConfig::load();
        let rl = file.rate_limit.unwrap_or_default();

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
            host: std::env::var("HOST")
                .ok()
                .or(file.host)
                .unwrap_or_else(|| "localhost".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .or(file.port)
                .unwrap_or(8090),
            grpc_port: std::env::var("GRPC_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .or(file.grpc_port)
                .unwrap_or(50055),
            redis_urls: std::env::var("REDIS_URLS")
                .ok()
                .or(file.redis_urls)
                .unwrap_or_else(|| "redis://localhost:6379".into()),
            #[cfg(feature = "kafka")]
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .ok()
                .or(file.kafka_brokers)
                .unwrap_or_else(|| "localhost:9092".into()),
            cors_origin: std::env::var("CORS_ORIGIN").ok().or(file.cors_origin),
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
                .or(file.slow_query_threshold_ms)
                .unwrap_or(500),
            def_cache_max_entries: std::env::var("DEF_CACHE_MAX_ENTRIES")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(file.def_cache_max_entries)
                .unwrap_or(1000),
            sweeper_min_interval_secs: std::env::var("SWEEPER_MIN_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(file.sweeper_min_interval_secs)
                .unwrap_or(10),
            sweeper_max_interval_secs: std::env::var("SWEEPER_MAX_INTERVAL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(file.sweeper_max_interval_secs)
                .unwrap_or(60),
            rate_limit_enabled: std::env::var("RATE_LIMIT_ENABLED")
                .ok()
                .map(|v| v != "false" && v != "0")
                .or(rl.enabled)
                .unwrap_or(true),
            rate_limit_max_requests: std::env::var("RATE_LIMIT_MAX_REQUESTS")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(rl.max_requests)
                .unwrap_or(1000),
            rate_limit_window_secs: std::env::var("RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .or(rl.window_secs)
                .unwrap_or(60),
        }
    }
}

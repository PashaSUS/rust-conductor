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
    /// Whether external payload storage (S3/MinIO) is configured.
    pub external_storage_enabled: bool,
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
            external_storage_enabled: std::env::var("S3_ENDPOINT").is_ok(),
        }
    }
}

pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub redis_url: String,
    /// Resolved list of shard database URLs (one per shard).
    pub shard_database_urls: Vec<String>,
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
                "postgres://conductor:conductor@pgbouncer-shard-{i}:6432/conductor".into()
            });
            (0..num)
                .map(|i| template.replace("{i}", &i.to_string()))
                .collect()
        } else {
            vec![std::env::var("DATABASE_URL").unwrap_or_else(|_| {
                "postgres://conductor:conductor@postgres:5432/conductor".into()
            })]
        };

        Self {
            host: std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into()),
            port: std::env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8080),
            redis_url: std::env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://redis:6379".into()),
            shard_database_urls,
        }
    }
}

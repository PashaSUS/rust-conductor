mod api;
mod config;
mod engine;
mod grpc;
mod models;
#[cfg(feature = "seq")]
mod seq;
mod store;
mod swagger;

use actix_cors::Cors;
use actix_web::{App, HttpServer, middleware, web};
use tracing_actix_web::TracingLogger;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use utoipa_swagger_ui::SwaggerUi;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "info".into());

    let fmt_layer = tracing_subscriber::fmt::layer();

    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer);

    // If SEQ_URL is set, also ship events to Seq
    #[cfg(feature = "seq")]
    {
        if let Ok(seq_url) = std::env::var("SEQ_URL") {
            let api_key = std::env::var("SEQ_API_KEY").ok();
            let seq_layer = seq::SeqLayer::new(seq_url.clone(), api_key);
            registry.with(seq_layer).init();
            tracing::info!(seq_url = %seq_url, "Seq logging enabled");
        } else {
            registry.init();
        }
    }
    #[cfg(not(feature = "seq"))]
    {
        registry.init();
    }

    let cfg = config::AppConfig::from_env();
    // Initialize sharded Redis pool from comma-separated URLs
    let redis_urls: Vec<String> = cfg
        .redis_urls
        .split(',')
        .map(|s| s.trim().to_string())
        .collect();

    let redis_pool = store::redis::ShardedRedis::new_async(&redis_urls)
        .await
        .expect("Failed to create sharded Redis pool");

    let skip_migrations = std::env::var("SKIP_MIGRATIONS").unwrap_or_default() == "true";
    let migrate_only = std::env::var("MIGRATE_ONLY").unwrap_or_default() == "true";

    // Create shard pools — one per database URL
    let mut shard_pools = Vec::with_capacity(cfg.shard_database_urls.len());
    for (i, url) in cfg.shard_database_urls.iter().enumerate() {
        let pool = store::postgres::create_pool(url).await;
        if !skip_migrations {
            store::postgres::run_migrations(&pool).await;
            tracing::info!(shard = i, "Shard database connected and migrated");
        } else {
            tracing::info!(shard = i, "Shard database connected (migrations skipped)");
        }
        shard_pools.push(pool);
    }
    tracing::info!(num_shards = shard_pools.len(), "Sharded pool initialized");

    if migrate_only {
        tracing::info!("Migrations complete — exiting (MIGRATE_ONLY=true)");
        return Ok(());
    }

    let sharded_pool = engine::ShardedPool::new(shard_pools);

    let kafka = store::kafka::KafkaTaskQueue::new(&cfg.kafka_brokers)
        .expect("Failed to create Kafka task queue");

    let engine = engine::WorkflowEngine::new(sharded_pool.clone(), redis_pool.clone(), kafka);

    // Start background sweeper for orphaned/stale tasks
    engine::WorkflowEngine::start_background_sweeper(std::sync::Arc::new(engine.clone()));

    let openapi = swagger::build_openapi();

    // ── gRPC server (dedicated multi-thread runtime) ──────────────────
    let grpc_engine = std::sync::Arc::new(engine.clone());
    let grpc_addr: std::net::SocketAddr = format!("{}:{}", cfg.host, cfg.grpc_port)
        .parse()
        .expect("Invalid gRPC address");
    tracing::info!("Starting gRPC server on {}", grpc_addr);
    let grpc_router = grpc::grpc_router(grpc_engine);
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(num_cpus::get())
            .enable_all()
            .build()
            .expect("Failed to build gRPC tokio runtime");
        rt.block_on(async move {
            if let Err(e) = grpc_router.serve(grpc_addr).await {
                tracing::error!(error = %e, "gRPC server failed");
            }
        });
    });

    tracing::info!("Starting Rust Conductor on {}:{}", cfg.host, cfg.port);
    tracing::info!(
        "Swagger UI available at http://{}:{}/swagger-ui/",
        cfg.host,
        cfg.port
    );

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        let json_cfg = web::JsonConfig::default().limit(10 * 1024 * 1024); // 10MB payload limit

        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .wrap(middleware::Compress::default())
            .app_data(json_cfg)
            .app_data(web::Data::new(redis_pool.clone()))
            .app_data(web::Data::new(engine.clone()))
            .configure(api::configure)
            .service(
                SwaggerUi::new("/swagger-ui/{_:.*}").url("/api-docs/openapi.json", openapi.clone()),
            )
    })
    .workers(num_cpus::get())
    .keep_alive(std::time::Duration::from_secs(75))
    .client_request_timeout(std::time::Duration::from_secs(60))
    .bind(format!("{}:{}", cfg.host, cfg.port))?
    .run()
    .await
}

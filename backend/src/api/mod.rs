pub mod admin;
pub mod bulk;
pub mod event;
#[cfg(feature = "graphql")]
pub mod graphql;
pub mod metadata;
pub mod proxy;
pub mod rate_limit;
pub mod schedule;
#[cfg(feature = "sse")]
pub mod sse;
pub mod tasks;
#[cfg(feature = "api-v2")]
pub mod v2;
#[cfg(feature = "websocket")]
pub mod websocket;
pub mod workflow;

use crate::engine::WorkflowEngine;
use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(metadata::configure)
            .configure(proxy::configure)
            .configure(bulk::configure)
            .configure(workflow::configure)
            .configure(tasks::configure)
            .configure(event::configure)
            .configure(admin::configure)
            .configure(schedule::configure)
            .configure(|_c| {
                #[cfg(feature = "graphql")]
                graphql::configure(_c);
                #[cfg(feature = "sse")]
                sse::configure(_c);
                #[cfg(feature = "websocket")]
                websocket::configure(_c);
                #[cfg(feature = "api-v2")]
                {
                    v2::configure_v2(_c);
                    v2::configure_batch(_c);
                }
            }),
    )
    .route("/health", web::get().to(health));
}

async fn health(engine: web::Data<WorkflowEngine>) -> actix_web::HttpResponse {
    match engine.health_check().await {
        Ok(status) => actix_web::HttpResponse::Ok().json(status),
        Err(_) => actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "healthy": false,
        })),
    }
}

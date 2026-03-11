pub mod admin;
pub mod bulk;
pub mod event;
pub mod metadata;
pub mod tasks;
pub mod workflow;

use actix_web::web;
use crate::engine::WorkflowEngine;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .configure(metadata::configure)
            .configure(bulk::configure)
            .configure(workflow::configure)
            .configure(tasks::configure)
            .configure(event::configure)
            .configure(admin::configure),
    )
    .route("/health", web::get().to(health));
}

async fn health(
    engine: web::Data<WorkflowEngine>,
) -> actix_web::HttpResponse {
    match engine.health_check().await {
        Ok(status) => actix_web::HttpResponse::Ok().json(status),
        Err(_) => actix_web::HttpResponse::ServiceUnavailable().json(serde_json::json!({
            "healthy": false,
        })),
    }
}

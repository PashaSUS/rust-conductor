use actix_web::{web, HttpResponse};

use crate::engine::WorkflowEngine;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/workflow/bulk")
            .route("/pause", web::put().to(bulk_pause))
            .route("/resume", web::put().to(bulk_resume))
            .route("/retry", web::post().to(bulk_retry))
            .route("/restart", web::post().to(bulk_restart))
            .route("/terminate", web::post().to(bulk_terminate)),
    );
}

async fn bulk_pause(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<String>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.bulk_pause(&body).await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn bulk_resume(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<String>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.bulk_resume(&body).await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn bulk_retry(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<String>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.bulk_retry(&body).await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn bulk_restart(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<String>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.bulk_restart(&body).await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn bulk_terminate(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<BulkTerminateRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine
        .bulk_terminate(&body.workflow_ids, body.reason.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BulkTerminateRequest {
    workflow_ids: Vec<String>,
    reason: Option<String>,
}

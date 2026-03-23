use actix_web::{web, HttpResponse};

use crate::engine::WorkflowEngine;
use crate::models::ScheduledWorkflow;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/schedule")
            .route("", web::post().to(create_schedule))
            .route("", web::get().to(list_schedules))
            .route("/{scheduleId}", web::get().to(get_schedule))
            .route("/{scheduleId}", web::delete().to(delete_schedule))
            .route("/{scheduleId}/enable", web::put().to(enable_schedule))
            .route("/{scheduleId}/disable", web::put().to(disable_schedule)),
    );
}

async fn create_schedule(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<ScheduledWorkflow>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.create_schedule(&body).await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn list_schedules(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let schedules = engine.list_schedules().await?;
    Ok(HttpResponse::Ok().json(schedules))
}

async fn get_schedule(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let schedule = engine.get_schedule(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(schedule))
}

async fn delete_schedule(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.delete_schedule(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn enable_schedule(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.toggle_schedule(&path.into_inner(), true).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn disable_schedule(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.toggle_schedule(&path.into_inner(), false).await?;
    Ok(HttpResponse::Ok().finish())
}

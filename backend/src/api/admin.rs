use actix_web::{HttpResponse, web};

use crate::engine::WorkflowEngine;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/admin")
            .route("/config", web::get().to(get_all_config))
            .route("/sweep/{workflowId}", web::post().to(sweep_workflow))
            .route("/task/{taskType}", web::get().to(get_tasks_for_type))
            .route(
                "/task/{taskType}/requeuetasks",
                web::post().to(requeue_pending_tasks),
            ),
    );
    cfg.service(
        web::scope("/queue")
            .route("/pause/{queueName}", web::put().to(pause_queue))
            .route("/resume/{queueName}", web::put().to(resume_queue))
            .route("/status/{queueName}", web::get().to(queue_status)),
    );
    cfg.route("/metrics", web::get().to(pool_metrics));
}

async fn get_all_config(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let config = engine.get_all_config().await?;
    Ok(HttpResponse::Ok().json(config))
}

async fn sweep_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.sweep_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn pause_queue(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.pause_queue(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn resume_queue(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.resume_queue(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn queue_status(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let paused = engine.is_queue_paused(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "paused": paused })))
}

async fn get_tasks_for_type(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let tasks = engine.get_tasks_for_type(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tasks))
}

async fn requeue_pending_tasks(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let count = engine.requeue_pending_tasks(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(count))
}

async fn pool_metrics(engine: web::Data<WorkflowEngine>) -> HttpResponse {
    HttpResponse::Ok().json(engine.pool_metrics())
}

use actix_web::{web, HttpResponse};

use crate::engine::WorkflowEngine;
use crate::models::TaskUpdateRequest;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            .route("", web::post().to(update_task))
            .route("/search", web::get().to(search_tasks))
            .route("/poll/{taskType}", web::get().to(poll_task))
            .route("/poll/batch/{taskType}", web::get().to(batch_poll))
            .route("/queue/sizes", web::get().to(queue_sizes))
            .route("/{taskId}", web::get().to(get_task))
            .route("/{taskId}/ack", web::post().to(ack_task))
            .route("/{taskId}/log", web::get().to(get_task_logs))
            .route("/{taskId}/log", web::post().to(add_task_log)),
    );
}

async fn poll_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<PollQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = path.into_inner();
    let task = engine
        .poll_task(&task_type, query.worker_id.as_deref())
        .await?;
    match task {
        Some(t) => Ok(HttpResponse::Ok().json(t)),
        None => Ok(HttpResponse::NoContent().finish()),
    }
}

async fn batch_poll(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<BatchPollQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = path.into_inner();
    let tasks = engine
        .batch_poll_tasks(
            &task_type,
            query.worker_id.as_deref(),
            query.count.unwrap_or(1) as usize,
            query.timeout.unwrap_or(100),
        )
        .await?;
    Ok(HttpResponse::Ok().json(tasks))
}

async fn update_task(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<TaskUpdateRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let id = engine.update_task(&body).await?;
    Ok(HttpResponse::Ok().json(id))
}

async fn get_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task = engine.get_task(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(task))
}

async fn ack_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<PollQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let acked = engine
        .ack_task(&path.into_inner(), query.worker_id.as_deref())
        .await?;
    Ok(HttpResponse::Ok().json(acked))
}

async fn get_task_logs(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let logs = engine.get_task_logs(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(logs))
}

async fn add_task_log(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    body: web::Json<LogMessage>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.add_task_log(&path.into_inner(), &body.log).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn search_tasks(
    engine: web::Data<WorkflowEngine>,
    query: web::Query<TaskSearchQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine
        .search_tasks(
            query.workflow_id.as_deref(),
            query.task_type.as_deref(),
            query.status.as_deref(),
            query.free_text.as_deref(),
            query.start.unwrap_or(0),
            query.size.unwrap_or(100),
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

async fn queue_sizes(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let sizes = engine.get_queue_sizes().await?;
    Ok(HttpResponse::Ok().json(sizes))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollQuery {
    worker_id: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchPollQuery {
    worker_id: Option<String>,
    count: Option<i32>,
    timeout: Option<u64>,
}

#[derive(serde::Deserialize)]
struct LogMessage {
    log: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskSearchQuery {
    workflow_id: Option<String>,
    task_type: Option<String>,
    status: Option<String>,
    free_text: Option<String>,
    start: Option<i64>,
    size: Option<i64>,
}

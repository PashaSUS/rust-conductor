use actix_web::{HttpResponse, web};
use serde_json::Value;

use crate::engine::WorkflowEngine;
use crate::models::{TaskDef, TaskUpdateRequest};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/tasks")
            .route("", web::post().to(update_task))
            .route("/search", web::get().to(search_tasks))
            .route("/poll/{taskType}", web::get().to(poll_task))
            .route("/poll/batch/{taskType}", web::get().to(batch_poll))
            .route("/queue/sizes", web::get().to(queue_sizes))
            .route("/queue/all", web::get().to(queue_all))
            .route("/queue/all/verbose", web::get().to(queue_all_verbose))
            .route("/queue/polldata", web::get().to(poll_data))
            .route("/queue/polldata/all", web::get().to(poll_data_all))
            .route("/in_progress/{taskType}", web::get().to(in_progress_tasks))
            .route(
                "/in_progress/{workflowId}/{taskRefName}",
                web::get().to(in_progress_task_for_workflow),
            )
            .route("/register/{taskType}", web::post().to(register_task_type))
            .route(
                "/{workflowId}/{taskRefName}/{status}",
                web::post().to(update_task_by_ref_name),
            )
            .route("/{taskId}", web::get().to(get_task))
            .route("/{taskId}/ack", web::post().to(ack_task))
            .route("/{taskId}/log", web::get().to(get_task_logs))
            .route("/{taskId}/log", web::post().to(add_task_log))
            .route("/{taskId}/heartbeat", web::post().to(heartbeat_task)),
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

async fn update_task_by_ref_name(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<(String, String, String)>,
    body: web::Json<Value>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let (workflow_id, task_ref_name, status) = path.into_inner();
    let output = body.into_inner();
    let task_id = engine
        .update_task_by_ref_name(&workflow_id, &task_ref_name, &status, &output)
        .await?;
    Ok(HttpResponse::Ok().json(task_id))
}

async fn in_progress_tasks(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let tasks = engine.get_in_progress_tasks(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tasks))
}

async fn in_progress_task_for_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<(String, String)>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let (workflow_id, task_ref_name) = path.into_inner();
    let task = engine
        .get_in_progress_task_for_workflow(&workflow_id, &task_ref_name)
        .await?;
    match task {
        Some(t) => Ok(HttpResponse::Ok().json(t)),
        None => Ok(HttpResponse::NoContent().finish()),
    }
}

async fn queue_all(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let details = engine.get_all_queue_details().await?;
    Ok(HttpResponse::Ok().json(details))
}

async fn queue_all_verbose(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let details = engine.get_all_queue_details_verbose().await?;
    Ok(HttpResponse::Ok().json(details))
}

async fn poll_data(
    engine: web::Data<WorkflowEngine>,
    query: web::Query<PollDataQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = query.task_type.as_deref().unwrap_or("");
    let data = engine.get_poll_data(task_type).await?;
    Ok(HttpResponse::Ok().json(data))
}

async fn poll_data_all(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let data = engine.get_all_poll_data().await?;
    Ok(HttpResponse::Ok().json(data))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollDataQuery {
    task_type: Option<String>,
}

// ── 110. Task Heartbeat ────────────────────────────────────────────────

async fn heartbeat_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.heartbeat_task(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(serde_json::json!({ "acknowledged": true })))
}

/// Dynamic task registration — workers can register a task type at runtime.
async fn register_task_type(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    body: Option<web::Json<TaskDef>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = path.into_inner();
    let def = match body {
        Some(b) => {
            let mut d = b.into_inner();
            d.name = task_type;
            d
        }
        None => TaskDef {
            name: task_type,
            description: None,
            retry_count: 3,
            retry_logic: Default::default(),
            retry_delay_seconds: 60,
            timeout_seconds: 3600,
            timeout_policy: Default::default(),
            response_timeout_seconds: 600,
            concurrent_exec_limit: None,
            input_keys: vec![],
            output_keys: vec![],
            input_parameter_definitions: vec![],
            output_parameter_definitions: vec![],
            input_template: Default::default(),
            rate_limit_per_frequency: None,
            rate_limit_frequency_in_seconds: None,
            owner_email: None,
            poll_timeout_seconds: None,
            backoff_scale_factor: 1,
            total_timeout_seconds: None,
            owner_app: None,
            create_time: None,
            update_time: None,
            created_by: None,
            updated_by: None,
            base_type: None,
            isolation_group_id: None,
            execution_name_space: None,
            input_schema: None,
            output_schema: None,
            enforce_schema: false,
            retry_on_errors: vec![],
            env_vars: None,
        },
    };
    let result = engine.register_task_def(&def).await?;
    Ok(HttpResponse::Ok().json(result))
}

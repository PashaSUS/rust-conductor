use actix_web::{web, HttpResponse};
use std::collections::HashMap;
use serde_json::Value;

use crate::engine::WorkflowEngine;
use crate::models::{RerunWorkflowRequest, SkipTaskRequest, StartWorkflowRequest};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/workflow")
            .route("", web::post().to(start_workflow))
            .route("/stats", web::get().to(workflow_stats))
            .route("/search", web::get().to(search_workflows))
            .route("/running/{name}", web::get().to(get_running_workflows))
            .route("/{workflowId}", web::get().to(get_workflow))
            .route("/{workflowId}", web::delete().to(terminate_workflow))
            .route("/{workflowId}/status", web::get().to(get_workflow_status))
            .route("/{workflowId}/remove", web::delete().to(delete_workflow))
            .route("/{workflowId}/pause", web::put().to(pause_workflow))
            .route("/{workflowId}/resume", web::put().to(resume_workflow))
            .route("/{workflowId}/restart", web::post().to(restart_workflow))
            .route("/{workflowId}/retry", web::post().to(retry_workflow))
            .route("/{workflowId}/rerun", web::post().to(rerun_workflow))
            .route("/{workflowId}/decide", web::put().to(decide_workflow))
            .route("/{workflowId}/variables", web::post().to(update_workflow_variables))
            .route(
                "/{workflowId}/skiptask/{taskReferenceName}",
                web::put().to(skip_task),
            )
            .route("/{name}", web::post().to(start_workflow_by_name)),
    );
}

async fn start_workflow(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<StartWorkflowRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let id = engine.start_workflow(&body).await?;
    Ok(HttpResponse::Ok().json(id))
}

async fn get_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let wf = engine.get_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(wf))
}

async fn terminate_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<ReasonQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine
        .terminate_workflow(&path.into_inner(), query.reason.as_deref())
        .await?;
    Ok(HttpResponse::Ok().finish())
}

async fn pause_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.pause_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn resume_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.resume_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn restart_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.restart_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn retry_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.retry_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn workflow_stats(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let stats = engine.workflow_stats().await?;
    Ok(HttpResponse::Ok().json(stats))
}

async fn search_workflows(
    engine: web::Data<WorkflowEngine>,
    query: web::Query<SearchQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let tags: Option<Vec<String>> = query.tags.as_ref().map(|t| {
        t.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
    });
    let result = engine
        .search_workflows_with_cursor(
            query.status.as_deref(),
            query.workflow_type.as_deref(),
            query.free_text.as_deref(),
            query.start.unwrap_or(0),
            query.size.unwrap_or(100),
            tags.as_deref(),
            query.cursor.as_deref(),
        )
        .await?;
    Ok(HttpResponse::Ok().json(result))
}

#[derive(serde::Deserialize)]
struct ReasonQuery {
    reason: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchQuery {
    status: Option<String>,
    workflow_type: Option<String>,
    free_text: Option<String>,
    start: Option<i64>,
    size: Option<i64>,
    tags: Option<String>,
    cursor: Option<String>,
}

async fn rerun_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    body: web::Json<RerunWorkflowRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let new_id = engine.rerun_workflow(&path.into_inner(), &body).await?;
    Ok(HttpResponse::Ok().json(new_id))
}

async fn decide_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.decide_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn skip_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<(String, String)>,
    body: web::Json<SkipTaskRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let (workflow_id, task_ref) = path.into_inner();
    engine.skip_task(&workflow_id, &task_ref, &body).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn get_running_workflows(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<RunningQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let ids = engine
        .get_running_workflows(
            &path.into_inner(),
            query.version,
            query.start_time,
            query.end_time,
        )
        .await?;
    Ok(HttpResponse::Ok().json(ids))
}

async fn delete_workflow(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<ArchiveQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine
        .delete_workflow(&path.into_inner(), query.archive_workflow.unwrap_or(true))
        .await?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunningQuery {
    version: Option<i32>,
    start_time: Option<i64>,
    end_time: Option<i64>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ArchiveQuery {
    archive_workflow: Option<bool>,
}

async fn get_workflow_status(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<IncludeTasksQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let wf = engine
        .get_workflow_status(&path.into_inner(), query.include_tasks.unwrap_or(true))
        .await?;
    Ok(HttpResponse::Ok().json(wf))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IncludeTasksQuery {
    include_tasks: Option<bool>,
}

async fn update_workflow_variables(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    body: web::Json<HashMap<String, Value>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let wf = engine
        .update_workflow_variables(&path.into_inner(), &body)
        .await?;
    Ok(HttpResponse::Ok().json(wf))
}

async fn start_workflow_by_name(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<StartByNameQuery>,
    body: web::Json<Value>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let req = StartWorkflowRequest {
        name: path.into_inner(),
        version: query.version.unwrap_or(1),
        input: body.into_inner(),
        correlation_id: query.correlation_id.clone(),
        priority: query.priority.unwrap_or(0),
        task_to_domain: HashMap::new(),
        workflow_def: None,
        external_input_payload_storage_path: None,
        created_by: None,
        idempotency_key: None,
        idempotency_strategy: None,
        tags: vec![],
    };
    let id = engine.start_workflow(&req).await?;
    Ok(HttpResponse::Ok().json(id))
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartByNameQuery {
    version: Option<i32>,
    correlation_id: Option<String>,
    priority: Option<i32>,
}

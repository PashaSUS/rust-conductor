use actix_web::{web, HttpResponse, HttpRequest};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::Duration;

use crate::engine::WorkflowEngine;

// ── API Versioning (#165) ───────────────────────────────────────────────────

/// V2 API routes providing enhanced responses (richer error details, envelope format).
/// The V1 API remains untouched at /api — V2 is additive, not replacing.
pub fn configure_v2(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/v2")
            .route("/workflow", web::post().to(start_workflow_v2))
            .route("/workflow/{workflowId}", web::get().to(get_workflow_v2))
            .route("/workflow/search", web::get().to(search_workflows_v2))
            .route("/metadata/workflow", web::get().to(list_workflow_defs_v2))
            .route("/metadata/workflow/{name}", web::get().to(get_workflow_def_v2))
            .route("/metadata/taskdefs", web::get().to(list_task_defs_v2))
            .route("/tasks/poll/{taskType}", web::get().to(poll_task_v2))
            .route("/tasks/poll/long/{taskType}", web::get().to(long_poll_task)),
    );
}

#[derive(Serialize)]
struct ApiEnvelope<T: Serialize> {
    data: T,
    #[serde(rename = "apiVersion")]
    api_version: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    deprecation: Option<&'static str>,
}

impl<T: Serialize> ApiEnvelope<T> {
    fn v2(data: T) -> Self {
        Self {
            data,
            api_version: "2.0",
            cursor: None,
            deprecation: None,
        }
    }
}

async fn start_workflow_v2(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<crate::models::StartWorkflowRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let id = engine.start_workflow(&body).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .json(ApiEnvelope::v2(serde_json::json!({ "workflowId": id }))))
}

async fn get_workflow_v2(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let wf = engine.get_workflow(&path.into_inner()).await?;
    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .json(ApiEnvelope::v2(wf)))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct V2SearchQuery {
    status: Option<String>,
    workflow_type: Option<String>,
    free_text: Option<String>,
    start: Option<i64>,
    size: Option<i64>,
    cursor: Option<String>,
}

async fn search_workflows_v2(
    engine: web::Data<WorkflowEngine>,
    query: web::Query<V2SearchQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine
        .search_workflows_with_cursor(
            query.status.as_deref(),
            query.workflow_type.as_deref(),
            query.free_text.as_deref(),
            query.start.unwrap_or(0),
            query.size.unwrap_or(100),
            None,
            query.cursor.as_deref(),
        )
        .await?;
    let mut env = ApiEnvelope::v2(&result);
    env.cursor = result.next_cursor.clone();
    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .json(env))
}

async fn list_workflow_defs_v2(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let defs = engine.list_workflow_defs().await?;
    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .json(ApiEnvelope::v2(defs)))
}

async fn get_workflow_def_v2(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<VersionQuery>,
    req: HttpRequest,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let name = path.into_inner();
    let def = engine.get_workflow_def(&name, query.version).await?;

    // #167 — Conditional requests: ETag support
    let etag = compute_etag(&def);

    if let Some(if_none_match) = req.headers().get("If-None-Match") {
        if let Ok(val) = if_none_match.to_str() {
            if val.trim_matches('"') == etag.trim_matches('"') {
                return Ok(HttpResponse::NotModified().finish());
            }
        }
    }

    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .insert_header(("ETag", format!("\"{}\"", etag)))
        .insert_header(("Cache-Control", "no-cache"))
        .json(ApiEnvelope::v2(def)))
}

async fn list_task_defs_v2(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let defs = engine.list_task_defs().await?;
    Ok(HttpResponse::Ok()
        .insert_header(("X-API-Version", "2.0"))
        .json(ApiEnvelope::v2(defs)))
}

async fn poll_task_v2(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<PollQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = path.into_inner();
    let task = engine
        .poll_task(&task_type, query.worker_id.as_deref())
        .await?;
    match task {
        Some(t) => Ok(HttpResponse::Ok()
            .insert_header(("X-API-Version", "2.0"))
            .json(ApiEnvelope::v2(t))),
        None => Ok(HttpResponse::NoContent()
            .insert_header(("X-API-Version", "2.0"))
            .finish()),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VersionQuery {
    version: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PollQuery {
    worker_id: Option<String>,
}

// ── Long Polling (#170) ────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LongPollQuery {
    worker_id: Option<String>,
    /// Maximum wait time in milliseconds (default 10000, max 30000)
    timeout_ms: Option<u64>,
}

/// Long-polling endpoint for workers — waits server-side for a task to become
/// available instead of returning immediately with 204.
async fn long_poll_task(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<LongPollQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let task_type = path.into_inner();
    let timeout = Duration::from_millis(query.timeout_ms.unwrap_or(10_000).min(30_000));
    let poll_interval = Duration::from_millis(250);

    let deadline = tokio::time::Instant::now() + timeout;
    let mut ticker = tokio::time::interval(poll_interval);

    loop {
        ticker.tick().await;

        if tokio::time::Instant::now() >= deadline {
            return Ok(HttpResponse::NoContent()
                .insert_header(("X-API-Version", "2.0"))
                .insert_header(("X-Long-Poll-Timeout", "true"))
                .finish());
        }

        let task = engine
            .poll_task(&task_type, query.worker_id.as_deref())
            .await?;

        if let Some(t) = task {
            return Ok(HttpResponse::Ok()
                .insert_header(("X-API-Version", "2.0"))
                .json(ApiEnvelope::v2(t)));
        }
    }
}

// ── Request Batching (#166) ────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchRequest {
    pub operations: Vec<BatchOperation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOperation {
    /// Unique ID for this operation within the batch
    pub id: String,
    /// HTTP method: GET, POST, PUT, DELETE
    pub method: String,
    /// Relative path (e.g. "/workflow/search?status=RUNNING")
    pub path: String,
    /// Optional body for POST/PUT operations
    pub body: Option<Value>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchResponse {
    pub results: Vec<BatchOperationResult>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchOperationResult {
    pub id: String,
    pub status: u16,
    pub body: Value,
}

pub async fn batch_handler(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<BatchRequest>,
) -> HttpResponse {
    let mut results = Vec::with_capacity(body.operations.len());

    for op in &body.operations {
        let result = execute_batch_operation(&engine, op).await;
        results.push(result);
    }

    HttpResponse::Ok().json(BatchResponse { results })
}

async fn execute_batch_operation(
    engine: &WorkflowEngine,
    op: &BatchOperation,
) -> BatchOperationResult {
    let method = op.method.to_uppercase();
    let path = op.path.trim_start_matches('/');

    // Route the operation to the appropriate engine method
    let result: Result<Value, String> = match (method.as_str(), path) {
        ("GET", p) if p.starts_with("workflow/search") => {
            match engine
                .search_workflows_with_cursor(None, None, None, 0, 100, None, None)
                .await
            {
                Ok(r) => Ok(serde_json::to_value(r).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        ("GET", p) if p.starts_with("workflow/stats") => {
            match engine.workflow_stats().await {
                Ok(s) => Ok(serde_json::to_value(s).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        ("GET", p) if p.starts_with("workflow/") => {
            let wf_id = p.trim_start_matches("workflow/");
            match engine.get_workflow(wf_id).await {
                Ok(wf) => Ok(serde_json::to_value(wf).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        ("POST", "workflow") => {
            if let Some(body) = &op.body {
                match serde_json::from_value::<crate::models::StartWorkflowRequest>(body.clone()) {
                    Ok(req) => match engine.start_workflow(&req).await {
                        Ok(id) => Ok(Value::String(id)),
                        Err(e) => Err(e.to_string()),
                    },
                    Err(e) => Err(format!("Invalid request body: {}", e)),
                }
            } else {
                Err("Missing request body".to_string())
            }
        }
        ("GET", "metadata/workflow") => {
            match engine.list_workflow_defs().await {
                Ok(defs) => Ok(serde_json::to_value(defs).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        ("GET", "metadata/taskdefs") => {
            match engine.list_task_defs().await {
                Ok(defs) => Ok(serde_json::to_value(defs).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        ("GET", "tasks/queue/sizes") => {
            match engine.get_queue_sizes().await {
                Ok(s) => Ok(serde_json::to_value(s).unwrap_or(Value::Null)),
                Err(e) => Err(e.to_string()),
            }
        }
        _ => Err(format!("Unsupported batch operation: {} {}", method, path)),
    };

    match result {
        Ok(body) => BatchOperationResult {
            id: op.id.clone(),
            status: 200,
            body,
        },
        Err(msg) => BatchOperationResult {
            id: op.id.clone(),
            status: 400,
            body: serde_json::json!({ "error": msg }),
        },
    }
}

pub fn configure_batch(cfg: &mut web::ServiceConfig) {
    cfg.route("/batch", web::post().to(batch_handler));
}

// ── ETag helpers (#167) ────────────────────────────────────────────────────

fn compute_etag<T: serde::Serialize>(value: &T) -> String {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let json = serde_json::to_string(value).unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    json.hash(&mut hasher);
    format!("{:x}", hasher.finish())
}

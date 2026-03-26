use actix_web::{HttpRequest, HttpResponse, web};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::interval;

use crate::engine::WorkflowEngine;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/sse")
            .route(
                "/workflow/{workflowId}",
                web::get().to(workflow_status_stream),
            )
            .route("/queue/sizes", web::get().to(queue_sizes_stream)),
    );
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SseQuery {
    /// Polling interval in milliseconds (default 2000, min 500)
    interval_ms: Option<u64>,
}

/// SSE endpoint that streams workflow status updates until the workflow reaches
/// a terminal state or the client disconnects.
async fn workflow_status_stream(
    _req: HttpRequest,
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<SseQuery>,
) -> HttpResponse {
    let workflow_id = path.into_inner();
    let poll_interval = Duration::from_millis(query.interval_ms.unwrap_or(2000).max(500));
    let engine = engine.into_inner();

    let stream = async_stream::stream! {
        let mut ticker = interval(poll_interval);
        let mut last_status = String::new();
        let mut last_task_count = 0usize;

        loop {
            ticker.tick().await;

            match engine.get_workflow(&workflow_id).await {
                Ok(wf) => {
                    let status = format!("{:?}", wf.status).to_uppercase();
                    let task_count = wf.tasks.len();
                    let changed = status != last_status || task_count != last_task_count;

                    if changed {
                        last_status = status.clone();
                        last_task_count = task_count;

                        let data = serde_json::json!({
                            "workflowId": wf.workflow_id,
                            "status": status,
                            "taskCount": task_count,
                            "tasks": wf.tasks.iter().map(|t| serde_json::json!({
                                "taskId": t.task_id,
                                "referenceTaskName": t.reference_task_name,
                                "taskType": t.task_type,
                                "status": format!("{:?}", t.status).to_uppercase(),
                            })).collect::<Vec<_>>(),
                            "output": wf.output,
                            "reasonForIncompletion": wf.reason_for_incompletion,
                        });

                        let event = format!("event: workflowStatus\ndata: {}\n\n", data);
                        yield Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(event));
                    }

                    // Terminal states: stop streaming
                    if matches!(status.as_str(), "COMPLETED" | "FAILED" | "TERMINATED" | "TIMED_OUT") {
                        let close = "event: complete\ndata: {}\n\n".to_string();
                        yield Ok(actix_web::web::Bytes::from(close));
                        break;
                    }
                }
                Err(e) => {
                    let error = format!(
                        "event: error\ndata: {}\n\n",
                        serde_json::json!({ "error": e.to_string() })
                    );
                    yield Ok(actix_web::web::Bytes::from(error));
                    break;
                }
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

/// SSE endpoint streaming queue size updates at a configurable interval.
async fn queue_sizes_stream(
    _req: HttpRequest,
    engine: web::Data<WorkflowEngine>,
    query: web::Query<SseQuery>,
) -> HttpResponse {
    let poll_interval = Duration::from_millis(query.interval_ms.unwrap_or(5000).max(1000));
    let engine = engine.into_inner();

    let stream = async_stream::stream! {
        let mut ticker = interval(poll_interval);

        loop {
            ticker.tick().await;

            match engine.get_queue_sizes().await {
                Ok(sizes) => {
                    let data = serde_json::to_string(&sizes).unwrap_or_default();
                    let event = format!("event: queueSizes\ndata: {data}\n\n");
                    yield Ok::<_, actix_web::Error>(actix_web::web::Bytes::from(event));
                }
                Err(e) => {
                    let error = format!(
                        "event: error\ndata: {}\n\n",
                        serde_json::json!({ "error": e.to_string() })
                    );
                    yield Ok(actix_web::web::Bytes::from(error));
                    break;
                }
            }
        }
    };

    HttpResponse::Ok()
        .insert_header(("Content-Type", "text/event-stream"))
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

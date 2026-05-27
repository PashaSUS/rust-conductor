use actix_web::{HttpRequest, HttpResponse, web};
use actix_ws::Message;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;

use crate::engine::WorkflowEngine;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/ws")
            .route("/workflow/{workflowId}", web::get().to(workflow_ws))
            .route("/tasks/{taskType}", web::get().to(task_feed_ws)),
    );
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WsClientMessage {
    /// Subscribe to workflow status updates
    Subscribe { workflow_id: String },
    /// Unsubscribe from workflow status updates
    Unsubscribe { workflow_id: String },
    /// Request current status on-demand
    GetStatus { workflow_id: String },
    /// Ping to maintain connection
    Ping,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
enum WsServerMessage {
    WorkflowStatus {
        workflow_id: String,
        status: String,
        task_count: usize,
        output: serde_json::Value,
    },
    #[allow(dead_code)]
    TaskUpdate {
        task_id: String,
        task_type: String,
        status: String,
        reference_task_name: String,
    },
    Error {
        message: String,
    },
    Pong,
}

/// WebSocket endpoint for bidirectional workflow status communication.
/// Clients can subscribe to workflow updates and receive real-time status changes.
async fn workflow_ws(
    req: HttpRequest,
    stream: web::Payload,
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let initial_workflow_id = path.into_inner();
    let engine = engine.into_inner();

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    actix_web::rt::spawn(async move {
        let mut tracked_workflows: Vec<String> = vec![initial_workflow_id.clone()];
        let mut ticker = interval(Duration::from_secs(2));

        loop {
            tokio::select! {
                // Handle incoming messages from the client
                msg = msg_stream.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(client_msg) = serde_json::from_str::<WsClientMessage>(&text) {
                                match client_msg {
                                    WsClientMessage::Subscribe { workflow_id } => {
                                        if !tracked_workflows.contains(&workflow_id) {
                                            tracked_workflows.push(workflow_id);
                                        }
                                    }
                                    WsClientMessage::Unsubscribe { workflow_id } => {
                                        tracked_workflows.retain(|id| id != &workflow_id);
                                    }
                                    WsClientMessage::GetStatus { workflow_id } => {
                                        send_workflow_status(&engine, &mut session, &workflow_id).await;
                                    }
                                    WsClientMessage::Ping => {
                                        let pong = serde_json::to_string(&WsServerMessage::Pong).unwrap_or_default();
                                        let _ = session.text(pong).await;
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Ping(bytes))) => {
                            let _ = session.pong(&bytes).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                // Periodically push status updates for tracked workflows
                _ = ticker.tick() => {
                    let mut to_remove = Vec::new();
                    for wf_id in &tracked_workflows {
                        let terminal = send_workflow_status(&engine, &mut session, wf_id).await;
                        if terminal {
                            to_remove.push(wf_id.clone());
                        }
                    }
                    for id in to_remove {
                        tracked_workflows.retain(|i| i != &id);
                    }
                    if tracked_workflows.is_empty() {
                        // No more workflows to track — keep connection open but stop polling
                    }
                }
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}

/// Send workflow status update, returns true if workflow is in terminal state.
async fn send_workflow_status(
    engine: &Arc<WorkflowEngine>,
    session: &mut actix_ws::Session,
    workflow_id: &str,
) -> bool {
    match engine.get_workflow(workflow_id).await {
        Ok(wf) => {
            let status = format!("{:?}", wf.status).to_uppercase();
            let msg = WsServerMessage::WorkflowStatus {
                workflow_id: wf.workflow_id,
                status: status.clone(),
                task_count: wf.tasks.len(),
                output: wf.output,
            };
            let json = serde_json::to_string(&msg).unwrap_or_default();
            let _ = session.text(json).await;

            matches!(
                status.as_str(),
                "COMPLETED" | "FAILED" | "TERMINATED" | "TIMED_OUT"
            )
        }
        Err(e) => {
            let msg = WsServerMessage::Error {
                message: e.to_string(),
            };
            let json = serde_json::to_string(&msg).unwrap_or_default();
            let _ = session.text(json).await;
            true
        }
    }
}

/// WebSocket endpoint for real-time task feed — streams polled tasks for a given type.
/// Workers can use this instead of HTTP polling for lower-latency task pickup.
async fn task_feed_ws(
    req: HttpRequest,
    stream: web::Payload,
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, actix_web::Error> {
    let task_type = path.into_inner();
    let engine = engine.into_inner();

    // Extract optional ?domain=... so workers can route to a Conductor task-domain
    let domain: Option<String> =
        web::Query::<std::collections::HashMap<String, String>>::from_query(req.query_string())
            .ok()
            .and_then(|q| q.get("domain").cloned())
            .filter(|s| !s.is_empty());

    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    actix_web::rt::spawn(async move {
        let mut ticker = interval(Duration::from_millis(500));
        let worker_id = format!("ws-worker-{}", uuid::Uuid::new_v4());

        loop {
            tokio::select! {
                msg = msg_stream.recv() => {
                    match msg {
                        Some(Ok(Message::Text(text))) => {
                            // Accept task updates from the worker via WebSocket
                            if let Ok(update) = serde_json::from_str::<crate::models::TaskUpdateRequest>(&text) {
                                match engine.update_task(&update).await {
                                    Ok(id) => {
                                        let ack = serde_json::json!({ "type": "taskUpdateAck", "taskId": id });
                                        let _ = session.text(serde_json::to_string(&ack).unwrap_or_default()).await;
                                    }
                                    Err(e) => {
                                        let err = serde_json::json!({ "type": "error", "message": e.to_string() });
                                        let _ = session.text(serde_json::to_string(&err).unwrap_or_default()).await;
                                    }
                                }
                            }
                        }
                        Some(Ok(Message::Ping(bytes))) => {
                            let _ = session.pong(&bytes).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
                _ = ticker.tick() => {
                    // Poll for tasks and push to the worker
                    match engine.poll_task(&task_type, Some(&worker_id), domain.as_deref()).await {
                        Ok(Some(task)) => {
                            let msg = serde_json::json!({
                                "type": "taskAssigned",
                                "task": task,
                            });
                            let _ = session.text(serde_json::to_string(&msg).unwrap_or_default()).await;
                        }
                        Ok(None) => {} // No tasks available
                        Err(e) => {
                            let err = serde_json::json!({ "type": "error", "message": e.to_string() });
                            let _ = session.text(serde_json::to_string(&err).unwrap_or_default()).await;
                        }
                    }
                }
            }
        }

        let _ = session.close(None).await;
    });

    Ok(response)
}

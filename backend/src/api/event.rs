use actix_web::{HttpResponse, web};

use crate::engine::WorkflowEngine;
use crate::models::EventHandler;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/event")
            .route("", web::post().to(register_event_handler))
            .route("", web::put().to(update_event_handler))
            .route("", web::get().to(get_event_handlers))
            .route("/{name}", web::delete().to(delete_event_handler))
            .route("/{event}", web::get().to(get_event_handlers_for_event)),
    );
}

async fn register_event_handler(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<EventHandler>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let handler = engine.register_event_handler(&body).await?;
    Ok(HttpResponse::Ok().json(handler))
}

async fn update_event_handler(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<EventHandler>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let handler = engine.register_event_handler(&body).await?;
    Ok(HttpResponse::Ok().json(handler))
}

async fn get_event_handlers(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let handlers = engine.get_event_handlers().await?;
    Ok(HttpResponse::Ok().json(handlers))
}

async fn get_event_handlers_for_event(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<ActiveQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let handlers = engine
        .get_event_handlers_for_event(&path.into_inner(), query.active_only.unwrap_or(true))
        .await?;
    Ok(HttpResponse::Ok().json(handlers))
}

async fn delete_event_handler(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.delete_event_handler(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActiveQuery {
    active_only: Option<bool>,
}

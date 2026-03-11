use actix_web::{web, HttpResponse};

use crate::engine::WorkflowEngine;
use crate::models::{TaskDef, WorkflowDef};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/metadata")
            // Workflow definitions
            .route("/workflow", web::post().to(register_workflow_def))
            .route("/workflow", web::put().to(update_workflow_defs))
            .route("/workflow", web::get().to(list_workflow_defs))
            .route("/workflow/{name}", web::get().to(get_workflow_def))
            .route(
                "/workflow/{name}/{version}",
                web::delete().to(delete_workflow_def),
            )
            // Task definitions
            .route("/taskdefs", web::post().to(register_task_defs))
            .route("/taskdefs", web::get().to(list_task_defs))
            .route("/taskdefs/{taskType}", web::get().to(get_task_def))
            .route("/taskdefs/{taskType}", web::delete().to(delete_task_def)),
    );
}

async fn register_workflow_def(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<WorkflowDef>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.register_workflow_def(&body).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn update_workflow_defs(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<WorkflowDef>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    for def in body.iter() {
        engine.register_workflow_def(def).await?;
    }
    Ok(HttpResponse::Ok().finish())
}

async fn list_workflow_defs(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let defs = engine.list_workflow_defs().await?;
    Ok(HttpResponse::Ok().json(defs))
}

async fn get_workflow_def(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
    query: web::Query<VersionQuery>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let name = path.into_inner();
    let def = engine.get_workflow_def(&name, query.version).await?;
    Ok(HttpResponse::Ok().json(def))
}

async fn delete_workflow_def(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<(String, i32)>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let (name, version) = path.into_inner();
    engine.delete_workflow_def(&name, version).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn register_task_defs(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<TaskDef>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    for def in body.iter() {
        engine.register_task_def(def).await?;
    }
    Ok(HttpResponse::Ok().finish())
}

async fn list_task_defs(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let defs = engine.list_task_defs().await?;
    Ok(HttpResponse::Ok().json(defs))
}

async fn get_task_def(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let name = path.into_inner();
    let def = engine.get_task_def(&name).await?;
    Ok(HttpResponse::Ok().json(def))
}

async fn delete_task_def(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let name = path.into_inner();
    engine.delete_task_def(&name).await?;
    Ok(HttpResponse::Ok().finish())
}

#[derive(serde::Deserialize)]
struct VersionQuery {
    version: Option<i32>,
}

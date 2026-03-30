use actix_web::{HttpResponse, web};

use crate::engine::WorkflowEngine;
use crate::models::{
    BulkResponse, InstantiateTemplateRequest, TaskDef, WorkflowDef, WorkflowTemplate,
};

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
            .route("/taskdefs", web::put().to(update_task_def))
            .route("/taskdefs", web::get().to(list_task_defs))
            .route("/taskdefs/{taskType}", web::get().to(get_task_def))
            .route("/taskdefs/{taskType}", web::delete().to(delete_task_def))
            // Workflow templates
            .route("/template", web::post().to(register_template))
            .route("/template", web::get().to(list_templates))
            .route("/template/{name}", web::get().to(get_template))
            .route("/template/{name}", web::delete().to(delete_template))
            .route(
                "/template/instantiate",
                web::post().to(instantiate_template),
            )
            // 109. Workflow validation
            .route("/workflow/validate", web::post().to(validate_workflow)),
    );
}

async fn register_workflow_def(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<WorkflowDef>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let def = engine.register_workflow_def(&body).await?;
    Ok(HttpResponse::Ok().json(def))
}

async fn update_workflow_defs(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<Vec<WorkflowDef>>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let mut response = BulkResponse::default();
    for def in body.iter() {
        match engine.register_workflow_def(def).await {
            Ok(_) => {
                response
                    .bulk_successful_results
                    .push(format!("{}:{}", def.name, def.version));
            }
            Err(e) => {
                response
                    .bulk_error_results
                    .insert(format!("{}:{}", def.name, def.version), e.to_string());
            }
        }
    }
    Ok(HttpResponse::Ok().json(response))
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
    let mut result = Vec::new();
    for def in body.iter() {
        result.push(engine.register_task_def(def).await?);
    }
    Ok(HttpResponse::Ok().json(result))
}

async fn update_task_def(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<TaskDef>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let def = engine.register_task_def(&body).await?;
    Ok(HttpResponse::Ok().json(def))
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

// ── Workflow Template Handlers ──

async fn register_template(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<WorkflowTemplate>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let tmpl = engine.register_template(&body).await?;
    Ok(HttpResponse::Ok().json(tmpl))
}

async fn list_templates(
    engine: web::Data<WorkflowEngine>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let templates = engine.list_templates().await?;
    Ok(HttpResponse::Ok().json(templates))
}

async fn get_template(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let tmpl = engine.get_template(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().json(tmpl))
}

async fn delete_template(
    engine: web::Data<WorkflowEngine>,
    path: web::Path<String>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    engine.delete_template(&path.into_inner()).await?;
    Ok(HttpResponse::Ok().finish())
}

async fn instantiate_template(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<InstantiateTemplateRequest>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let wf_id = engine.instantiate_template(&body).await?;
    Ok(HttpResponse::Ok().json(wf_id))
}

// ── 109. Workflow Validation ───────────────────────────────────────────

async fn validate_workflow(
    engine: web::Data<WorkflowEngine>,
    body: web::Json<WorkflowDef>,
) -> Result<HttpResponse, crate::engine::EngineError> {
    let result = engine.validate_workflow_def(&body);
    Ok(HttpResponse::Ok().json(result))
}

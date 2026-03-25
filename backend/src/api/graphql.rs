use actix_web::{web, HttpResponse};
use async_graphql::*;
use async_graphql_actix_web::{GraphQLRequest, GraphQLResponse};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::engine::WorkflowEngine;
use crate::models;

// ── GraphQL Object Types ────────────────────────────────────────────────────

#[derive(SimpleObject)]
#[graphql(name = "WorkflowDef")]
struct GqlWorkflowDef {
    name: String,
    description: Option<String>,
    version: i32,
    tasks: Value,
    input_parameters: Vec<String>,
    output_parameters: Value,
    failure_workflow: Option<String>,
    schema_version: i32,
    restartable: bool,
    timeout_seconds: i64,
    owner_email: Option<String>,
    tags: Vec<String>,
    sla_deadline_seconds: Option<i64>,
}

impl From<models::WorkflowDef> for GqlWorkflowDef {
    fn from(d: models::WorkflowDef) -> Self {
        Self {
            name: d.name,
            description: d.description,
            version: d.version,
            tasks: serde_json::to_value(&d.tasks).unwrap_or(Value::Array(vec![])),
            input_parameters: d.input_parameters,
            output_parameters: serde_json::to_value(&d.output_parameters)
                .unwrap_or(Value::Object(Default::default())),
            failure_workflow: d.failure_workflow,
            schema_version: d.schema_version,
            restartable: d.restartable,
            timeout_seconds: d.timeout_seconds,
            owner_email: d.owner_email,
            tags: d.tags,
            sla_deadline_seconds: d.sla_deadline_seconds,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "TaskDef")]
struct GqlTaskDef {
    name: String,
    description: Option<String>,
    retry_count: i32,
    timeout_seconds: i64,
    response_timeout_seconds: i64,
    owner_email: Option<String>,
    input_keys: Vec<String>,
    output_keys: Vec<String>,
}

impl From<models::TaskDef> for GqlTaskDef {
    fn from(d: models::TaskDef) -> Self {
        Self {
            name: d.name,
            description: d.description,
            retry_count: d.retry_count,
            timeout_seconds: d.timeout_seconds,
            response_timeout_seconds: d.response_timeout_seconds,
            owner_email: d.owner_email,
            input_keys: d.input_keys,
            output_keys: d.output_keys,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "Workflow")]
struct GqlWorkflow {
    workflow_id: String,
    workflow_name: String,
    workflow_version: i32,
    status: String,
    input: Value,
    output: Value,
    tasks: Vec<GqlTaskResult>,
    correlation_id: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    update_time: Option<String>,
    priority: i32,
    reason_for_incompletion: Option<String>,
    tags: Vec<String>,
}

impl From<models::Workflow> for GqlWorkflow {
    fn from(w: models::Workflow) -> Self {
        Self {
            workflow_id: w.workflow_id,
            workflow_name: w.workflow_name,
            workflow_version: w.workflow_version,
            status: format!("{:?}", w.status).to_uppercase(),
            input: w.input,
            output: w.output,
            tasks: w.tasks.into_iter().map(GqlTaskResult::from).collect(),
            correlation_id: w.correlation_id,
            start_time: Some(w.start_time.to_string()),
            end_time: w.end_time.map(|t| t.to_string()),
            update_time: Some(w.update_time.to_string()),
            priority: w.priority,
            reason_for_incompletion: w.reason_for_incompletion,
            tags: Vec::new(),
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "TaskResult")]
struct GqlTaskResult {
    task_id: String,
    workflow_instance_id: String,
    task_type: String,
    task_def_name: String,
    reference_task_name: String,
    status: String,
    input_data: Value,
    output_data: Value,
    scheduled_time: Option<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    poll_count: i32,
    worker_id: Option<String>,
    seq: i32,
    retry_count: i32,
    reason_for_incompletion: Option<String>,
    sub_workflow_id: Option<String>,
}

impl From<models::TaskResult> for GqlTaskResult {
    fn from(t: models::TaskResult) -> Self {
        Self {
            task_id: t.task_id,
            workflow_instance_id: t.workflow_instance_id,
            task_type: t.task_type,
            task_def_name: t.task_def_name,
            reference_task_name: t.reference_task_name,
            status: format!("{:?}", t.status).to_uppercase(),
            input_data: t.input_data,
            output_data: t.output_data,
            scheduled_time: t.scheduled_time.map(|t| t.to_string()),
            start_time: t.start_time.map(|t| t.to_string()),
            end_time: t.end_time.map(|t| t.to_string()),
            poll_count: t.poll_count,
            worker_id: t.worker_id,
            seq: t.seq,
            retry_count: t.retry_count,
            reason_for_incompletion: t.reason_for_incompletion,
            sub_workflow_id: t.sub_workflow_id,
        }
    }
}

#[derive(SimpleObject)]
#[graphql(name = "WorkflowSearchResult")]
struct GqlWorkflowSearchResult {
    total_hits: i64,
    results: Vec<GqlWorkflowSummary>,
}

#[derive(SimpleObject)]
#[graphql(name = "WorkflowSummary")]
struct GqlWorkflowSummary {
    workflow_id: String,
    workflow_type: String,
    version: i32,
    status: String,
    start_time: Option<String>,
    end_time: Option<String>,
    correlation_id: Option<String>,
    priority: i32,
}

#[derive(SimpleObject)]
#[graphql(name = "QueueSizes")]
struct GqlQueueSizes {
    sizes: Value,
}

// ── Query Root ──────────────────────────────────────────────────────────────

pub(crate) struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get a workflow execution by ID
    async fn workflow(&self, ctx: &Context<'_>, id: String) -> Result<GqlWorkflow> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let wf = engine.get_workflow(&id).await?;
        Ok(GqlWorkflow::from(wf))
    }

    /// Search workflow executions
    async fn search_workflows(
        &self,
        ctx: &Context<'_>,
        status: Option<String>,
        workflow_type: Option<String>,
        free_text: Option<String>,
        start: Option<i64>,
        size: Option<i64>,
    ) -> Result<GqlWorkflowSearchResult> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let result = engine
            .search_workflows_with_cursor(
                status.as_deref(),
                workflow_type.as_deref(),
                free_text.as_deref(),
                start.unwrap_or(0),
                size.unwrap_or(100),
                None,
                None,
            )
            .await?;

        Ok(GqlWorkflowSearchResult {
            total_hits: result.total_hits,
            results: result
                .results
                .into_iter()
                .map(|s| GqlWorkflowSummary {
                    workflow_id: s.workflow_id,
                    workflow_type: s.workflow_type,
                    version: s.version,
                    status: s.status.to_string(),
                    start_time: s.start_time,
                    end_time: s.end_time,
                    correlation_id: s.correlation_id,
                    priority: s.priority,
                })
                .collect(),
        })
    }

    /// Get a workflow definition by name and optional version
    async fn workflow_def(
        &self,
        ctx: &Context<'_>,
        name: String,
        version: Option<i32>,
    ) -> Result<GqlWorkflowDef> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let def = engine.get_workflow_def(&name, version).await?;
        Ok(GqlWorkflowDef::from(def))
    }

    /// List all workflow definitions
    async fn workflow_defs(&self, ctx: &Context<'_>) -> Result<Vec<GqlWorkflowDef>> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let defs = engine.list_workflow_defs().await?;
        Ok(defs.into_iter().map(GqlWorkflowDef::from).collect())
    }

    /// Get a task definition by name
    async fn task_def(&self, ctx: &Context<'_>, name: String) -> Result<GqlTaskDef> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let def = engine.get_task_def(&name).await?;
        Ok(GqlTaskDef::from(def))
    }

    /// List all task definitions
    async fn task_defs(&self, ctx: &Context<'_>) -> Result<Vec<GqlTaskDef>> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let defs = engine.list_task_defs().await?;
        Ok(defs.into_iter().map(GqlTaskDef::from).collect())
    }

    /// Get a task by ID
    async fn task(&self, ctx: &Context<'_>, task_id: String) -> Result<GqlTaskResult> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let task = engine.get_task(&task_id).await?;
        Ok(GqlTaskResult::from(task))
    }

    /// Get queue sizes for all task types
    async fn queue_sizes(&self, ctx: &Context<'_>) -> Result<GqlQueueSizes> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let sizes = engine.get_queue_sizes().await?;
        Ok(GqlQueueSizes {
            sizes: serde_json::to_value(sizes).unwrap_or(Value::Object(Default::default())),
        })
    }

    /// Get workflow execution statistics
    async fn workflow_stats(&self, ctx: &Context<'_>) -> Result<Value> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let stats = engine.workflow_stats().await?;
        Ok(serde_json::to_value(stats).unwrap_or(Value::Object(Default::default())))
    }
}

// ── Mutation Root ───────────────────────────────────────────────────────────

pub(crate) struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Start a new workflow execution
    async fn start_workflow(
        &self,
        ctx: &Context<'_>,
        name: String,
        version: Option<i32>,
        input: Option<Value>,
        correlation_id: Option<String>,
        priority: Option<i32>,
        tags: Option<Vec<String>>,
    ) -> Result<String> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let req = models::StartWorkflowRequest {
            name,
            version: version.unwrap_or(1),
            input: input.unwrap_or(Value::Object(Default::default())),
            correlation_id,
            priority: priority.unwrap_or(0),
            task_to_domain: HashMap::new(),
            workflow_def: None,
            external_input_payload_storage_path: None,
            created_by: None,
            idempotency_key: None,
            idempotency_strategy: None,
            tags: tags.unwrap_or_default(),
        };
        let id = engine.start_workflow(&req).await?;
        Ok(id)
    }

    /// Terminate a running workflow
    async fn terminate_workflow(
        &self,
        ctx: &Context<'_>,
        workflow_id: String,
        reason: Option<String>,
    ) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        engine
            .terminate_workflow(&workflow_id, reason.as_deref())
            .await?;
        Ok(true)
    }

    /// Pause a running workflow
    async fn pause_workflow(&self, ctx: &Context<'_>, workflow_id: String) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        engine.pause_workflow(&workflow_id).await?;
        Ok(true)
    }

    /// Resume a paused workflow
    async fn resume_workflow(&self, ctx: &Context<'_>, workflow_id: String) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        engine.resume_workflow(&workflow_id).await?;
        Ok(true)
    }

    /// Restart a completed/failed workflow
    async fn restart_workflow(&self, ctx: &Context<'_>, workflow_id: String) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        engine.restart_workflow(&workflow_id).await?;
        Ok(true)
    }

    /// Retry a failed workflow from the failed task
    async fn retry_workflow(&self, ctx: &Context<'_>, workflow_id: String) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        engine.retry_workflow(&workflow_id).await?;
        Ok(true)
    }

    /// Update a task result
    async fn update_task(
        &self,
        ctx: &Context<'_>,
        task_id: String,
        workflow_instance_id: String,
        status: String,
        output_data: Option<Value>,
        reason_for_incompletion: Option<String>,
        worker_id: Option<String>,
    ) -> Result<String> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let req = models::TaskUpdateRequest {
            task_id,
            workflow_instance_id,
            status: serde_json::from_value(Value::String(status))?,
            output_data: output_data.unwrap_or(Value::Object(Default::default())),
            reason_for_incompletion,
            worker_id,
            callback_after_seconds: 0,
            logs: Vec::new(),
            external_output_payload_storage_path: None,
            sub_workflow_id: None,
            extend_lease: false,
        };
        let id = engine.update_task(&req).await?;
        Ok(id)
    }

    /// Register or update a workflow definition
    async fn register_workflow_def(&self, ctx: &Context<'_>, def: Value) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let workflow_def: models::WorkflowDef = serde_json::from_value(def)?;
        engine.register_workflow_def(&workflow_def).await?;
        Ok(true)
    }

    /// Register or update task definitions
    async fn register_task_defs(&self, ctx: &Context<'_>, defs: Value) -> Result<bool> {
        let engine = ctx.data::<Arc<WorkflowEngine>>()?;
        let task_defs: Vec<models::TaskDef> = serde_json::from_value(defs)?;
        for def in &task_defs {
            engine.register_task_def(def).await?;
        }
        Ok(true)
    }
}

// ── Schema Builder ──────────────────────────────────────────────────────────

pub type ConductorSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

pub fn build_schema(engine: Arc<WorkflowEngine>) -> ConductorSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(engine)
        .finish()
}

// ── Actix-web Handlers ──────────────────────────────────────────────────────

pub async fn graphql_handler(
    schema: web::Data<ConductorSchema>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    schema.execute(req.into_inner()).await.into()
}

pub async fn graphql_playground() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(async_graphql::http::playground_source(
            async_graphql::http::GraphQLPlaygroundConfig::new("/api/graphql"),
        ))
}

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/graphql")
            .route("", web::post().to(graphql_handler))
            .route("", web::get().to(graphql_playground)),
    );
}

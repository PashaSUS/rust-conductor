use utoipa::openapi::path::{
    HttpMethod, OperationBuilder, ParameterBuilder, ParameterIn, PathItemBuilder,
};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Ref as SchemaRef, SchemaType, Type};
use utoipa::openapi::{
    ComponentsBuilder, ContentBuilder, InfoBuilder, OpenApiBuilder, PathsBuilder, ServerBuilder,
};

fn s() -> ObjectBuilder {
    ObjectBuilder::new().schema_type(SchemaType::Type(Type::String))
}
fn i() -> ObjectBuilder {
    ObjectBuilder::new().schema_type(SchemaType::Type(Type::Integer))
}
fn b() -> ObjectBuilder {
    ObjectBuilder::new().schema_type(SchemaType::Type(Type::Boolean))
}
fn sref(name: &str) -> SchemaRef {
    SchemaRef::new(format!("#/components/schemas/{}", name))
}
fn ref_content(name: &str) -> utoipa::openapi::Content {
    ContentBuilder::new().schema(Some(sref(name))).build()
}
fn arr_ref_content(name: &str) -> utoipa::openapi::Content {
    ContentBuilder::new()
        .schema(Some(ArrayBuilder::new().items(sref(name))))
        .build()
}
fn str_content() -> utoipa::openapi::Content {
    ContentBuilder::new()
        .schema(Some(
            ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)),
        ))
        .build()
}
fn json_content() -> utoipa::openapi::Content {
    ContentBuilder::new()
        .schema(Some(ObjectBuilder::new()))
        .build()
}
fn str_arr_content() -> utoipa::openapi::Content {
    ContentBuilder::new()
        .schema(Some(ArrayBuilder::new().items(
            ObjectBuilder::new().schema_type(SchemaType::Type(Type::String)),
        )))
        .build()
}

pub fn build_openapi() -> utoipa::openapi::OpenApi {
    let info = InfoBuilder::new()
        .title("Rust Conductor API")
        .description(Some(
            "Netflix Conductor-compatible workflow orchestration engine built in Rust. \
             Provides REST, GraphQL, gRPC, SSE, and WebSocket endpoints for workflow \
             definitions, task management, event handling, and administrative operations. \
             V1 API is fully Conductor-compatible; V2 API adds envelope format, \
             long-polling, and request batching.",
        ))
        .version("0.2.0")
        .build();

    let server = ServerBuilder::new().url("/").build();

    let ref_body = |name: &str| {
        RequestBodyBuilder::new()
            .content("application/json", ref_content(name))
            .required(Some(utoipa::openapi::Required::True))
            .build()
    };

    let arr_ref_body = |name: &str| {
        RequestBodyBuilder::new()
            .content("application/json", arr_ref_content(name))
            .required(Some(utoipa::openapi::Required::True))
            .build()
    };

    let json_body = || {
        RequestBodyBuilder::new()
            .content("application/json", json_content())
            .required(Some(utoipa::openapi::Required::True))
            .build()
    };

    let str_arr_body = || {
        RequestBodyBuilder::new()
            .content("application/json", str_arr_content())
            .required(Some(utoipa::openapi::Required::True))
            .build()
    };

    let ok = || ResponseBuilder::new().description("OK").build();
    let ok_ref = |name: &str, desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content("application/json", ref_content(name))
            .build()
    };
    let ok_arr_ref = |name: &str, desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content("application/json", arr_ref_content(name))
            .build()
    };
    let ok_str = |desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content("application/json", str_content())
            .build()
    };
    let ok_str_arr = |desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content("application/json", str_arr_content())
            .build()
    };
    let ok_json = |desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content("application/json", json_content())
            .build()
    };
    let ok_bool = |desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content(
                "application/json",
                ContentBuilder::new()
                    .schema(Some(
                        ObjectBuilder::new().schema_type(SchemaType::Type(Type::Boolean)),
                    ))
                    .build(),
            )
            .build()
    };
    let ok_int = |desc: &str| {
        ResponseBuilder::new()
            .description(desc)
            .content(
                "application/json",
                ContentBuilder::new()
                    .schema(Some(
                        ObjectBuilder::new().schema_type(SchemaType::Type(Type::Integer)),
                    ))
                    .build(),
            )
            .build()
    };
    let no_content = || ResponseBuilder::new().description("No Content").build();
    let not_found = || ResponseBuilder::new().description("Not Found").build();
    let conflict = || {
        ResponseBuilder::new()
            .description("Conflict - Invalid State")
            .build()
    };

    let path_param = |name: &str, desc: &str| {
        ParameterBuilder::new()
            .name(name)
            .parameter_in(ParameterIn::Path)
            .required(utoipa::openapi::Required::True)
            .description(Some(desc))
            .build()
    };

    let query_param = |name: &str, desc: &str| {
        ParameterBuilder::new()
            .name(name)
            .parameter_in(ParameterIn::Query)
            .required(utoipa::openapi::Required::False)
            .description(Some(desc))
            .build()
    };

    let tag = |t: &str| vec![t.to_string()];

    let paths = PathsBuilder::new()
        // ── Health ──
        .path(
            "/health",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("health")))
                        .summary(Some("Health check"))
                        .description(Some("Returns the health status of all backing services (Postgres shards, Redis, Kafka)"))
                        .operation_id(Some("healthCheck"))
                        .response("200", ok_ref("HealthCheckStatus", "Health status of the system"))
                        .response("503", ResponseBuilder::new().description("Service Unavailable").build())
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Metadata: Workflow Definitions ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/metadata/workflow",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("List all workflow definitions"))
                        .description(Some("Retrieves all workflow definitions across all versions"))
                        .operation_id(Some("listWorkflowDefs"))
                        .response("200", ok_arr_ref("WorkflowDef", "Array of WorkflowDef objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Register a workflow definition"))
                        .description(Some("Creates or updates a workflow definition. Returns the registered WorkflowDef."))
                        .operation_id(Some("registerWorkflowDef"))
                        .request_body(Some(ref_body("WorkflowDef")))
                        .response("200", ok_ref("WorkflowDef", "The registered WorkflowDef"))
                        .build(),
                )
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Update workflow definitions"))
                        .description(Some("Creates or updates multiple workflow definitions. Returns all updated WorkflowDef objects."))
                        .operation_id(Some("updateWorkflowDefs"))
                        .request_body(Some(arr_ref_body("WorkflowDef")))
                        .response("200", ok_arr_ref("WorkflowDef", "Array of updated WorkflowDef objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/metadata/workflow/{name}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Get workflow definition"))
                        .description(Some("Returns a workflow definition by name. Use ?version= to get a specific version, otherwise returns latest."))
                        .operation_id(Some("getWorkflowDef"))
                        .parameter(path_param("name", "Workflow definition name"))
                        .parameter(query_param("version", "Specific version number (omit for latest)"))
                        .response("200", ok_ref("WorkflowDef", "WorkflowDef object"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/metadata/workflow/{name}/{version}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Delete workflow definition"))
                        .description(Some("Deletes a specific version of a workflow definition"))
                        .operation_id(Some("deleteWorkflowDef"))
                        .parameter(path_param("name", "Workflow definition name"))
                        .parameter(path_param("version", "Workflow definition version"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )

        // ── Metadata: Task Definitions ──
        .path(
            "/api/metadata/taskdefs",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("List all task definitions"))
                        .description(Some("Returns all registered task definitions"))
                        .operation_id(Some("listTaskDefs"))
                        .response("200", ok_arr_ref("TaskDef", "Array of TaskDef objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Register task definitions"))
                        .description(Some("Creates or updates multiple task definitions. Returns the registered TaskDef objects."))
                        .operation_id(Some("registerTaskDefs"))
                        .request_body(Some(arr_ref_body("TaskDef")))
                        .response("200", ok_arr_ref("TaskDef", "Array of registered TaskDef objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Update a task definition"))
                        .description(Some("Creates or updates a single task definition. Returns the updated TaskDef."))
                        .operation_id(Some("updateTaskDef"))
                        .request_body(Some(ref_body("TaskDef")))
                        .response("200", ok_ref("TaskDef", "The updated TaskDef"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/metadata/taskdefs/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Get task definition"))
                        .description(Some("Returns a task definition by its name"))
                        .operation_id(Some("getTaskDef"))
                        .parameter(path_param("taskType", "Task definition name"))
                        .response("200", ok_ref("TaskDef", "TaskDef object"))
                        .response("404", not_found())
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Delete task definition"))
                        .description(Some("Deletes a task definition by name"))
                        .operation_id(Some("deleteTaskDef"))
                        .parameter(path_param("taskType", "Task definition name"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Workflow ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/workflow",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Start a workflow"))
                        .description(Some("Starts a new workflow execution from a StartWorkflowRequest. Returns the workflow ID."))
                        .operation_id(Some("startWorkflow"))
                        .request_body(Some(ref_body("StartWorkflowRequest")))
                        .response("200", ok_str("Workflow ID (string)"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/stats",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Get workflow statistics"))
                        .description(Some("Returns counts of workflows grouped by status"))
                        .operation_id(Some("getWorkflowStats"))
                        .response("200", ok_json("Map of status to count"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/search",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Search workflows"))
                        .description(Some("Search for workflows with optional filters for status, type, and free text"))
                        .operation_id(Some("searchWorkflows"))
                        .parameter(query_param("status", "Workflow status filter (RUNNING, COMPLETED, FAILED, etc.)"))
                        .parameter(query_param("workflowType", "Workflow type/name filter"))
                        .parameter(query_param("freeText", "Free text search across name, ID, correlationId"))
                        .parameter(query_param("start", "Pagination offset (default: 0)"))
                        .parameter(query_param("size", "Page size (default: 100)"))
                        .response("200", ok_ref("SearchResultWorkflowSummary", "Search results with totalHits and WorkflowSummary array"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/running/{name}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Get running workflows by name"))
                        .description(Some("Returns workflow IDs of all currently running workflows of a given type"))
                        .operation_id(Some("getRunningWorkflows"))
                        .parameter(path_param("name", "Workflow type name"))
                        .parameter(query_param("version", "Workflow version filter"))
                        .parameter(query_param("startTime", "Start time filter (epoch ms)"))
                        .parameter(query_param("endTime", "End time filter (epoch ms)"))
                        .response("200", ok_str_arr("Array of workflow ID strings"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Get workflow by ID"))
                        .description(Some("Returns the full workflow execution including all tasks"))
                        .operation_id(Some("getWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok_ref("Workflow", "Workflow object with tasks"))
                        .response("404", not_found())
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Terminate workflow"))
                        .description(Some("Terminates a running workflow. Optionally provide a reason via query parameter."))
                        .operation_id(Some("terminateWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(query_param("reason", "Reason for termination"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/status",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Get workflow status"))
                        .description(Some("Returns workflow execution status. Use includeTasks=false for lightweight status check."))
                        .operation_id(Some("getWorkflowStatus"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(query_param("includeTasks", "Include task details (default: true)"))
                        .response("200", ok_ref("Workflow", "Workflow object (optionally without tasks)"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/remove",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Remove (delete) workflow"))
                        .description(Some("Permanently removes a workflow execution. Use archiveWorkflow=true (default) to terminate instead of hard delete."))
                        .operation_id(Some("deleteWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(query_param("archiveWorkflow", "Archive instead of hard delete (default: true)"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/pause",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Pause workflow"))
                        .description(Some("Pauses a running workflow. No new tasks will be scheduled until resumed."))
                        .operation_id(Some("pauseWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/resume",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Resume workflow"))
                        .description(Some("Resumes a paused workflow"))
                        .operation_id(Some("resumeWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/restart",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Restart workflow"))
                        .description(Some("Restarts a completed/failed workflow from the beginning, deleting all existing tasks"))
                        .operation_id(Some("restartWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/retry",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Retry workflow"))
                        .description(Some("Retries a failed workflow by re-scheduling all failed tasks"))
                        .operation_id(Some("retryWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .response("404", not_found())
                        .response("409", conflict())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/rerun",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Rerun workflow"))
                        .description(Some("Re-runs a workflow from a specific task or from the beginning with new input. Returns the new workflow ID."))
                        .operation_id(Some("rerunWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .request_body(Some(ref_body("RerunWorkflowRequest")))
                        .response("200", ok_str("New workflow ID (string)"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/decide",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Decide workflow"))
                        .description(Some("Triggers a workflow decision/advancement. Forces the engine to evaluate the next pending task(s)."))
                        .operation_id(Some("decideWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/variables",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Update workflow variables"))
                        .description(Some("Updates (merges) workflow variables. Returns the updated Workflow object."))
                        .operation_id(Some("updateWorkflowVariables"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .request_body(Some(json_body()))
                        .response("200", ok_ref("Workflow", "Updated Workflow object"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{workflowId}/skiptask/{taskReferenceName}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Skip task in workflow"))
                        .description(Some("Skips a task in the workflow by marking it SKIPPED with optional output"))
                        .operation_id(Some("skipTask"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(path_param("taskReferenceName", "Task reference name to skip"))
                        .request_body(Some(ref_body("SkipTaskRequest")))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/{name}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Start a workflow by name"))
                        .description(Some("Starts a new workflow by name with the request body as input. Returns the workflow ID."))
                        .operation_id(Some("startWorkflowByName"))
                        .parameter(path_param("name", "Workflow definition name"))
                        .parameter(query_param("version", "Workflow version (default: 1)"))
                        .parameter(query_param("correlationId", "Correlation ID"))
                        .parameter(query_param("priority", "Workflow priority (default: 0)"))
                        .request_body(Some(json_body()))
                        .response("200", ok_str("Workflow ID (string)"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Bulk Workflow Operations ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/workflow/bulk/pause",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk pause workflows"))
                        .description(Some("Pauses multiple workflows. Returns BulkResponse with successful and failed IDs."))
                        .operation_id(Some("bulkPauseWorkflows"))
                        .request_body(Some(str_arr_body()))
                        .response("200", ok_ref("BulkResponse", "BulkResponse with bulkSuccessfulResults and bulkErrorResults"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/bulk/resume",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk resume workflows"))
                        .description(Some("Resumes multiple paused workflows. Returns BulkResponse with results."))
                        .operation_id(Some("bulkResumeWorkflows"))
                        .request_body(Some(str_arr_body()))
                        .response("200", ok_ref("BulkResponse", "BulkResponse with results"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/bulk/retry",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk retry workflows"))
                        .description(Some("Retries multiple failed workflows. Returns BulkResponse with results."))
                        .operation_id(Some("bulkRetryWorkflows"))
                        .request_body(Some(str_arr_body()))
                        .response("200", ok_ref("BulkResponse", "BulkResponse with results"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/bulk/restart",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk restart workflows"))
                        .description(Some("Restarts multiple workflows from the beginning. Returns BulkResponse with results."))
                        .operation_id(Some("bulkRestartWorkflows"))
                        .request_body(Some(str_arr_body()))
                        .response("200", ok_ref("BulkResponse", "BulkResponse with results"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/workflow/bulk/terminate",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk terminate workflows"))
                        .description(Some("Terminates multiple workflows with an optional reason. Returns BulkResponse with results."))
                        .operation_id(Some("bulkTerminateWorkflows"))
                        .request_body(Some(json_body()))
                        .response("200", ok_ref("BulkResponse", "BulkResponse with results"))
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Tasks ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/tasks",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Update task"))
                        .description(Some("Updates a task's status and output data. Returns the task ID. Triggers workflow advancement on completion."))
                        .operation_id(Some("updateTask"))
                        .request_body(Some(ref_body("TaskUpdateRequest")))
                        .response("200", ok_str("Task ID (string)"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/search",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Search tasks"))
                        .description(Some("Search for tasks with optional filters"))
                        .operation_id(Some("searchTasks"))
                        .parameter(query_param("workflowId", "Filter by workflow instance ID"))
                        .parameter(query_param("taskType", "Filter by task type"))
                        .parameter(query_param("status", "Filter by task status"))
                        .parameter(query_param("freeText", "Free text search"))
                        .parameter(query_param("start", "Pagination offset (default: 0)"))
                        .parameter(query_param("size", "Page size (default: 100)"))
                        .response("200", ok_ref("SearchResultTaskSummary", "Search results with totalHits and TaskSummary array"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/poll/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Poll for a task"))
                        .description(Some("Polls for a single task of the given type. Returns 204 No Content if no task available. Transitions the task to IN_PROGRESS."))
                        .operation_id(Some("pollTask"))
                        .parameter(path_param("taskType", "Task type to poll for"))
                        .parameter(query_param("workerId", "Worker ID claiming the task"))
                        .parameter(query_param("domain", "Task domain to poll, using Netflix Conductor domain routing"))
                        .response("200", ok_ref("PollTask", "PollTask object with task details and input"))
                        .response("204", no_content())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/poll/batch/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Batch poll for tasks"))
                        .description(Some("Polls for multiple tasks of the given type"))
                        .operation_id(Some("batchPollTasks"))
                        .parameter(path_param("taskType", "Task type to poll for"))
                        .parameter(query_param("workerId", "Worker ID claiming the tasks"))
                        .parameter(query_param("domain", "Task domain to poll, using Netflix Conductor domain routing"))
                        .parameter(query_param("count", "Number of tasks to poll (default: 1)"))
                        .parameter(query_param("timeout", "Poll timeout in ms (default: 100)"))
                        .response("200", ok_arr_ref("PollTask", "Array of PollTask objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/queue/sizes",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get task queue sizes"))
                        .description(Some("Returns the number of SCHEDULED tasks for each task type"))
                        .operation_id(Some("getQueueSizes"))
                        .response("200", ok_json("Map of task type name to count"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/queue/all",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get all queue details"))
                        .description(Some("Returns queue sizes for all task types"))
                        .operation_id(Some("getAllQueueDetails"))
                        .response("200", ok_json("Map of task type name to count"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/queue/all/verbose",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get all queue details (verbose)"))
                        .description(Some("Returns detailed queue information including per-status counts for each task type"))
                        .operation_id(Some("getAllQueueDetailsVerbose"))
                        .response("200", ok_json("Map of task type to map of status to count"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/queue/polldata",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get poll data for a task type"))
                        .description(Some("Returns poll metadata (worker IDs, last poll times) for a specific task type"))
                        .operation_id(Some("getPollData"))
                        .parameter(query_param("taskType", "Task type to get poll data for"))
                        .response("200", ok_arr_ref("PollData", "Array of PollData objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/queue/polldata/all",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get poll data for all task types"))
                        .description(Some("Returns poll metadata for all task types"))
                        .operation_id(Some("getAllPollData"))
                        .response("200", ok_arr_ref("PollData", "Array of PollData objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/in_progress/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get in-progress tasks by type"))
                        .description(Some("Returns all tasks currently IN_PROGRESS for a given task type"))
                        .operation_id(Some("getInProgressTasks"))
                        .parameter(path_param("taskType", "Task type name"))
                        .response("200", ok_arr_ref("TaskResult", "Array of TaskResult objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/in_progress/{workflowId}/{taskRefName}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get in-progress task for a workflow"))
                        .description(Some("Returns the in-progress task for a specific workflow and task reference name"))
                        .operation_id(Some("getInProgressTaskForWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(path_param("taskRefName", "Task reference name"))
                        .response("200", ok_ref("TaskResult", "TaskResult object"))
                        .response("204", no_content())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/{workflowId}/{taskRefName}/{status}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Update task by reference name"))
                        .description(Some("Updates a task identified by workflow ID and reference name to the given status. Request body is the output data. Returns the task ID."))
                        .operation_id(Some("updateTaskByRefName"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .parameter(path_param("taskRefName", "Task reference name"))
                        .parameter(path_param("status", "New task status (COMPLETED, FAILED, etc.)"))
                        .request_body(Some(json_body()))
                        .response("200", ok_str("Task ID (string)"))
                        .response("404", not_found())
                        .response("409", conflict())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/{taskId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get task by ID"))
                        .description(Some("Returns the full task execution details"))
                        .operation_id(Some("getTask"))
                        .parameter(path_param("taskId", "Task instance ID"))
                        .response("200", ok_ref("TaskResult", "TaskResult object"))
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/{taskId}/ack",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Acknowledge task"))
                        .description(Some("Acknowledges a task is being processed by a worker. Returns true if acknowledged."))
                        .operation_id(Some("ackTask"))
                        .parameter(path_param("taskId", "Task instance ID"))
                        .parameter(query_param("workerId", "Worker ID acknowledging the task"))
                        .response("200", ok_bool("Boolean - true if task was acknowledged"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/tasks/{taskId}/log",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Get task execution logs"))
                        .description(Some("Returns execution log entries for a task"))
                        .operation_id(Some("getTaskLogs"))
                        .parameter(path_param("taskId", "Task instance ID"))
                        .response("200", ok_arr_ref("TaskExecLog", "Array of TaskExecLog objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Add task execution log"))
                        .description(Some("Appends a log entry to the task's execution log"))
                        .operation_id(Some("addTaskLog"))
                        .parameter(path_param("taskId", "Task instance ID"))
                        .request_body(Some(ref_body("TaskExecLog")))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Event Handlers ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/event",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Get all event handlers"))
                        .description(Some("Returns all registered event handlers"))
                        .operation_id(Some("getEventHandlers"))
                        .response("200", ok_arr_ref("EventHandler", "Array of EventHandler objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Register event handler"))
                        .description(Some("Creates or updates an event handler. Returns the registered EventHandler."))
                        .operation_id(Some("registerEventHandler"))
                        .request_body(Some(ref_body("EventHandler")))
                        .response("200", ok_ref("EventHandler", "The registered EventHandler"))
                        .build(),
                )
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Update event handler"))
                        .description(Some("Updates an existing event handler. Returns the updated EventHandler."))
                        .operation_id(Some("updateEventHandler"))
                        .request_body(Some(ref_body("EventHandler")))
                        .response("200", ok_ref("EventHandler", "The updated EventHandler"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/event/{name}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Get event handlers for event"))
                        .description(Some("Returns event handlers registered for a specific event name"))
                        .operation_id(Some("getEventHandlersForEvent"))
                        .parameter(path_param("name", "Event name"))
                        .parameter(query_param("activeOnly", "Return only active handlers (default: true)"))
                        .response("200", ok_arr_ref("EventHandler", "Array of EventHandler objects"))
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Delete event handler"))
                        .description(Some("Deletes an event handler by name"))
                        .operation_id(Some("deleteEventHandler"))
                        .parameter(path_param("name", "Event handler name"))
                        .response("200", ok())
                        .response("404", not_found())
                        .build(),
                )
                .build(),
        )

        // ══════════════════════════════════════════════════════════════
        // ── Admin ──
        // ══════════════════════════════════════════════════════════════
        .path(
            "/api/admin/config",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("admin")))
                        .summary(Some("Get all configuration"))
                        .description(Some("Returns all configuration key-value pairs"))
                        .operation_id(Some("getAllConfig"))
                        .response("200", ok_json("Map of config key to value"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/admin/sweep/{workflowId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("admin")))
                        .summary(Some("Sweep workflow"))
                        .description(Some("Triggers a workflow sweep/decide for the given workflow ID"))
                        .operation_id(Some("sweepWorkflow"))
                        .parameter(path_param("workflowId", "Workflow instance ID"))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/admin/task/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("admin")))
                        .summary(Some("Get all tasks for a task type"))
                        .description(Some("Returns tasks (up to 100) for a specific task type across all shards"))
                        .operation_id(Some("getTasksForType"))
                        .parameter(path_param("taskType", "Task type name"))
                        .response("200", ok_arr_ref("TaskResult", "Array of TaskResult objects"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/admin/task/{taskType}/requeuetasks",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("admin")))
                        .summary(Some("Requeue pending tasks"))
                        .description(Some("Re-enqueues all SCHEDULED tasks for a given task type to Kafka. Returns count of requeued tasks."))
                        .operation_id(Some("requeuePendingTasks"))
                        .parameter(path_param("taskType", "Task type name"))
                        .response("200", ok_int("Count of requeued tasks (number)"))
                        .build(),
                )
                .build(),
        )

        // ── Queue Admin ──
        .path(
            "/api/queue/pause/{queueName}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("queue-admin")))
                        .summary(Some("Pause queue"))
                        .description(Some("Pauses a task queue, preventing any tasks from being polled"))
                        .operation_id(Some("pauseQueue"))
                        .parameter(path_param("queueName", "Queue/task type name"))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/queue/resume/{queueName}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("queue-admin")))
                        .summary(Some("Resume queue"))
                        .description(Some("Resumes a previously paused task queue"))
                        .operation_id(Some("resumeQueue"))
                        .parameter(path_param("queueName", "Queue/task type name"))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/queue/status/{queueName}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("queue-admin")))
                        .summary(Some("Get queue pause status"))
                        .description(Some("Returns whether a queue is currently paused"))
                        .operation_id(Some("getQueueStatus"))
                        .parameter(path_param("queueName", "Queue/task type name"))
                        .response("200", ok_json("Object with 'paused' boolean field"))
                        .build(),
                )
                .build(),
        )

        // ── SSE Endpoints (#162) ──
        .path(
            "/api/sse/workflow/{workflowId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("SSE")
                        .summary(Some("Stream workflow status via SSE"))
                        .description(Some("Server-Sent Events endpoint that streams real-time workflow status updates until the workflow reaches a terminal state."))
                        .operation_id(Some("sseWorkflowStatus"))
                        .parameter(path_param("workflowId", "Workflow execution ID"))
                        .parameter(
                            ParameterBuilder::new()
                                .name("intervalMs")
                                .parameter_in(ParameterIn::Query)
                                .description(Some("Polling interval in milliseconds (default 2000, min 500)"))
                                .schema(Some(i()))
                                .build(),
                        )
                        .response("200", ResponseBuilder::new().description("SSE event stream (text/event-stream)").build())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/sse/queue/sizes",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("SSE")
                        .summary(Some("Stream queue sizes via SSE"))
                        .description(Some("Server-Sent Events endpoint that streams queue size updates at a configurable interval."))
                        .operation_id(Some("sseQueueSizes"))
                        .response("200", ResponseBuilder::new().description("SSE event stream (text/event-stream)").build())
                        .build(),
                )
                .build(),
        )

        // ── WebSocket Endpoints (#163) ──
        .path(
            "/api/ws/workflow/{workflowId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("WebSocket")
                        .summary(Some("WebSocket for bidirectional workflow communication"))
                        .description(Some("Upgrade to WebSocket for real-time workflow status. Supports subscribe/unsubscribe/getStatus messages."))
                        .operation_id(Some("wsWorkflow"))
                        .parameter(path_param("workflowId", "Initial workflow ID to track"))
                        .response("101", ResponseBuilder::new().description("WebSocket upgrade").build())
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/ws/tasks/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("WebSocket")
                        .summary(Some("WebSocket for task worker feed"))
                        .description(Some("Upgrade to WebSocket for bidirectional task communication. Tasks are pushed to the worker; worker sends task updates back."))
                        .operation_id(Some("wsTaskFeed"))
                        .parameter(path_param("taskType", "Task type to poll for"))
                        .response("101", ResponseBuilder::new().description("WebSocket upgrade").build())
                        .build(),
                )
                .build(),
        )

        // ── GraphQL Endpoint (#161) ──
        .path(
            "/api/graphql",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tag("GraphQL")
                        .summary(Some("GraphQL endpoint"))
                        .description(Some("Execute GraphQL queries and mutations against the Conductor API. Visit GET /api/graphql for the interactive playground."))
                        .operation_id(Some("graphql"))
                        .request_body(Some(json_body()))
                        .response("200", ok_json("GraphQL response"))
                        .build(),
                )
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("GraphQL")
                        .summary(Some("GraphQL playground"))
                        .description(Some("Interactive GraphQL playground for exploring the API."))
                        .operation_id(Some("graphqlPlayground"))
                        .response("200", ResponseBuilder::new().description("HTML playground").build())
                        .build(),
                )
                .build(),
        )

        // ── Batch Endpoint (#166) ──
        .path(
            "/api/batch",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tag("Batch")
                        .summary(Some("Execute multiple API calls in a single request"))
                        .description(Some("Request batching endpoint — execute multiple operations atomically in a single HTTP request."))
                        .operation_id(Some("batchExecute"))
                        .request_body(Some(json_body()))
                        .response("200", ok_json("Array of operation results"))
                        .build(),
                )
                .build(),
        )

        // ── V2 API Endpoints (#165) ──
        .path(
            "/api/v2/workflow",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tag("V2 API")
                        .summary(Some("Start workflow (V2 envelope)"))
                        .description(Some("Start a new workflow execution. V2 returns an envelope with apiVersion and structured data."))
                        .operation_id(Some("startWorkflowV2"))
                        .request_body(Some(ref_body("StartWorkflowRequest")))
                        .response("200", ok_json("V2 envelope with workflowId"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/v2/workflow/{workflowId}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("V2 API")
                        .summary(Some("Get workflow (V2 envelope)"))
                        .description(Some("Retrieve workflow execution details. V2 returns an envelope with apiVersion."))
                        .operation_id(Some("getWorkflowV2"))
                        .parameter(path_param("workflowId", "Workflow execution ID"))
                        .response("200", ok_json("V2 envelope with Workflow"))
                        .build(),
                )
                .build(),
        )
        .path(
            "/api/v2/tasks/poll/long/{taskType}",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tag("V2 API")
                        .summary(Some("Long-poll for tasks"))
                        .description(Some("Long-polling endpoint for workers — waits server-side for a task to become available (up to timeoutMs). Returns 204 if timeout expires with no task."))
                        .operation_id(Some("longPollTask"))
                        .parameter(path_param("taskType", "Task type name"))
                        .parameter(
                            ParameterBuilder::new()
                                .name("workerId")
                                .parameter_in(ParameterIn::Query)
                                .description(Some("Worker identification"))
                                .schema(Some(s()))
                                .build(),
                        )
                        .parameter(
                            ParameterBuilder::new()
                                .name("timeoutMs")
                                .parameter_in(ParameterIn::Query)
                                .description(Some("Maximum wait time in milliseconds (default 10000, max 30000)"))
                                .schema(Some(i()))
                                .build(),
                        )
                        .response("200", ok_json("V2 envelope with task"))
                        .response("204", ResponseBuilder::new().description("No task available within timeout").build())
                        .build(),
                )
                .build(),
        )

        .build();

    let components = ComponentsBuilder::new()
        // ── Enums ──
        .schema(
            "WorkflowStatus",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .description(Some("Status of a workflow execution"))
                .enum_values(Some([
                    "RUNNING",
                    "COMPLETED",
                    "FAILED",
                    "TIMED_OUT",
                    "TERMINATED",
                    "PAUSED",
                ])),
        )
        .schema(
            "TaskStatus",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .description(Some("Status of a task"))
                .enum_values(Some([
                    "IN_PROGRESS",
                    "CANCELED",
                    "FAILED",
                    "FAILED_WITH_TERMINAL_ERROR",
                    "COMPLETED",
                    "COMPLETED_WITH_ERRORS",
                    "SCHEDULED",
                    "TIMED_OUT",
                    "SKIPPED",
                ])),
        )
        .schema(
            "TimeoutPolicy",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(["TIME_OUT_WF", "ALERT_ONLY"])),
        )
        .schema(
            "TaskTimeoutPolicy",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(["RETRY", "TIME_OUT_WF", "ALERT_ONLY"])),
        )
        .schema(
            "RetryLogic",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(["FIXED", "EXPONENTIAL_BACKOFF", "LINEAR_BACKOFF"])),
        )
        .schema(
            "IdempotencyStrategy",
            ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::String))
                .enum_values(Some(["FAIL", "RETURN_EXISTING", "FAIL_ON_RUNNING"])),
        )
        // ── Health ──
        .schema(
            "Health",
            ObjectBuilder::new()
                .property("healthy", b())
                .required("healthy")
                .property(
                    "details",
                    ObjectBuilder::new().description(Some("Detailed health info per service")),
                )
                .property("errorMessage", s()),
        )
        .schema(
            "HealthCheckStatus",
            ObjectBuilder::new()
                .property("healthy", b())
                .required("healthy")
                .property("healthResults", ArrayBuilder::new().items(sref("Health")))
                .property(
                    "suppressedHealthResults",
                    ArrayBuilder::new().items(sref("Health")),
                ),
        )
        // ── Workflow Definition ──
        .schema(
            "WorkflowTask",
            ObjectBuilder::new()
                .description(Some("A task within a workflow definition"))
                .property("name", s())
                .required("name")
                .property("taskReferenceName", s())
                .required("taskReferenceName")
                .property(
                    "type",
                    s().description(Some(
                        "Task type: SIMPLE, SUB_WORKFLOW, DECISION, FORK_JOIN, etc.",
                    )),
                )
                .property("description", s())
                .property(
                    "inputParameters",
                    ObjectBuilder::new()
                        .description(Some("Map of input parameter name to value/expression")),
                )
                .property("optional", b())
                .property("startDelay", i())
                .property(
                    "subWorkflowParam",
                    ObjectBuilder::new()
                        .property("name", s())
                        .property("version", i())
                        .property("taskToDomain", ObjectBuilder::new()),
                )
                .property("joinOn", ArrayBuilder::new().items(s()))
                .property(
                    "forkTasks",
                    ArrayBuilder::new().items(ArrayBuilder::new().items(sref("WorkflowTask"))),
                )
                .property(
                    "decisionCases",
                    ObjectBuilder::new().description(Some("Map of case value to task list")),
                )
                .property(
                    "defaultCase",
                    ArrayBuilder::new().items(sref("WorkflowTask")),
                )
                .property("caseExpression", s())
                .property("caseValueParam", s())
                .property("loopCondition", s())
                .property("loopOver", ArrayBuilder::new().items(sref("WorkflowTask")))
                .property("retryCount", i())
                .property("evaluatorType", s())
                .property("expression", s())
                .property("scriptExpression", s())
                .property("sink", s())
                .property("asyncComplete", b()),
        )
        .schema(
            "WorkflowDef",
            ObjectBuilder::new()
                .description(Some("Workflow definition"))
                .property("name", s())
                .required("name")
                .property("description", s())
                .property(
                    "version",
                    i().description(Some("Version number (default: 1)")),
                )
                .property("tasks", ArrayBuilder::new().items(sref("WorkflowTask")))
                .required("tasks")
                .property("inputParameters", ArrayBuilder::new().items(s()))
                .property(
                    "outputParameters",
                    ObjectBuilder::new()
                        .description(Some("Map of output parameter name to expression")),
                )
                .property(
                    "failureWorkflow",
                    s().description(Some("Workflow to run on failure")),
                )
                .property("schemaVersion", i())
                .property("restartable", b())
                .property("workflowStatusListenerEnabled", b())
                .property("ownerEmail", s())
                .property("timeoutPolicy", sref("TimeoutPolicy"))
                .property("timeoutSeconds", i())
                .property(
                    "variables",
                    ObjectBuilder::new().description(Some("Workflow-level variables")),
                )
                .property("inputTemplate", ObjectBuilder::new())
                .property("ownerApp", s())
                .property("createTime", i())
                .property("updateTime", i())
                .property("createdBy", s())
                .property("updatedBy", s()),
        )
        // ── Workflow Execution ──
        .schema(
            "Workflow",
            ObjectBuilder::new()
                .description(Some("Workflow execution instance"))
                .property("workflowId", s())
                .required("workflowId")
                .property("workflowName", s())
                .required("workflowName")
                .property("workflowVersion", i())
                .property("status", sref("WorkflowStatus"))
                .required("status")
                .property(
                    "input",
                    ObjectBuilder::new().description(Some("Workflow input data")),
                )
                .property(
                    "output",
                    ObjectBuilder::new().description(Some("Workflow output data")),
                )
                .property("tasks", ArrayBuilder::new().items(sref("TaskResult")))
                .property("correlationId", s())
                .property("startTime", i().description(Some("Epoch milliseconds")))
                .property("endTime", i())
                .property("updateTime", i())
                .property("createdBy", s())
                .property("updatedBy", s())
                .property("reasonForIncompletion", s())
                .property("workflowDefinition", sref("WorkflowDef"))
                .property("priority", i())
                .property("variables", ObjectBuilder::new())
                .property("failedReferenceTaskNames", ArrayBuilder::new().items(s()))
                .property("ownerApp", s())
                .property("parentWorkflowId", s())
                .property("parentWorkflowTaskId", s())
                .property("reRunFromWorkflowId", s())
                .property("event", s())
                .property("taskToDomain", ObjectBuilder::new())
                .property("failedTaskNames", ArrayBuilder::new().items(s()))
                .property("lastRetriedTime", i())
                .property("idempotencyKey", s()),
        )
        .schema(
            "StartWorkflowRequest",
            ObjectBuilder::new()
                .description(Some("Request to start a new workflow"))
                .property("name", s().description(Some("Workflow definition name")))
                .required("name")
                .property(
                    "version",
                    i().description(Some("Workflow version (default: 1)")),
                )
                .property(
                    "input",
                    ObjectBuilder::new().description(Some("Workflow input data as JSON object")),
                )
                .property(
                    "correlationId",
                    s().description(Some("User-supplied correlation ID")),
                )
                .property("priority", i().description(Some("Priority (default: 0)")))
                .property(
                    "taskToDomain",
                    ObjectBuilder::new().description(Some("Map of task ref name to domain")),
                )
                .property("workflowDef", sref("WorkflowDef"))
                .property("createdBy", s())
                .property("idempotencyKey", s())
                .property("idempotencyStrategy", sref("IdempotencyStrategy")),
        )
        .schema(
            "RerunWorkflowRequest",
            ObjectBuilder::new()
                .description(Some("Request to re-run a workflow"))
                .property("reRunFromWorkflowId", s())
                .property("workflowInput", ObjectBuilder::new())
                .property("reRunFromTaskId", s())
                .property("taskInput", ObjectBuilder::new())
                .property("correlationId", s()),
        )
        .schema(
            "SkipTaskRequest",
            ObjectBuilder::new()
                .description(Some("Request to skip a task in a workflow"))
                .property("taskInput", ObjectBuilder::new())
                .property("taskOutput", ObjectBuilder::new()),
        )
        // ── Task Definition ──
        .schema(
            "TaskDef",
            ObjectBuilder::new()
                .description(Some("Task type definition"))
                .property("name", s())
                .required("name")
                .property("description", s())
                .property(
                    "retryCount",
                    i().description(Some("Number of retries (default: 3)")),
                )
                .property("retryLogic", sref("RetryLogic"))
                .property(
                    "retryDelaySeconds",
                    i().description(Some("Delay between retries (default: 60)")),
                )
                .property(
                    "timeoutSeconds",
                    i().description(Some("Task timeout in seconds (default: 3600)")),
                )
                .property("timeoutPolicy", sref("TaskTimeoutPolicy"))
                .property(
                    "responseTimeoutSeconds",
                    i().description(Some("Worker response timeout (default: 600)")),
                )
                .property("concurrentExecLimit", i())
                .property("inputKeys", ArrayBuilder::new().items(s()))
                .property("outputKeys", ArrayBuilder::new().items(s()))
                .property("inputTemplate", ObjectBuilder::new())
                .property("rateLimitPerFrequency", i())
                .property("rateLimitFrequencyInSeconds", i())
                .property("ownerEmail", s())
                .property("pollTimeoutSeconds", i())
                .property("backoffScaleFactor", i())
                .property("totalTimeoutSeconds", i())
                .property("ownerApp", s())
                .property("createTime", i())
                .property("updateTime", i())
                .property("createdBy", s())
                .property("updatedBy", s())
                .property("enforceSchema", b()),
        )
        // ── Task Execution ──
        .schema(
            "TaskResult",
            ObjectBuilder::new()
                .description(Some("Task execution result"))
                .property("taskId", s())
                .required("taskId")
                .property("workflowInstanceId", s())
                .required("workflowInstanceId")
                .property("taskType", s())
                .property("taskDefName", s())
                .property("referenceTaskName", s())
                .property("status", sref("TaskStatus"))
                .required("status")
                .property(
                    "inputData",
                    ObjectBuilder::new().description(Some("Task input data")),
                )
                .property(
                    "outputData",
                    ObjectBuilder::new().description(Some("Task output data")),
                )
                .property("reasonForIncompletion", s())
                .property("scheduledTime", i())
                .property("startTime", i())
                .property("endTime", i())
                .property("updateTime", i())
                .property("pollCount", i())
                .property("workerId", s())
                .property("seq", i())
                .property("retryCount", i())
                .property("callbackAfterSeconds", i())
                .property("logs", ArrayBuilder::new().items(sref("TaskExecLog")))
                .property("correlationId", s())
                .property("retriedTaskId", s())
                .property("retried", b())
                .property("executed", b())
                .property("callbackFromWorker", b())
                .property("responseTimeoutSeconds", i())
                .property("workflowType", s())
                .property("domain", s())
                .property("iteration", i())
                .property("subWorkflowId", s()),
        )
        .schema(
            "TaskUpdateRequest",
            ObjectBuilder::new()
                .description(Some("Request to update a task's status"))
                .property("taskId", s())
                .required("taskId")
                .property("workflowInstanceId", s())
                .required("workflowInstanceId")
                .property("status", sref("TaskStatus"))
                .required("status")
                .property(
                    "outputData",
                    ObjectBuilder::new().description(Some("Task output data")),
                )
                .property("reasonForIncompletion", s())
                .property("callbackAfterSeconds", i())
                .property("workerId", s())
                .property("logs", ArrayBuilder::new().items(sref("TaskExecLog")))
                .property("extendLease", b()),
        )
        .schema(
            "TaskExecLog",
            ObjectBuilder::new()
                .description(Some("Task execution log entry"))
                .property("log", s())
                .required("log")
                .property("taskId", s())
                .property("createdTime", i()),
        )
        .schema(
            "PollTask",
            ObjectBuilder::new()
                .description(Some("Task returned from polling"))
                .property("taskId", s())
                .required("taskId")
                .property("workflowInstanceId", s())
                .required("workflowInstanceId")
                .property("taskType", s())
                .property("taskDefName", s())
                .property("referenceTaskName", s())
                .property("status", sref("TaskStatus"))
                .property(
                    "inputData",
                    ObjectBuilder::new().description(Some("Task input data")),
                )
                .property("scheduledTime", i())
                .property("startTime", i())
                .property("callbackAfterSeconds", i())
                .property("pollCount", i())
                .property("retryCount", i()),
        )
        .schema(
            "PollData",
            ObjectBuilder::new()
                .description(Some("Poll metadata for a task queue"))
                .property("queueName", s())
                .property("domain", s())
                .property("workerId", s())
                .property("lastPollTime", i()),
        )
        // ── Event Handler ──
        .schema(
            "EventAction",
            ObjectBuilder::new()
                .description(Some("Action to execute when an event is received"))
                .property(
                    "action",
                    ObjectBuilder::new()
                        .schema_type(SchemaType::Type(Type::String))
                        .enum_values(Some([
                            "start_workflow",
                            "complete_task",
                            "fail_task",
                            "terminate_workflow",
                            "update_workflow_variables",
                        ])),
                )
                .property(
                    "startWorkflow",
                    ObjectBuilder::new()
                        .property("name", s())
                        .property("version", i())
                        .property("correlationId", s())
                        .property("input", ObjectBuilder::new())
                        .property("taskToDomain", ObjectBuilder::new()),
                )
                .property(
                    "completeTask",
                    ObjectBuilder::new()
                        .property("workflowId", s())
                        .property("taskRefName", s())
                        .property("output", ObjectBuilder::new())
                        .property("taskId", s()),
                )
                .property(
                    "failTask",
                    ObjectBuilder::new()
                        .property("workflowId", s())
                        .property("taskRefName", s())
                        .property("output", ObjectBuilder::new())
                        .property("taskId", s()),
                )
                .property(
                    "terminateWorkflow",
                    ObjectBuilder::new()
                        .property("workflowId", s())
                        .property("terminationReason", s()),
                )
                .property(
                    "updateWorkflowVariables",
                    ObjectBuilder::new()
                        .property("workflowId", s())
                        .property("variables", ObjectBuilder::new())
                        .property("appendArray", b()),
                )
                .property("expandInlineJson", b()),
        )
        .schema(
            "EventHandler",
            ObjectBuilder::new()
                .description(Some("Event handler definition"))
                .property("name", s())
                .required("name")
                .property(
                    "event",
                    s().description(Some("Event source (e.g., conductor:workflow_name:status)")),
                )
                .required("event")
                .property(
                    "condition",
                    s().description(Some("ECMAScript expression to evaluate")),
                )
                .property("actions", ArrayBuilder::new().items(sref("EventAction")))
                .required("actions")
                .property(
                    "active",
                    b().description(Some("Whether handler is active (default: true)")),
                )
                .property("evaluatorType", s()),
        )
        // ── Search Results ──
        .schema(
            "WorkflowSummary",
            ObjectBuilder::new()
                .description(Some("Summary of a workflow execution"))
                .property("workflowId", s())
                .property("workflowType", s())
                .property("version", i())
                .property("status", sref("WorkflowStatus"))
                .property("startTime", s())
                .property("updateTime", s())
                .property("endTime", s())
                .property("input", s().description(Some("Truncated JSON string")))
                .property("output", s().description(Some("Truncated JSON string")))
                .property("correlationId", s())
                .property("reasonForIncompletion", s())
                .property("executionTime", i())
                .property("event", s())
                .property("failedReferenceTaskNames", s())
                .property("priority", i())
                .property("failedTaskNames", ArrayBuilder::new().items(s()))
                .property("createdBy", s())
                .property("idempotencyKey", s()),
        )
        .schema(
            "TaskSummary",
            ObjectBuilder::new()
                .description(Some("Summary of a task execution"))
                .property("workflowId", s())
                .property("workflowType", s())
                .property("correlationId", s())
                .property("scheduledTime", s())
                .property("startTime", s())
                .property("updateTime", s())
                .property("endTime", s())
                .property("status", sref("TaskStatus"))
                .property("reasonForIncompletion", s())
                .property("executionTime", i())
                .property("queueWaitTime", i())
                .property("taskDefName", s())
                .property("taskType", s())
                .property("input", s())
                .property("output", s())
                .property("taskId", s())
                .property("domain", s()),
        )
        .schema(
            "SearchResultWorkflowSummary",
            ObjectBuilder::new()
                .description(Some("Paginated search result for workflows"))
                .property(
                    "totalHits",
                    i().description(Some("Total number of matching results")),
                )
                .required("totalHits")
                .property(
                    "results",
                    ArrayBuilder::new().items(sref("WorkflowSummary")),
                )
                .required("results"),
        )
        .schema(
            "SearchResultTaskSummary",
            ObjectBuilder::new()
                .description(Some("Paginated search result for tasks"))
                .property(
                    "totalHits",
                    i().description(Some("Total number of matching results")),
                )
                .required("totalHits")
                .property("results", ArrayBuilder::new().items(sref("TaskSummary")))
                .required("results"),
        )
        // ── Bulk Response ──
        .schema(
            "BulkResponse",
            ObjectBuilder::new()
                .description(Some("Response from bulk operations"))
                .property(
                    "bulkErrorResults",
                    ObjectBuilder::new()
                        .description(Some("Map of failed workflow ID to error message")),
                )
                .property("bulkSuccessfulResults", ArrayBuilder::new().items(s())),
        )
        .build();

    OpenApiBuilder::new()
        .info(info)
        .servers(Some(vec![server]))
        .paths(paths)
        .components(Some(components))
        .build()
}

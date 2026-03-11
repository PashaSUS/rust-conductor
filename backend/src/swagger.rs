use utoipa::openapi::path::{HttpMethod, OperationBuilder, PathItemBuilder};
use utoipa::openapi::request_body::RequestBodyBuilder;
use utoipa::openapi::response::ResponseBuilder;
use utoipa::openapi::{
    ComponentsBuilder, ContentBuilder, InfoBuilder, OpenApiBuilder, PathsBuilder,
    ServerBuilder,
};

pub fn build_openapi() -> utoipa::openapi::OpenApi {
    let info = InfoBuilder::new()
        .title("Rust Conductor API")
        .description(Some(
            "A fully Rust-based Conductor-compliant microservice orchestrator",
        ))
        .version("0.1.0")
        .build();

    let server = ServerBuilder::new().url("/").build();

    let json_content = || {
        ContentBuilder::new()
            .schema(Some(utoipa::openapi::schema::ObjectBuilder::new()))
            .build()
    };

    let json_body = || {
        RequestBodyBuilder::new()
            .content("application/json", json_content())
            .build()
    };

    let ok = || ResponseBuilder::new().description("OK").build();
    let ok_json = || {
        ResponseBuilder::new()
            .description("OK")
            .content("application/json", json_content())
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
                        .response("200", ok_json())
                        .build(),
                )
                .build(),
        )
        // ── Metadata: Workflow Definitions ──
        .path(
            "/api/metadata/workflow",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("List all workflow definitions"))
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Register a workflow definition"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
                        .build(),
                )
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Update workflow definitions"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
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
                        .response("200", ok_json())
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
                        .response("200", ok())
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
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Register task definitions"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
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
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("metadata")))
                        .summary(Some("Delete task definition"))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        // ── Workflow ──
        .path(
            "/api/workflow",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Start a workflow"))
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow")))
                        .summary(Some("Terminate workflow"))
                        .response("200", ok())
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
                        .response("200", ok())
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
                        .response("200", ok())
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
                        .response("200", ok())
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
                        .response("200", ok())
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
                        .response("200", ok())
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
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .response("200", ok())
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
                        .request_body(Some(json_body()))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        // ── Bulk Workflow Operations ──
        .path(
            "/api/workflow/bulk/pause",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("workflow-bulk")))
                        .summary(Some("Bulk pause workflows"))
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
                        .build(),
                )
                .build(),
        )
        // ── Tasks ──
        .path(
            "/api/tasks",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Update task"))
                        .request_body(Some(json_body()))
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
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
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("tasks")))
                        .summary(Some("Add task execution log"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        // ── Event Handlers ──
        .path(
            "/api/event",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Get all event handlers"))
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Post,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Register event handler"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
                        .build(),
                )
                .operation(
                    HttpMethod::Put,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Update event handler"))
                        .request_body(Some(json_body()))
                        .response("200", ok())
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
                        .response("200", ok_json())
                        .build(),
                )
                .operation(
                    HttpMethod::Delete,
                    OperationBuilder::new()
                        .tags(Some(tag("event")))
                        .summary(Some("Delete event handler"))
                        .response("200", ok())
                        .build(),
                )
                .build(),
        )
        // ── Admin ──
        .path(
            "/api/admin/config",
            PathItemBuilder::new()
                .operation(
                    HttpMethod::Get,
                    OperationBuilder::new()
                        .tags(Some(tag("admin")))
                        .summary(Some("Get all configuration"))
                        .response("200", ok_json())
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
                        .response("200", ok())
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
                        .response("200", ok_json())
                        .build(),
                )
                .build(),
        )
        .build();

    let components = ComponentsBuilder::new().build();

    OpenApiBuilder::new()
        .info(info)
        .servers(Some(vec![server]))
        .paths(paths)
        .components(Some(components))
        .build()
}

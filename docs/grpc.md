# gRPC API — Rust Conductor

## Overview

Rust Conductor exposes a **high-performance gRPC API** alongside the existing REST/HTTP API. Both interfaces are backed by the identical `WorkflowEngine`, so there is zero behavioral difference between them — use whichever protocol fits your stack.

| Protocol | Default Port | Transport       | Use Case                         |
|----------|-------------|-----------------|----------------------------------|
| REST     | `8090`      | HTTP/1.1 + JSON | Browsers, curl, OpenAPI tooling  |
| gRPC     | `50055`     | HTTP/2 + Protobuf | Microservice workers, SDKs, streaming-ready |

### Why gRPC?

- **Lower latency** — binary Protobuf serialization is ~5-10× faster than JSON for large payloads  
- **Multiplexing** — HTTP/2 allows many concurrent RPCs on a single TCP connection  
- **Streaming-ready** — future server-push / bidirectional streaming for task polling  
- **Type safety** — generated client/server stubs in any language from `.proto` files  
- **Backpressure** — built-in flow control via HTTP/2303  

---

## Architecture

```
                       ┌──────────────────────┐
   gRPC clients ──────►│  nginx-lb (:50055)   │──── grpc_pass ────►  backend:50055
                       │  (HTTP/2 LB)         │                       (tonic)
                       └──────────────────────┘
                       ┌──────────────────────┐
   REST clients ──────►│  nginx-lb (:8090)    │──── proxy_pass ───►  backend:8090
                       │  (HTTP/1.1 LB)       │                       (actix-web)
                       └──────────────────────┘

   Both paths hit the same WorkflowEngine → ShardedPool (Postgres) + ShardedRedis
```

The gRPC server runs in-process on a separate port via `tokio::spawn`, sharing the same `WorkflowEngine` Arc reference. There is no inter-process communication overhead.

---

## Services

Six gRPC services mirror the six REST API resource groups:

### 1. MetadataService

Manage workflow and task **definitions** (schemas).

| RPC                    | REST Equivalent                                  | Description                            |
|------------------------|--------------------------------------------------|----------------------------------------|
| `RegisterWorkflowDef`  | `POST /api/metadata/workflow`                   | Register a new workflow definition      |
| `UpdateWorkflowDefs`   | `PUT /api/metadata/workflow`                    | Batch update workflow definitions       |
| `GetWorkflowDef`       | `GET /api/metadata/workflow/{name}?version=`    | Get a workflow definition by name       |
| `ListWorkflowDefs`     | `GET /api/metadata/workflow`                    | List all workflow definitions           |
| `DeleteWorkflowDef`    | `DELETE /api/metadata/workflow/{name}/{version}` | Delete a workflow definition           |
| `RegisterTaskDefs`     | `POST /api/metadata/taskdefs`                   | Register task definitions (batch)       |
| `GetTaskDef`           | `GET /api/metadata/taskdefs/{taskType}`         | Get a task definition                   |
| `ListTaskDefs`         | `GET /api/metadata/taskdefs`                    | List all task definitions               |
| `DeleteTaskDef`        | `DELETE /api/metadata/taskdefs/{taskType}`      | Delete a task definition                |

### 2. WorkflowService

Control workflow **execution lifecycle**.

| RPC                    | REST Equivalent                                    | Description                           |
|------------------------|----------------------------------------------------|---------------------------------------|
| `StartWorkflow`        | `POST /api/workflow`                               | Start a new workflow instance          |
| `GetWorkflow`          | `GET /api/workflow/{workflowId}`                   | Get full workflow state                |
| `TerminateWorkflow`    | `DELETE /api/workflow/{workflowId}?reason=`        | Terminate a running workflow           |
| `DeleteWorkflow`       | `DELETE /api/workflow/{workflowId}/remove`          | Permanently delete workflow            |
| `PauseWorkflow`        | `PUT /api/workflow/{workflowId}/pause`              | Pause a running workflow               |
| `ResumeWorkflow`       | `PUT /api/workflow/{workflowId}/resume`             | Resume a paused workflow               |
| `RestartWorkflow`      | `POST /api/workflow/{workflowId}/restart`           | Restart from the beginning             |
| `RetryWorkflow`        | `POST /api/workflow/{workflowId}/retry`             | Retry a failed workflow                |
| `RerunWorkflow`        | `POST /api/workflow/{workflowId}/rerun`             | Rerun from a specific task             |
| `DecideWorkflow`       | `PUT /api/workflow/{workflowId}/decide`             | Trigger decision evaluation            |
| `SkipTask`             | `PUT /api/workflow/{workflowId}/skiptask/{ref}`     | Skip a task in the workflow            |
| `GetWorkflowStats`     | `GET /api/workflow/stats`                           | Get workflow status counts             |
| `SearchWorkflows`      | `GET /api/workflow/search?...`                      | Search workflows with filters          |
| `GetRunningWorkflows`  | `GET /api/workflow/running/{name}`                  | List running workflow IDs              |

### 3. TaskService

Worker-facing operations: **poll, update, ack**.

| RPC                | REST Equivalent                                    | Description                            |
|--------------------|----------------------------------------------------|----------------------------------------|
| `UpdateTask`        | `POST /api/tasks`                                 | Update task status/output               |
| `GetTask`           | `GET /api/tasks/{taskId}`                          | Get full task details                   |
| `PollTask`          | `GET /api/tasks/poll/{taskType}?workerId=`         | Poll for a single task                  |
| `BatchPollTasks`    | `GET /api/tasks/poll/batch/{taskType}?count=&...`  | Poll for multiple tasks                 |
| `AckTask`           | `POST /api/tasks/{taskId}/ack?workerId=`           | Acknowledge a polled task               |
| `GetTaskLogs`       | `GET /api/tasks/{taskId}/log`                      | Get task execution logs                 |
| `AddTaskLog`        | `POST /api/tasks/{taskId}/log`                     | Add a task execution log entry          |
| `SearchTasks`       | `GET /api/tasks/search?...`                        | Search tasks with filters               |
| `GetQueueSizes`     | `GET /api/tasks/queue/sizes`                       | Get queue depth per task type           |

### 4. EventService

Manage **event handlers** for event-driven triggers.

| RPC                          | REST Equivalent                  | Description                     |
|------------------------------|----------------------------------|---------------------------------|
| `RegisterEventHandler`        | `POST /api/event`               | Register an event handler        |
| `UpdateEventHandler`          | `PUT /api/event`                | Update an event handler          |
| `ListEventHandlers`           | `GET /api/event`                | List all event handlers          |
| `GetEventHandlersForEvent`    | `GET /api/event/{event}`        | Get handlers for a specific event |
| `DeleteEventHandler`          | `DELETE /api/event/{name}`      | Delete an event handler          |

### 5. AdminService

Operational utilities: **config, sweep, queue control, health**.

| RPC              | REST Equivalent                          | Description                       |
|------------------|------------------------------------------|-----------------------------------|
| `GetConfig`       | `GET /api/admin/config`                 | Get system configuration           |
| `SweepWorkflow`   | `POST /api/admin/sweep/{workflowId}`    | Cleanup stale workflow/tasks       |
| `PauseQueue`      | `PUT /api/queue/pause/{queueName}`      | Pause a task queue                 |
| `ResumeQueue`     | `PUT /api/queue/resume/{queueName}`     | Resume a task queue                |
| `GetQueueStatus`  | `GET /api/queue/status/{queueName}`     | Check if a queue is paused         |
| `HealthCheck`     | `GET /health`                            | Health check all subsystems        |

### 6. BulkService

Batch operations on multiple workflows.

| RPC              | REST Equivalent                            | Description                        |
|------------------|--------------------------------------------|-------------------------------------|
| `BulkPause`       | `PUT /api/workflow/bulk/pause`            | Pause multiple workflows             |
| `BulkResume`      | `PUT /api/workflow/bulk/resume`           | Resume multiple workflows            |
| `BulkRetry`       | `POST /api/workflow/bulk/retry`           | Retry multiple failed workflows      |
| `BulkRestart`     | `POST /api/workflow/bulk/restart`         | Restart multiple workflows           |
| `BulkTerminate`   | `POST /api/workflow/bulk/terminate`       | Terminate multiple workflows         |

---

## Proto File

The complete service definition lives at:

```
backend/proto/conductor.proto
```

### Message Encoding Strategy

Complex nested objects (workflow definitions, task payloads, etc.) are carried as **JSON-encoded strings** inside proto messages. This approach:

1. **Preserves full fidelity** — `serde_json::Value` fields (arbitrary JSON) round-trip perfectly  
2. **Avoids schema explosion** — no need to replicate dozens of deeply nested structs in proto  
3. **Matches the REST API** — same JSON format clients already use  
4. **Zero-copy hot path** — structural fields (`workflow_id`, `status`, `total_hits`) remain native proto types  

Example:
```protobuf
message StartWorkflowRequest {
  string start_workflow_json = 1;  // JSON-encoded StartWorkflowRequest
}

message WorkflowStatsResponse {
  map<string, int64> stats = 1;    // Native proto map — no JSON needed
}
```

---

## Configuration

| Environment Variable | Default   | Description                          |
|---------------------|-----------|--------------------------------------|
| `GRPC_PORT`          | `50055`  | Port for the gRPC server             |
| `HOST`               | `localhost` | Bind address (shared with HTTP)   |

The gRPC server starts automatically alongside the HTTP server. No additional flags are needed.

---

## Client Generation

Generate clients in any language from the proto file:

### Rust (tonic)
```bash
# The proto is already compiled at build time via build.rs
# For external Rust clients:
cargo add tonic prost
# In build.rs:
tonic_build::compile_protos("proto/conductor.proto")?;
```

### Go
```bash
protoc --go_out=. --go-grpc_out=. proto/conductor.proto
```

### Python
```bash
pip install grpcio-tools
python -m grpc_tools.protoc -I proto --python_out=. --grpc_python_out=. proto/conductor.proto
```

### Java / Kotlin
```bash
protoc --java_out=. --grpc-java_out=. proto/conductor.proto
```

### Node.js / TypeScript
```bash
npm install @grpc/grpc-js @grpc/proto-loader
# or use ts-proto / nice-grpc for typed generation
npx grpc_tools_node_protoc --ts_out=. --grpc_out=. proto/conductor.proto
```

---

## Usage Examples

### grpcurl (CLI)

```bash
# List services
grpcurl -plaintext localhost:50055 list

# Health check
grpcurl -plaintext localhost:50055 conductor.AdminService/HealthCheck

# Register a workflow definition
grpcurl -plaintext -d '{
  "workflow_def_json": "{\"name\":\"my_workflow\",\"version\":1,\"tasks\":[{\"name\":\"task1\",\"taskReferenceName\":\"t1\",\"type\":\"SIMPLE\"}]}"
}' localhost:50055 conductor.MetadataService/RegisterWorkflowDef

# Start a workflow
grpcurl -plaintext -d '{
  "start_workflow_json": "{\"name\":\"my_workflow\",\"version\":1,\"input\":{\"key\":\"value\"}}"
}' localhost:50055 conductor.WorkflowService/StartWorkflow

# Poll for a task
grpcurl -plaintext -d '{
  "task_type": "SIMPLE",
  "worker_id": "worker-1"
}' localhost:50055 conductor.TaskService/PollTask

# Get workflow stats
grpcurl -plaintext localhost:50055 conductor.WorkflowService/GetWorkflowStats

# Bulk terminate
grpcurl -plaintext -d '{
  "workflow_ids": ["id-1", "id-2"],
  "reason": "cleanup"
}' localhost:50055 conductor.BulkService/BulkTerminate
```

### Rust Client Example

```rust
use tonic::transport::Channel;

// Include the generated proto code
pub mod conductor {
    tonic::include_proto!("conductor");
}

use conductor::workflow_service_client::WorkflowServiceClient;
use conductor::task_service_client::TaskServiceClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://localhost:50055")
        .connect()
        .await?;

    // Start a workflow
    let mut wf_client = WorkflowServiceClient::new(channel.clone());
    let resp = wf_client.start_workflow(conductor::StartWorkflowRequest {
        start_workflow_json: serde_json::json!({
            "name": "my_workflow",
            "version": 1,
            "input": {"key": "value"}
        }).to_string(),
    }).await?;
    println!("Started workflow: {}", resp.into_inner().workflow_id);

    // Poll for tasks
    let mut task_client = TaskServiceClient::new(channel);
    let poll_resp = task_client.poll_task(conductor::PollTaskRequest {
        task_type: "SIMPLE".into(),
        worker_id: "rust-worker-1".into(),
    }).await?;

    let inner = poll_resp.into_inner();
    if inner.found {
        println!("Got task: {}", inner.poll_task_json);
    }

    Ok(())
}
```

### Python Client Example

```python
import grpc
import json
import conductor_pb2
import conductor_pb2_grpc

channel = grpc.insecure_channel("localhost:50055")

# Start a workflow
wf_stub = conductor_pb2_grpc.WorkflowServiceStub(channel)
resp = wf_stub.StartWorkflow(conductor_pb2.StartWorkflowRequest(
    start_workflow_json=json.dumps({
        "name": "my_workflow",
        "version": 1,
        "input": {"key": "value"}
    })
))
print(f"Started workflow: {resp.workflow_id}")

# Poll for a task
task_stub = conductor_pb2_grpc.TaskServiceStub(channel)
poll_resp = task_stub.PollTask(conductor_pb2.PollTaskRequest(
    task_type="SIMPLE",
    worker_id="python-worker-1"
))
if poll_resp.found:
    task = json.loads(poll_resp.poll_task_json)
    print(f"Got task: {task['taskId']}")
```

---

## Docker Compose

The gRPC port is automatically exposed through nginx as an HTTP/2 load balancer:

```yaml
# docker-compose.yml (relevant sections)
backend:
  expose:
    - "8090"    # REST
    - "50055"   # gRPC

nginx-lb:
  ports:
    - "8090:8090"    # REST (external)
    - "50055:50055"  # gRPC (external)
```

The nginx gRPC block uses `grpc_pass` for native HTTP/2 proxying, providing load balancing across all 4 backend replicas.

---

## Error Mapping

Engine errors are mapped to standard gRPC status codes:

| Engine Error     | gRPC Status            | HTTP Equivalent |
|-----------------|------------------------|-----------------|
| `NotFound`       | `NOT_FOUND` (5)       | 404             |
| `InvalidState`   | `FAILED_PRECONDITION` (9) | 409          |
| `Serde`          | `INVALID_ARGUMENT` (3) | 400            |
| `Database`       | `INTERNAL` (13)        | 500            |
| `Redis`          | `INTERNAL` (13)        | 500            |

---

## Performance Considerations

- **Binary encoding**: Protobuf messages are 3-10× smaller than equivalent JSON  
- **HTTP/2 multiplexing**: many concurrent RPCs over a single TCP connection  
- **Zero-copy engine**: gRPC handlers call the same `Arc<WorkflowEngine>` — no serialization between API layers  
- **Connection pooling**: tonic uses hyper's connection pool; nginx load-balances with keepalive  
- **Batch operations**: `BatchPollTasks`, `BulkPause`, etc. reduce round-trips  

### Benchmarking

```bash
# Install ghz (gRPC benchmarking tool)
# https://ghz.sh

# Benchmark task polling (high-frequency worker operation)
ghz --insecure \
    --proto proto/conductor.proto \
    --call conductor.TaskService.PollTask \
    -d '{"task_type":"SIMPLE","worker_id":"bench-1"}' \
    -c 100 -n 10000 \
    localhost:50055
```

---

## Build Requirements

| Tool     | Version | Purpose                    |
|----------|---------|----------------------------|
| `protoc` | ≥ 3.15  | Protobuf compiler          |
| `rustc`  | ≥ 1.85  | Rust compiler (edition 2024)           |

The `build.rs` script automatically invokes `protoc` via `tonic-build`. Set the `PROTOC` environment variable if protoc is not on your PATH:

```bash
# Linux/macOS
export PROTOC=/path/to/protoc

# Windows
set PROTOC=C:\path\to\protoc.exe
```

For Docker builds, protoc is installed automatically in the builder stage (`protobuf-compiler` apt package).

---

## File Structure

```
backend/
├── build.rs                    # tonic-build proto compilation
├── proto/
│   └── conductor.proto         # Service + message definitions
└── src/
    └── grpc/
        ├── mod.rs              # Router assembly, proto import
        ├── metadata.rs         # MetadataService implementation
        ├── workflow.rs         # WorkflowService implementation
        ├── tasks.rs            # TaskService implementation
        ├── events.rs           # EventService implementation
        ├── admin.rs            # AdminService implementation
        └── bulk.rs             # BulkService implementation
```

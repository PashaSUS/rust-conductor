# API Reference

rust-conductor provides multiple API interfaces: REST (V1 & V2), GraphQL, gRPC, SSE, and WebSocket.

## REST API V1

The core Conductor-compatible REST API.

### Workflow Definitions
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/metadata/workflow` | List all workflow definitions |
| `GET` | `/api/metadata/workflow/{name}` | Get latest version of a workflow definition |
| `GET` | `/api/metadata/workflow/{name}/{version}` | Get specific version |
| `POST` | `/api/metadata/workflow` | Create/register a workflow definition |
| `PUT` | `/api/metadata/workflow` | Update a workflow definition |
| `DELETE` | `/api/metadata/workflow/{name}/{version}` | Delete a workflow definition version |
| `POST` | `/api/metadata/workflow/validate` | Validate a workflow definition graph (109) |

### Task Definitions
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/metadata/taskdefs` | List all task definitions |
| `GET` | `/api/metadata/taskdefs/{name}` | Get a task definition |
| `POST` | `/api/metadata/taskdefs` | Register task definitions |
| `DELETE` | `/api/metadata/taskdefs/{name}` | Delete a task definition |

### Workflow Execution
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/workflow` | Start a workflow |
| `GET` | `/api/workflow/{workflowId}` | Get workflow execution status |
| `GET` | `/api/workflow/search` | Search workflow executions |
| `POST` | `/api/workflow/{workflowId}/pause` | Pause a running workflow |
| `POST` | `/api/workflow/{workflowId}/resume` | Resume a paused workflow |
| `POST` | `/api/workflow/{workflowId}/restart` | Restart a failed workflow |
| `POST` | `/api/workflow/{workflowId}/retry` | Retry a failed workflow |
| `DELETE` | `/api/workflow/{workflowId}` | Terminate a workflow |
| `POST` | `/api/workflow/{workflowId}/modify` | Modify a running workflow (102) |
| `POST` | `/api/workflow/signal` | Send a signal to waiting workflows (103) |
| `POST` | `/api/workflow/{workflowId}/checkpoint` | Create a workflow checkpoint/snapshot (105) |
| `GET` | `/api/workflow/{workflowId}/checkpoints` | List checkpoints for a workflow (105) |
| `POST` | `/api/workflow/{workflowId}/restore/{checkpointId}` | Restore workflow from checkpoint (105) |

#### Advanced Endpoint Details

**POST `/api/workflow/{workflowId}/modify`** — Adds, removes, or replaces tasks in a running workflow. Body: `ModifyWorkflowRequest { workflow_id, add_tasks[], remove_task_refs[], replace_tasks {} }`. Only non-terminal workflows can be modified.

**POST `/api/workflow/signal`** — Delivers a named signal to all workflows with a `WAIT_FOR_SIGNAL` task listening for that signal name. Body: `SendSignalRequest { signal_name, payload }`. Returns `{ signal_name, delivered_count }`.

**POST `/api/workflow/{workflowId}/checkpoint`** — Snapshots the current workflow state (tasks, variables). Body: `{ label? }`. Returns the created `WorkflowCheckpoint`.

**GET `/api/workflow/{workflowId}/checkpoints`** — Lists all checkpoints for a workflow, ordered by creation time descending.

**POST `/api/workflow/{workflowId}/restore/{checkpointId}`** — Resets the workflow to the state captured in the given checkpoint. The workflow is paused after restoration.

**POST `/api/metadata/workflow/validate`** — Validates a workflow definition without registering it. Body: `WorkflowDef`. Returns `ValidationResult { valid, errors[], warnings[] }`. Checks for missing task definitions, unreachable tasks, cycles, and empty graphs.

**POST `/api/tasks/{taskId}/heartbeat`** — Extends the heartbeat timeout for a long-running task. Workers should call this periodically (at less than `heartbeatTimeoutSeconds` intervals) to prevent the sweeper from timing out the task.

### Tasks
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/tasks/poll/{taskType}` | Poll for a task |
| `GET` | `/api/tasks/poll/batch/{taskType}` | Batch poll for tasks |
| `POST` | `/api/tasks` | Update task status |
| `POST` | `/api/tasks/{taskId}/ack` | Acknowledge a task |
| `GET` | `/api/tasks/queue/sizes` | Get task queue sizes |
| `GET` | `/api/tasks/{taskId}` | Get task details |
| `GET` | `/api/tasks/{taskId}/log` | Get task execution logs |
| `POST` | `/api/tasks/{taskId}/heartbeat` | Send task heartbeat (110) |

### Events
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/event` | Register an event handler |
| `GET` | `/api/event` | List event handlers |
| `DELETE` | `/api/event/{name}` | Delete an event handler |

### Admin
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/admin/config` | Get system configuration |
| `POST` | `/api/admin/sweep` | Trigger a manual sweeper run |

### Schedules
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/schedules` | Create a schedule |
| `GET` | `/api/schedules` | List schedules |
| `GET` | `/api/schedules/{name}` | Get a schedule |
| `DELETE` | `/api/schedules/{name}` | Delete a schedule |
| `POST` | `/api/schedules/{name}/pause` | Pause a schedule |
| `POST` | `/api/schedules/{name}/resume` | Resume a schedule |

## REST API V2 (feature: `api-v2`)

Enhanced API with envelope format, richer error details, and batch operations.

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/v2/workflow` | Start workflow (V2 envelope) |
| `GET` | `/v2/workflow/{workflowId}` | Get workflow (V2 envelope) |
| `GET` | `/v2/workflow/search` | Search workflows (V2 envelope) |
| `GET` | `/v2/metadata/workflow` | List definitions (V2 envelope) |
| `GET` | `/v2/metadata/workflow/{name}` | Get definition (V2 envelope) |
| `GET` | `/v2/metadata/taskdefs` | List task definitions (V2 envelope) |
| `POST` | `/v2/batch` | Execute multiple API calls in one request |
| `GET` | `/v2/workflow/{workflowId}/poll` | Long-poll for workflow completion |

## GraphQL (feature: `graphql`)

Available at `/api/graphql` (GET for playground, POST for queries).

### Queries
- `workflowDefs` — List all workflow definitions
- `workflowDef(name, version)` — Get a specific definition
- `taskDefs` — List all task definitions
- `workflow(id)` — Get a workflow execution
- `searchWorkflows(query, start, size)` — Search executions
- `queueSizes` — Get task queue sizes

### Mutations
- `startWorkflow(name, version, input, correlationId)` — Start a workflow
- `pauseWorkflow(id)` — Pause a workflow
- `resumeWorkflow(id)` — Resume a workflow
- `terminateWorkflow(id)` — Terminate a workflow

## SSE — Server-Sent Events (feature: `sse`)

| Path | Description |
|------|-------------|
| `/api/sse/workflow/{workflowId}` | Stream workflow status updates |
| `/api/sse/queue/sizes` | Stream task queue size changes |

Query parameter: `?intervalMs=2000` (polling interval, min 500ms)

## WebSocket (feature: `websocket`)

| Path | Description |
|------|-------------|
| `/api/ws/workflow/{workflowId}` | Bidirectional workflow status |
| `/api/ws/tasks/{taskType}` | Real-time task feed for workers |

### Client Messages
```json
{ "type": "subscribe", "workflowId": "..." }
{ "type": "unsubscribe", "workflowId": "..." }
{ "type": "getStatus" }
```

## gRPC

Available on port 50055 (configurable via `GRPC_PORT`).

Services registered under both `conductor.*` and `conductor.grpc.*` packages for compatibility with the official Conductor SDK.

When the `grpc-reflection` feature is enabled, clients can use gRPC reflection to discover available services dynamically.

See [grpc.md](grpc.md) for detailed proto definitions.

## Health Check

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Returns health status JSON |

## Swagger UI

Interactive API documentation available at `/swagger-ui/`.

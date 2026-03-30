# Workflow Engine — Core Design

The `WorkflowEngine` is the central orchestration component of rust-conductor. All workflow and task logic passes through it — the REST API and gRPC server are thin transport adapters that delegate to the engine.

## Why a Single Shared Engine?

```
REST API (Actix-web)  ──┐
                        ├──► Arc<WorkflowEngine> ──► PostgreSQL / Redis / Kafka / S3
gRPC Server (Tonic)   ──┘
```

| Alternative | Why Not |
|-------------|---------|
| Separate engine per protocol | Duplicated logic, divergent behavior between REST and gRPC |
| Microservices (scheduler, worker, API) | Added network hops, deployment complexity, harder debugging |
| Event-sourced architecture | Complexity of event replay, eventual consistency headaches for task scheduling |

**Design choice:** A single `Arc<WorkflowEngine>` shared across threads. No logic duplication, no inter-process communication, zero serialization overhead between API and engine. The engine is plain Rust — no trait objects, no dynamic dispatch on the hot path.

---

## Engine Structure

```rust
pub struct WorkflowEngine {
    shards:           ShardedPool,                      // PostgreSQL shard routing
    redis:            ShardedRedis,                     // Task routing + locks
    queue:            KafkaTaskQueue | RedisTaskQueue,  // Task delivery
    external_storage: Option<ExternalPayloadStorage>,   // S3 (optional)
}
```

The engine holds only connections to external services — no in-memory state, no caches, no local queues. This means:
- **Stateless horizontally**: any replica can handle any request
- **Crash-safe**: restarting loses nothing (all state is in Postgres/Redis)
- **Simple mental model**: engine = function(request) → side effects on external services

---

## Workflow Lifecycle

```
START → RUNNING → [ tasks execute ] → COMPLETED
                                     → FAILED
         ↕ PAUSED
         → TERMINATED (manual kill)
         → TIMED_OUT (deadline exceeded)
```

### Start Workflow

1. Validate definition exists (name + version)
2. Create workflow record in PostgreSQL (routed shard)
3. Optionally offload input to S3 if above threshold
4. Trigger initial advancement → schedules first tasks

### Advancement (Core Loop)

When a task completes (or workflow starts), the engine advances:

1. **Acquire lock** — `SET NX EX 30s` on `conductor:advance:{workflow_id}` in Redis
2. **Evaluate next tasks** — walk definition tree, check conditions, resolve fork/join
3. **Schedule ready tasks** — create task records, set routing in Redis, enqueue to Kafka/Redis
4. **Check completion** — if all tasks terminal, mark workflow done, fire webhooks

**Why pull-based?** The engine reacts to events (task completion, timers) rather than proactively pushing. Simpler, no thundering-herd, no polling overhead.

**Why per-workflow locks?** Different workflows advance independently. Only the same workflow needs mutual exclusion (to prevent duplicate task scheduling). This maximizes parallelism.

### Pause / Resume

- **Pause**: sets workflow status to PAUSED, halts advancement
- **Resume**: sets status back to RUNNING, triggers advancement

### Terminate

Sets status to TERMINATED. In-progress tasks may still complete, but their results are discarded.

### Restart

Creates a new execution of the same workflow definition with the same input. The old execution is terminated.

### Retry

For FAILED workflows: resets the failed task to SCHEDULED and re-triggers advancement. Already-completed tasks are not re-executed.

---

## Task Types

### Worker Tasks (Enqueued)

These require external workers to poll, execute, and report back:

| Type | Purpose | Why a Separate Type? |
|------|---------|---------------------|
| `SIMPLE` | Generic worker task | Default catch-all for custom business logic |
| `HTTP` | HTTP request task | Signals that the worker should make an HTTP call (can use a generic HTTP worker) |

**Why enqueue instead of execute inline?** Worker tasks run arbitrary external code (API calls, database operations, ML inference). Running them inside the engine would block the orchestration loop and create a single point of failure.

### System Tasks (Inline)

These are evaluated during advancement — no queue round-trip:

| Type | Purpose | Why Inline? |
|------|---------|-------------|
| `FORK` | Spawn parallel branches | Just creates multiple task records — no external work |
| `JOIN` | Wait for fork branches | Condition check — no external work |
| `DECISION` | Evaluate expression, pick branch | Expression evaluation — microseconds |
| `SUB_WORKFLOW` | Start a child workflow | Calls `start_workflow` internally |
| `LAMBDA` | Execute inline expression | Small computation, no external dependency |
| `SET_VARIABLE` | Update workflow variables | Metadata update only |
| `TERMINATE` | End workflow with status | State transition only |
| `WAIT` | Wait for duration or event | Sets a timer, no external work |
| `EVENT` | Wait for external event | Registers a listener, no external work |
| `MAP` | Fan-out across array items | Creates N sub-tasks + auto-JOIN — configurable parallelism |
| `WAIT_FOR_SIGNAL` | Wait for inter-workflow signal | Creates IN_PROGRESS task, completed by `send_signal()` |

**Why this split?** System tasks take microseconds. Enqueuing them to Kafka and back would add milliseconds of latency per task for no benefit.

---

## Template Resolution

Workflow task definitions can reference dynamic data:

```
${workflow.input.userId}           → value from workflow input
${fetch_user.output.email}         → output from a completed task named "fetch_user"
```

**When it happens:** At task scheduling time (during advancement), not at definition time.

**Why resolve in the engine?** Workers receive fully resolved inputs. This keeps workers stateless — they don't need access to the workflow context, other task outputs, or the engine. A worker is just: receive input → do work → return output.

---

## Sweeper (Background Recovery)

A periodic background task that ensures the system self-heals:

| Check | Condition | Action | Why? |
|-------|-----------|--------|------|
| Orphaned tasks | SCHEDULED > 30s ago | Re-enqueue to task queue | Worker may have crashed before acknowledging |
| Task timeouts | IN_PROGRESS > 10 min | Mark TIMED_OUT, retry or fail | Worker may have crashed mid-execution |
| Sub-workflow stuck | Parent waiting, child finished | Advance parent | Completion event may have been lost |
| Stuck workflows | RUNNING, zero pending tasks | Re-trigger advance | Edge case: all tasks done but workflow not completed |

**Why a sweeper?** In distributed systems, messages can be lost, workers can crash, and race conditions can leave state inconsistent. The sweeper provides **eventual consistency** — any workflow that gets stuck will be recovered within the sweep interval.

**Why not rely only on retries?** Retries handle expected failures (worker returns error). The sweeper handles unexpected failures (worker disappears, network partition, engine crash during advancement).

---

## Scheduler (CRON)

Runs inside the sweeper loop:

1. Query `scheduled_workflow` table for schedules where `next_run_at <= now()`
2. For each due schedule: call `start_workflow` with the configured input
3. Compute and store the next `next_run_at` from the cron expression

**Why not a separate scheduler service?** One less process to deploy, monitor, and coordinate. The sweep loop already runs periodically — CRON evaluation adds negligible overhead.

---

## Webhook Notifications

Workflow definitions can specify:
- `on_complete_webhook` — URL to POST when workflow completes
- `on_failure_webhook` — URL to POST when workflow fails

**Design: Fire-and-forget.** The engine sends the HTTP POST and logs the result, but does not retry failures or block on the response.

**Why fire-and-forget?** Webhook endpoints are external systems the engine doesn't control. Retrying indefinitely would leak goroutines/tasks. If reliable delivery is needed, the webhook receiver should be idempotent and the caller should use an event system instead.

---

## Idempotency

### Workflow Start

`POST /api/workflow` supports `X-Idempotency-Key`. If the same key is sent twice, the second call returns the existing workflow ID instead of creating a duplicate.

**Why?** Network failures between client and server can cause retries. Without idempotency, each retry creates a new workflow execution.

### Task Deduplication

A partial unique index prevents scheduling the same task reference twice for a workflow:

```sql
CREATE UNIQUE INDEX idx_task_unique_ref 
ON task (workflow_id, reference_task_name) 
WHERE status NOT IN ('COMPLETED', 'FAILED', ...);
```

**Why?** Concurrent advancement attempts (before the lock takes effect) could create duplicate tasks. The unique index is the last line of defense.

---

## Error Handling

| Error Type | HTTP Status | gRPC Status | Meaning |
|-----------|-------------|-------------|---------|
| `NotFound` | 404 | `NOT_FOUND` | Workflow/task/definition doesn't exist |
| `Conflict` | 409 | `FAILED_PRECONDITION` | Invalid state transition or duplicate key |
| `InvalidInput` | 400 | `INVALID_ARGUMENT` | Malformed request |
| `DatabaseError` | 500 | `INTERNAL` | PostgreSQL failure |
| `QueueError` | 500 | `INTERNAL` | Kafka/Redis Streams failure |
| `StorageError` | 500 | `INTERNAL` | S3 failure |

---

## Code Reference

| File | Role |
|------|------|
| `engine/mod.rs` | `WorkflowEngine` struct and constructor |
| `engine/advance.rs` | Workflow advancement state machine |
| `engine/advanced.rs` | Advanced features: signals, saga, checkpoints, inheritance, validation, heartbeat |
| `engine/execution.rs` | Workflow start, get, search |
| `engine/workflow_ops.rs` | Pause, resume, terminate, restart, retry |
| `engine/task_ops.rs` | Task poll, update, routing |
| `engine/system_tasks.rs` | FORK, JOIN, DECISION, SUB_WORKFLOW, MAP, WAIT_FOR_SIGNAL, etc. |
| `engine/sweeper.rs` | Background recovery + CRON scheduler + heartbeat timeout |
| `engine/scheduler.rs` | CRON expression evaluation |
| `engine/events.rs` | Event handler registration and triggering |
| `engine/metadata.rs` | Definition CRUD |
| `engine/admin.rs` | Health, config, queue management |
| `engine/error.rs` | Typed error variants |
| `engine/rows.rs` | SQL row mapping helpers |
| `engine/shard.rs` | Shard routing (see [sharding.md](sharding.md)) |

---

## Advanced Workflow Engine Features

### Dynamic Workflow Modification (102)

Running workflows can be modified via `POST /api/workflow/{id}/modify`. Only tasks that have not yet been scheduled can be added or removed. The modified definition is validated, and the workflow is re-advanced to pick up new tasks.

### Workflow Inter-Communication — Signals (103)

The `WAIT_FOR_SIGNAL` task type creates an IN_PROGRESS task that blocks until a matching signal is received. Signals are sent via `POST /api/workflow/signal` with a `signalName` and `payload`. All waiting workflows across all shards with matching signal names are completed.

**Frontend path:** `/signals`

### Saga Pattern (104)

Workflow definitions with `sagaEnabled: true` automatically run compensation tasks in reverse order when the workflow fails. Each task can define a `compensationTask` that runs with the original task's output as input.

### Workflow Checkpointing (105)

Create snapshots of workflow state via `POST /api/workflow/{id}/checkpoint`. List all checkpoints via `GET /api/workflow/{id}/checkpoints`. Restore from a checkpoint (terminates current, starts new) via `POST /api/workflow/{id}/restore/{checkpointId}`.

**Frontend:** Checkpoints tab on workflow detail page (`/executions/:id`).

### Conditional Branching Combinators (106)

DECISION tasks can use a `conditionTree` field with nested AND/OR/NOT/Compare nodes instead of simple `caseValueParam`. The `Compare` node supports operators: Eq, Neq, Gt, Gte, Lt, Lte, Contains, StartsWith, EndsWith.

### MAP Task Type (107)

The `MAP` task fans out across an array of items. Specify `mapItemsParam` (name of array field in input), `mapTask` (template task), and optional `mapParallelism`. Creates N sub-tasks with `mapItem` and `mapIndex` injected into input, plus an auto-JOIN.

### Workflow Inheritance (108)

Workflow definitions can specify `baseWorkflow` and `baseWorkflowVersion` to inherit tasks from a parent definition. Inheritance is resolved at `start_workflow` time. Child tasks override parent tasks with the same reference name. Maximum chain depth: 10. Circular inheritance is detected and rejected.

### Task Dependency Graph Validation (109)

Validate workflow definitions via `POST /api/metadata/workflow/validate`. Checks for: empty names, duplicate references, invalid JOIN references, FORK structure, DECISION cases, SUB_WORKFLOW params, MAP params, cycle detection (DFS), and unreachable tasks.

**Frontend path:** `/validate`

### Long-Running Task Heartbeat (110)

Workers can report liveness via `POST /api/tasks/{taskId}/heartbeat`. Tasks with `heartbeatTimeoutSeconds` set in their definition are monitored by the sweeper — if `update_time` exceeds the timeout, the task is marked TIMED_OUT. Heartbeats are also stored in Redis with a 10-minute TTL for fast lookup.

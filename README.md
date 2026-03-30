# rust-conductor

A fully Rust-based Conductor-compliant microservice orchestrator with sharded PostgreSQL, Kafka task queues, Redis routing, and dual REST + gRPC APIs.

## Quick Start (no clone needed)

Run the full stack with a single command — just needs Docker:

```bash
curl -fsSL https://raw.githubusercontent.com/PashaSUS/rust-conductor/main/docker-compose.ghcr.yml -o docker-compose.yml
docker compose up -d
```

| Service       | URL                                    |
|---------------|----------------------------------------|
| **Frontend**  | http://localhost:3170                   |
| **REST API**  | http://localhost:8090                   |
| **Swagger UI**| http://localhost:8090/swagger-ui/       |
| **gRPC**      | `localhost:50055`                       |
| **Seq Logs**  | http://localhost:9321                   |

To stop: `docker compose down` (add `-v` to also remove data volumes).

### Run from source

```bash
git clone https://github.com/PashaSUS/rust-conductor.git
cd rust-conductor
docker compose up -d --build
```

The main `docker-compose.yml` pulls pre-built images from GHCR by default.
Pass `--build` to build from source instead.

## API Interfaces

| Protocol | Port    | Transport         | Documentation                    |
|----------|---------|-------------------|----------------------------------|
| REST     | `8090`  | HTTP/1.1 + JSON   | Swagger UI at `/swagger-ui/`     |
| gRPC     | `50055` | HTTP/2 + Protobuf  | [docs/grpc.md](docs/grpc.md)    |

Both APIs are backed by the same `WorkflowEngine` — choose the protocol that fits your client.

### Quick Start (gRPC)

```bash
# Health check
grpcurl -plaintext localhost:50055 conductor.AdminService/HealthCheck

# Start a workflow
grpcurl -plaintext -d '{
  "start_workflow_json": "{\"name\":\"my_workflow\",\"version\":1,\"input\":{}}"
}' localhost:50055 conductor.WorkflowService/StartWorkflow

# Poll for tasks
grpcurl -plaintext -d '{"task_type":"SIMPLE","worker_id":"w1"}' \
  localhost:50055 conductor.TaskService/PollTask
```

See [docs/grpc.md](docs/grpc.md) for full gRPC documentation, client generation guides, and examples.

## Architecture

| Component       | Technology             | Role                                       |
|-----------------|------------------------|---------------------------------------------|
| **PostgreSQL**  | 4 shards + PgBouncer   | Workflow & task state storage               |
| **Kafka**       | KRaft single broker    | Durable task queues (at-least-once delivery)|
| **Redis**       | 4 shards               | Task routing (O(1) lookups) & queue pauses  |
| **Seq**         | Structured log server  | Centralized structured logging              |

See [docs/kafka.md](docs/kafka.md) for Kafka integration details.

## Features

### Backend
- **Dual API** — REST (Actix-web) and gRPC (Tonic) sharing a single `WorkflowEngine`
- **Sharded PostgreSQL** — Deterministic UUID-based routing across configurable shards
- **Durable Task Queues** — Kafka (production) with Redis Streams fallback (development)
- **Redis Routing** — O(1) task-to-shard lookups, distributed advance locks, queue pause flags
- **GraphQL** — Full query/mutation API alongside REST (feature-gated)
- **Server-Sent Events** — Real-time workflow status and queue size streaming
- **WebSocket** — Bidirectional task/workflow feeds for workers and clients
- **V2 API** — Enhanced REST with envelope format, long-polling, ETag caching, batch operations
- **CRON Scheduler** — Cron-based workflow scheduling with timezone support
- **Rate Limiting** — Token bucket per-client rate limiter backed by Redis
- **Background Sweeper** — Adaptive recovery for orphaned tasks, stuck workflows, SLA breaches
- **External Storage** — S3/MinIO offloading for large payloads (feature-gated)
- **Webhooks** — Fire-and-forget notifications on workflow completion/failure

### Frontend
- **Dashboard** — Customizable widgets, health status, sparkline charts
- **Execution Management** — Search/filter, bulk operations, pause/resume/terminate/retry
- **Workflow Detail** — 9 tabs: tasks, timeline, flame chart, dependency graph, diagram, replay, I/O, data explorer
- **Visual Designer** — Drag-and-drop workflow builder with React Flow
- **Definition Versioning** — Version history, clone, edit-as-next-version, visual diff
- **Task Queue Monitoring** — Live queue sizes with configurable alert thresholds
- **Execution Comparison** — Side-by-side execution diff with timeline overlay
- **Stress Tester** — Load testing tool with random data generation
- **Template Marketplace** — Built-in workflow patterns for common use cases
- **Theme System** — Multiple themes with localized text

### Infrastructure
- **Helm Charts** — Kubernetes deployment with HPA autoscaling
- **Terraform** — Cloud infrastructure provisioning templates
- **Docker Compose** — Single-command setup for dev and production
- **Setup Scripts** — Interactive configuration for dev/prod profiles

## Documentation

| Document | Description |
|----------|-------------|
| [Architecture](docs/architecture.md) | System overview, design patterns, feature flags |
| [Engine](docs/engine.md) | Workflow lifecycle, task types, sweeper, scheduler |
| [Configuration](docs/configuration.md) | Complete environment variable reference |
| [Feature Flags](docs/feature-flags.md) | Build profiles, compile-time and runtime flags |
| [PostgreSQL](docs/postgresql.md) | Schema, indexes, migrations, PgBouncer |
| [Redis](docs/redis.md) | Task routing, locks, pause flags, Redis Streams |
| [Kafka](docs/kafka.md) | Durable task queues, producer/consumer config |
| [gRPC](docs/grpc.md) | gRPC API, client generation, Nginx proxying |
| [S3 Storage](docs/s3-storage.md) | External payload storage, threshold offloading |
| [Sharding](docs/sharding.md) | UUID-based routing, fan-out, metadata routing |
| [Rate Limiting](docs/rate-limiting.md) | Token bucket, CORS, payload limits |
| [Frontend](docs/frontend.md) | React dashboard, tech stack, theme system |
| [Testing](docs/testing.md) | Unit, integration, benchmarks, fuzz, mutation testing |
| [API Reference](docs/api-reference.md) | REST, GraphQL, gRPC, SSE, WebSocket endpoints |

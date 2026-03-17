# rust-conductor

A fully Rust-based Conductor-compliant microservice orchestrator with sharded PostgreSQL, Kafka task queues, Redis routing, and dual REST + gRPC APIs.

## API Interfaces

| Protocol | Port    | Transport         | Documentation                    |
|----------|---------|-------------------|----------------------------------|
| REST     | `8080`  | HTTP/1.1 + JSON   | Swagger UI at `/swagger-ui/`     |
| gRPC     | `50051` | HTTP/2 + Protobuf  | [docs/grpc.md](docs/grpc.md)    |

Both APIs are backed by the same `WorkflowEngine` — choose the protocol that fits your client.

### Quick Start (gRPC)

```bash
# Health check
grpcurl -plaintext localhost:50051 conductor.AdminService/HealthCheck

# Start a workflow
grpcurl -plaintext -d '{
  "start_workflow_json": "{\"name\":\"my_workflow\",\"version\":1,\"input\":{}}"
}' localhost:50051 conductor.WorkflowService/StartWorkflow

# Poll for tasks
grpcurl -plaintext -d '{"task_type":"SIMPLE","worker_id":"w1"}' \
  localhost:50051 conductor.TaskService/PollTask
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

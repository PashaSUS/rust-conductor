# Kafka Integration

rust-conductor uses **Apache Kafka** as the durable task queue backing all worker task delivery. Redis remains in the stack for O(1) task-routing lookups and queue pause/resume flags.

## Why Kafka for Task Queues?

The previous architecture used Redis lists (LPUSH / RPOP) as task queues. While fast, this had limitations:

| Concern              | Redis Lists             | Kafka                                    |
|----------------------|-------------------------|------------------------------------------|
| Durability           | In-memory, data at risk on crash | Replicated, persisted to disk     |
| Delivery guarantee   | At-most-once (RPOP is destructive) | At-least-once (offset-based)    |
| Consumer groups      | Manual round-robin      | Native competing-consumer pattern         |
| Replay / audit       | Not possible            | Offset reset replays full history         |
| Backpressure         | None                    | Built-in via consumer lag                 |
| Monitoring           | LLEN per queue          | Consumer group lag = natural queue depth  |

Redis is **kept** for two things it excels at:

- **Task routing** — `HSET/HGET/HDEL` on `conductor:task_routing` maps `task_id → workflow_instance_id` for O(1) shard resolution.
- **Queue pause flags** — `SET/DEL/EXISTS` on `conductor:queue:paused:{name}` for instant pause/resume.

## Architecture

```
┌────────────┐       ┌───────────────┐       ┌──────────────────┐
│  API Layer │──────>│ WorkflowEngine│──────>│  Kafka Broker     │
│ REST/gRPC  │       │               │       │  (task queues)    │
└────────────┘       │               │──────>│  Redis            │
                     │               │       │  (routing + flags)│
                     │               │──────>│  PostgreSQL       │
                     └───────────────┘       │  (state storage)  │
                                             └──────────────────┘
```

### Topic Naming

Each task type maps to a dedicated Kafka topic:

```
conductor.task.{task_type}
```

Examples:
- `conductor.task.SIMPLE` — default simple worker tasks
- `conductor.task.HTTP` — HTTP system tasks queued for workers
- `conductor.task.my_custom_task` — user-defined task types

Topics are auto-created on first produce (Kafka `auto.create.topics.enable = true`).

### Consumer Group

All conductor backend instances share a single consumer group:

```
group.id = conductor-workers
```

Kafka distributes partitions across group members automatically. With 4 partitions per topic and 4 backend replicas, each replica handles ~1 partition per task type.

### Message Format

- **Key**: `task_id` (UUID string) — ensures tasks with the same ID always land on the same partition
- **Payload**: `task_id` (UUID string) — the engine fetches full task data from PostgreSQL by ID

## Configuration

| Environment Variable | Default           | Description                    |
|---------------------|-------------------|--------------------------------|
| `KAFKA_BROKERS`     | `localhost:9092`  | Comma-separated broker list    |

### Docker Compose

The included `docker-compose.yml` runs a single Kafka broker in **KRaft mode** (no Zookeeper):

```yaml
kafka:
  image: apache/kafka:3.8.0
  environment:
    KAFKA_NODE_ID: "1"
    KAFKA_PROCESS_ROLES: "broker,controller"
    KAFKA_NUM_PARTITIONS: "4"
    KAFKA_AUTO_CREATE_TOPICS_ENABLE: "true"
```

### Production Recommendations

- Run **3+ Kafka brokers** with `replication.factor = 3` for fault tolerance.
- Set `KAFKA_NUM_PARTITIONS` to at least the number of backend replicas.
- Monitor consumer group lag via `kafka-consumer-groups.sh --describe` or a dashboard (Kafka UI, Grafana + Burrow).
- Tune `log.retention.hours` based on your audit/replay requirements (default: 168h = 7 days).

## Producer Configuration

The `KafkaTaskQueue` producer is tuned for low-latency reliable delivery:

| Setting                     | Value     | Rationale                                 |
|-----------------------------|-----------|-------------------------------------------|
| `acks`                      | `all`     | Wait for all in-sync replicas             |
| `enable.idempotence`        | `true`    | Exactly-once per producer session         |
| `compression.type`          | `lz4`     | Fast compression, ~60% size reduction     |
| `batch.size`                | `65536`   | 64KB batches for throughput               |
| `linger.ms`                 | `5`       | Small delay to fill batches               |
| `message.timeout.ms`        | `10000`   | 10s delivery timeout                      |

## Consumer Configuration

Per-task-type consumers use manual offset commits:

| Setting                     | Value              | Rationale                                  |
|-----------------------------|--------------------|--------------------------------------------|
| `group.id`                  | `conductor-workers`| Shared across all backend replicas         |
| `enable.auto.commit`        | `false`            | Commit only after poll returns task to worker |
| `auto.offset.reset`         | `earliest`         | Process all pending tasks on fresh start   |
| `session.timeout.ms`        | `10000`            | Quick rebalance on crash                   |
| `fetch.wait.max.ms`         | `100`              | Low-latency polling                        |

## Task Lifecycle with Kafka

1. **Task scheduled** → `KafkaTaskQueue::enqueue(task_type, task_id)` produces message to `conductor.task.{task_type}`
2. **Worker polls** → `KafkaTaskQueue::dequeue(task_type)` consumes next message, commits offset
3. **Engine looks up task** → Fetches full task data from PostgreSQL by `task_id`
4. **Worker processes** → Updates task status via REST/gRPC
5. **Task fails** → Retry logic produces new message to Kafka topic
6. **Orphan recovery** → Background sweeper detects stale IN_PROGRESS tasks and re-enqueues via Kafka

### Failure Scenarios

| Scenario                    | Behavior                                           |
|-----------------------------|-----------------------------------------------------|
| Worker crashes after poll   | Sweeper detects orphan, re-enqueues to Kafka        |
| Kafka broker down           | Producer retries for 10s, then returns error         |
| Backend restart             | Consumer group rebalances, resumes from last commit  |
| DB unavailable after dequeue| Task stays IN_PROGRESS, sweeper recovers later       |

## Health Check

The `/api/admin/health` endpoint and `AdminService/HealthCheck` gRPC method include a Kafka health entry:

```json
{
  "healthy": true,
  "details": [
    { "component": "postgres-shard-0", "healthy": true },
    { "component": "redis", "healthy": true },
    { "component": "kafka", "healthy": true }
  ]
}
```

The Kafka health check performs a metadata fetch against the brokers with a 3-second timeout.

## Code Reference

| File                        | Role                                               |
|-----------------------------|-----------------------------------------------------|
| `store/kafka.rs`            | `KafkaTaskQueue` — producer, consumer pool, enqueue/dequeue |
| `engine/task_ops.rs`        | `poll_task` / `batch_poll_tasks` use Kafka dequeue  |
| `engine/system_tasks.rs`    | Worker task creation enqueues via Kafka              |
| `engine/workflow_ops.rs`    | Retry re-queue uses Kafka                           |
| `engine/sweeper.rs`         | Orphan recovery re-enqueues via Kafka               |
| `engine/admin.rs`           | Health check includes Kafka broker connectivity     |
| `store/redis.rs`            | Still used for task routing hash + queue pause flags |

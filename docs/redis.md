# Redis — Task Routing & Queue Control

rust-conductor uses **Redis** for two performance-critical, low-latency operations: task-to-shard routing and queue pause/resume flags. Redis does **not** store workflow state — that lives in PostgreSQL.

## Why Redis?

| Requirement | Why Redis Fits |
|-------------|----------------|
| **Sub-millisecond lookups** | When a worker polls for a task, the engine needs to know which PostgreSQL shard holds that task's workflow. Redis hash lookups (`HGET`) are O(1) and return in <1ms. |
| **Distributed locking** | Per-workflow advance locks (`SET NX EX`) prevent two engine instances from scheduling duplicate tasks for the same workflow simultaneously. |
| **Ephemeral flags** | Queue pause/resume is a transient operational flag — it doesn't need ACID guarantees or disk persistence. Redis's in-memory model is ideal. |
| **Mature Rust ecosystem** | `deadpool-redis` provides async connection pooling with health checks, and `redis` crate has full command coverage. |

### Why Not Use Redis for Everything?

| Concern | Redis Limitation |
|---------|-----------------|
| **Durability** | In-memory by default. A crash can lose routing entries (recoverable, but adds latency). |
| **Complex queries** | No SQL-like filtering for workflow search, status aggregation, or join queries. |
| **Transaction isolation** | Redis transactions (MULTI/EXEC) are weaker than PostgreSQL's MVCC. |
| **Large payloads** | Workflow inputs/outputs can be megabytes — storing them in Redis wastes expensive RAM. |

Redis is used **only** for what it does best: fast lookups and lightweight coordination.

---

## Data Stored in Redis

### 1. Task Routing Hash

```
Key:   conductor:task_routing
Type:  Hash
Field: {task_id}
Value: {workflow_id}
```

**Purpose:** When a worker completes a task, the engine needs to find the workflow's PostgreSQL shard. Instead of broadcasting a query to all shards, the engine does a single `HGET conductor:task_routing {task_id}` to get the `workflow_id`, then uses deterministic shard routing to find the correct shard.

**Lifecycle:**
- **Set** when a task is scheduled (`HSET`)
- **Read** when a task is polled or updated (`HGET`)
- **Deleted** when a task reaches a terminal state (`HDEL`) — prevents unbounded growth

### 2. Queue Pause Flags

```
Key:   conductor:queue:paused:{queue_name}
Type:  String
Value: "1"
```

**Purpose:** Operators can pause a task queue (e.g., `SIMPLE`) to stop task delivery while debugging. Workers polling that queue type will receive no tasks.

**Operations:**
- `SET conductor:queue:paused:SIMPLE 1` — pause
- `DEL conductor:queue:paused:SIMPLE` — resume
- `EXISTS conductor:queue:paused:SIMPLE` — check status

### 3. Advance Locks

```
Key:   conductor:advance:{workflow_id}
Type:  String (with NX + EX)
TTL:   30 seconds
```

**Purpose:** Prevents concurrent advancement of the same workflow. When the engine processes a task completion, it acquires this lock before evaluating what to schedule next. If a second task completes for the same workflow at the same time, its advance attempt will wait or skip.

**Why SET NX EX?** `NX` ensures only one holder. `EX 30s` ensures the lock is released even if the holder crashes.

---

## Sharding

Redis supports multiple instances via hash-based deterministic routing. Given a key, the engine hashes it to select a Redis shard.

```
hash(key) % num_redis_shards → redis pool index
```

Each shard gets a `deadpool-redis` connection pool with up to 128 connections.

**Why shard Redis?** A single Redis instance can be a bottleneck when thousands of workers are polling simultaneously. Sharding spreads the load across multiple instances.

---

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `REDIS_URLS` | `redis://127.0.0.1:6379` | Comma-separated Redis URLs |

Multiple URLs enable sharding:
```bash
REDIS_URLS=redis://redis-0:6379,redis://redis-1:6379,redis://redis-2:6379
```

### Docker Compose

```yaml
redis-0:
  image: redis:7-alpine
  ports:
    - "6379:6379"
  command: redis-server --appendonly yes --maxmemory 256mb --maxmemory-policy allkeys-lru
```

### Production Recommendations

- Run **multiple Redis instances** (one per backend replica, or 2-4 total)
- Enable AOF persistence (`appendonly yes`) for routing data recovery after crashes
- Set `maxmemory-policy` to `allkeys-lru` — routing entries are reconstructable, so eviction is safe
- Monitor memory usage — routing hash grows with active task count
- Consider Redis Sentinel or Redis Cluster for high availability

---

## Redis Streams — Development Task Queue

When the `kafka` feature is **disabled**, Redis also serves as the task queue via Redis Streams. This is a development convenience — in production, Kafka is recommended.

### How It Works

```
Stream key: conductor:queue:{task_type}
Consumer group: conductor-workers
```

| Operation | Redis Command |
|-----------|--------------|
| Enqueue | `XADD conductor:queue:SIMPLE * task_id {id}` |
| Dequeue | `XREADGROUP GROUP conductor-workers consumer-0 COUNT 1 BLOCK 1000 STREAMS conductor:queue:SIMPLE >` |
| Ack | `XACK conductor:queue:SIMPLE conductor-workers {message_id}` |

### Why Redis Streams as Fallback?

| Concern | Redis Streams | Kafka |
|---------|---------------|-------|
| Setup complexity | Zero — Redis is already in the stack | Requires broker, KRaft/ZooKeeper |
| Latency | Lower (in-memory) | Higher (disk-backed) |
| Durability | Limited (in-memory) | Replicated to disk |
| Consumer groups | Basic | Full-featured (lag, offsets, replay) |
| Production scale | Single-node bottleneck | Horizontally partitioned |

Redis Streams is the **drop-in replacement** — identical interface, zero code changes, just a feature flag toggle.

---

## Health Check

The `/health` endpoint includes Redis connectivity status:

```json
{
  "healthy": true,
  "details": [
    { "component": "redis", "healthy": true }
  ]
}
```

Health is checked via `PING` on each Redis shard.

---

## Code Reference

| File | Role |
|------|------|
| `store/redis.rs` | `ShardedRedis` — connection pools, task routing operations, queue pause flags |
| `store/redis_queue.rs` | `RedisTaskQueue` — Redis Streams-based task queue (dev fallback) |
| `engine/advance.rs` | Advance lock acquisition (`SET NX EX`) |
| `engine/task_ops.rs` | Task routing lookups during poll/update |
| `engine/system_tasks.rs` | Routing entry creation on task schedule |
| `engine/admin.rs` | Queue pause/resume + health check |

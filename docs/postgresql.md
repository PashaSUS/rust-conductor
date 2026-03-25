# PostgreSQL — Primary State Storage

rust-conductor uses **PostgreSQL** as the sole persistent store for all workflow state, task state, definitions, event handlers, and schedules.

## Why PostgreSQL?

| Requirement | Why PostgreSQL Fits |
|-------------|---------------------|
| **ACID transactions** | Workflow state transitions (RUNNING → COMPLETED) must be atomic. A half-written status update would corrupt the entire execution. PostgreSQL's transactional guarantees prevent this. |
| **Flexible payloads** | Workflow/task inputs and outputs are arbitrary JSON. PostgreSQL's `JSONB` columns allow storing dynamic payloads without schema migrations every time a workflow definition changes. |
| **Concurrency control** | Advisory locks (`pg_advisory_xact_lock`) serialize database migrations across multi-replica startups, preventing race conditions. |
| **Deduplication** | Partial unique indexes (`idx_task_unique_ref`) prevent duplicate task scheduling — something hard to achieve with document stores. |
| **Ecosystem maturity** | Battle-tested connection pooling (PgBouncer), monitoring (pg_stat), backup (pg_basebackup), and replication (streaming replication). |
| **Async Rust support** | `sqlx` provides compile-time checked queries with native async/await — no ORM overhead, no runtime SQL surprises. |

### Why Not Other Databases?

| Alternative | Why Not |
|-------------|---------|
| **MySQL** | Weaker JSONB support, no partial indexes, advisory locks are connection-scoped (not transaction-scoped) |
| **MongoDB** | No multi-document ACID transactions (until recently), harder to enforce referential integrity between workflows and tasks |
| **DynamoDB/Cassandra** | Eventually consistent by default — workflow state machines need strong consistency to avoid duplicate task scheduling |
| **SQLite** | Single-writer, no concurrent access from multiple backend replicas |

---

## Schema

Tables are auto-created via embedded migrations on startup. Each shard runs the same schema.

| Table | Purpose | Key Columns |
|-------|---------|-------------|
| `workflow_def` | Workflow definition versions | `(name, version)` composite PK; `definition` JSONB |
| `task_def` | Task type definitions | `name` PK; `definition` JSONB |
| `workflow` | Workflow execution instances | `workflow_id` PK; `status`, `input`, `output` JSONB |
| `task` | Task execution instances | `task_id` PK; FK `workflow_id`; `status`, `input_data`, `output_data` JSONB |
| `task_log` | Task execution log entries | `id` SERIAL; FK `task_id`; `log_message`, `created_time` |
| `event_handler` | Event handler configurations | `name`; event topic; action definitions |
| `scheduled_workflow` | CRON schedule configurations | `schedule_id` PK; `cron_expression`, `timezone`, `next_run_at` |

### Indexes

| Index | Table | Purpose |
|-------|-------|---------|
| `idx_workflow_status` | `workflow` | Filter by status (RUNNING, COMPLETED, etc.) |
| `idx_workflow_name` | `workflow` | Filter by workflow type |
| `idx_task_workflow` | `task` | Fetch all tasks for a workflow |
| `idx_task_type_status` | `task` | Route tasks by type + status (poll queries) |
| `idx_task_unique_ref` | `task` | **Partial unique** on `(workflow_id, reference_task_name)` for non-terminal tasks — prevents duplicate scheduling |

---

## Connection Management

### sqlx Pool

Each shard gets its own `sqlx::PgPool` (async, compile-time checked queries). The pool handles connection lifecycle, retries, and health checks automatically.

### PgBouncer (Production)

In the Docker Compose stack, PgBouncer sits between the backend and PostgreSQL:

```
Backend → PgBouncer (:6432) → PostgreSQL (:5432)
```

**Why PgBouncer?** PostgreSQL creates a heavyweight process per connection. With many backend replicas and many async tasks, the connection count can exceed PostgreSQL's `max_connections`. PgBouncer multiplexes many client connections onto fewer server connections using transaction-level pooling.

---

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `DATABASE_URL` | — | Single-shard PostgreSQL URL (fallback) |
| `SHARD_DATABASE_URLS` | — | Comma-separated explicit shard URLs |
| `NUM_SHARDS` | `1` | Number of shards (used with template) |
| `SHARD_DB_URL_TEMPLATE` | — | Template URL with `{i}` placeholder |
| `SKIP_MIGRATIONS` | `false` | Skip database migrations on startup |
| `MIGRATE_ONLY` | `false` | Run migrations and exit (for init containers) |

### Docker Compose

```yaml
postgres-shard-0:
  image: postgres:16-alpine
  environment:
    POSTGRES_DB: conductor
    POSTGRES_USER: conductor
    POSTGRES_PASSWORD: conductor
  command: >
    -c max_connections=200
    -c shared_buffers=256MB
    -c effective_cache_size=512MB
    -c max_locks_per_transaction=256
```

### Production Recommendations

- Run **multiple shards** on separate hosts for horizontal scaling (see [sharding.md](sharding.md))
- Deploy PgBouncer with `pool_mode = transaction` and `max_client_conn` sized to your backend replica count
- Set `shared_buffers` to ~25% of available RAM
- Configure streaming replication for each shard for high availability
- Use PITR (Point-In-Time Recovery) backups via `pg_basebackup` + WAL archiving

---

## Migration Strategy

1. On startup, the first backend replica acquires a PostgreSQL advisory lock on shard-0
2. Migrations run sequentially on **every shard** while the lock is held
3. Other replicas block on the advisory lock, waiting for migrations to complete
4. Once released, all replicas proceed to normal operation

This ensures exactly-once migration execution in multi-replica deployments without external coordination tools.

---

## Code Reference

| File | Role |
|------|------|
| `store/postgres.rs` | Pool initialization, migrations, all SQL queries |
| `engine/shard.rs` | Shard routing + pool selection |
| `config.rs` | Database URL parsing and shard configuration |
| `main.rs` | Advisory lock acquisition, migration orchestration |

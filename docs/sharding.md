# Sharding Architecture

rust-conductor distributes workflow data across **multiple PostgreSQL databases** (shards) to scale horizontally. Each workflow is deterministically assigned to one shard, so all mutations for that workflow are single-shard operations with no distributed transactions.

## Why Sharding?

| Problem | Impact Without Sharding |
|---------|------------------------|
| Workflow tables grow unbounded | Single database becomes a bottleneck — queries slow, vacuuming takes longer, WAL grows |
| Write contention | Many concurrent workflow advancements compete for the same database locks |
| Connection limits | PostgreSQL's per-connection process model limits concurrency to `max_connections` |
| Backup/recovery time | A single large database takes longer to back up and restore |

**Sharding solves all of these** by splitting the data across N independent databases. Each shard handles 1/N of the traffic.

### Why Not Read Replicas?

Read replicas help with read-heavy workloads, but workflow orchestration is **write-heavy** — every task poll, update, and advancement writes to the database. Sharding distributes writes, replicas don't.

### Why Not a Distributed Database (CockroachDB, Spanner)?

| Concern | Distributed SQL | PostgreSQL Sharding |
|---------|----------------|---------------------|
| Complexity | High (consensus protocols, distributed transactions) | Low (standard PostgreSQL) |
| Latency | Higher (multi-node consensus per write) | Lower (single-node writes) |
| Operational cost | Specialized expertise required | Standard PostgreSQL ops |
| Rust driver support | Limited | Excellent (sqlx) |

PostgreSQL sharding is simpler, faster for single-entity operations, and uses battle-tested infrastructure.

---

## Routing Algorithm

Given a `workflow_id`, the engine determines its shard:

### UUID Path (Primary)

```
1. Parse workflow_id as UUID
2. Extract last 4 bytes → interpret as u32
3. shard_index = u32 % num_shards
```

### Non-UUID Fallback

```
1. Sum all bytes of the workflow_id string
2. shard_index = byte_sum % num_shards
```

**Why the last 4 bytes of a UUID?** UUID v4 (random) has uniform distribution. The last 4 bytes provide 2³² possible values, giving excellent uniformity even with a small number of shards. Using the last bytes (rather than first) avoids UUID version/variant bits that are non-random.

**Why is this deterministic?** The same `workflow_id` always maps to the same shard. No routing table needed, no coordination between replicas, no external service dependency.

---

## Metadata vs. Workflow Data

| Data Type | Storage Strategy |
|-----------|-----------------|
| **Workflow/task definitions** | Written to shard-0 (canonical), but readable from any shard |
| **Workflow instances** | Routed by `workflow_id` hash |
| **Task instances** | Same shard as their parent workflow |
| **Event handlers** | Shard-0 (global config) |
| **Schedules** | Shard-0 (global config) |

**Why shard-0 for metadata?** Definitions are read frequently but written rarely. Having a single canonical source (shard-0) avoids stale-read issues across shards.

---

## Operations

### Single-Workflow Operations (O(1) shard)

These operations touch exactly one shard:

```
shard_for(workflow_id) → single PgPool
```

- Start workflow
- Get workflow
- Pause / Resume / Terminate
- Advance (schedule next tasks)
- Update task status

### Fan-Out Operations (all shards)

These operations query every shard and merge results:

```
all_shards() → Vec<PgPool>
```

- Search workflows (status, name, free text)
- Get workflow stats (count per status)
- Queue size aggregation

**Why fan-out for search?** Search needs global visibility. Since each shard holds a subset of workflows, the engine broadcasts the query and merges. For most deployments (1-4 shards), this is fast. For very large deployments, a dedicated search index (e.g., Elasticsearch) would be added.

---

## Configuration

Three tiers of configuration, evaluated in priority order:

### Tier 1: Explicit URLs (Production)

```bash
SHARD_DATABASE_URLS=postgres://host1/shard0,postgres://host2/shard1,postgres://host3/shard2
```

Use when shards are on different hosts with different credentials.

### Tier 2: Template (Staging)

```bash
NUM_SHARDS=4
SHARD_DB_URL_TEMPLATE=postgres://user:pass@pghost/conductor-shard-{i}
```

Generates: `conductor-shard-0`, `conductor-shard-1`, `conductor-shard-2`, `conductor-shard-3`

Use when shards follow a consistent naming pattern on the same host.

### Tier 3: Single Database (Development)

```bash
DATABASE_URL=postgres://localhost/conductor
```

One shard, zero configuration. The sharding layer still works — it's just a single-element pool.

**Why three tiers?** Developers don't want to configure 4 databases to run locally. Production operators want full control over each shard's connection string. The tiered approach satisfies both without conditional logic in the application.

---

## Task Routing via Redis

When a task is scheduled, the engine writes a routing entry to Redis:

```
HSET conductor:task_routing {task_id} {workflow_id}
```

When a worker updates a task, the engine looks up the workflow:

```
HGET conductor:task_routing {task_id} → workflow_id
shard_for(workflow_id) → correct PgPool
```

**Why not just hash the task_id?** Task IDs are generated independently — they don't carry shard affinity. Only the `workflow_id` determines the shard. Redis bridges this gap with O(1) lookups, avoiding a broadcast query to all shards.

See [redis.md](redis.md) for full Redis documentation.

---

## Adding a Shard

1. Create the new PostgreSQL database
2. Update `SHARD_DATABASE_URLS` (or increment `NUM_SHARDS`)
3. Restart the backend — migrations run on the new shard automatically
4. **Existing workflows are not migrated** — they remain on their original shard. Only new workflows may route to the new shard.

**Why no re-balancing?** Re-sharding (moving workflows between shards) requires copying data, updating routing, and handling in-flight operations. This complexity isn't justified for most deployments. Adding shards absorbs new traffic growth.

---

## Code Reference

| File | Role |
|------|------|
| `engine/shard.rs` | `ShardedPool` — shard routing, pool selection, fan-out helpers |
| `config.rs` | Shard URL resolution (3-tier priority) |
| `store/postgres.rs` | Per-shard pool creation, migrations |
| `main.rs` | Sequential shard initialization with advisory lock |
| `store/redis.rs` | Task routing hash (`task_id → workflow_id`) |

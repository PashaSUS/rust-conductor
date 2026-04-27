# Production Performance Tuning Guide

This document captures the validated configuration for high-throughput production deployments
of rust-conductor, based on stress-testing the multi-shard build (4 postgres shards, 4 redis,
4 kafka, 4 backend replicas behind nginx-lb) on a 32-core / 50 GB host.

## Recommended Production Settings

### Backend (`docker-compose.yml`)

```yaml
environment:
  RUST_LOG: "warn"                  # Drop info logs — they cost ~15-20% throughput at scale
  NUM_SHARDS: "4"                   # 4 shards is the sweet spot for 32-core hosts; bump to 8 only when CPU is saturated
  RATE_LIMIT_ENABLED: "true"        # Keep ENABLED in real prod; only disable for stress-tests where the LB hides client IPs
  SQLX_MAX_CONNECTIONS: "100"       # default; raise to 150 only if pgbouncer pool also raised
  SQLX_MIN_CONNECTIONS: "5"
deploy:
  replicas: 4                       # 4 replicas × 32 actix workers ≈ optimal for 32 cores; more replicas mostly add contention
```

### PgBouncer (per shard)

```yaml
DEFAULT_POOL_SIZE: "80"             # validated under load; 200 caused server-side lock contention
MAX_CLIENT_CONN:   "800"
RESERVE_POOL_SIZE: "20"
POOL_MODE:         "transaction"
```

### Postgres (per shard)

```
-c max_connections=200
-c shared_buffers=256MB
-c effective_cache_size=512MB
-c work_mem=8MB
```

### nginx-lb

The default `nginx:alpine` defaults `worker_connections` to 1024 which becomes a bottleneck
at scale. We mount `nginx-main.conf` with:

- `worker_processes auto`
- `worker_rlimit_nofile 65536`
- `worker_connections 16384`
- `multi_accept on`
- `use epoll`
- `listen 8090 reuseport backlog=4096` for the proxy server block

Plus `ulimits.nofile: 65536` on the container.

### Engine (`ExecutionManagerConfig`)

```rust
ExecutionManagerConfig {
    max_concurrent_workers: 1000,   // raise only if the backend can keep up
    poller_count: 1,                // 1 = stable; 2-4 helps on Linux hosts with high task fanout
    ..Default::default()
}
```

The engine now supports `poller_count > 1` — N parallel poll loops per registered worker,
each with a distinct worker_id (`{base}-p{N}`) so server-side leasing treats them as
separate consumers. Use this when a single batch poll round-trip becomes the bottleneck.

**Caveat:** on Windows hosts running the engine, ephemeral-port exhaustion kicks in
quickly with `poller_count > 1` + large batch sizes + sub-100ms poll intervals.
On Linux hosts this scales cleanly.

### Sweeper

Set in `backend/src/engine/sweeper.rs`:
- SUB_WORKFLOW sweep filtered by `update_time < NOW() - INTERVAL '30 seconds'`
  — without this filter, every parent waiting on an in-flight sub-workflow is re-scooped
  every cycle, causing massive write amplification on the parent-advance path.

### Parent Advance Retry (`backend/src/engine/advance.rs`)

`try_complete_parent_sub_workflow` retries 3× with quadratic backoff on transient
PgBouncer `query_wait_timeout` errors.

## Measured Throughput

- **scale 1000** (5000 workflows, 5 sub-workflows each = ~15k tasks):
  ~25 wf/s end-to-end on a 32-core Windows host running engine externally.
  All 6995 task-level completions clean, 0 failures.
- Pure task throughput: ~50 tasks/sec sustained.
- Launch rate: ~870 wf/s (engine can submit far faster than backend can drain).

## Known Bottlenecks (in order)

1. **Parent-chain advancement** under bursty completion — sub-workflow → parent → grandparent
   transitions serialize on row-level locks. Mitigated by retry+backoff and 30s sweep filter.
2. **Windows host networking** when engine runs externally — port reuse limits.
   Linux hosts are not affected.
3. **Kafka consumer-group rebalancing** during cold-start when many backend replicas come
   up simultaneously. Fixed by staggered startup or adjusted `session.timeout.ms`.

## Tuning Knobs Available via `setup-prod.bat`

The `THROUGHPUT` preset bumps:
- `NUM_SHARDS` 4 → 8
- `SWEEPER_BATCH_SIZE` and intervals
- `RUST_LOG` to `warn`
- PgBouncer pool sizes

Choose `BALANCED` for the validated default, `THROUGHPUT` if you have >64 cores,
or `CUSTOM` to set every knob manually.

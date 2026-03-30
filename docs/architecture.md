# Rust-Conductor — Architecture Overview

A high-performance, Rust-native workflow orchestration engine inspired by Netflix Conductor.
This document provides a high-level overview of the system and links to detailed topic-specific documentation.

---

## System Architecture

```
┌──────────────┐       ┌──────────────────────────────────────────┐
│   Frontend   │       │              Backend (Rust)               │
│  React SPA   │──────▶│  ┌──────────┐      ┌──────────────────┐  │
│  (Vite)      │  REST │  │ Actix-Web│      │  Tonic gRPC      │  │
└──────────────┘  API  │  │ :8090    │      │  :50055           │  │
                       │  └────┬─────┘      └────────┬─────────┘  │
                       │       │     Shared Engine    │            │
                       │       └──────────┬───────────┘            │
                       │          ┌───────▼────────┐               │
                       │          │ WorkflowEngine │               │
                       │          └──┬──┬──┬──┬────┘               │
                       └─────────────┼──┼──┼──┼────────────────────┘
                                     │  │  │  │
               ┌─────────────────────┘  │  │  └──────────────────────┐
               ▼                        ▼  ▼                         ▼
        ┌─────────────┐     ┌────────┐ ┌────────┐          ┌───────────────┐
        │ PostgreSQL  │     │ Redis   │ │ Kafka  │          │ S3 / RustFS   │
        │ (Sharded)   │     │(Sharded)││(Topics)│          │ (Ext. Storage)│
        └─────────────┘     └────────┘ └────────┘          └───────────────┘
```

Both the REST API (Actix-web) and the gRPC server (Tonic) share a single `Arc<WorkflowEngine>` instance. They are thin transport adapters — all orchestration logic lives in the engine.

---

## Documentation Index

Each topic has its own dedicated document with design rationale, configuration, and code references.

### Core Engine

| Document                | Description                                                                   |
| ----------------------- | ----------------------------------------------------------------------------- |
| [Engine](engine.md)     | Workflow engine core — lifecycle, advancement, task types, sweeper, scheduler |
| [Sharding](sharding.md) | Deterministic UUID-based shard routing, fan-out, metadata routing             |

### External Services

| Document                    | Description                                                            |
| --------------------------- | ---------------------------------------------------------------------- |
| [PostgreSQL](postgresql.md) | Primary storage — schema, indexes, migrations, PgBouncer, tuning       |
| [Redis](redis.md)           | Task routing, advance locks, queue pause flags, Redis Streams fallback |
| [Kafka](kafka.md)           | Durable task queue — producer/consumer config, topics, consumer groups |
| [gRPC](grpc.md)             | gRPC API — dual proto support, server tuning, Nginx proxying           |
| [S3 Storage](s3-storage.md) | External payload storage — threshold offloading, key format, RustFS    |

### Frontend & Testing

| Document                              | Description                                                                |
| ------------------------------------- | -------------------------------------------------------------------------- |
| [Frontend](frontend.md)               | React dashboard — tech stack choices, state management, theme system       |
| [Testing](testing.md)                 | Four-layer strategy — unit, integration (testcontainers), benchmarks, fuzz |

### Configuration & Operations

| Document                                     | Description                                                                  |
| -------------------------------------------- | ---------------------------------------------------------------------------- |
| [Configuration](configuration.md)            | Complete environment variable reference with defaults                        |
| [Feature Flags](feature-flags.md)            | Compile-time feature flags, build profiles, runtime toggles                  |
| [Rate Limiting](rate-limiting.md)            | Token bucket rate limiter, CORS, payload limits                              |

---

## Key Design Patterns

| Pattern                                  | Where                                   | Why                                     |
| ---------------------------------------- | --------------------------------------- | --------------------------------------- |
| **Single engine, multiple transports**   | REST + gRPC share `Arc<WorkflowEngine>` | No logic duplication between API layers |
| **Deterministic shard routing**          | UUID-based hash → shard index           | Single-shard transactions, no 2PC       |
| **Hybrid queue abstraction**             | Kafka (prod) / Redis Streams (dev)      | Same interface, different durability    |
| **Task routing via Redis**               | `task_id → workflow_id` hash            | O(1) shard lookup for workers           |
| **Per-workflow advance lock**            | Redis SET NX                            | Prevents duplicate task scheduling      |
| **Advisory lock for migrations**         | PostgreSQL advisory locks               | Safe multi-replica startup              |
| **Sweeper for eventual consistency**     | Periodic background task                | Recovers stuck/orphaned work            |
| **Feature flags**                        | Cargo features                          | Minimal binary for simple deployments   |
| **Idempotency keys**                     | Workflow start + task update            | Safe retries in distributed systems     |
| **Template resolution at schedule time** | `${...}` expansion in engine            | Workers receive resolved inputs         |
| **Fire-and-forget webhooks**             | Non-blocking HTTP POST                  | Engine doesn't block on external calls  |
| **12-factor configuration**              | All config from env vars                | Container-friendly, no config files     |
| **Server-state via React Query**         | Frontend data fetching                  | No client-side store needed             |

---

## Feature Flags

Compile-time feature flags in `Cargo.toml`:

| Flag               | Default | Purpose                                                                       |
| ------------------ | ------- | ----------------------------------------------------------------------------- |
| `seq`              | **Yes** | Enable structured log shipping to Seq server.                                 |
| `kafka`            | **Yes** | Use Apache Kafka for task queues. When disabled, falls back to Redis Streams. |
| `graphql`          | **Yes** | Enable GraphQL API endpoint at `/api/graphql`.                                |
| `sse`              | **Yes** | Enable Server-Sent Events for real-time workflow/queue streaming.             |
| `websocket`        | **Yes** | Enable WebSocket API for bidirectional task/workflow feeds.                    |
| `grpc-reflection`  | **Yes** | Enable gRPC reflection service for dynamic client discovery.                  |
| `api-v2`           | **Yes** | Enable enhanced V2 REST API with envelope format, long-polling, and batching. |
| `external-storage` | No      | Enable S3 external payload storage for large payloads.                        |

See [Feature Flags & Build Profiles](feature-flags.md) for build profiles and detailed configuration.

---

## Environment Variables Reference

### Database & Sharding

| Variable                | Default | Description                            |
| ----------------------- | ------- | -------------------------------------- |
| `DATABASE_URL`          | —       | Single-shard PostgreSQL URL (fallback) |
| `SHARD_DATABASE_URLS`   | —       | Comma-separated explicit shard URLs    |
| `NUM_SHARDS`            | 1       | Number of shards (used with template)  |
| `SHARD_DB_URL_TEMPLATE` | —       | Template with `{i}` placeholder        |
| `SKIP_MIGRATIONS`       | false   | Skip database migrations on startup    |
| `MIGRATE_ONLY`          | false   | Run migrations and exit                |

### Redis

| Variable     | Default                  | Description                |
| ------------ | ------------------------ | -------------------------- |
| `REDIS_URLS` | `redis://127.0.0.1:6379` | Comma-separated Redis URLs |

### Kafka

| Variable        | Default          | Description                            |
| --------------- | ---------------- | -------------------------------------- |
| `KAFKA_BROKERS` | `localhost:9092` | Comma-separated Kafka broker addresses |

### Server

| Variable      | Default     | Description                    |
| ------------- | ----------- | ------------------------------ |
| `HOST`        | `localhost` | Bind address                   |
| `PORT`        | `8090`      | REST API port                  |
| `GRPC_PORT`   | `50055`     | gRPC server port               |
| `CORS_ORIGIN` | —           | Allowed CORS origin (optional) |

### Logging

| Variable      | Default | Description                                           |
| ------------- | ------- | ----------------------------------------------------- |
| `RUST_LOG`    | `info`  | Tracing env-filter (e.g., `debug`, `conductor=trace`) |
| `SEQ_URL`     | —       | Seq server URL (requires `seq` feature)               |
| `SEQ_API_KEY` | —       | Seq API key                                           |

### External Storage

| Variable                     | Default | Description                            |
| ---------------------------- | ------- | -------------------------------------- |
| `S3_ENDPOINT`                | —       | S3-compatible endpoint URL             |
| `S3_BUCKET`                  | —       | Bucket name                            |
| `S3_REGION`                  | —       | AWS region (or `us-east-1` for MinIO)  |
| `S3_ACCESS_KEY`              | —       | Access key ID                          |
| `S3_SECRET_KEY`              | —       | Secret access key                      |
| `EXTERNAL_PAYLOAD_THRESHOLD` | `32768` | Bytes threshold for S3 externalization |

---

## Infrastructure

### Docker Compose Stack

| Service             | Image                  | Role                  | Ports       |
| ------------------- | ---------------------- | --------------------- | ----------- |
| `postgres-shard-0`  | `postgres:17-alpine`   | Database shard 0      | 5432        |
| `pgbouncer-shard-0` | `edoburu/pgbouncer`    | Connection pooler     | 6432        |
| `redis-0`           | `redis:8-alpine`       | Task routing, locks   | 6379        |
| `kafka-0`           | `apache/kafka:4.0.0`   | Task queue (KRaft)    | 9092        |
| `seq`               | `datalust/seq:latest`  | Log aggregation       | 9321        |
| `rustfs`            | `rustfs/rustfs:latest` | S3-compatible storage | 9000, 9001  |
| `backend-migrate`   | _(Dockerfile)_         | Rust backend          | 8090, 50055 |

### Container Images

**Backend** (`backend/Dockerfile`): Multi-stage build `rust:1.94-slim` → `debian:trixie-slim`. Exposes 8090 (REST) + 50055 (gRPC).

**Frontend** (`frontend/Dockerfile`): Multi-stage build `node:24` → `serve`. Exposes 3170.

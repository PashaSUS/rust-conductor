# Feature Flags & Build Profiles

rust-conductor uses Cargo feature flags to control which optional services are compiled into the binary. This allows you to tailor the build for your specific deployment scenario.

## Feature Flags

| Feature | Default | Dependencies | Description |
|---------|---------|-------------|-------------|
| `seq` | ✅ | — | Structured logging to [Seq](https://datalust.co/seq) via `SEQ_URL` env var |
| `kafka` | ✅ | `rdkafka` | Apache Kafka for task queue (instead of Redis Streams) |
| `graphql` | ✅ | `async-graphql`, `async-graphql-actix-web` | GraphQL API at `/api/graphql` |
| `sse` | ✅ | `async-stream` | Server-Sent Events at `/api/sse/*` |
| `websocket` | ✅ | `actix-ws` | WebSocket endpoints at `/api/ws/*` |
| `grpc-reflection` | ✅ | `tonic-reflection` | gRPC server reflection for dynamic client discovery |
| `api-v2` | ✅ | — | V2 REST API with envelope format, batch ops, long-polling |
| `external-storage` | ❌ | `rust-s3` | S3/MinIO external payload storage |

## Build Profiles

Convenience feature groups for common deployment scenarios:

### bare-metal
```bash
cargo build --release --no-default-features --features bare-metal
```
- No optional services (no GraphQL, SSE, WebSocket, Kafka, etc.)
- Fastest compile time, smallest binary
- Uses Redis Streams for task queuing
- Ideal for: development, testing, minimal deployments

### feature-rich
```bash
cargo build --release --features feature-rich
```
- **All** features enabled: Kafka, GraphQL, SSE, WebSocket, gRPC reflection, V2 API, external storage, Seq
- Ideal for: production deployments that need everything

### fast
```bash
cargo build --release --no-default-features --features fast
```
- Seq logging only, no Kafka overhead
- Optimised for low-latency local development
- Ideal for: development machines, quick iteration

### scalable
```bash
cargo build --release --no-default-features --features scalable
```
- Kafka for event queuing, S3 for external payload storage, gRPC reflection
- No GraphQL/SSE/WebSocket overhead
- Ideal for: high-throughput production clusters

## Setup Script

Run `setup.bat` to interactively choose a build profile:

```
1. DEV          - Lightweight Docker setup (1 shard, Redis Streams)
2. PROD         - Full production Docker setup (scaling, pooling, logging)
3. BARE-METAL   - Minimal core only (no optional services)
4. FEATURE-RICH - All features enabled
5. FAST         - Optimised for low-latency development
6. SCALABLE     - Kafka + external storage + gRPC reflection
```

## Runtime Configuration

Even when features are compiled in, some can be toggled at runtime:

| Env Variable | Default | Description |
|-------------|---------|-------------|
| `RATE_LIMIT_ENABLED` | `true` | Set to `false` to disable rate limiting (useful for stress tests) |
| `RATE_LIMIT_MAX_REQUESTS` | `1000` | Max requests per window per client IP |
| `RATE_LIMIT_WINDOW_SECS` | `60` | Rate limit window in seconds |
| `SEQ_URL` | — | Seq logging server URL (only with `seq` feature) |
| `SEQ_API_KEY` | — | Optional Seq API key |
| `S3_ENDPOINT` | — | S3/MinIO endpoint URL (only with `external-storage` feature) |

## Custom Feature Combinations

You can combine features freely:

```bash
# Just Kafka + GraphQL
cargo build --release --no-default-features --features "kafka,graphql"

# Everything except WebSocket
cargo build --release --features "default" --no-default-features --features "seq,kafka,graphql,sse,grpc-reflection,api-v2"
```

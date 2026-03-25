# Configuration Reference

All configuration is done through environment variables. Below is the complete list.

## Core Settings

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `localhost` | Bind address for HTTP server |
| `PORT` | `8090` | HTTP server port |
| `GRPC_PORT` | `50055` | gRPC server port |
| `CORS_ORIGIN` | *(any)* | Allowed CORS origin (unset = allow all) |
| `RUST_LOG` | `info` | Log level filter (tracing-subscriber `EnvFilter`) |

## Database

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_URL` | `postgres://conductor:conductor@localhost:5432/conductor` | Single-shard database URL |
| `SHARD_DATABASE_URLS` | — | Comma-separated list of shard database URLs |
| `NUM_SHARDS` | — | Number of shards (used with `SHARD_DB_URL_TEMPLATE`) |
| `SHARD_DB_URL_TEMPLATE` | — | Template URL with `{i}` placeholder for shard index |
| `SHARD_REPLICA_DB_URLS` | — | Comma-separated read-replica URLs (parallel to shards) |
| `SKIP_MIGRATIONS` | `false` | Skip database migrations on startup |
| `MIGRATE_ONLY` | `false` | Run migrations and exit |
| `SLOW_QUERY_THRESHOLD_MS` | `500` | Log queries slower than this (milliseconds) |

## Redis

| Variable | Default | Description |
|----------|---------|-------------|
| `REDIS_URLS` | `redis://localhost:6379` | Comma-separated Redis URLs for sharded pool |

## Kafka (feature: `kafka`)

| Variable | Default | Description |
|----------|---------|-------------|
| `KAFKA_BROKERS` | `localhost:9092` | Comma-separated Kafka broker URLs |

## External Storage (feature: `external-storage`)

| Variable | Default | Description |
|----------|---------|-------------|
| `S3_ENDPOINT` | — | S3/MinIO endpoint URL (enables external storage) |
| `S3_BUCKET` | `conductor-payloads` | Bucket name |
| `S3_REGION` | `us-east-1` | AWS region |
| `S3_ACCESS_KEY` | — | Access key |
| `S3_SECRET_KEY` | — | Secret key |

## Seq Logging (feature: `seq`)

| Variable | Default | Description |
|----------|---------|-------------|
| `SEQ_URL` | — | Seq server URL (enables Seq logging) |
| `SEQ_API_KEY` | — | Optional Seq API key |

## Rate Limiting

| Variable | Default | Description |
|----------|---------|-------------|
| `RATE_LIMIT_ENABLED` | `true` | Set to `false` to disable rate limiting |
| `RATE_LIMIT_MAX_REQUESTS` | `1000` | Token bucket capacity per client IP |
| `RATE_LIMIT_WINDOW_SECS` | `60` | Refill window in seconds |

## Engine Tuning

| Variable | Default | Description |
|----------|---------|-------------|
| `DEF_CACHE_MAX_ENTRIES` | `1000` | Max entries in the in-memory LRU definition cache |
| `SWEEPER_MIN_INTERVAL_SECS` | `10` | Minimum background sweeper interval |
| `SWEEPER_MAX_INTERVAL_SECS` | `60` | Maximum background sweeper interval |

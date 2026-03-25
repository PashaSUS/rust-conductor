# Rate Limiting & Security

## Rate Limiting

rust-conductor uses a **token bucket** rate limiter backed by Redis. Each client IP gets a bucket with configurable capacity that refills at a steady rate.

### Configuration

| Env Variable | Default | Description |
|-------------|---------|-------------|
| `RATE_LIMIT_ENABLED` | `true` | Set to `false` or `0` to completely disable rate limiting |
| `RATE_LIMIT_MAX_REQUESTS` | `1000` | Maximum tokens (bucket capacity) per client |
| `RATE_LIMIT_WINDOW_SECS` | `60` | Time window over which tokens refill |

### How It Works

- Each client IP gets a token bucket with `MAX_REQUESTS` capacity
- Tokens refill at `MAX_REQUESTS / WINDOW_SECS` per second
- Each request consumes 1 token
- When the bucket is empty, the client receives `429 Too Many Requests`
- On Redis failure, the limiter **fails open** (requests are allowed)

### Response Headers

Every response includes rate limit info:
- `X-RateLimit-Remaining` — Tokens left in the bucket
- `X-RateLimit-Limit` — Maximum bucket capacity
- `Retry-After` — Seconds until a token is available (only on 429)

### Disabling for Stress Tests

Set `RATE_LIMIT_ENABLED=false` in your environment:

```bash
# Windows
set RATE_LIMIT_ENABLED=false
cargo run --release

# Linux/Mac
RATE_LIMIT_ENABLED=false cargo run --release

# Docker
docker run -e RATE_LIMIT_ENABLED=false rust-conductor
```

### Custom Limits

For stress testing with limits enabled but relaxed:

```bash
RATE_LIMIT_MAX_REQUESTS=100000
RATE_LIMIT_WINDOW_SECS=1
```

## CORS

CORS is configured via the `CORS_ORIGIN` environment variable:

| Value | Behavior |
|-------|----------|
| Not set | Allow any origin (`*`) |
| `http://example.com` | Only allow that specific origin |

## Payload Limits

- JSON body limit: **10 MB** (configurable in source)
- gRPC message size: Uses default tonic limits with gzip compression
- Connection-level: 1024 concurrent requests per gRPC connection, 512 global gRPC concurrency limit

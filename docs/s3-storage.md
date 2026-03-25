# S3 External Payload Storage

rust-conductor can offload large workflow/task payloads to **S3-compatible object storage** instead of storing them inline in PostgreSQL. This feature is **optional** and controlled by the `external-storage` compile-time feature flag.

## Why External Storage?

| Problem | Impact |
|---------|--------|
| Workflow inputs can be megabytes (e.g., batch processing manifests, ML feature vectors) | PostgreSQL JSONB columns become bloated, slowing queries and increasing WAL size |
| Task outputs can be large (e.g., HTTP response bodies, file processing results) | Database backups grow disproportionately, recovery times increase |
| `SELECT * FROM workflow WHERE workflow_id = ?` pulls the full JSONB payload | Every workflow status check transfers unnecessary payload data over the wire |

**Solution:** Payloads exceeding a configurable threshold are stored in S3; the database only holds a reference path.

### Why S3?

| Requirement | Why S3 Fits |
|-------------|-------------|
| **Cheap, scalable storage** | Object storage costs a fraction of database storage per GB |
| **Universal API** | The S3 API is supported by AWS S3, MinIO, RustFS, Cloudflare R2, Google Cloud Storage (interop mode), and many others |
| **No schema changes** | Objects are opaque JSON blobs — no need to model payload structure |
| **Independent scaling** | Storage capacity scales independently from database capacity |
| **Access patterns** | Payloads are written once and read occasionally — a perfect fit for object storage |

### Why Not Store Everything in S3?

Small payloads (< 32 KB) are faster to read inline from PostgreSQL than making a separate S3 round-trip. The threshold-based approach gives the best of both worlds:
- Small payloads: single DB query (fast)
- Large payloads: DB query + S3 fetch (avoids DB bloat)

---

## How It Works

### Write Path

1. Engine prepares a workflow input or task output
2. If `byte_size > EXTERNAL_PAYLOAD_THRESHOLD` (default 32 KB):
   - Generate S3 key: `{entity_type}/{workflow_id}/{uuid}.json`
   - Upload JSON payload to S3
   - Store only the S3 key path in the PostgreSQL record (e.g., `external_input_payload_storage_path`)
3. If below threshold: store inline in PostgreSQL as usual

### Read Path

1. Engine loads workflow/task from PostgreSQL
2. If `external_input_payload_storage_path` is set:
   - Fetch payload from S3 using the stored key
   - Merge into the response object
3. If not set: return inline JSONB directly

### Delete Path

When a workflow is permanently deleted, associated S3 objects are cleaned up.

---

## S3 Key Format

```
{entity_type}/{workflow_id}/{uuid}.json
```

Examples:
```
workflow/550e8400-e29b-41d4-a716-446655440000/a1b2c3d4.json
task/550e8400-e29b-41d4-a716-446655440000/e5f6g7h8.json
```

**Why this structure?**
- Prefix by entity type allows S3 lifecycle policies (e.g., delete old task outputs after 90 days)
- Grouping by `workflow_id` makes cleanup easy when deleting a workflow
- UUID suffix prevents key collisions

---

## Configuration

| Environment Variable | Default | Description |
|---------------------|---------|-------------|
| `S3_ENDPOINT` | — | S3-compatible endpoint URL (e.g., `http://minio:9000`) |
| `S3_BUCKET` | — | Bucket name for payload storage |
| `S3_REGION` | — | AWS region (or `us-east-1` for MinIO/RustFS) |
| `S3_ACCESS_KEY` | — | Access key ID |
| `S3_SECRET_KEY` | — | Secret access key |
| `EXTERNAL_PAYLOAD_THRESHOLD` | `32768` | Byte threshold — payloads above this size are offloaded to S3 |

### Docker Compose

The default stack uses **RustFS**, a Rust-native S3-compatible server:

```yaml
rustfs:
  image: rustfs/rustfs:latest
  ports:
    - "9000:9000"   # S3 API
    - "9001:9001"   # Console UI
  environment:
    RUSTFS_ROOT_USER: minioadmin
    RUSTFS_ROOT_PASSWORD: minioadmin
```

**Why RustFS over MinIO?** RustFS is lightweight, fast to start, and written in Rust — making it a natural fit for the dev environment. In production, any S3-compatible backend works.

### Enabling the Feature

The feature is compile-time gated:

```bash
# Development (disabled by default)
cargo build

# Enable external storage
cargo build --features external-storage
```

When disabled, all payloads are stored inline in PostgreSQL regardless of size.

---

## Production Recommendations

- Use **AWS S3** or **MinIO** (clustered mode) for durability
- Set `EXTERNAL_PAYLOAD_THRESHOLD` based on your workload:
  - Low (8 KB): aggressive offloading, smaller DB
  - Default (32 KB): balanced
  - High (256 KB+): only offload very large payloads
- Enable S3 server-side encryption for sensitive workflow data
- Configure S3 lifecycle policies to auto-delete old objects
- Ensure the S3 bucket is created before starting the backend (the engine calls `ensure_bucket()` on startup)

---

## Code Reference

| File | Role |
|------|------|
| `store/s3.rs` | `ExternalPayloadStorage` — upload, download, delete, ensure_bucket |
| `config.rs` | S3 configuration parsing, `external_storage_enabled()` check |
| `engine/mod.rs` | Conditional S3 initialization |
| `engine/workflow_ops.rs` | Payload offloading during workflow start |
| `engine/task_ops.rs` | Payload offloading during task update |

# Testing Strategy — Rust Conductor

## Overview

Rust Conductor uses a **four-layer testing strategy** to catch bugs at every level: fast synchronous unit tests for core logic, Docker-based integration tests for real infrastructure, performance benchmarks to prevent regressions, and fuzz testing to surface unexpected input crashes.

| Layer              | Tool                   | What It Tests                              | Speed      |
|--------------------|------------------------|--------------------------------------------|------------|
| Unit tests         | `cargo test`           | Engine logic, models, template resolution  | ~2 s       |
| Integration tests  | testcontainers         | Full stack with real Postgres/Redis/Kafka  | ~30 s      |
| Benchmarks         | Criterion              | Hot-path throughput (serde, shard routing) | ~10 s      |
| Fuzz tests         | libFuzzer              | JSON deserialization crash resistance      | continuous |
| Mutation tests     | cargo-mutants          | Test suite kill-rate for core modules      | ~minutes   |

---

## Unit Tests

### Why In-Process Unit Tests?

Unit tests run inside the same binary (via `#[cfg(test)]` modules), giving them direct access to private functions without needing a lib.rs re-export. This is the fastest feedback loop for logic-only code that doesn't touch I/O.

| Alternative             | Why Not                                      |
|-------------------------|----------------------------------------------|
| Integration-only tests  | Too slow for TDD — 30 s per run with Docker  |
| Mock-based unit tests   | Engine has no trait-based store abstraction yet; mocking would require major refactoring |
| Doc tests               | Less useful for internal engine functions     |

### What's Covered

~110 unit tests in `src/engine/tests.rs` and `src/engine/shard.rs`:

- **Task status helpers** — `is_task_terminal`, `is_task_successful`, `is_task_failed`
- **Status display** — `TaskStatus::Display`, `WorkflowStatus::Display` round-trips
- **JSON navigation** — `navigate_json` for nested paths, arrays, missing fields
- **Template resolution** — `resolve_value`, `resolve_string_value`, `resolve_expression` covering `${workflow.input.*}`, `${task.output.*}`, numeric/bool coercion, nested objects
- **Loop condition evaluation** — `evaluate_loop_condition` with booleans, strings, numbers, `shouldContinue` precedence
- **Model serde** — `TaskStatus`, `WorkflowStatus`, `WorkflowDef`, `TaskDef` serialization round-trips, SCREAMING_SNAKE_CASE compat
- **Shard routing** — UUID-based shard assignment, distribution uniformity

### Running

```bash
cargo test                    # all unit tests
cargo test -- --nocapture     # with stdout
cargo test navigate_json      # filter by name
```

---

## Integration Tests

### Why testcontainers?

Integration tests spin up **real Docker containers** for PostgreSQL, Redis, and Apache Kafka—no mocks, no embedded stubs. This catches issues that unit tests can't: schema migration failures, wire protocol mismatches, connection lifecycle bugs.

| Alternative              | Why Not                                                  |
|--------------------------|----------------------------------------------------------|
| SQLite as Postgres stand-in | Missing JSONB, `TIMESTAMPTZ`, PostgreSQL-specific SQL |
| Embedded Redis (mini-redis) | Different behavior for HSET/Streams, no Lua           |
| In-memory Kafka (mock)   | rdkafka links to librdkafka C library—can't fake it      |
| Shared dev containers     | Non-deterministic state between runs, port conflicts     |

### Why testcontainers over docker-compose?

- **Per-test isolation** — each `#[tokio::test]` gets fresh containers
- **Random ports** — no collisions when running parallel CI jobs
- **Automatic cleanup** — containers stop when the test function returns
- **No external process** — no `docker-compose up` step in CI scripts

### Test Cases

| Test                              | What It Validates                                          |
|-----------------------------------|------------------------------------------------------------|
| `containers_start_and_are_reachable` | Postgres `SELECT 1`, Redis `PING` — baseline connectivity |
| `kafka_produce_consume_roundtrip` | rdkafka producer → consumer on `conductor.task.*` topic    |
| `postgres_schema_migration`       | Full DDL (4 tables), insert + query `workflow_def`         |
| `workflow_task_lifecycle_in_db`   | Insert workflow → insert task → poll (SCHEDULED→IN_PROGRESS) → complete → verify |
| `redis_task_routing_pattern`      | HSET/HGET/HDEL on `conductor:task_routing` key             |

### Container Setup

All tests share a `start_infra()` helper that returns containers + connection URLs:

```rust
async fn start_infra() -> (
    ContainerAsync<Postgres>,
    ContainerAsync<Redis>,
    ContainerAsync<Kafka>,
    String, // pg_url
    String, // redis_url
    String, // kafka_brokers
)
```

Containers are kept alive by returning them into the test function's scope — when the tuple drops, testcontainers tears them down.

### Running

```bash
# Requires Docker running
cargo test --test integration -- --test-threads=1
```

The `--test-threads=1` flag prevents parallel container startup, which can exhaust Docker resources on CI runners.

### Dependencies

```toml
[dev-dependencies]
testcontainers = "0.23"
testcontainers-modules = { version = "0.11", features = ["postgres", "redis", "kafka"] }
```

---

## Benchmarks

### Why Criterion?

Criterion provides **statistical benchmarking** with confidence intervals, outlier detection, and HTML reports. The built-in `#[bench]` harness is unstable and lacks these features.

| Alternative             | Why Not                                          |
|-------------------------|--------------------------------------------------|
| `#[bench]` (nightly)    | Unstable, no statistics, no HTML reports         |
| `divan`                 | Newer but less ecosystem adoption                |
| Manual `Instant::now`   | No statistical rigor, no regression detection    |

### Benchmarked Paths

| Benchmark                      | What It Measures                               |
|--------------------------------|------------------------------------------------|
| `template_resolve_serde_overhead` | serde round-trip cost (serialize + deserialize JSON template) |
| `json_navigate`                | `input.get("config").get("timeout")` lookup speed |
| `uuid_parse_and_hash`          | UUID parse + last-4-bytes extraction for shard routing |

These are the **hot paths** — template resolution runs for every task scheduled, and shard routing runs for every workflow/task operation.

### Running

```bash
cargo bench                        # all benchmarks
cargo bench -- json_navigate       # single benchmark
```

Reports are generated in `target/criterion/` with iteration-over-iteration comparison charts.

### Configuration

```toml
[[bench]]
name = "workflow_bench"
harness = false    # use Criterion's own main()
```

---

## Fuzz Testing

### Why libFuzzer?

libFuzzer is the de facto standard for Rust fuzz testing via `cargo-fuzz`. It generates millions of random byte sequences per second to find panics, OOM, and undefined behavior in deserialization code.

| Alternative     | Why Not                                          |
|-----------------|--------------------------------------------------|
| AFL             | Requires special build instrumentation, slower setup |
| proptest        | Property testing (used separately); not coverage-guided |
| Honggfuzz       | Less integration with Rust ecosystem             |

### Fuzz Targets

| Target               | What It Fuzzes                              |
|----------------------|---------------------------------------------|
| `fuzz_start_request` | `serde_json::from_slice::<Value>` on random bytes |
| `fuzz_task_def`      | `serde_json::from_slice::<Value>` on random bytes |
| `fuzz_task_update`   | `serde_json::from_slice::<Value>` on random bytes |
| `fuzz_workflow_def`  | JSON parse + typed `WorkflowDef` deserialization attempt |

The targets currently fuzz at the `serde_json::Value` level. When a `lib.rs` is added to the crate, they will upgrade to typed model deserialization (e.g., `serde_json::from_value::<WorkflowDef>`).

### Running

```bash
# Requires nightly toolchain
cargo +nightly fuzz run fuzz_start_request
cargo +nightly fuzz run fuzz_workflow_def -- -max_total_time=60
```

### Structure

```
backend/fuzz/
├── Cargo.toml              # separate workspace member
└── fuzz_targets/
    ├── fuzz_start_request.rs
    ├── fuzz_task_def.rs
    ├── fuzz_task_update.rs
    └── fuzz_workflow_def.rs
```

---

## Mutation Testing

### Why cargo-mutants?

Mutation testing measures **test suite quality** by injecting small code changes (mutants) and checking whether tests catch them. A mutant that survives means a gap in test coverage. `cargo-mutants` is the standard Rust tool for this.

| Alternative        | Why Not                                        |
|--------------------|------------------------------------------------|
| Manual review      | Doesn't scale, subjective                      |
| Coverage-only      | High line coverage ≠ meaningful assertions     |
| `mutagen`          | Abandoned / unmaintained                       |

### Configuration

```toml
# mutants.toml
timeout_multiplier = 2.0
minimum_test_timeout = 60

examine_globs = [
    "src/engine/*.rs",
    "src/models/*.rs",
    "src/store/*.rs",
]

exclude_globs = [
    "src/main.rs",    # entry point, not unit-testable
    "src/api/**",     # HTTP handlers tested via integration
    "src/grpc/**",    # gRPC handlers tested via integration
    "src/swagger.rs", # generated documentation
    "src/config.rs",  # environment parsing
]

exclude_re = [
    "^create_pool$",      # infrastructure setup
    "^run_migrations$",   # DDL execution
    "^configure$",        # Actix app builder
]
```

The scope focuses on `engine/`, `models/`, and `store/` — the modules with complex logic worth mutating. API layers are excluded because they're thin wrappers tested at the integration level.

### Running

```bash
cargo install cargo-mutants
cargo mutants --in-place -j 4
cargo mutants --in-place --output mutants.out   # save report
```

---

## Additional Test Dependencies

| Crate           | Purpose                                          |
|-----------------|--------------------------------------------------|
| `proptest`      | Property-based testing for model invariants       |
| `criterion`     | Benchmark harness for hot-path workflow helpers   |
| `testcontainers` | Integration-test containers for Postgres/Redis/Kafka |

---

## Code Reference

| Path                          | Purpose                                    |
|-------------------------------|--------------------------------------------|
| `src/engine/tests.rs`         | ~110 unit tests for engine logic           |
| `src/engine/shard.rs`         | Shard routing unit tests (`#[cfg(test)]`)  |
| `tests/integration.rs`        | 5 testcontainers integration tests         |
| `benches/workflow_bench.rs`   | Criterion benchmarks (3 bench functions)   |
| `fuzz/fuzz_targets/`          | 4 libFuzzer targets                        |
| `mutants.toml`                | cargo-mutants scope configuration          |

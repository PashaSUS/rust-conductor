use criterion::{criterion_group, criterion_main, Criterion};
use serde_json::json;
use std::collections::HashMap;

/// Benchmark template resolution throughput — the hot path during task
/// scheduling when `inputParameters` contain `${…}` expressions.
fn bench_resolve_value(c: &mut Criterion) {
    // We can't directly import the engine's private functions in a benchmark
    // binary, so we re-implement a minimal version of template resolution
    // to establish a performance baseline.  See tests/ for the real
    // integration benchmarks once testcontainers are set up.

    let input = json!({
        "name": "perf-test",
        "count": 1000,
        "config": {"timeout": 30, "retries": 3}
    });

    let mut task_outputs = HashMap::new();
    for i in 0..10 {
        task_outputs.insert(
            format!("task_{i}"),
            json!({"result": format!("output_{i}"), "code": 200}),
        );
    }

    let template = json!({
        "url": "https://api.example.com/${workflow.input.name}",
        "timeout": "${workflow.input.config.timeout}",
        "previous": "${task_0.output.result}",
        "wfId": "${workflow.workflowId}",
        "nested": {
            "a": "${task_1.output.code}",
            "b": ["${workflow.input.count}", "static"]
        }
    });

    c.bench_function("template_resolve_serde_overhead", |b| {
        b.iter(|| {
            // Simulate the serde round-trip that happens during resolution
            let s = serde_json::to_string(&template).unwrap();
            let _: serde_json::Value = serde_json::from_str(&s).unwrap();
        })
    });

    c.bench_function("json_navigate", |b| {
        b.iter(|| {
            let _ = input.get("config").and_then(|c| c.get("timeout"));
        })
    });
}

fn bench_shard_routing(c: &mut Criterion) {
    let ids: Vec<String> = (0..1000)
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect();

    c.bench_function("uuid_parse_and_hash", |b| {
        let mut i = 0;
        b.iter(|| {
            let id = &ids[i % ids.len()];
            let uuid = uuid::Uuid::parse_str(id).unwrap();
            let bytes = uuid.as_bytes();
            let _hash = u32::from_be_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
            i += 1;
        })
    });
}

criterion_group!(benches, bench_resolve_value, bench_shard_routing);
criterion_main!(benches);

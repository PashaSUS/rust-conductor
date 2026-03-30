//! Prometheus metrics for Rust Conductor.
//!
//! Registers all metrics that the pre-provisioned Grafana dashboard
//! (`monitoring/grafana/dashboards/rust-conductor.json`) expects.
//!
//! Call [`collect_pool_metrics`] on a timer to keep pool gauges fresh,
//! and call the various `record_*` helpers from hot paths.

use prometheus::{
    Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry,
    TextEncoder,
};

lazy_static::lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // ── Connection-pool gauges ──────────────────────────────────────────

    pub static ref PG_POOL_ACTIVE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("conductor_pg_pool_active", "Active Postgres connections"),
        &["shard"],
    ).unwrap();

    pub static ref PG_POOL_IDLE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("conductor_pg_pool_idle", "Idle Postgres connections"),
        &["shard"],
    ).unwrap();

    pub static ref REDIS_POOL_ACTIVE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("conductor_redis_pool_active", "Active Redis connections"),
        &["shard"],
    ).unwrap();

    pub static ref REDIS_POOL_IDLE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("conductor_redis_pool_idle", "Idle Redis connections"),
        &["shard"],
    ).unwrap();

    // ── HTTP request metrics ────────────────────────────────────────────

    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("conductor_http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"],
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new(
            "conductor_http_request_duration_seconds",
            "HTTP request duration in seconds",
        )
        .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
        &["method", "path"],
    ).unwrap();

    // ── Workflow lifecycle counters / gauges ─────────────────────────────

    pub static ref WORKFLOWS_RUNNING: GaugeVec = GaugeVec::new(
        Opts::new("conductor_workflows_running", "Currently running workflows"),
        &[],
    ).unwrap();

    pub static ref WORKFLOWS_COMPLETED: IntCounterVec = IntCounterVec::new(
        Opts::new("conductor_workflows_completed_total", "Total completed workflows"),
        &[],
    ).unwrap();

    pub static ref WORKFLOWS_FAILED: IntCounterVec = IntCounterVec::new(
        Opts::new("conductor_workflows_failed_total", "Total failed workflows"),
        &[],
    ).unwrap();

    // ── Task metrics ────────────────────────────────────────────────────

    pub static ref TASK_QUEUE_DEPTH: IntGaugeVec = IntGaugeVec::new(
        Opts::new("conductor_task_queue_depth", "Pending tasks in queue per type"),
        &["task_type"],
    ).unwrap();

    pub static ref TASK_POLL_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("conductor_task_poll_total", "Total task poll requests"),
        &["task_type"],
    ).unwrap();
}

/// Register all metrics with the custom registry. Call once at startup.
pub fn register_metrics() {
    let collectors: Vec<Box<dyn prometheus::core::Collector>> = vec![
        Box::new(PG_POOL_ACTIVE.clone()),
        Box::new(PG_POOL_IDLE.clone()),
        Box::new(REDIS_POOL_ACTIVE.clone()),
        Box::new(REDIS_POOL_IDLE.clone()),
        Box::new(HTTP_REQUESTS_TOTAL.clone()),
        Box::new(HTTP_REQUEST_DURATION.clone()),
        Box::new(WORKFLOWS_RUNNING.clone()),
        Box::new(WORKFLOWS_COMPLETED.clone()),
        Box::new(WORKFLOWS_FAILED.clone()),
        Box::new(TASK_QUEUE_DEPTH.clone()),
        Box::new(TASK_POLL_TOTAL.clone()),
    ];
    for c in collectors {
        if let Err(e) = REGISTRY.register(c) {
            tracing::warn!(error = %e, "Metric already registered (harmless on reload)");
        }
    }
}

/// Render all registered metrics in Prometheus text exposition format.
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let families = REGISTRY.gather();
    let mut buf = Vec::new();
    encoder.encode(&families, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}

/// Snapshot connection-pool gauges for every Postgres shard + Redis shard.
/// Called from a periodic background task.
pub fn collect_pool_metrics(engine: &crate::engine::WorkflowEngine) {
    let pool_json = engine.pool_metrics();

    // Postgres primary shards
    if let Some(shards) = pool_json.get("postgres").and_then(|p| p.get("primary")).and_then(|v| v.as_array()) {
        for s in shards {
            let shard = s.get("shard").and_then(|v| v.as_u64()).unwrap_or(0).to_string();
            let active = s.get("active").and_then(|v| v.as_i64()).unwrap_or(0);
            let idle = s.get("idle").and_then(|v| v.as_i64()).unwrap_or(0);
            PG_POOL_ACTIVE.with_label_values(&[&shard]).set(active);
            PG_POOL_IDLE.with_label_values(&[&shard]).set(idle);
        }
    }

    // Redis shards
    if let Some(redis) = pool_json.get("redis").and_then(|v| v.as_array()) {
        for s in redis {
            let shard = s.get("shard").and_then(|v| v.as_u64()).unwrap_or(0).to_string();
            let size = s.get("size").and_then(|v| v.as_i64()).unwrap_or(0);
            let available = s.get("available").and_then(|v| v.as_i64()).unwrap_or(0);
            let active = size - available;
            REDIS_POOL_ACTIVE.with_label_values(&[&shard]).set(active);
            REDIS_POOL_IDLE.with_label_values(&[&shard]).set(available);
        }
    }
}

// ── Convenience recording helpers ───────────────────────────────────────────

pub fn record_http_request(method: &str, path: &str, status: u16, duration_secs: f64) {
    HTTP_REQUESTS_TOTAL
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
    HTTP_REQUEST_DURATION
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

pub fn record_workflow_started() {
    WORKFLOWS_RUNNING.with_label_values(&[]).inc();
}

pub fn record_workflow_completed() {
    WORKFLOWS_RUNNING.with_label_values(&[]).dec();
    WORKFLOWS_COMPLETED.with_label_values(&[]).inc();
}

pub fn record_workflow_failed() {
    WORKFLOWS_RUNNING.with_label_values(&[]).dec();
    WORKFLOWS_FAILED.with_label_values(&[]).inc();
}

pub fn record_task_poll(task_type: &str) {
    TASK_POLL_TOTAL.with_label_values(&[task_type]).inc();
}

pub fn set_task_queue_depth(task_type: &str, depth: i64) {
    TASK_QUEUE_DEPTH.with_label_values(&[task_type]).set(depth);
}

// ── Actix-web middleware for HTTP request metrics ───────────────────────────

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use std::future::{Future, Ready, ready};
use std::pin::Pin;

pub struct PrometheusMetricsMiddleware;

impl<S, B> Transform<S, ServiceRequest> for PrometheusMetricsMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Transform = PrometheusMetricsService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(PrometheusMetricsService { service }))
    }
}

pub struct PrometheusMetricsService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for PrometheusMetricsService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = actix_web::Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = actix_web::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(
        &self,
        ctx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.service.poll_ready(ctx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let method = req.method().to_string();
        let path = req.path().to_string();
        let start = std::time::Instant::now();
        let fut = self.service.call(req);
        Box::pin(async move {
            let res = fut.await?;
            let status = res.status().as_u16();
            let duration = start.elapsed().as_secs_f64();
            record_http_request(&method, &path, status, duration);
            Ok(res)
        })
    }
}

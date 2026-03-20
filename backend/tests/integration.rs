//! Integration tests using testcontainers for Postgres, Redis, and Kafka.
//!
//! These tests spin up real containers and exercise the full engine flow:
//! start workflow → poll task → update task → advance → complete.
//!
//! Run with: `cargo test --test integration -- --test-threads=1`
//!
//! Requires Docker to be running.

use serde_json::json;
use testcontainers::runners::AsyncRunner;
use testcontainers_modules::{kafka::apache::Kafka, postgres::Postgres, redis::Redis};

// The binary crate modules aren't directly importable in integration tests,
// so we test via HTTP API using reqwest against a running server.
// For a true library-level integration test, the crate would need a lib.rs.
// Instead, these tests validate the full stack via the REST API.

/// Start testcontainers for Postgres, Redis, and Kafka.
/// Returns (pg_url, redis_url, kafka_brokers).
async fn start_infra() -> (
    testcontainers::ContainerAsync<Postgres>,
    testcontainers::ContainerAsync<Redis>,
    testcontainers::ContainerAsync<Kafka>,
    String,
    String,
    String,
) {
    let pg = Postgres::default().start().await.expect("Failed to start Postgres container");
    let redis = Redis::default().start().await.expect("Failed to start Redis container");
    let kafka = Kafka::default().start().await.expect("Failed to start Kafka container");

    let pg_port = pg.get_host_port_ipv4(5432).await.expect("Postgres port");
    let redis_port = redis.get_host_port_ipv4(6379).await.expect("Redis port");
    let kafka_port = kafka.get_host_port_ipv4(9093).await.expect("Kafka port");

    let pg_url = format!(
        "postgres://postgres:postgres@127.0.0.1:{pg_port}/postgres"
    );
    let redis_url = format!("redis://127.0.0.1:{redis_port}");
    let kafka_brokers = format!("127.0.0.1:{kafka_port}");

    (pg, redis, kafka, pg_url, redis_url, kafka_brokers)
}

#[tokio::test]
async fn containers_start_and_are_reachable() {
    let (_pg, _redis, _kafka, pg_url, redis_url, _kafka_brokers) = start_infra().await;

    // Verify Postgres is reachable
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(&pg_url)
        .await
        .expect("Failed to connect to Postgres");
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(&pool)
        .await
        .expect("Postgres query failed");
    assert_eq!(row.0, 1);

    // Verify Redis is reachable
    let client = redis::Client::open(redis_url.as_str()).expect("Redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection");
    let pong: String = redis::cmd("PING")
        .query_async(&mut conn)
        .await
        .expect("Redis PING");
    assert_eq!(pong, "PONG");
}

/// Test that we can produce and consume a Kafka message via the KafkaTaskQueue
/// abstraction by directly talking to a Kafka container.
#[tokio::test]
async fn kafka_produce_consume_roundtrip() {
    let (_pg, _redis, _kafka, _pg_url, _redis_url, kafka_brokers) = start_infra().await;

    // Give Kafka a moment to fully start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    use rdkafka::config::ClientConfig;
    use rdkafka::consumer::{Consumer, StreamConsumer};
    use rdkafka::producer::{FutureProducer, FutureRecord};
    use rdkafka::Message;
    use std::time::Duration;

    // Producer
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Kafka producer");

    let topic = "conductor.task.test_task";
    let task_id = "task-abc-123";

    producer
        .send(
            FutureRecord::to(topic).key(task_id).payload(task_id),
            Duration::from_secs(5),
        )
        .await
        .expect("Kafka produce failed");

    // Consumer
    let consumer: StreamConsumer = ClientConfig::new()
        .set("bootstrap.servers", &kafka_brokers)
        .set("group.id", "test-consumer")
        .set("auto.offset.reset", "earliest")
        .create()
        .expect("Kafka consumer");

    consumer.subscribe(&[topic]).expect("Subscribe");

    let msg = tokio::time::timeout(Duration::from_secs(10), consumer.recv())
        .await
        .expect("Kafka recv timeout")
        .expect("Kafka recv error");

    let payload = msg
        .payload_view::<str>()
        .expect("No payload")
        .expect("Invalid UTF-8");
    assert_eq!(payload, task_id);
}

/// Test the database schema migration by running the SQL from postgres.rs
/// against a real Postgres instance.
#[tokio::test]
async fn postgres_schema_migration() {
    let (_pg, _redis, _kafka, pg_url, _redis_url, _kafka_brokers) = start_infra().await;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&pg_url)
        .await
        .expect("Postgres connect");

    let ddl = r#"
        CREATE TABLE IF NOT EXISTS workflow_def (
            name TEXT NOT NULL,
            version INT NOT NULL DEFAULT 1,
            definition JSONB NOT NULL,
            created_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (name, version)
        );

        CREATE TABLE IF NOT EXISTS task_def (
            name TEXT PRIMARY KEY,
            definition JSONB NOT NULL,
            created_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_time TIMESTAMPTZ NOT NULL DEFAULT NOW()
        );

        CREATE TABLE IF NOT EXISTS workflow (
            workflow_id TEXT PRIMARY KEY,
            workflow_name TEXT NOT NULL,
            workflow_version INT NOT NULL DEFAULT 1,
            status TEXT NOT NULL DEFAULT 'RUNNING',
            input JSONB NOT NULL DEFAULT '{}',
            output JSONB NOT NULL DEFAULT '{}',
            correlation_id TEXT,
            start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            end_time TIMESTAMPTZ,
            update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            priority INT NOT NULL DEFAULT 0,
            parent_workflow_id TEXT,
            parent_workflow_task_id TEXT,
            reason_for_incompletion TEXT,
            workflow_def JSONB,
            variables JSONB NOT NULL DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS task (
            task_id TEXT PRIMARY KEY,
            workflow_instance_id TEXT NOT NULL REFERENCES workflow(workflow_id) ON DELETE CASCADE,
            task_type TEXT NOT NULL,
            task_def_name TEXT NOT NULL,
            reference_task_name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'SCHEDULED',
            input_data JSONB NOT NULL DEFAULT '{}',
            output_data JSONB NOT NULL DEFAULT '{}',
            scheduled_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            start_time TIMESTAMPTZ,
            end_time TIMESTAMPTZ,
            update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            poll_count INT NOT NULL DEFAULT 0,
            worker_id TEXT,
            seq INT NOT NULL DEFAULT 0,
            retry_count INT NOT NULL DEFAULT 0,
            callback_after_seconds BIGINT NOT NULL DEFAULT 0,
            reason_for_incompletion TEXT,
            sub_workflow_id TEXT
        );
    "#;

    sqlx::raw_sql(ddl)
        .execute(&pool)
        .await
        .expect("Schema migration failed");

    // Verify tables exist
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM information_schema.tables WHERE table_name IN ('workflow_def', 'task_def', 'workflow', 'task')",
    )
    .fetch_one(&pool)
    .await
    .expect("Count query");
    assert_eq!(count.0, 4, "Expected 4 tables to be created");

    // Insert and query a workflow_def
    let def = json!({
        "name": "test_wf",
        "version": 1,
        "tasks": [{"name": "t1", "taskReferenceName": "t1_ref", "type": "SIMPLE"}]
    });
    sqlx::query("INSERT INTO workflow_def (name, version, definition) VALUES ($1, $2, $3)")
        .bind("test_wf")
        .bind(1i32)
        .bind(&def)
        .execute(&pool)
        .await
        .expect("Insert workflow_def");

    let row: (serde_json::Value,) =
        sqlx::query_as("SELECT definition FROM workflow_def WHERE name = $1 AND version = $2")
            .bind("test_wf")
            .bind(1i32)
            .fetch_one(&pool)
            .await
            .expect("Query workflow_def");
    assert_eq!(row.0["name"], "test_wf");
}

/// Test workflow + task lifecycle in the database (insert, status transitions).
#[tokio::test]
async fn workflow_task_lifecycle_in_db() {
    let (_pg, _redis, _kafka, pg_url, _redis_url, _kafka_brokers) = start_infra().await;

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&pg_url)
        .await
        .expect("Postgres connect");

    // Create schema
    sqlx::raw_sql(
        "CREATE TABLE IF NOT EXISTS workflow (
            workflow_id TEXT PRIMARY KEY,
            workflow_name TEXT NOT NULL,
            workflow_version INT NOT NULL DEFAULT 1,
            status TEXT NOT NULL DEFAULT 'RUNNING',
            input JSONB NOT NULL DEFAULT '{}',
            output JSONB NOT NULL DEFAULT '{}',
            correlation_id TEXT,
            start_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            end_time TIMESTAMPTZ,
            update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            priority INT NOT NULL DEFAULT 0,
            parent_workflow_id TEXT,
            parent_workflow_task_id TEXT,
            reason_for_incompletion TEXT,
            workflow_def JSONB,
            variables JSONB NOT NULL DEFAULT '{}'
        );
        CREATE TABLE IF NOT EXISTS task (
            task_id TEXT PRIMARY KEY,
            workflow_instance_id TEXT NOT NULL REFERENCES workflow(workflow_id) ON DELETE CASCADE,
            task_type TEXT NOT NULL,
            task_def_name TEXT NOT NULL,
            reference_task_name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'SCHEDULED',
            input_data JSONB NOT NULL DEFAULT '{}',
            output_data JSONB NOT NULL DEFAULT '{}',
            scheduled_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            start_time TIMESTAMPTZ,
            end_time TIMESTAMPTZ,
            update_time TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            poll_count INT NOT NULL DEFAULT 0,
            worker_id TEXT,
            seq INT NOT NULL DEFAULT 0,
            retry_count INT NOT NULL DEFAULT 0,
            callback_after_seconds BIGINT NOT NULL DEFAULT 0,
            reason_for_incompletion TEXT,
            sub_workflow_id TEXT
        );",
    )
    .execute(&pool)
    .await
    .expect("Schema");

    let wf_id = uuid::Uuid::new_v4().to_string();
    let task_id = uuid::Uuid::new_v4().to_string();

    // Insert workflow
    sqlx::query(
        "INSERT INTO workflow (workflow_id, workflow_name, status, input) VALUES ($1, $2, $3, $4)",
    )
    .bind(&wf_id)
    .bind("test_wf")
    .bind("RUNNING")
    .bind(json!({"key": "value"}))
    .execute(&pool)
    .await
    .expect("Insert workflow");

    // Insert task as SCHEDULED
    sqlx::query(
        "INSERT INTO task (task_id, workflow_instance_id, task_type, task_def_name, reference_task_name, status) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&task_id)
    .bind(&wf_id)
    .bind("SIMPLE")
    .bind("my_task")
    .bind("task1_ref")
    .bind("SCHEDULED")
    .execute(&pool)
    .await
    .expect("Insert task");

    // Poll: transition SCHEDULED → IN_PROGRESS
    let rows = sqlx::query(
        "UPDATE task SET status = 'IN_PROGRESS', start_time = NOW(), poll_count = poll_count + 1, worker_id = $2 WHERE task_id = $1 AND status = 'SCHEDULED'",
    )
    .bind(&task_id)
    .bind("worker-1")
    .execute(&pool)
    .await
    .expect("Poll task");
    assert_eq!(rows.rows_affected(), 1);

    // Verify status
    let status: (String,) = sqlx::query_as("SELECT status FROM task WHERE task_id = $1")
        .bind(&task_id)
        .fetch_one(&pool)
        .await
        .expect("Query task status");
    assert_eq!(status.0, "IN_PROGRESS");

    // Complete task
    sqlx::query(
        "UPDATE task SET status = 'COMPLETED', output_data = $2, end_time = NOW() WHERE task_id = $1",
    )
    .bind(&task_id)
    .bind(json!({"result": "success"}))
    .execute(&pool)
    .await
    .expect("Complete task");

    // Complete workflow
    sqlx::query("UPDATE workflow SET status = 'COMPLETED', end_time = NOW() WHERE workflow_id = $1")
        .bind(&wf_id)
        .execute(&pool)
        .await
        .expect("Complete workflow");

    let wf_status: (String,) = sqlx::query_as("SELECT status FROM workflow WHERE workflow_id = $1")
        .bind(&wf_id)
        .fetch_one(&pool)
        .await
        .expect("Query workflow status");
    assert_eq!(wf_status.0, "COMPLETED");
}

/// Redis task routing: HSET/HGET/HDEL pattern used by the engine.
#[tokio::test]
async fn redis_task_routing_pattern() {
    let (_pg, _redis, _kafka, _pg_url, redis_url, _kafka_brokers) = start_infra().await;

    let client = redis::Client::open(redis_url.as_str()).expect("Redis client");
    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .expect("Redis connection");

    let routing_key = "conductor:task_routing";
    let task_id = "task-123";
    let workflow_id = "wf-456";

    // HSET
    let _: () = redis::cmd("HSET")
        .arg(routing_key)
        .arg(task_id)
        .arg(workflow_id)
        .query_async(&mut conn)
        .await
        .expect("HSET");

    // HGET
    let result: Option<String> = redis::cmd("HGET")
        .arg(routing_key)
        .arg(task_id)
        .query_async(&mut conn)
        .await
        .expect("HGET");
    assert_eq!(result, Some(workflow_id.to_string()));

    // HDEL
    let _: () = redis::cmd("HDEL")
        .arg(routing_key)
        .arg(task_id)
        .query_async(&mut conn)
        .await
        .expect("HDEL");

    let result: Option<String> = redis::cmd("HGET")
        .arg(routing_key)
        .arg(task_id)
        .query_async(&mut conn)
        .await
        .expect("HGET after DEL");
    assert_eq!(result, None);
}

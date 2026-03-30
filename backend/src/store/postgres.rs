use sqlx::ConnectOptions;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::time::Duration;

pub type DbPool = PgPool;

#[allow(dead_code)]
pub async fn create_pool(database_url: &str) -> DbPool {
    create_pool_with_options(database_url, 500).await
}

pub async fn create_pool_with_options(database_url: &str, slow_query_threshold_ms: u64) -> DbPool {
    let connect_opts: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()
        .expect("Invalid database URL")
        .log_slow_statements(
            tracing::log::LevelFilter::Warn,
            Duration::from_millis(slow_query_threshold_ms),
        );

    PgPoolOptions::new()
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(connect_opts)
        .await
        .expect("Failed to create Postgres connection pool")
}

/// Create a read-replica pool with lower connection limits.
pub async fn create_replica_pool(database_url: &str, slow_query_threshold_ms: u64) -> DbPool {
    let connect_opts: PgConnectOptions = database_url
        .parse::<PgConnectOptions>()
        .expect("Invalid replica database URL")
        .log_slow_statements(
            tracing::log::LevelFilter::Warn,
            Duration::from_millis(slow_query_threshold_ms),
        );

    PgPoolOptions::new()
        .max_connections(50)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect_with(connect_opts)
        .await
        .expect("Failed to create Postgres replica connection pool")
}

/// Pool metrics for observability.
pub struct PoolMetrics {
    pub size: u32,
    pub num_idle: u32,
    pub active: u32,
}

pub fn pool_metrics(pool: &DbPool) -> PoolMetrics {
    let size = pool.size();
    let idle = pool.num_idle() as u32;
    PoolMetrics {
        size,
        num_idle: idle,
        active: size.saturating_sub(idle),
    }
}

pub async fn run_migrations(pool: &DbPool) {
    // Use a transaction-level advisory lock so that even if multiple
    // processes connect to the same shard concurrently, only one runs
    // DDL at a time.  This prevents the pg_type_typname_nsp_index race
    // that occurs when two sessions both attempt CREATE TABLE IF NOT
    // EXISTS for the same table simultaneously.
    let mut tx = pool
        .begin()
        .await
        .expect("Failed to start migration transaction");

    // Lock id 819_2023 is arbitrary but must be the same across all callers.
    // Use try-lock to avoid blocking forever if a previous migration was killed.
    let max_attempts = 30;
    for attempt in 1..=max_attempts {
        let acquired: (bool,) = sqlx::query_as("SELECT pg_try_advisory_xact_lock(8192023)")
            .fetch_one(&mut *tx)
            .await
            .expect("Failed to try migration advisory lock");
        if acquired.0 {
            break;
        }
        if attempt == max_attempts {
            panic!(
                "Could not acquire migration advisory lock after {max_attempts} attempts. \
                 Another migration may be running — check pg_stat_activity for \
                 lingering connections holding advisory lock 8192023."
            );
        }
        tracing::warn!(
            attempt,
            "Migration lock held by another process, retrying in 1s…"
        );
        // Drop the transaction so we don't hold a connection while waiting
        drop(tx);
        tokio::time::sleep(Duration::from_secs(1)).await;
        tx = pool
            .begin()
            .await
            .expect("Failed to restart migration transaction");
    }

    // Each statement must be executed separately — PG doesn't allow multiple
    // commands in a single prepared statement.
    let statements: &[&str] = &[
        r#"CREATE TABLE IF NOT EXISTS workflow_def (
            name        TEXT NOT NULL,
            version     INT NOT NULL DEFAULT 1,
            definition  JSONB NOT NULL,
            created_on  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_on  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            PRIMARY KEY (name, version)
        )"#,
        r#"CREATE TABLE IF NOT EXISTS task_def (
            name        TEXT PRIMARY KEY,
            definition  JSONB NOT NULL,
            created_on  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_on  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        r#"CREATE TABLE IF NOT EXISTS workflow (
            workflow_id     TEXT PRIMARY KEY,
            workflow_name   TEXT NOT NULL,
            workflow_version INT NOT NULL DEFAULT 1,
            status          TEXT NOT NULL DEFAULT 'RUNNING',
            input           JSONB NOT NULL DEFAULT '{}',
            output          JSONB NOT NULL DEFAULT '{}',
            correlation_id  TEXT,
            start_time      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            end_time        TIMESTAMPTZ,
            update_time     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            created_by      TEXT,
            priority        INT NOT NULL DEFAULT 0,
            variables       JSONB NOT NULL DEFAULT '{}',
            reason_for_incompletion TEXT,
            workflow_def    JSONB
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_workflow_status ON workflow(status)",
        "CREATE INDEX IF NOT EXISTS idx_workflow_name ON workflow(workflow_name)",
        r#"CREATE TABLE IF NOT EXISTS task (
            task_id                 TEXT PRIMARY KEY,
            workflow_instance_id    TEXT NOT NULL REFERENCES workflow(workflow_id) ON DELETE CASCADE,
            task_type               TEXT NOT NULL,
            task_def_name           TEXT NOT NULL,
            reference_task_name     TEXT NOT NULL,
            status                  TEXT NOT NULL DEFAULT 'SCHEDULED',
            input_data              JSONB NOT NULL DEFAULT '{}',
            output_data             JSONB NOT NULL DEFAULT '{}',
            scheduled_time          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            start_time              TIMESTAMPTZ,
            end_time                TIMESTAMPTZ,
            update_time             TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            poll_count              INT NOT NULL DEFAULT 0,
            worker_id               TEXT,
            seq                     INT NOT NULL DEFAULT 0,
            retry_count             INT NOT NULL DEFAULT 0,
            callback_after_seconds  BIGINT NOT NULL DEFAULT 0,
            reason_for_incompletion TEXT
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_task_workflow ON task(workflow_instance_id)",
        "CREATE INDEX IF NOT EXISTS idx_task_type_status ON task(task_type, status)",
        r#"CREATE TABLE IF NOT EXISTS task_log (
            id              BIGSERIAL PRIMARY KEY,
            task_id         TEXT NOT NULL,
            log_message     TEXT NOT NULL,
            created_time    TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_task_log_task ON task_log(task_id)",
        r#"CREATE TABLE IF NOT EXISTS event_handler (
            name        TEXT PRIMARY KEY,
            event       TEXT NOT NULL,
            definition  JSONB NOT NULL,
            created_on  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_on  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_event_handler_event ON event_handler(event)",
        r#"CREATE TABLE IF NOT EXISTS config (
            key     TEXT PRIMARY KEY,
            value   JSONB NOT NULL DEFAULT '{}'
        )"#,
        // Schema evolution — add columns needed for system-task support
        "ALTER TABLE task ADD COLUMN IF NOT EXISTS sub_workflow_id TEXT",
        "ALTER TABLE task ADD COLUMN IF NOT EXISTS parent_task_id TEXT",
        "ALTER TABLE workflow ADD COLUMN IF NOT EXISTS parent_workflow_id TEXT",
        "ALTER TABLE workflow ADD COLUMN IF NOT EXISTS parent_workflow_task_id TEXT",
        // Prevent duplicate tasks per workflow by reference name (only for active tasks)
        "CREATE UNIQUE INDEX IF NOT EXISTS idx_task_unique_ref \
         ON task (workflow_instance_id, reference_task_name) \
         WHERE status NOT IN ('FAILED', 'TIMED_OUT', 'CANCELED')",
        // Sweeper: find stale IN_PROGRESS worker tasks efficiently
        "CREATE INDEX IF NOT EXISTS idx_task_status_start \
         ON task (status, start_time) WHERE status IN ('IN_PROGRESS', 'SCHEDULED')",
        // Parent task lookups for sub-workflow propagation
        "CREATE INDEX IF NOT EXISTS idx_task_sub_workflow \
         ON task (sub_workflow_id) WHERE sub_workflow_id IS NOT NULL",
        // Workflow parent relationship lookups
        "CREATE INDEX IF NOT EXISTS idx_workflow_parent \
         ON workflow (parent_workflow_id) WHERE parent_workflow_id IS NOT NULL",
        // Task reference name lookups during advance_workflow
        "CREATE INDEX IF NOT EXISTS idx_task_wf_ref \
         ON task (workflow_instance_id, reference_task_name, seq DESC)",
        // CRON scheduled workflows
        r#"CREATE TABLE IF NOT EXISTS scheduled_workflow (
            schedule_id     TEXT PRIMARY KEY,
            name            TEXT NOT NULL,
            cron_expression TEXT NOT NULL,
            timezone        TEXT NOT NULL DEFAULT 'UTC',
            workflow_name   TEXT NOT NULL,
            workflow_version INT NOT NULL DEFAULT 1,
            workflow_input  JSONB NOT NULL DEFAULT '{}',
            enabled         BOOLEAN NOT NULL DEFAULT TRUE,
            last_run_at     TIMESTAMPTZ,
            next_run_at     TIMESTAMPTZ,
            created_on      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_on      TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_scheduled_workflow_next \
         ON scheduled_workflow (next_run_at) WHERE enabled = TRUE",
        // Add last_error column for schedule error visibility
        "ALTER TABLE scheduled_workflow ADD COLUMN IF NOT EXISTS last_error TEXT",
        // Workflow tags column
        "ALTER TABLE workflow ADD COLUMN IF NOT EXISTS tags JSONB NOT NULL DEFAULT '[]'",
        "CREATE INDEX IF NOT EXISTS idx_workflow_tags ON workflow USING GIN (tags)",
        // Workflow SLA deadline tracking
        "ALTER TABLE workflow ADD COLUMN IF NOT EXISTS sla_deadline TIMESTAMPTZ",
        "CREATE INDEX IF NOT EXISTS idx_workflow_sla \
         ON workflow (sla_deadline) WHERE status = 'RUNNING' AND sla_deadline IS NOT NULL",
        // Task priority for priority-based polling
        "ALTER TABLE task ADD COLUMN IF NOT EXISTS priority INT NOT NULL DEFAULT 0",
        "CREATE INDEX IF NOT EXISTS idx_task_priority \
         ON task (task_type, status, priority DESC) WHERE status = 'SCHEDULED'",
        // Task env_vars for secrets injection
        "ALTER TABLE task ADD COLUMN IF NOT EXISTS env_vars JSONB",
        // Workflow templates
        r#"CREATE TABLE IF NOT EXISTS workflow_template (
            name        TEXT PRIMARY KEY,
            definition  JSONB NOT NULL,
            created_on  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_on  TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )"#,
        // 105. Workflow checkpointing
        r#"CREATE TABLE IF NOT EXISTS workflow_checkpoint (
            checkpoint_id   TEXT PRIMARY KEY,
            workflow_id     TEXT NOT NULL,
            created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            workflow_snapshot JSONB NOT NULL,
            tasks_snapshot  JSONB NOT NULL,
            variables_snapshot JSONB NOT NULL,
            label           TEXT
        )"#,
        "CREATE INDEX IF NOT EXISTS idx_checkpoint_workflow \
         ON workflow_checkpoint (workflow_id, created_at DESC)",
    ];

    for stmt in statements {
        sqlx::query(stmt)
            .execute(&mut *tx)
            .await
            .expect("Failed to run database migration");
    }

    tx.commit()
        .await
        .expect("Failed to commit migration transaction");

    tracing::info!("Database migrations completed");
}

use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;

pub type DbPool = PgPool;

pub async fn create_pool(database_url: &str) -> DbPool {
    PgPoolOptions::new()
        .max_connections(100)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(300))
        .max_lifetime(Duration::from_secs(1800))
        .connect(database_url)
        .await
        .expect("Failed to create Postgres connection pool")
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
    sqlx::query("SELECT pg_advisory_xact_lock(8192023)")
        .execute(&mut *tx)
        .await
        .expect("Failed to acquire migration advisory lock");

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

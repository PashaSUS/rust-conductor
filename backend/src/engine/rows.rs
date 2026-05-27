use chrono::Utc;
use serde_json::Value;

use crate::models::*;

#[derive(sqlx::FromRow)]
pub(crate) struct OrphanedTaskRow {
    pub task_id: String,
    pub task_def_name: String,
    pub workflow_instance_id: String,
    pub domain: Option<String>,
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct WorkflowRow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub workflow_version: i32,
    pub status: String,
    pub input: Value,
    pub output: Value,
    pub correlation_id: Option<String>,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: Option<chrono::DateTime<Utc>>,
    pub update_time: chrono::DateTime<Utc>,
    pub created_by: Option<String>,
    pub priority: i32,
    pub variables: Value,
    pub reason_for_incompletion: Option<String>,
    pub workflow_def: Option<Value>,
    pub parent_workflow_id: Option<String>,
    pub parent_workflow_task_id: Option<String>,
    pub tags: Value,
    pub sla_deadline: Option<chrono::DateTime<Utc>>,
    pub task_to_domain: Value,
}

#[derive(sqlx::FromRow)]
pub(crate) struct TaskRow {
    pub task_id: String,
    pub workflow_instance_id: String,
    pub task_type: String,
    pub task_def_name: String,
    pub reference_task_name: String,
    pub status: String,
    pub input_data: Value,
    pub output_data: Value,
    pub scheduled_time: chrono::DateTime<Utc>,
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub end_time: Option<chrono::DateTime<Utc>>,
    pub update_time: chrono::DateTime<Utc>,
    pub poll_count: i32,
    pub worker_id: Option<String>,
    pub seq: i32,
    pub retry_count: i32,
    pub callback_after_seconds: i64,
    pub reason_for_incompletion: Option<String>,
    pub sub_workflow_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub priority: i32,
    pub env_vars: Option<Value>,
    pub domain: Option<String>,
}

impl From<TaskRow> for TaskResult {
    fn from(r: TaskRow) -> Self {
        TaskResult {
            task_id: r.task_id.clone(),
            workflow_instance_id: r.workflow_instance_id,
            task_type: r.task_type,
            task_def_name: r.task_def_name,
            reference_task_name: r.reference_task_name,
            status: serde_json::from_value(Value::String(r.status))
                .unwrap_or(TaskStatus::Scheduled),
            input_data: r.input_data,
            output_data: r.output_data,
            reason_for_incompletion: r.reason_for_incompletion,
            scheduled_time: Some(r.scheduled_time.timestamp_millis()),
            start_time: r.start_time.map(|t| t.timestamp_millis()),
            end_time: r.end_time.map(|t| t.timestamp_millis()),
            update_time: Some(r.update_time.timestamp_millis()),
            poll_count: r.poll_count,
            worker_id: r.worker_id,
            seq: r.seq,
            retry_count: r.retry_count,
            callback_after_seconds: r.callback_after_seconds,
            logs: vec![],
            external_input_payload_storage_path: None,
            external_output_payload_storage_path: None,
            correlation_id: None,
            start_delay_in_seconds: None,
            retried_task_id: None,
            retried: false,
            executed: false,
            callback_from_worker: true,
            response_timeout_seconds: None,
            workflow_type: None,
            domain: r.domain,
            rate_limit_per_frequency: None,
            rate_limit_frequency_in_seconds: None,
            workflow_priority: None,
            execution_name_space: None,
            isolation_group_id: None,
            iteration: None,
            sub_workflow_id: r.sub_workflow_id,
            subworkflow_changed: false,
            first_start_time: None,
            parent_task_id: r.parent_task_id,
            loop_over_task: false,
            queue_wait_time: None,
            workflow_task: None,
            task_definition: None,
            last_heartbeat_time: None,
        }
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
pub(crate) struct TaskSummaryRow {
    pub task_id: String,
    pub workflow_instance_id: String,
    pub task_type: String,
    pub task_def_name: String,
    pub reference_task_name: String,
    pub status: String,
    pub scheduled_time: chrono::DateTime<Utc>,
    pub start_time: Option<chrono::DateTime<Utc>>,
    pub end_time: Option<chrono::DateTime<Utc>>,
    pub update_time: chrono::DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct TaskLogRow {
    pub log_message: String,
    pub task_id: String,
    pub created_time: chrono::DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub(crate) struct ConfigRow {
    pub key: String,
    pub value: Value,
}

#[derive(sqlx::FromRow)]
pub(crate) struct WorkflowSummaryRow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub workflow_version: i32,
    pub status: String,
    pub start_time: chrono::DateTime<Utc>,
    pub end_time: Option<chrono::DateTime<Utc>>,
    pub input: Option<String>,
    pub output: Option<String>,
    pub correlation_id: Option<String>,
    pub priority: i32,
    pub parent_workflow_id: Option<String>,
}

impl From<WorkflowSummaryRow> for WorkflowSummary {
    fn from(r: WorkflowSummaryRow) -> Self {
        WorkflowSummary {
            workflow_id: r.workflow_id,
            workflow_type: r.workflow_name,
            version: r.workflow_version,
            status: serde_json::from_value(Value::String(r.status))
                .unwrap_or(WorkflowStatus::Running),
            start_time: Some(r.start_time.timestamp_millis().to_string()),
            end_time: r.end_time.map(|t| t.timestamp_millis().to_string()),
            input: r.input,
            output: r.output,
            correlation_id: r.correlation_id,
            priority: r.priority,
            parent_workflow_id: r.parent_workflow_id,
            update_time: None,
            reason_for_incompletion: None,
            execution_time: None,
            event: None,
            failed_reference_task_names: None,
            external_input_payload_storage_path: None,
            external_output_payload_storage_path: None,
            failed_task_names: vec![],
            created_by: None,
            task_to_domain: None,
            idempotency_key: None,
            output_size: None,
            input_size: None,
        }
    }
}

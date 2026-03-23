use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::common::SchemaDef;

//  Defaults 

fn default_retry_count() -> i32 { 3 }
fn default_retry_delay() -> i32 { 60 }
fn default_timeout() -> i64 { 3600 }
fn default_response_timeout() -> i64 { 600 }
fn default_backoff() -> i32 { 1 }

//  Task Definition 

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_retry_count")]
    pub retry_count: i32,
    #[serde(default)]
    pub retry_logic: RetryLogic,
    #[serde(default = "default_retry_delay")]
    pub retry_delay_seconds: i32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: i64,
    #[serde(default)]
    pub timeout_policy: TaskTimeoutPolicy,
    #[serde(default = "default_response_timeout")]
    pub response_timeout_seconds: i64,
    #[serde(default)]
    pub concurrent_exec_limit: Option<i32>,
    #[serde(default)]
    pub input_keys: Vec<String>,
    #[serde(default)]
    pub output_keys: Vec<String>,
    #[serde(default)]
    pub input_template: HashMap<String, Value>,
    #[serde(default)]
    pub rate_limit_per_frequency: Option<i32>,
    #[serde(default)]
    pub rate_limit_frequency_in_seconds: Option<i32>,
    #[serde(default)]
    pub owner_email: Option<String>,
    #[serde(default)]
    pub poll_timeout_seconds: Option<i32>,
    #[serde(default = "default_backoff")]
    pub backoff_scale_factor: i32,
    #[serde(default)]
    pub total_timeout_seconds: Option<i64>,
    #[serde(default)]
    pub owner_app: Option<String>,
    #[serde(default)]
    pub create_time: Option<i64>,
    #[serde(default)]
    pub update_time: Option<i64>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub updated_by: Option<String>,
    #[serde(default)]
    pub base_type: Option<String>,
    #[serde(default)]
    pub isolation_group_id: Option<String>,
    #[serde(default)]
    pub execution_name_space: Option<String>,
    #[serde(default)]
    pub input_schema: Option<SchemaDef>,
    #[serde(default)]
    pub output_schema: Option<SchemaDef>,
    #[serde(default)]
    pub enforce_schema: bool,
    /// If non-empty, only retry on failures whose reason contains one of these strings.
    #[serde(default)]
    pub retry_on_errors: Vec<String>,
    /// Environment variables/secrets to inject into poll responses for this task type.
    #[serde(default)]
    pub env_vars: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RetryLogic {
    #[default]
    Fixed,
    ExponentialBackoff,
    LinearBackoff,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskTimeoutPolicy {
    #[default]
    Retry,
    TimeOutWf,
    AlertOnly,
}

//  Runtime Task 

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskResult {
    pub task_id: String,
    pub workflow_instance_id: String,
    #[serde(default)]
    pub task_type: String,
    #[serde(default)]
    pub task_def_name: String,
    #[serde(default)]
    pub reference_task_name: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub input_data: Value,
    #[serde(default)]
    pub output_data: Value,
    #[serde(default)]
    pub reason_for_incompletion: Option<String>,
    #[serde(default)]
    pub scheduled_time: Option<i64>,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub end_time: Option<i64>,
    #[serde(default)]
    pub update_time: Option<i64>,
    #[serde(default)]
    pub poll_count: i32,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub seq: i32,
    #[serde(default)]
    pub retry_count: i32,
    #[serde(default)]
    pub callback_after_seconds: i64,
    #[serde(default)]
    pub logs: Vec<TaskExecLog>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub external_output_payload_storage_path: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub start_delay_in_seconds: Option<i32>,
    #[serde(default)]
    pub retried_task_id: Option<String>,
    #[serde(default)]
    pub retried: bool,
    #[serde(default)]
    pub executed: bool,
    #[serde(default)]
    pub callback_from_worker: bool,
    #[serde(default)]
    pub response_timeout_seconds: Option<i64>,
    #[serde(default)]
    pub workflow_type: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub rate_limit_per_frequency: Option<i32>,
    #[serde(default)]
    pub rate_limit_frequency_in_seconds: Option<i32>,
    #[serde(default)]
    pub workflow_priority: Option<i32>,
    #[serde(default)]
    pub execution_name_space: Option<String>,
    #[serde(default)]
    pub isolation_group_id: Option<String>,
    #[serde(default)]
    pub iteration: Option<i32>,
    #[serde(default)]
    pub sub_workflow_id: Option<String>,
    #[serde(default)]
    pub subworkflow_changed: bool,
    #[serde(default)]
    pub first_start_time: Option<i64>,
    #[serde(default)]
    pub parent_task_id: Option<String>,
    #[serde(default)]
    pub loop_over_task: bool,
    #[serde(default)]
    pub queue_wait_time: Option<i64>,
    #[serde(default)]
    pub workflow_task: Option<Value>,
    #[serde(default)]
    pub task_definition: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    InProgress,
    Canceled,
    Failed,
    FailedWithTerminalError,
    Completed,
    CompletedWithErrors,
    Scheduled,
    TimedOut,
    Skipped,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::Canceled => write!(f, "CANCELED"),
            Self::Failed => write!(f, "FAILED"),
            Self::FailedWithTerminalError => write!(f, "FAILED_WITH_TERMINAL_ERROR"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::CompletedWithErrors => write!(f, "COMPLETED_WITH_ERRORS"),
            Self::Scheduled => write!(f, "SCHEDULED"),
            Self::TimedOut => write!(f, "TIMED_OUT"),
            Self::Skipped => write!(f, "SKIPPED"),
        }
    }
}

//  Task Update Request 

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskUpdateRequest {
    pub task_id: String,
    pub workflow_instance_id: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub output_data: Value,
    #[serde(default)]
    pub reason_for_incompletion: Option<String>,
    #[serde(default)]
    pub callback_after_seconds: i64,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub logs: Vec<TaskExecLog>,
    #[serde(default)]
    pub external_output_payload_storage_path: Option<String>,
    #[serde(default)]
    pub sub_workflow_id: Option<String>,
    #[serde(default)]
    pub extend_lease: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskExecLog {
    pub log: String,
    #[serde(default)]
    pub task_id: String,
    #[serde(default)]
    pub created_time: Option<i64>,
}

//  Poll / Queue 

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PollTask {
    pub task_id: String,
    pub workflow_instance_id: String,
    #[serde(default)]
    pub task_type: String,
    #[serde(default)]
    pub task_def_name: String,
    #[serde(default)]
    pub reference_task_name: String,
    pub status: TaskStatus,
    #[serde(default)]
    pub input_data: Value,
    #[serde(default)]
    pub scheduled_time: Option<i64>,
    #[serde(default)]
    pub start_time: Option<i64>,
    #[serde(default)]
    pub callback_after_seconds: i64,
    #[serde(default)]
    pub poll_count: i32,
    #[serde(default)]
    pub retry_count: i32,
    /// Task priority (higher = polled first in batch).
    #[serde(default)]
    pub priority: i32,
    /// Environment variables/secrets injected from task definition.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_vars: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct PollData {
    #[serde(default)]
    pub queue_name: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub worker_id: Option<String>,
    #[serde(default)]
    pub last_poll_time: Option<i64>,
}

/// Queue size response
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct QueueSizes(pub HashMap<String, i64>);

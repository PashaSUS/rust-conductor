use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::common::IdempotencyStrategy;
use super::task::TaskStatus;
use super::workflow::{WorkflowDef, WorkflowStatus};

// ── Workflow Summary (search results) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSummary {
    pub workflow_id: String,
    pub workflow_type: String,
    #[serde(default)]
    pub version: i32,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub reason_for_incompletion: Option<String>,
    #[serde(default)]
    pub execution_time: Option<i64>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub failed_reference_task_names: Option<String>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub external_output_payload_storage_path: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub failed_task_names: Vec<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub task_to_domain: Option<HashMap<String, String>>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub output_size: Option<i64>,
    #[serde(default)]
    pub input_size: Option<i64>,
    #[serde(default)]
    pub parent_workflow_id: Option<String>,
}

// ── Task Summary (search results) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSummary {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_type: Option<String>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub scheduled_time: Option<String>,
    #[serde(default)]
    pub start_time: Option<String>,
    #[serde(default)]
    pub update_time: Option<String>,
    #[serde(default)]
    pub end_time: Option<String>,
    #[serde(default)]
    pub status: Option<TaskStatus>,
    #[serde(default)]
    pub reason_for_incompletion: Option<String>,
    #[serde(default)]
    pub execution_time: Option<i64>,
    #[serde(default)]
    pub queue_wait_time: Option<i64>,
    #[serde(default)]
    pub task_def_name: Option<String>,
    #[serde(default)]
    pub task_type: Option<String>,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub external_output_payload_storage_path: Option<String>,
    #[serde(default)]
    pub workflow_priority: Option<i32>,
    #[serde(default)]
    pub domain: Option<String>,
}

// ── Workflow Test ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct WorkflowTestRequest {
    pub name: String,
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub input: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub task_to_domain: Option<HashMap<String, String>>,
    #[serde(default)]
    pub workflow_def: Option<WorkflowDef>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub idempotency_strategy: Option<IdempotencyStrategy>,
    #[serde(default)]
    pub task_ref_to_mock_output: Option<HashMap<String, Vec<TaskMock>>>,
    #[serde(default)]
    pub sub_workflow_test_request: Option<HashMap<String, WorkflowTestRequest>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct TaskMock {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub output: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub execution_time: Option<i64>,
    #[serde(default)]
    pub queue_wait_time: Option<i64>,
}

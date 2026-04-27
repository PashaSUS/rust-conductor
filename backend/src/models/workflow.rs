use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::common::{CacheConfig, ConditionNode, IdempotencyStrategy, RateLimitConfig, SchemaDef};
use super::task::TaskResult;

//  Defaults

pub(crate) fn default_version() -> i32 {
    1
}
fn default_schema_version() -> i32 {
    2
}
pub(crate) fn default_true() -> bool {
    true
}

//  Workflow Definition

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDef {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_version")]
    pub version: i32,
    pub tasks: Vec<WorkflowTask>,
    /// Netflix-Conductor–compatible list of input parameter names.
    /// Kept as a plain `Vec<String>` for backwards compatibility.
    #[serde(default)]
    pub input_parameters: Vec<String>,
    /// Optional, richer parameter metadata (name + description + type + default + required).
    /// This is an *additive* field that is ignored by standard Netflix Conductor clients,
    /// so workflow definitions remain fully round-trip compatible.
    /// When present, the engine uses these entries at `startWorkflow` time to:
    ///   * apply `default_value` for missing inputs,
    ///   * reject the request with 400 if a `required` parameter is absent and no default is set.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_parameter_definitions: Vec<WorkflowInputParameterDef>,
    #[serde(default)]
    pub output_parameters: HashMap<String, Value>,
    #[serde(default)]
    pub failure_workflow: Option<String>,
    #[serde(default = "default_schema_version")]
    pub schema_version: i32,
    #[serde(default = "default_true")]
    pub restartable: bool,
    #[serde(default)]
    pub workflow_status_listener_enabled: bool,
    #[serde(default)]
    pub owner_email: Option<String>,
    #[serde(default)]
    pub timeout_policy: TimeoutPolicy,
    #[serde(default)]
    pub timeout_seconds: i64,
    #[serde(default)]
    pub variables: HashMap<String, Value>,
    #[serde(default)]
    pub input_template: HashMap<String, Value>,
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
    pub workflow_status_listener_sink: Option<String>,
    #[serde(default)]
    pub rate_limit_config: Option<RateLimitConfig>,
    #[serde(default)]
    pub input_schema: Option<SchemaDef>,
    #[serde(default)]
    pub output_schema: Option<SchemaDef>,
    #[serde(default)]
    pub enforce_schema: bool,
    #[serde(default)]
    pub metadata: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub cache_config: Option<CacheConfig>,
    #[serde(default)]
    pub masked_fields: Vec<String>,
    /// Webhook URL called (POST) when the workflow completes successfully.
    #[serde(default)]
    pub on_complete_webhook: Option<String>,
    /// Webhook URL called (POST) when the workflow fails.
    #[serde(default)]
    pub on_failure_webhook: Option<String>,
    /// Tags for categorization and label-based filtering.
    #[serde(default)]
    pub tags: Vec<String>,
    /// SLA deadline in seconds from workflow start. Breaches are detected by the sweeper.
    #[serde(default)]
    pub sla_deadline_seconds: Option<i64>,
    /// Enable saga pattern — run compensation tasks in reverse on failure.
    #[serde(default)]
    pub saga_enabled: bool,
    /// Base workflow name for inheritance. The child extends the parent's tasks.
    #[serde(default)]
    pub base_workflow: Option<String>,
    /// Base workflow version (defaults to latest if omitted).
    #[serde(default)]
    pub base_workflow_version: Option<i32>,
}

//  Workflow Task

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTask {
    pub name: String,
    pub task_reference_name: String,
    #[serde(rename = "type", default = "default_task_type_str")]
    pub task_type: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub input_parameters: HashMap<String, Value>,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub start_delay: i32,
    #[serde(default)]
    pub sub_workflow_param: Option<SubWorkflowParams>,
    #[serde(default)]
    pub join_on: Vec<String>,
    #[serde(default)]
    pub fork_tasks: Vec<Vec<WorkflowTask>>,
    #[serde(default)]
    pub decision_cases: HashMap<String, Vec<WorkflowTask>>,
    #[serde(default)]
    pub default_case: Vec<WorkflowTask>,
    #[serde(default)]
    pub case_expression: Option<String>,
    #[serde(default)]
    pub case_value_param: Option<String>,
    #[serde(default)]
    pub loop_condition: Option<String>,
    #[serde(default)]
    pub loop_over: Vec<WorkflowTask>,
    #[serde(default)]
    pub retry_count: Option<i32>,
    #[serde(default)]
    pub evaluator_type: Option<String>,
    #[serde(default)]
    pub expression: Option<String>,
    #[serde(default)]
    pub script_expression: Option<String>,
    #[serde(default)]
    pub dynamic_task_name_param: Option<String>,
    #[serde(default)]
    pub dynamic_fork_join_tasks_param: Option<String>,
    #[serde(default)]
    pub dynamic_fork_tasks_param: Option<String>,
    #[serde(default)]
    pub dynamic_fork_tasks_input_param_name: Option<String>,
    #[serde(default)]
    pub sink: Option<String>,
    #[serde(default)]
    pub task_definition: Option<Value>,
    #[serde(default)]
    pub rate_limited: Option<bool>,
    #[serde(default)]
    pub default_exclusive_join_task: Vec<String>,
    #[serde(default)]
    pub async_complete: bool,
    #[serde(default)]
    pub on_state_change: Option<HashMap<String, Vec<StateChangeEvent>>>,
    #[serde(default)]
    pub join_status: Option<String>,
    #[serde(default)]
    pub cache_config: Option<CacheConfig>,
    #[serde(default)]
    pub permissive: Option<bool>,
    /// Compensation task to run if this task fails (saga pattern).
    #[serde(default)]
    pub compensation_task: Option<Box<WorkflowTask>>,
    /// For MAP tasks: the array items to iterate over (from input param name).
    #[serde(default)]
    pub map_items_param: Option<String>,
    /// For MAP tasks: max parallel sub-tasks (0 = unlimited).
    #[serde(default)]
    pub map_parallelism: Option<i32>,
    /// For MAP tasks: the template task to fan-out for each item.
    #[serde(default)]
    pub map_task: Option<Box<WorkflowTask>>,
    /// Heartbeat timeout in seconds. Workers must heartbeat within this.
    #[serde(default)]
    pub heartbeat_timeout_seconds: Option<i64>,
    /// Composite condition tree for advanced branching (AND/OR/NOT combinators).
    /// Used when evaluator_type = "composite" on DECISION/SWITCH tasks.
    #[serde(default)]
    pub condition_tree: Option<ConditionNode>,
}

fn default_task_type_str() -> String {
    "SIMPLE".to_string()
}

/// Rich metadata for a single workflow input parameter.
///
/// This is an **additive** structure carried alongside the legacy
/// `inputParameters: [string]` array so that definitions remain 100 %
/// backwards-compatible with Netflix Conductor clients (they simply
/// ignore the extra `inputParameterDefinitions` field).
///
/// Example JSON:
/// ```json
/// {
///   "name": "orderId",
///   "description": "Unique order identifier assigned by the checkout service.",
///   "type": "string",
///   "required": true
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowInputParameterDef {
    /// Parameter name (must match the key expected in `StartWorkflowRequest.input`).
    pub name: String,
    /// Human-readable description shown in the UI / OpenAPI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional type hint ("string" | "number" | "boolean" | "object" | "array").
    #[serde(default, rename = "type", skip_serializing_if = "Option::is_none")]
    pub param_type: Option<String>,
    /// If true, startWorkflow will fail with 400 when this parameter is missing
    /// and no `default_value` is provided.
    #[serde(default)]
    pub required: bool,
    /// Default value applied to the workflow input when the parameter is missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_value: Option<Value>,
    /// Optional example used purely for UI/documentation purposes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub example: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateChangeEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default)]
    pub payload: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimeoutPolicy {
    #[default]
    TimeOutWf,
    AlertOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubWorkflowParams {
    pub name: String,
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub task_to_domain: Option<HashMap<String, String>>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub idempotency_strategy: Option<IdempotencyStrategy>,
    #[serde(default)]
    pub priority: Option<Value>,
    #[serde(default)]
    pub workflow_definition: Option<Value>,
}

//  Runtime Workflow

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub workflow_id: String,
    pub workflow_name: String,
    pub workflow_version: i32,
    pub status: WorkflowStatus,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub output: Value,
    #[serde(default)]
    pub tasks: Vec<TaskResult>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    pub start_time: i64,
    #[serde(default)]
    pub end_time: Option<i64>,
    pub update_time: i64,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub updated_by: Option<String>,
    #[serde(default)]
    pub reason_for_incompletion: Option<String>,
    #[serde(default)]
    pub workflow_definition: Option<WorkflowDef>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub variables: HashMap<String, Value>,
    #[serde(default)]
    pub failed_reference_task_names: Vec<String>,
    #[serde(default)]
    pub owner_app: Option<String>,
    #[serde(default)]
    pub parent_workflow_id: Option<String>,
    #[serde(default)]
    pub parent_workflow_task_id: Option<String>,
    #[serde(default)]
    pub re_run_from_workflow_id: Option<String>,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub task_to_domain: HashMap<String, String>,
    #[serde(default)]
    pub failed_task_names: Vec<String>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub external_output_payload_storage_path: Option<String>,
    #[serde(default)]
    pub last_retried_time: Option<i64>,
    #[serde(default)]
    pub history: Vec<Workflow>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub rate_limit_key: Option<String>,
    #[serde(default)]
    pub rate_limited: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkflowStatus {
    Running,
    Completed,
    Failed,
    TimedOut,
    Terminated,
    Paused,
}

impl std::fmt::Display for WorkflowStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Running => write!(f, "RUNNING"),
            Self::Completed => write!(f, "COMPLETED"),
            Self::Failed => write!(f, "FAILED"),
            Self::TimedOut => write!(f, "TIMED_OUT"),
            Self::Terminated => write!(f, "TERMINATED"),
            Self::Paused => write!(f, "PAUSED"),
        }
    }
}

//  Requests

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWorkflowRequest {
    pub name: String,
    #[serde(default = "default_version")]
    pub version: i32,
    #[serde(default)]
    pub input: Value,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub task_to_domain: HashMap<String, String>,
    #[serde(default)]
    pub workflow_def: Option<WorkflowDef>,
    #[serde(default)]
    pub external_input_payload_storage_path: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub idempotency_strategy: Option<IdempotencyStrategy>,
    /// Tags for categorization.
    #[serde(default)]
    pub tags: Vec<String>,
    /// Internal: parent workflow id when started as a SUB_WORKFLOW child.
    /// Set atomically with the workflow INSERT to avoid a race where the
    /// child completes before parent linkage is recorded.
    #[serde(skip)]
    pub parent_workflow_id: Option<String>,
    /// Internal: parent SUB_WORKFLOW task id (paired with parent_workflow_id).
    #[serde(skip)]
    pub parent_workflow_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RerunWorkflowRequest {
    #[serde(default)]
    pub re_run_from_workflow_id: Option<String>,
    #[serde(default)]
    pub workflow_input: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub re_run_from_task_id: Option<String>,
    #[serde(default)]
    pub task_input: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub correlation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkipTaskRequest {
    #[serde(default)]
    pub task_input: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub task_output: Option<HashMap<String, Value>>,
}

//  CRON Scheduled Workflow

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledWorkflow {
    #[serde(default)]
    pub schedule_id: String,
    pub name: String,
    pub cron_expression: String,
    #[serde(default = "default_utc")]
    pub timezone: String,
    pub workflow_name: String,
    #[serde(default = "default_version")]
    pub workflow_version: i32,
    #[serde(default)]
    pub workflow_input: Value,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub last_run_at: Option<i64>,
    #[serde(default)]
    pub next_run_at: Option<i64>,
    #[serde(default)]
    pub last_error: Option<String>,
}

fn default_utc() -> String {
    "UTC".to_string()
}

//  Workflow Template

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowTemplate {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    pub workflow_def: WorkflowDef,
    /// Parameter names that can be substituted when instantiating the template.
    #[serde(default)]
    pub parameters: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstantiateTemplateRequest {
    pub template_name: String,
    #[serde(default)]
    pub parameter_values: HashMap<String, Value>,
    #[serde(default)]
    pub input: Value,
}

//  Dynamic Workflow Modification

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModifyWorkflowRequest {
    /// Tasks to append to the running workflow.
    #[serde(default)]
    pub add_tasks: Vec<WorkflowTask>,
    /// Reference names of tasks to remove (only unscheduled tasks).
    #[serde(default)]
    pub remove_task_refs: Vec<String>,
}

//  Workflow Signal (inter-communication)

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendSignalRequest {
    /// Signal name.
    pub signal_name: String,
    /// Payload to deliver with the signal.
    #[serde(default)]
    pub payload: Value,
}

//  Workflow Checkpoint

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowCheckpoint {
    pub checkpoint_id: String,
    pub workflow_id: String,
    pub created_at: i64,
    /// Snapshot of workflow state at checkpoint time.
    pub workflow_snapshot: Value,
    /// Snapshot of all task states at checkpoint time.
    pub tasks_snapshot: Value,
    /// Snapshot of workflow variables.
    pub variables_snapshot: Value,
    /// Optional label for the checkpoint.
    #[serde(default)]
    pub label: Option<String>,
}

//  Graph Validation Result

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(default)]
    pub errors: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

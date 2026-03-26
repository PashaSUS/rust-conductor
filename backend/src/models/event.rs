use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

fn default_true() -> bool {
    true
}

// ── Event Handler ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventHandler {
    pub name: String,
    pub event: String,
    #[serde(default)]
    pub condition: Option<String>,
    pub actions: Vec<EventAction>,
    #[serde(default = "default_true")]
    pub active: bool,
    #[serde(default)]
    pub evaluator_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAction {
    #[serde(default)]
    pub action: Option<EventActionType>,
    #[serde(default)]
    pub start_workflow: Option<StartWorkflowAction>,
    #[serde(default)]
    pub complete_task: Option<TaskDetails>,
    #[serde(default)]
    pub fail_task: Option<TaskDetails>,
    #[serde(default)]
    pub expand_inline_json: Option<bool>,
    #[serde(default)]
    pub terminate_workflow: Option<TerminateWorkflowAction>,
    #[serde(default)]
    pub update_workflow_variables: Option<UpdateWorkflowVariablesAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventActionType {
    StartWorkflow,
    CompleteTask,
    FailTask,
    TerminateWorkflow,
    UpdateWorkflowVariables,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartWorkflowAction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub correlation_id: Option<String>,
    #[serde(default)]
    pub input: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub task_to_domain: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDetails {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub task_ref_name: Option<String>,
    #[serde(default)]
    pub output: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminateWorkflowAction {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub termination_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateWorkflowVariablesAction {
    #[serde(default)]
    pub workflow_id: Option<String>,
    #[serde(default)]
    pub variables: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub append_array: Option<bool>,
}

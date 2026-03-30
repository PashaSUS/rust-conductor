use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

// ── Shared Schema / Config Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaDef {
    pub name: String,
    pub version: i32,
    #[serde(rename = "type")]
    pub schema_type: SchemaType,
    #[serde(default)]
    pub data: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub external_ref: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SchemaType {
    Json,
    Avro,
    Protobuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheConfig {
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub ttl_in_second: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimitConfig {
    #[serde(default)]
    pub rate_limit_key: Option<String>,
    #[serde(default)]
    pub concurrent_exec_limit: Option<i32>,
}

// ── Search / Bulk ──

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult<T> {
    pub total_hits: i64,
    pub results: Vec<T>,
    /// Opaque cursor for cursor-based pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkResponse {
    #[serde(default)]
    pub bulk_error_results: HashMap<String, String>,
    #[serde(default)]
    pub bulk_successful_results: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowMetrics {
    pub workflow_name: String,
    pub sample_size: i64,
    pub status_distribution: HashMap<String, i64>,
    pub success_rate: f64,
    pub failure_rate: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50_duration_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_duration_ms: Option<i64>,
}

// ── External Storage ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ExternalStorageLocation {
    #[serde(default)]
    pub uri: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

// ── Idempotency ──

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum IdempotencyStrategy {
    Fail,
    ReturnExisting,
    FailOnRunning,
}

// ── Composite Condition Combinators ──

/// A composable boolean expression node for DECISION/SWITCH tasks.
/// Used when evaluator_type = "composite".
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ConditionNode {
    /// All children must be true.
    And(Vec<ConditionNode>),
    /// At least one child must be true.
    Or(Vec<ConditionNode>),
    /// Inverts the child result.
    Not(Box<ConditionNode>),
    /// Leaf comparison: field op value.
    Compare {
        field: String,
        op: CompareOp,
        value: Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CompareOp {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    Contains,
    StartsWith,
    EndsWith,
}

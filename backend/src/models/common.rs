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

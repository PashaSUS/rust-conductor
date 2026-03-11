use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    #[serde(default)]
    pub details: HashMap<String, Value>,
    pub healthy: bool,
    #[serde(default)]
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCheckStatus {
    #[serde(default)]
    pub health_results: Vec<Health>,
    #[serde(default)]
    pub suppressed_health_results: Vec<Health>,
    pub healthy: bool,
}

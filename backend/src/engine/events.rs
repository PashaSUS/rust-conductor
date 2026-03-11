use serde_json::Value;

use super::error::EngineError;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    pub async fn register_event_handler(&self, handler: &EventHandler) -> Result<(), EngineError> {
        let json = serde_json::to_value(handler).map_err(|e| EngineError::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO event_handler (name, event, definition) VALUES ($1, $2, $3)
             ON CONFLICT (name) DO UPDATE SET event = $2, definition = $3, updated_on = NOW()",
        )
        .bind(&handler.name)
        .bind(&handler.event)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_event_handlers(&self) -> Result<Vec<EventHandler>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>("SELECT definition FROM event_handler ORDER BY name")
            .fetch_all(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;
        rows.into_iter()
            .map(|r| serde_json::from_value(r).map_err(|e| EngineError::Serde(e.to_string())))
            .collect()
    }

    pub async fn get_event_handlers_for_event(&self, event: &str, active_only: bool) -> Result<Vec<EventHandler>, EngineError> {
        let mut query = String::from("SELECT definition FROM event_handler WHERE event = $1");
        if active_only {
            query.push_str(" AND (definition->>'active')::bool = true");
        }
        let rows = sqlx::query_scalar::<_, Value>(&query)
            .bind(event)
            .fetch_all(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;
        rows.into_iter()
            .map(|r| serde_json::from_value(r).map_err(|e| EngineError::Serde(e.to_string())))
            .collect()
    }

    pub async fn delete_event_handler(&self, name: &str) -> Result<(), EngineError> {
        let result = sqlx::query("DELETE FROM event_handler WHERE name = $1")
            .bind(name)
            .execute(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;
        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!("Event handler not found: {name}")));
        }
        Ok(())
    }
}

use serde_json::Value;

use super::error::EngineError;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    // ── Workflow Definition CRUD ──

    pub async fn register_workflow_def(&self, def: &WorkflowDef) -> Result<(), EngineError> {
        let json = serde_json::to_value(def).map_err(|e| EngineError::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO workflow_def (name, version, definition) VALUES ($1, $2, $3)
             ON CONFLICT (name, version) DO UPDATE SET definition = $3, updated_on = NOW()",
        )
        .bind(&def.name)
        .bind(def.version)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_workflow_def(
        &self,
        name: &str,
        version: Option<i32>,
    ) -> Result<WorkflowDef, EngineError> {
        let row = match version {
            Some(v) => {
                sqlx::query_scalar::<_, Value>(
                    "SELECT definition FROM workflow_def WHERE name = $1 AND version = $2",
                )
                .bind(name)
                .bind(v)
                .fetch_optional(self.shards.primary())
                .await
            }
            None => {
                sqlx::query_scalar::<_, Value>(
                    "SELECT definition FROM workflow_def WHERE name = $1 ORDER BY version DESC LIMIT 1",
                )
                .bind(name)
                .fetch_optional(self.shards.primary())
                .await
            }
        }
        .map_err(|e| EngineError::Database(e.to_string()))?;

        match row {
            Some(json) => serde_json::from_value(json).map_err(|e| EngineError::Serde(e.to_string())),
            None => Err(EngineError::NotFound(format!(
                "Workflow definition not found: {name}"
            ))),
        }
    }

    pub async fn list_workflow_defs(&self) -> Result<Vec<WorkflowDef>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM workflow_def ORDER BY name, version",
        )
        .fetch_all(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|r| serde_json::from_value(r).map_err(|e| EngineError::Serde(e.to_string())))
            .collect()
    }

    pub async fn delete_workflow_def(&self, name: &str, version: i32) -> Result<(), EngineError> {
        let result =
            sqlx::query("DELETE FROM workflow_def WHERE name = $1 AND version = $2")
                .bind(name)
                .bind(version)
                .execute(self.shards.primary())
                .await
                .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!(
                "Workflow def {name} v{version} not found"
            )));
        }
        Ok(())
    }

    // ── Task Definition CRUD ──

    pub async fn register_task_def(&self, def: &TaskDef) -> Result<(), EngineError> {
        let json = serde_json::to_value(def).map_err(|e| EngineError::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO task_def (name, definition) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET definition = $2, updated_on = NOW()",
        )
        .bind(&def.name)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(())
    }

    pub async fn get_task_def(&self, name: &str) -> Result<TaskDef, EngineError> {
        let row = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM task_def WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        match row {
            Some(json) => serde_json::from_value(json).map_err(|e| EngineError::Serde(e.to_string())),
            None => Err(EngineError::NotFound(format!("Task def not found: {name}"))),
        }
    }

    pub async fn list_task_defs(&self) -> Result<Vec<TaskDef>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM task_def ORDER BY name",
        )
        .fetch_all(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|r| serde_json::from_value(r).map_err(|e| EngineError::Serde(e.to_string())))
            .collect()
    }

    pub async fn delete_task_def(&self, name: &str) -> Result<(), EngineError> {
        let result = sqlx::query("DELETE FROM task_def WHERE name = $1")
            .bind(name)
            .execute(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!("Task def {name} not found")));
        }
        Ok(())
    }
}

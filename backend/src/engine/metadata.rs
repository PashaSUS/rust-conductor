use serde_json::Value;

use super::error::EngineError;
use super::WorkflowEngine;
use crate::models::*;

impl WorkflowEngine {
    // ── Workflow Definition CRUD ──

    pub async fn register_workflow_def(&self, def: &WorkflowDef) -> Result<WorkflowDef, EngineError> {
        let json = serde_json::to_value(def).map_err(|e| {
            tracing::error!(name = %def.name, version = def.version, error = %e, "Failed to serialize workflow def");
            EngineError::Serde(e.to_string())
        })?;
        sqlx::query(
            "INSERT INTO workflow_def (name, version, definition) VALUES ($1, $2, $3)
             ON CONFLICT (name, version) DO UPDATE SET definition = $3, updated_on = NOW()",
        )
        .bind(&def.name)
        .bind(def.version)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| {
            tracing::error!(name = %def.name, version = def.version, error = %e, "DB error registering workflow def");
            EngineError::Database(e.to_string())
        })?;
        self.cache_put_workflow_def(def);
        Ok(def.clone())
    }

    pub async fn get_workflow_def(
        &self,
        name: &str,
        version: Option<i32>,
    ) -> Result<WorkflowDef, EngineError> {
        // Check LRU cache first
        if let Some(v) = version {
            if let Some(cached) = self.cache_get_workflow_def(name, v) {
                return Ok(cached);
            }
        }

        let db = self.shards.read_primary();
        let row = match version {
            Some(v) => {
                sqlx::query_scalar::<_, Value>(
                    "SELECT definition FROM workflow_def WHERE name = $1 AND version = $2",
                )
                .bind(name)
                .bind(v)
                .fetch_optional(db)
                .await
            }
            None => {
                sqlx::query_scalar::<_, Value>(
                    "SELECT definition FROM workflow_def WHERE name = $1 ORDER BY version DESC LIMIT 1",
                )
                .bind(name)
                .fetch_optional(db)
                .await
            }
        }
        .map_err(|e| EngineError::Database(e.to_string()))?;

        match row {
            Some(json) => {
                let def: WorkflowDef = serde_json::from_value(json).map_err(|e| {
                    tracing::error!(name = %name, version = ?version, error = %e, "Failed to deserialize workflow def from DB");
                    EngineError::Serde(e.to_string())
                })?;
                self.cache_put_workflow_def(&def);
                Ok(def)
            }
            None => {
                tracing::error!(name = %name, version = ?version, "Workflow definition not found");
                Err(EngineError::NotFound(format!(
                    "Workflow definition not found: {name}"
                )))
            }
        }
    }

    pub async fn list_workflow_defs(&self) -> Result<Vec<WorkflowDef>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM workflow_def ORDER BY name, version",
        )
        .fetch_all(self.shards.read_primary())
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
        self.cache_invalidate_workflow_def(name, version);
        Ok(())
    }

    // ── Task Definition CRUD ──

    pub async fn register_task_def(&self, def: &TaskDef) -> Result<TaskDef, EngineError> {
        let json = serde_json::to_value(def).map_err(|e| {
            tracing::error!(name = %def.name, error = %e, "Failed to serialize task def");
            EngineError::Serde(e.to_string())
        })?;
        sqlx::query(
            "INSERT INTO task_def (name, definition) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET definition = $2, updated_on = NOW()",
        )
        .bind(&def.name)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| {
            tracing::error!(name = %def.name, error = %e, "DB error registering task def");
            EngineError::Database(e.to_string())
        })?;
        self.cache_put_task_def(def);
        Ok(def.clone())
    }

    pub async fn get_task_def(&self, name: &str) -> Result<TaskDef, EngineError> {
        // Check LRU cache first
        if let Some(cached) = self.cache_get_task_def(name) {
            return Ok(cached);
        }

        let db = self.shards.read_primary();
        let row = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM task_def WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(db)
        .await
        .map_err(|e| {
            tracing::error!(name = %name, error = %e, "DB error fetching task def");
            EngineError::Database(e.to_string())
        })?;

        match row {
            Some(json) => {
                let def: TaskDef = serde_json::from_value(json).map_err(|e| {
                    tracing::error!(name = %name, error = %e, "Failed to deserialize task def from DB");
                    EngineError::Serde(e.to_string())
                })?;
                self.cache_put_task_def(&def);
                Ok(def)
            }
            None => {
                tracing::error!(name = %name, "Task def not found");
                Err(EngineError::NotFound(format!("Task def not found: {name}")))
            }
        }
    }

    pub async fn list_task_defs(&self) -> Result<Vec<TaskDef>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM task_def ORDER BY name",
        )
        .fetch_all(self.shards.read_primary())
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
        self.cache_invalidate_task_def(name);
        Ok(())
    }

    // ── Workflow Template CRUD ──

    pub async fn register_template(&self, tmpl: &WorkflowTemplate) -> Result<WorkflowTemplate, EngineError> {
        let json = serde_json::to_value(tmpl).map_err(|e| EngineError::Serde(e.to_string()))?;
        sqlx::query(
            "INSERT INTO workflow_template (name, definition) VALUES ($1, $2)
             ON CONFLICT (name) DO UPDATE SET definition = $2, updated_on = NOW()",
        )
        .bind(&tmpl.name)
        .bind(&json)
        .execute(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;
        Ok(tmpl.clone())
    }

    pub async fn get_template(&self, name: &str) -> Result<WorkflowTemplate, EngineError> {
        let row = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM workflow_template WHERE name = $1",
        )
        .bind(name)
        .fetch_optional(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        match row {
            Some(json) => serde_json::from_value(json).map_err(|e| EngineError::Serde(e.to_string())),
            None => Err(EngineError::NotFound(format!("Template not found: {name}"))),
        }
    }

    pub async fn list_templates(&self) -> Result<Vec<WorkflowTemplate>, EngineError> {
        let rows = sqlx::query_scalar::<_, Value>(
            "SELECT definition FROM workflow_template ORDER BY name",
        )
        .fetch_all(self.shards.primary())
        .await
        .map_err(|e| EngineError::Database(e.to_string()))?;

        rows.into_iter()
            .map(|r| serde_json::from_value(r).map_err(|e| EngineError::Serde(e.to_string())))
            .collect()
    }

    pub async fn delete_template(&self, name: &str) -> Result<(), EngineError> {
        let result = sqlx::query("DELETE FROM workflow_template WHERE name = $1")
            .bind(name)
            .execute(self.shards.primary())
            .await
            .map_err(|e| EngineError::Database(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(EngineError::NotFound(format!("Template {name} not found")));
        }
        Ok(())
    }

    /// Instantiate a template: replace parameter placeholders and start.
    pub async fn instantiate_template(&self, req: &InstantiateTemplateRequest) -> Result<String, EngineError> {
        let tmpl = self.get_template(&req.template_name).await?;
        let mut def_json = serde_json::to_string(&tmpl.workflow_def)
            .map_err(|e| EngineError::Serde(e.to_string()))?;

        // Replace ${paramName} placeholders in the serialized JSON
        for (key, val) in &req.parameter_values {
            let placeholder = format!("${{{key}}}");
            let replacement = match val {
                Value::String(s) => s.clone(),
                other => other.to_string(),
            };
            def_json = def_json.replace(&placeholder, &replacement);
        }

        let resolved_def: WorkflowDef = serde_json::from_str(&def_json)
            .map_err(|e| EngineError::Serde(format!("Template parameter resolution failed: {e}")))?;

        // Register the resolved def (auto-version) and start
        let _ = self.register_workflow_def(&resolved_def).await?;
        let start_req = StartWorkflowRequest {
            name: resolved_def.name,
            version: resolved_def.version,
            input: req.input.clone(),
            correlation_id: Some(format!("template:{}", req.template_name)),
            priority: 0,
            task_to_domain: Default::default(),
            workflow_def: None,
            external_input_payload_storage_path: None,
            created_by: Some("template-engine".to_string()),
            idempotency_key: None,
            idempotency_strategy: None,
            tags: vec![format!("template:{}", req.template_name)],
        };
        self.start_workflow(&start_req).await
    }
}

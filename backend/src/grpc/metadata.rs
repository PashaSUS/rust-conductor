use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use crate::models::WorkflowDef;
use super::pb;

pub struct MetadataServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl MetadataServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl pb::metadata_service_server::MetadataService for MetadataServiceImpl {
    async fn register_workflow_def(
        &self,
        request: Request<pb::RegisterWorkflowDefRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let def: WorkflowDef = serde_json::from_str(&request.into_inner().workflow_def_json)
            .map_err(|e| Status::invalid_argument(format!("Invalid WorkflowDef JSON: {e}")))?;
        self.engine
            .register_workflow_def(&def)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn update_workflow_defs(
        &self,
        request: Request<pb::UpdateWorkflowDefsRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let inner = request.into_inner();
        for json_str in &inner.workflow_defs_json {
            let def: WorkflowDef = serde_json::from_str(json_str)
                .map_err(|e| Status::invalid_argument(format!("Invalid WorkflowDef JSON: {e}")))?;
            self.engine
                .register_workflow_def(&def)
                .await
                .map_err(engine_err_to_status)?;
        }
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_workflow_def(
        &self,
        request: Request<pb::GetWorkflowDefRequest>,
    ) -> Result<Response<pb::WorkflowDefResponse>, Status> {
        let req = request.into_inner();
        let version = if req.version == 0 { None } else { Some(req.version) };
        let def = self
            .engine
            .get_workflow_def(&req.name, version)
            .await
            .map_err(engine_err_to_status)?;
        let json = serde_json::to_string(&def)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::WorkflowDefResponse {
            workflow_def_json: json,
        }))
    }

    async fn list_workflow_defs(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::ListWorkflowDefsResponse>, Status> {
        let defs = self
            .engine
            .list_workflow_defs()
            .await
            .map_err(engine_err_to_status)?;
        let jsons: Result<Vec<String>, _> = defs.iter().map(|d| serde_json::to_string(d)).collect();
        let jsons = jsons.map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::ListWorkflowDefsResponse {
            workflow_defs_json: jsons,
        }))
    }

    async fn delete_workflow_def(
        &self,
        request: Request<pb::DeleteWorkflowDefRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let req = request.into_inner();
        self.engine
            .delete_workflow_def(&req.name, req.version)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn register_task_defs(
        &self,
        request: Request<pb::RegisterTaskDefsRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let inner = request.into_inner();
        for json_str in &inner.task_defs_json {
            let def: crate::models::TaskDef = serde_json::from_str(json_str)
                .map_err(|e| Status::invalid_argument(format!("Invalid TaskDef JSON: {e}")))?;
            self.engine
                .register_task_def(&def)
                .await
                .map_err(engine_err_to_status)?;
        }
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_task_def(
        &self,
        request: Request<pb::GetTaskDefRequest>,
    ) -> Result<Response<pb::TaskDefResponse>, Status> {
        let req = request.into_inner();
        let def = self
            .engine
            .get_task_def(&req.name)
            .await
            .map_err(engine_err_to_status)?;
        let json = serde_json::to_string(&def)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::TaskDefResponse {
            task_def_json: json,
        }))
    }

    async fn list_task_defs(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::ListTaskDefsResponse>, Status> {
        let defs = self
            .engine
            .list_task_defs()
            .await
            .map_err(engine_err_to_status)?;
        let jsons: Result<Vec<String>, _> = defs.iter().map(|d| serde_json::to_string(d)).collect();
        let jsons = jsons.map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::ListTaskDefsResponse {
            task_defs_json: jsons,
        }))
    }

    async fn delete_task_def(
        &self,
        request: Request<pb::DeleteTaskDefRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let req = request.into_inner();
        self.engine
            .delete_task_def(&req.name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }
}

/// Map engine errors to gRPC status codes.
pub(crate) fn engine_err_to_status(e: crate::engine::EngineError) -> Status {
    use crate::engine::EngineError;
    match &e {
        EngineError::NotFound(_) => Status::not_found(e.to_string()),
        EngineError::InvalidState(_) => Status::failed_precondition(e.to_string()),
        EngineError::Serde(_) => Status::invalid_argument(e.to_string()),
        EngineError::Database(_) | EngineError::Redis(_) => Status::internal(e.to_string()),
    }
}

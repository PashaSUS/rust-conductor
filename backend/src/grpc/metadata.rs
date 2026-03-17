use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use super::pb;
use super::proto_conv;

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
        let pb_def = request.into_inner().workflow_def
            .ok_or_else(|| Status::invalid_argument("Missing workflow_def"))?;
        let def = proto_conv::workflow_def_from_proto(&pb_def);
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
        for pb_def in &inner.workflow_defs {
            let def = proto_conv::workflow_def_from_proto(pb_def);
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
    ) -> Result<Response<pb::WorkflowDefPb>, Status> {
        let req = request.into_inner();
        let version = if req.version == 0 { None } else { Some(req.version) };
        let def = self
            .engine
            .get_workflow_def(&req.name, version)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(proto_conv::workflow_def_to_proto(&def)))
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
        Ok(Response::new(pb::ListWorkflowDefsResponse {
            workflow_defs: defs.iter().map(proto_conv::workflow_def_to_proto).collect(),
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
        for pb_def in &inner.task_defs {
            let def = proto_conv::task_def_from_proto(pb_def);
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
    ) -> Result<Response<pb::TaskDefPb>, Status> {
        let req = request.into_inner();
        let def = self
            .engine
            .get_task_def(&req.name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(proto_conv::task_def_to_proto(&def)))
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
        Ok(Response::new(pb::ListTaskDefsResponse {
            task_defs: defs.iter().map(proto_conv::task_def_to_proto).collect(),
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

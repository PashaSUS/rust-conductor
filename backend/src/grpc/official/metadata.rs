use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::pb;
use crate::engine::WorkflowEngine;
use crate::grpc::metadata::engine_err_to_status;
use crate::grpc::proto_conv;

pub struct OfficialMetadataServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl OfficialMetadataServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl pb::metadata_service_server::MetadataService for OfficialMetadataServiceImpl {
    async fn create_workflow(
        &self,
        request: Request<pb::CreateWorkflowRequest>,
    ) -> Result<Response<pb::CreateWorkflowResponse>, Status> {
        let inner = request.into_inner();
        let wf_def = inner
            .workflow_def
            .ok_or_else(|| Status::invalid_argument("Missing workflow_def"))?;
        let def = proto_conv::workflow_def_from_official(&wf_def);
        self.engine
            .register_workflow_def(&def)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::CreateWorkflowResponse {}))
    }

    async fn validate_workflow(
        &self,
        request: Request<pb::ValidateWorkflowRequest>,
    ) -> Result<Response<pb::ValidateWorkflowResponse>, Status> {
        let inner = request.into_inner();
        let _wf_def = inner
            .workflow_def
            .ok_or_else(|| Status::invalid_argument("Missing workflow_def"))?;
        Ok(Response::new(pb::ValidateWorkflowResponse {}))
    }

    async fn update_workflows(
        &self,
        request: Request<pb::UpdateWorkflowsRequest>,
    ) -> Result<Response<pb::UpdateWorkflowsResponse>, Status> {
        let inner = request.into_inner();
        for wf_pb in &inner.workflow_defs {
            let def = proto_conv::workflow_def_from_official(wf_pb);
            self.engine
                .register_workflow_def(&def)
                .await
                .map_err(engine_err_to_status)?;
        }
        Ok(Response::new(pb::UpdateWorkflowsResponse {}))
    }

    async fn get_workflow(
        &self,
        request: Request<pb::GetWorkflowDefRequest>,
    ) -> Result<Response<pb::GetWorkflowDefResponse>, Status> {
        let req = request.into_inner();
        let version = if req.version == 0 {
            None
        } else {
            Some(req.version)
        };
        let def = self
            .engine
            .get_workflow_def(&req.name, version)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::GetWorkflowDefResponse {
            workflow_def: Some(proto_conv::workflow_def_to_official(&def)),
        }))
    }

    async fn create_tasks(
        &self,
        request: Request<pb::CreateTasksRequest>,
    ) -> Result<Response<pb::CreateTasksResponse>, Status> {
        let inner = request.into_inner();
        for td_pb in &inner.task_defs {
            let def = proto_conv::task_def_from_official(td_pb);
            self.engine
                .register_task_def(&def)
                .await
                .map_err(engine_err_to_status)?;
        }
        Ok(Response::new(pb::CreateTasksResponse {}))
    }

    async fn update_task(
        &self,
        request: Request<pb::UpdateTaskDefRequest>,
    ) -> Result<Response<pb::UpdateTaskDefResponse>, Status> {
        let inner = request.into_inner();
        let td_pb = inner
            .task_def
            .ok_or_else(|| Status::invalid_argument("Missing task_def"))?;
        let def = proto_conv::task_def_from_official(&td_pb);
        self.engine
            .register_task_def(&def)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::UpdateTaskDefResponse {}))
    }

    async fn get_task(
        &self,
        request: Request<pb::GetTaskDefRequest>,
    ) -> Result<Response<pb::GetTaskDefResponse>, Status> {
        let def = self
            .engine
            .get_task_def(&request.into_inner().name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::GetTaskDefResponse {
            task_def: Some(proto_conv::task_def_to_official(&def)),
        }))
    }

    async fn delete_task(
        &self,
        request: Request<pb::DeleteTaskDefRequest>,
    ) -> Result<Response<pb::DeleteTaskDefResponse>, Status> {
        self.engine
            .delete_task_def(&request.into_inner().name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::DeleteTaskDefResponse {}))
    }
}

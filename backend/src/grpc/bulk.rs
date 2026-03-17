use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use super::pb;
use super::metadata::engine_err_to_status;

pub struct BulkServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl BulkServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

fn to_proto(resp: crate::models::BulkResponse) -> pb::BulkResponseProto {
    pb::BulkResponseProto {
        bulk_error_results: resp.bulk_error_results,
        bulk_successful_results: resp.bulk_successful_results,
    }
}

#[tonic::async_trait]
impl pb::bulk_service_server::BulkService for BulkServiceImpl {
    async fn bulk_pause(
        &self,
        request: Request<pb::BulkWorkflowIdsRequest>,
    ) -> Result<Response<pb::BulkResponseProto>, Status> {
        let ids = request.into_inner().workflow_ids;
        let resp = self
            .engine
            .bulk_pause(&ids)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(to_proto(resp)))
    }

    async fn bulk_resume(
        &self,
        request: Request<pb::BulkWorkflowIdsRequest>,
    ) -> Result<Response<pb::BulkResponseProto>, Status> {
        let ids = request.into_inner().workflow_ids;
        let resp = self
            .engine
            .bulk_resume(&ids)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(to_proto(resp)))
    }

    async fn bulk_retry(
        &self,
        request: Request<pb::BulkWorkflowIdsRequest>,
    ) -> Result<Response<pb::BulkResponseProto>, Status> {
        let ids = request.into_inner().workflow_ids;
        let resp = self
            .engine
            .bulk_retry(&ids)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(to_proto(resp)))
    }

    async fn bulk_restart(
        &self,
        request: Request<pb::BulkWorkflowIdsRequest>,
    ) -> Result<Response<pb::BulkResponseProto>, Status> {
        let ids = request.into_inner().workflow_ids;
        let resp = self
            .engine
            .bulk_restart(&ids)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(to_proto(resp)))
    }

    async fn bulk_terminate(
        &self,
        request: Request<pb::BulkTerminateRequest>,
    ) -> Result<Response<pb::BulkResponseProto>, Status> {
        let req = request.into_inner();
        let reason = if req.reason.is_empty() { None } else { Some(req.reason.as_str()) };
        let resp = self
            .engine
            .bulk_terminate(&req.workflow_ids, reason)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(to_proto(resp)))
    }
}

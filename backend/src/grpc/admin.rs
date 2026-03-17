use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use super::pb;
use super::proto_conv;
use super::metadata::engine_err_to_status;

pub struct AdminServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl AdminServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl pb::admin_service_server::AdminService for AdminServiceImpl {
    async fn get_config(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::ConfigResponse>, Status> {
        let config = self
            .engine
            .get_all_config()
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::ConfigResponse {
            config: Some(proto_conv::hashmap_to_struct(&config)),
        }))
    }

    async fn sweep_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .sweep_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn pause_queue(
        &self,
        request: Request<pb::QueueNameRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .pause_queue(&request.into_inner().queue_name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn resume_queue(
        &self,
        request: Request<pb::QueueNameRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .resume_queue(&request.into_inner().queue_name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_queue_status(
        &self,
        request: Request<pb::QueueNameRequest>,
    ) -> Result<Response<pb::QueueStatusResponse>, Status> {
        let paused = self
            .engine
            .is_queue_paused(&request.into_inner().queue_name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::QueueStatusResponse { paused }))
    }

    async fn health_check(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::HealthCheckResponse>, Status> {
        let status = self.engine.health_check().await.map_err(engine_err_to_status)?;
        Ok(Response::new(proto_conv::health_to_proto(&status)))
    }
}

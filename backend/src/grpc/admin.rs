use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use super::pb;
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
        let json = serde_json::to_string(&config)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::ConfigResponse { config_json: json }))
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
        let json = serde_json::to_string(&status)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::HealthCheckResponse {
            healthy: status.healthy,
            health_results_json: json,
        }))
    }
}

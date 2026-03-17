use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use crate::models;
use super::pb;
use super::metadata::engine_err_to_status;

pub struct WorkflowServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl WorkflowServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

fn opt(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

#[tonic::async_trait]
impl pb::workflow_service_server::WorkflowService for WorkflowServiceImpl {
    async fn start_workflow(
        &self,
        request: Request<pb::StartWorkflowRequest>,
    ) -> Result<Response<pb::StartWorkflowResponse>, Status> {
        let req: models::StartWorkflowRequest =
            serde_json::from_str(&request.into_inner().start_workflow_json)
                .map_err(|e| Status::invalid_argument(format!("Invalid StartWorkflowRequest JSON: {e}")))?;
        let wf_id = self.engine.start_workflow(&req).await.map_err(engine_err_to_status)?;
        Ok(Response::new(pb::StartWorkflowResponse { workflow_id: wf_id }))
    }

    async fn get_workflow(
        &self,
        request: Request<pb::GetWorkflowRequest>,
    ) -> Result<Response<pb::WorkflowResponse>, Status> {
        let wf = self
            .engine
            .get_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        let json = serde_json::to_string(&wf)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::WorkflowResponse { workflow_json: json }))
    }

    async fn terminate_workflow(
        &self,
        request: Request<pb::TerminateWorkflowRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let req = request.into_inner();
        self.engine
            .terminate_workflow(&req.workflow_id, opt(&req.reason))
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn delete_workflow(
        &self,
        request: Request<pb::DeleteWorkflowRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let req = request.into_inner();
        self.engine
            .delete_workflow(&req.workflow_id, req.archive_workflow)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn pause_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .pause_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn resume_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .resume_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn restart_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .restart_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn retry_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .retry_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn rerun_workflow(
        &self,
        request: Request<pb::RerunWorkflowRequest>,
    ) -> Result<Response<pb::StartWorkflowResponse>, Status> {
        let inner = request.into_inner();
        let req: models::RerunWorkflowRequest =
            serde_json::from_str(&inner.rerun_request_json)
                .map_err(|e| Status::invalid_argument(format!("Invalid RerunWorkflowRequest JSON: {e}")))?;
        let wf_id = self
            .engine
            .rerun_workflow(&inner.workflow_id, &req)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::StartWorkflowResponse { workflow_id: wf_id }))
    }

    async fn decide_workflow(
        &self,
        request: Request<pb::WorkflowIdRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .decide_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn skip_task(
        &self,
        request: Request<pb::SkipTaskRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let inner = request.into_inner();
        let skip_req: models::SkipTaskRequest =
            serde_json::from_str(&inner.skip_request_json)
                .map_err(|e| Status::invalid_argument(format!("Invalid SkipTaskRequest JSON: {e}")))?;
        self.engine
            .skip_task(&inner.workflow_id, &inner.task_reference_name, &skip_req)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn get_workflow_stats(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::WorkflowStatsResponse>, Status> {
        let stats = self.engine.workflow_stats().await.map_err(engine_err_to_status)?;
        Ok(Response::new(pb::WorkflowStatsResponse { stats }))
    }

    async fn search_workflows(
        &self,
        request: Request<pb::SearchWorkflowsRequest>,
    ) -> Result<Response<pb::WorkflowSearchResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .search_workflows(
                opt(&req.status),
                opt(&req.workflow_type),
                opt(&req.free_text),
                req.start,
                req.size,
            )
            .await
            .map_err(engine_err_to_status)?;
        let jsons: Result<Vec<String>, _> =
            result.results.iter().map(|r| serde_json::to_string(r)).collect();
        let jsons = jsons.map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::WorkflowSearchResponse {
            total_hits: result.total_hits,
            results_json: jsons,
        }))
    }

    async fn get_running_workflows(
        &self,
        request: Request<pb::GetRunningWorkflowsRequest>,
    ) -> Result<Response<pb::RunningWorkflowsResponse>, Status> {
        let req = request.into_inner();
        let version = if req.version == 0 { None } else { Some(req.version) };
        let start_time = if req.start_time == 0 { None } else { Some(req.start_time) };
        let end_time = if req.end_time == 0 { None } else { Some(req.end_time) };
        let ids = self
            .engine
            .get_running_workflows(&req.name, version, start_time, end_time)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::RunningWorkflowsResponse { workflow_ids: ids }))
    }
}

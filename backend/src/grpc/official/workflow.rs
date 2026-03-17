use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use crate::grpc::metadata::engine_err_to_status;
use crate::grpc::proto_conv;
use super::pb;

pub struct OfficialWorkflowServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl OfficialWorkflowServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

fn opt(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

#[tonic::async_trait]
impl pb::workflow_service_server::WorkflowService for OfficialWorkflowServiceImpl {
    async fn start_workflow(
        &self,
        request: Request<pb::OfStartWorkflowRequest>,
    ) -> Result<Response<pb::OfStartWorkflowResponse>, Status> {
        let req = proto_conv::start_workflow_from_official(request.into_inner());
        let wf_id = self.engine.start_workflow(&req).await.map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfStartWorkflowResponse { workflow_id: wf_id }))
    }

    async fn get_workflows(
        &self,
        request: Request<pb::OfGetWorkflowRequest>,
    ) -> Result<Response<pb::OfGetWorkflowResponse>, Status> {
        let wf = self
            .engine
            .get_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfGetWorkflowResponse {
            workflow: Some(proto_conv::workflow_to_official(wf)),
        }))
    }

    async fn get_workflow_status(
        &self,
        request: Request<pb::OfGetWorkflowStatusRequest>,
    ) -> Result<Response<pb::OfGetWorkflowStatusResponse>, Status> {
        let wf = self
            .engine
            .get_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfGetWorkflowStatusResponse {
            workflow: Some(proto_conv::workflow_to_official(wf)),
        }))
    }

    async fn remove_workflow(
        &self,
        request: Request<pb::OfRemoveWorkflowRequest>,
    ) -> Result<Response<pb::OfRemoveWorkflowResponse>, Status> {
        let req = request.into_inner();
        self.engine
            .delete_workflow(&req.workflow_id, req.archive_workflow)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfRemoveWorkflowResponse {}))
    }

    async fn get_running_workflows(
        &self,
        request: Request<pb::OfGetRunningWorkflowsRequest>,
    ) -> Result<Response<pb::OfGetRunningWorkflowsResponse>, Status> {
        let req = request.into_inner();
        let version = if req.version == 0 { None } else { Some(req.version) };
        let start_time = if req.start_time == 0 { None } else { Some(req.start_time) };
        let end_time = if req.end_time == 0 { None } else { Some(req.end_time) };
        let ids = self
            .engine
            .get_running_workflows(&req.name, version, start_time, end_time)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfGetRunningWorkflowsResponse { workflow_ids: ids }))
    }

    async fn decide_workflow(
        &self,
        request: Request<pb::OfDecideWorkflowRequest>,
    ) -> Result<Response<pb::OfDecideWorkflowResponse>, Status> {
        self.engine
            .decide_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfDecideWorkflowResponse {}))
    }

    async fn pause_workflow(
        &self,
        request: Request<pb::OfPauseWorkflowRequest>,
    ) -> Result<Response<pb::OfPauseWorkflowResponse>, Status> {
        self.engine
            .pause_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfPauseWorkflowResponse {}))
    }

    async fn resume_workflow(
        &self,
        request: Request<pb::OfResumeWorkflowRequest>,
    ) -> Result<Response<pb::OfResumeWorkflowResponse>, Status> {
        self.engine
            .resume_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfResumeWorkflowResponse {}))
    }

    async fn skip_task_from_workflow(
        &self,
        request: Request<pb::OfSkipTaskRequest>,
    ) -> Result<Response<pb::OfSkipTaskResponse>, Status> {
        let inner = request.into_inner();
        let skip_req = proto_conv::skip_task_from_official(&inner);
        self.engine
            .skip_task(&inner.workflow_id, &inner.task_reference_name, &skip_req)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfSkipTaskResponse {}))
    }

    async fn rerun_workflow(
        &self,
        request: Request<pb::OfRerunWorkflowRequest>,
    ) -> Result<Response<pb::OfRerunWorkflowResponse>, Status> {
        let inner = request.into_inner();
        let req = proto_conv::rerun_from_official(&inner);
        let wf_id = self
            .engine
            .rerun_workflow(&inner.workflow_id, &req)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfRerunWorkflowResponse { workflow_id: wf_id }))
    }

    async fn restart_workflow(
        &self,
        request: Request<pb::OfRestartWorkflowRequest>,
    ) -> Result<Response<pb::OfRestartWorkflowResponse>, Status> {
        self.engine
            .restart_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfRestartWorkflowResponse {}))
    }

    async fn retry_workflow(
        &self,
        request: Request<pb::OfRetryWorkflowRequest>,
    ) -> Result<Response<pb::OfRetryWorkflowResponse>, Status> {
        self.engine
            .retry_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfRetryWorkflowResponse {}))
    }

    async fn reset_workflow_callbacks(
        &self,
        request: Request<pb::OfResetCallbacksRequest>,
    ) -> Result<Response<pb::OfResetCallbacksResponse>, Status> {
        self.engine
            .decide_workflow(&request.into_inner().workflow_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfResetCallbacksResponse {}))
    }

    async fn terminate_workflow(
        &self,
        request: Request<pb::OfTerminateWorkflowRequest>,
    ) -> Result<Response<pb::OfTerminateWorkflowResponse>, Status> {
        let req = request.into_inner();
        self.engine
            .terminate_workflow(&req.workflow_id, opt(&req.reason))
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfTerminateWorkflowResponse {}))
    }

    async fn search(
        &self,
        request: Request<pb::OfSearchWorkflowsRequest>,
    ) -> Result<Response<pb::OfSearchWorkflowsResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .search_workflows(
                None,
                None,
                opt(&req.free_text),
                req.start,
                req.size,
            )
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfSearchWorkflowsResponse {
            total_hits: result.total_hits,
            results: result.results.iter().map(proto_conv::workflow_summary_to_official).collect(),
        }))
    }

    async fn search_v2(
        &self,
        request: Request<pb::OfSearchWorkflowsRequest>,
    ) -> Result<Response<pb::OfSearchWorkflowsResponse>, Status> {
        self.search(request).await
    }

    async fn search_by_tasks(
        &self,
        request: Request<pb::OfSearchByTasksRequest>,
    ) -> Result<Response<pb::OfSearchByTasksResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .search_tasks(
                None,
                None,
                None,
                opt(&req.free_text),
                req.start,
                req.size,
            )
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfSearchByTasksResponse {
            total_hits: result.total_hits,
            results: result.results.iter().map(proto_conv::task_summary_to_official).collect(),
        }))
    }

    async fn search_by_tasks_v2(
        &self,
        request: Request<pb::OfSearchByTasksRequest>,
    ) -> Result<Response<pb::OfSearchByTasksResponse>, Status> {
        self.search_by_tasks(request).await
    }
}

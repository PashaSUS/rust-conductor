use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use crate::grpc::metadata::engine_err_to_status;
use crate::grpc::proto_conv;
use super::pb;

pub struct OfficialTaskServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl OfficialTaskServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

fn opt(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

#[tonic::async_trait]
impl pb::task_service_server::TaskService for OfficialTaskServiceImpl {
    async fn poll(
        &self,
        request: Request<pb::OfPollRequest>,
    ) -> Result<Response<pb::OfPollResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .poll_task(&req.task_type, opt(&req.worker_id))
            .await
            .map_err(engine_err_to_status)?;
        match result {
            Some(task) => Ok(Response::new(pb::OfPollResponse {
                found: true,
                task: Some(proto_conv::poll_task_to_official(task)),
            })),
            None => Ok(Response::new(pb::OfPollResponse {
                found: false,
                task: None,
            })),
        }
    }

    async fn batch_poll(
        &self,
        request: Request<pb::OfBatchPollRequest>,
    ) -> Result<Response<pb::OfBatchPollResponse>, Status> {
        let req = request.into_inner();
        let tasks = self
            .engine
            .batch_poll_tasks(
                &req.task_type,
                opt(&req.worker_id),
                req.count as usize,
                req.timeout as u64,
            )
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfBatchPollResponse {
            tasks: tasks.into_iter().map(proto_conv::poll_task_to_official).collect(),
        }))
    }

    async fn get_task(
        &self,
        request: Request<pb::OfGetTaskRequest>,
    ) -> Result<Response<pb::OfGetTaskResponse>, Status> {
        let task = self
            .engine
            .get_task(&request.into_inner().task_id)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfGetTaskResponse {
            task: Some(proto_conv::task_result_to_official(task)),
        }))
    }

    async fn update_task(
        &self,
        request: Request<pb::OfUpdateTaskRequest>,
    ) -> Result<Response<pb::OfUpdateTaskResponse>, Status> {
        let update = proto_conv::task_update_from_official(request.into_inner())?;
        let task_id = self
            .engine
            .update_task(&update)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfUpdateTaskResponse { task_id }))
    }

    async fn ack_task(
        &self,
        request: Request<pb::OfAckTaskRequest>,
    ) -> Result<Response<pb::OfAckTaskResponse>, Status> {
        let req = request.into_inner();
        let acked = self
            .engine
            .ack_task(&req.task_id, opt(&req.worker_id))
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfAckTaskResponse { acknowledged: acked }))
    }

    async fn add_log(
        &self,
        request: Request<pb::OfAddLogRequest>,
    ) -> Result<Response<pb::OfAddLogResponse>, Status> {
        let req = request.into_inner();
        self.engine
            .add_task_log(&req.task_id, &req.log)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfAddLogResponse {}))
    }

    async fn get_task_logs(
        &self,
        request: Request<pb::OfGetTaskLogsRequest>,
    ) -> Result<Response<pb::OfGetTaskLogsResponse>, Status> {
        let logs = self
            .engine
            .get_task_logs(&request.into_inner().task_id)
            .await
            .map_err(engine_err_to_status)?;
        let proto_logs: Vec<pb::OfTaskExecLog> = logs
            .into_iter()
            .map(|l| pb::OfTaskExecLog {
                log: l.log,
                task_id: l.task_id,
                created_time: l.created_time.unwrap_or(0),
            })
            .collect();
        Ok(Response::new(pb::OfGetTaskLogsResponse { logs: proto_logs }))
    }

    async fn search(
        &self,
        request: Request<pb::OfSearchTasksRequest>,
    ) -> Result<Response<pb::OfSearchTasksResponse>, Status> {
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
        Ok(Response::new(pb::OfSearchTasksResponse {
            total_hits: result.total_hits,
            results: result.results.iter().map(proto_conv::task_summary_to_official).collect(),
        }))
    }

    async fn search_v2(
        &self,
        request: Request<pb::OfSearchTasksRequest>,
    ) -> Result<Response<pb::OfSearchTasksResponse>, Status> {
        self.search(request).await
    }
}

use std::sync::Arc;
use tonic::{Request, Response, Status};

use crate::engine::WorkflowEngine;
use crate::models;
use super::pb;
use super::metadata::engine_err_to_status;

pub struct TaskServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl TaskServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

fn opt(s: &str) -> Option<&str> {
    if s.is_empty() { None } else { Some(s) }
}

#[tonic::async_trait]
impl pb::task_service_server::TaskService for TaskServiceImpl {
    async fn update_task(
        &self,
        request: Request<pb::UpdateTaskRequest>,
    ) -> Result<Response<pb::UpdateTaskResponse>, Status> {
        let update: models::TaskUpdateRequest =
            serde_json::from_str(&request.into_inner().task_update_json)
                .map_err(|e| Status::invalid_argument(format!("Invalid TaskUpdateRequest JSON: {e}")))?;
        let task_id = self
            .engine
            .update_task(&update)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::UpdateTaskResponse { task_id }))
    }

    async fn get_task(
        &self,
        request: Request<pb::GetTaskRequest>,
    ) -> Result<Response<pb::TaskResultResponse>, Status> {
        let task = self
            .engine
            .get_task(&request.into_inner().task_id)
            .await
            .map_err(engine_err_to_status)?;
        let json = serde_json::to_string(&task)
            .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::TaskResultResponse {
            task_result_json: json,
        }))
    }

    async fn poll_task(
        &self,
        request: Request<pb::PollTaskRequest>,
    ) -> Result<Response<pb::PollTaskResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .poll_task(&req.task_type, opt(&req.worker_id))
            .await
            .map_err(engine_err_to_status)?;
        match result {
            Some(task) => {
                let json = serde_json::to_string(&task)
                    .map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
                Ok(Response::new(pb::PollTaskResponse {
                    found: true,
                    poll_task_json: json,
                }))
            }
            None => Ok(Response::new(pb::PollTaskResponse {
                found: false,
                poll_task_json: String::new(),
            })),
        }
    }

    async fn batch_poll_tasks(
        &self,
        request: Request<pb::BatchPollTasksRequest>,
    ) -> Result<Response<pb::BatchPollTasksResponse>, Status> {
        let req = request.into_inner();
        let tasks = self
            .engine
            .batch_poll_tasks(
                &req.task_type,
                opt(&req.worker_id),
                req.count as usize,
                req.timeout_ms,
            )
            .await
            .map_err(engine_err_to_status)?;
        let jsons: Result<Vec<String>, _> =
            tasks.iter().map(|t| serde_json::to_string(t)).collect();
        let jsons = jsons.map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::BatchPollTasksResponse {
            poll_tasks_json: jsons,
        }))
    }

    async fn ack_task(
        &self,
        request: Request<pb::AckTaskRequest>,
    ) -> Result<Response<pb::AckTaskResponse>, Status> {
        let req = request.into_inner();
        let acked = self
            .engine
            .ack_task(&req.task_id, opt(&req.worker_id))
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::AckTaskResponse { acknowledged: acked }))
    }

    async fn get_task_logs(
        &self,
        request: Request<pb::GetTaskLogsRequest>,
    ) -> Result<Response<pb::TaskLogsResponse>, Status> {
        let logs = self
            .engine
            .get_task_logs(&request.into_inner().task_id)
            .await
            .map_err(engine_err_to_status)?;
        let proto_logs: Vec<pb::TaskExecLogProto> = logs
            .into_iter()
            .map(|l| pb::TaskExecLogProto {
                log: l.log,
                task_id: l.task_id,
                created_time: l.created_time.unwrap_or(0),
            })
            .collect();
        Ok(Response::new(pb::TaskLogsResponse { logs: proto_logs }))
    }

    async fn add_task_log(
        &self,
        request: Request<pb::AddTaskLogRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        let req = request.into_inner();
        self.engine
            .add_task_log(&req.task_id, &req.log)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn search_tasks(
        &self,
        request: Request<pb::SearchTasksRequest>,
    ) -> Result<Response<pb::TaskSearchResponse>, Status> {
        let req = request.into_inner();
        let result = self
            .engine
            .search_tasks(
                opt(&req.workflow_id),
                opt(&req.task_type),
                opt(&req.status),
                opt(&req.free_text),
                req.start,
                req.size,
            )
            .await
            .map_err(engine_err_to_status)?;
        let jsons: Result<Vec<String>, _> =
            result.results.iter().map(|r| serde_json::to_string(r)).collect();
        let jsons = jsons.map_err(|e| Status::internal(format!("Serialization error: {e}")))?;
        Ok(Response::new(pb::TaskSearchResponse {
            total_hits: result.total_hits,
            results_json: jsons,
        }))
    }

    async fn get_queue_sizes(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::QueueSizesResponse>, Status> {
        let sizes = self
            .engine
            .get_queue_sizes()
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::QueueSizesResponse { sizes }))
    }
}

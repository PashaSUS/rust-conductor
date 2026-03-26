use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::metadata::engine_err_to_status;
use super::pb;
use super::proto_conv;
use crate::engine::WorkflowEngine;

pub struct EventServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl EventServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl pb::event_service_server::EventService for EventServiceImpl {
    async fn register_event_handler(
        &self,
        request: Request<pb::EventHandlerPb>,
    ) -> Result<Response<pb::Empty>, Status> {
        let handler = proto_conv::event_handler_from_proto(&request.into_inner());
        self.engine
            .register_event_handler(&handler)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn update_event_handler(
        &self,
        request: Request<pb::EventHandlerPb>,
    ) -> Result<Response<pb::Empty>, Status> {
        let handler = proto_conv::event_handler_from_proto(&request.into_inner());
        self.engine
            .register_event_handler(&handler)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }

    async fn list_event_handlers(
        &self,
        _request: Request<pb::Empty>,
    ) -> Result<Response<pb::ListEventHandlersResponse>, Status> {
        let handlers = self
            .engine
            .get_event_handlers()
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::ListEventHandlersResponse {
            event_handlers: handlers
                .iter()
                .map(proto_conv::event_handler_to_proto)
                .collect(),
        }))
    }

    async fn get_event_handlers_for_event(
        &self,
        request: Request<pb::GetEventHandlersRequest>,
    ) -> Result<Response<pb::ListEventHandlersResponse>, Status> {
        let req = request.into_inner();
        let handlers = self
            .engine
            .get_event_handlers_for_event(&req.event, req.active_only)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::ListEventHandlersResponse {
            event_handlers: handlers
                .iter()
                .map(proto_conv::event_handler_to_proto)
                .collect(),
        }))
    }

    async fn delete_event_handler(
        &self,
        request: Request<pb::DeleteEventHandlerRequest>,
    ) -> Result<Response<pb::Empty>, Status> {
        self.engine
            .delete_event_handler(&request.into_inner().name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::Empty {}))
    }
}

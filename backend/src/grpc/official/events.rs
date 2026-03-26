use std::sync::Arc;
use tonic::{Request, Response, Status};

use super::pb;
use crate::engine::WorkflowEngine;
use crate::grpc::metadata::engine_err_to_status;
use crate::grpc::proto_conv;

pub struct OfficialEventServiceImpl {
    engine: Arc<WorkflowEngine>,
}

impl OfficialEventServiceImpl {
    pub fn new(engine: Arc<WorkflowEngine>) -> Self {
        Self { engine }
    }
}

#[tonic::async_trait]
impl pb::event_service_server::EventService for OfficialEventServiceImpl {
    async fn add_event_handler(
        &self,
        request: Request<pb::OfAddEventHandlerRequest>,
    ) -> Result<Response<pb::OfAddEventHandlerResponse>, Status> {
        let inner = request.into_inner();
        let eh = inner
            .event_handler
            .ok_or_else(|| Status::invalid_argument("Missing event_handler"))?;
        let handler = proto_conv::event_handler_from_official(&eh);
        self.engine
            .register_event_handler(&handler)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfAddEventHandlerResponse {}))
    }

    async fn update_event_handler(
        &self,
        request: Request<pb::OfUpdateEventHandlerRequest>,
    ) -> Result<Response<pb::OfUpdateEventHandlerResponse>, Status> {
        let inner = request.into_inner();
        let eh = inner
            .event_handler
            .ok_or_else(|| Status::invalid_argument("Missing event_handler"))?;
        let handler = proto_conv::event_handler_from_official(&eh);
        self.engine
            .register_event_handler(&handler)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfUpdateEventHandlerResponse {}))
    }

    async fn remove_event_handler(
        &self,
        request: Request<pb::OfRemoveEventHandlerRequest>,
    ) -> Result<Response<pb::OfRemoveEventHandlerResponse>, Status> {
        self.engine
            .delete_event_handler(&request.into_inner().name)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfRemoveEventHandlerResponse {}))
    }

    async fn get_event_handlers(
        &self,
        request: Request<pb::OfGetEventHandlersRequest>,
    ) -> Result<Response<pb::OfGetEventHandlersResponse>, Status> {
        let req = request.into_inner();
        let handlers = if req.event.is_empty() {
            self.engine
                .get_event_handlers()
                .await
                .map_err(engine_err_to_status)?
        } else {
            self.engine
                .get_event_handlers_for_event(&req.event, req.active_only)
                .await
                .map_err(engine_err_to_status)?
        };
        Ok(Response::new(pb::OfGetEventHandlersResponse {
            event_handlers: handlers
                .iter()
                .map(proto_conv::event_handler_to_official)
                .collect(),
        }))
    }

    async fn get_event_handlers_for_event(
        &self,
        request: Request<pb::OfGetEventHandlersForEventRequest>,
    ) -> Result<Response<pb::OfGetEventHandlersForEventResponse>, Status> {
        let req = request.into_inner();
        let handlers = self
            .engine
            .get_event_handlers_for_event(&req.event, req.active_only)
            .await
            .map_err(engine_err_to_status)?;
        Ok(Response::new(pb::OfGetEventHandlersForEventResponse {
            event_handlers: handlers
                .iter()
                .map(proto_conv::event_handler_to_official)
                .collect(),
        }))
    }
}

pub mod metadata;
pub mod workflow;
pub mod tasks;
pub mod events;
pub mod admin;
pub mod bulk;

pub mod pb {
    tonic::include_proto!("conductor");
}

use crate::engine::WorkflowEngine;
use std::sync::Arc;

/// Build a tonic Router containing all Conductor gRPC services.
pub fn grpc_router(engine: Arc<WorkflowEngine>) -> tonic::transport::server::Router {
    use pb::metadata_service_server::MetadataServiceServer;
    use pb::workflow_service_server::WorkflowServiceServer;
    use pb::task_service_server::TaskServiceServer;
    use pb::event_service_server::EventServiceServer;
    use pb::admin_service_server::AdminServiceServer;
    use pb::bulk_service_server::BulkServiceServer;

    tonic::transport::Server::builder()
        .add_service(MetadataServiceServer::new(metadata::MetadataServiceImpl::new(engine.clone())))
        .add_service(WorkflowServiceServer::new(workflow::WorkflowServiceImpl::new(engine.clone())))
        .add_service(TaskServiceServer::new(tasks::TaskServiceImpl::new(engine.clone())))
        .add_service(EventServiceServer::new(events::EventServiceImpl::new(engine.clone())))
        .add_service(AdminServiceServer::new(admin::AdminServiceImpl::new(engine.clone())))
        .add_service(BulkServiceServer::new(bulk::BulkServiceImpl::new(engine)))
}

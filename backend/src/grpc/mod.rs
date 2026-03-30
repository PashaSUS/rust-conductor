pub mod admin;
pub mod bulk;
pub mod events;
pub mod metadata;
pub mod official;
pub mod proto_conv;
pub mod proto_conv_official;
pub mod tasks;
pub mod workflow;

pub mod pb {
    tonic::include_proto!("conductor");

    /// File descriptor set for gRPC reflection
    #[cfg(feature = "grpc-reflection")]
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("conductor_descriptor");
}

use crate::engine::WorkflowEngine;
use std::sync::Arc;
use std::time::Duration;
use tonic::codec::CompressionEncoding;

/// Build a tonic Router containing all Conductor gRPC services.
///
/// Registers both the custom `conductor.*` services and the official
/// `conductor.grpc.*` services so clients using either contract are served.
pub fn grpc_router(
    engine: Arc<WorkflowEngine>,
) -> tonic::transport::server::Router<
    tower::layer::util::Stack<tower::limit::ConcurrencyLimitLayer, tower::layer::util::Identity>,
> {
    // Custom services (package: conductor)
    use pb::admin_service_server::AdminServiceServer;
    use pb::bulk_service_server::BulkServiceServer;
    use pb::event_service_server::EventServiceServer;
    use pb::metadata_service_server::MetadataServiceServer;
    use pb::task_service_server::TaskServiceServer;
    use pb::workflow_service_server::WorkflowServiceServer;

    // Official Conductor-compatible services (package: conductor.grpc)
    use official::pb::event_service_server::EventServiceServer as OfficialEventServiceServer;
    use official::pb::metadata_service_server::MetadataServiceServer as OfficialMetadataServiceServer;
    use official::pb::task_service_server::TaskServiceServer as OfficialTaskServiceServer;
    use official::pb::workflow_service_server::WorkflowServiceServer as OfficialWorkflowServiceServer;

    #[cfg(feature = "grpc-reflection")]
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(pb::FILE_DESCRIPTOR_SET)
        .build_v1()
        .expect("Failed to build gRPC reflection service");

    let server = tonic::transport::Server::builder()
        .tcp_nodelay(true)
        .initial_connection_window_size(Some(16 * 1024 * 1024)) // 16 MB connection window
        .initial_stream_window_size(Some(4 * 1024 * 1024)) // 4 MB per-stream window
        .concurrency_limit_per_connection(1024)
        .tcp_keepalive(Some(Duration::from_secs(60)))
        .http2_keepalive_interval(Some(Duration::from_secs(30)))
        .http2_keepalive_timeout(Some(Duration::from_secs(20)))
        .layer(tower::limit::ConcurrencyLimitLayer::new(512))
        // Custom services
        .add_service(
            MetadataServiceServer::new(metadata::MetadataServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            WorkflowServiceServer::new(workflow::WorkflowServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            TaskServiceServer::new(tasks::TaskServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            EventServiceServer::new(events::EventServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            AdminServiceServer::new(admin::AdminServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            BulkServiceServer::new(bulk::BulkServiceImpl::new(engine.clone()))
                .send_compressed(CompressionEncoding::Gzip)
                .accept_compressed(CompressionEncoding::Gzip),
        )
        // Official Conductor-compatible services
        .add_service(
            OfficialMetadataServiceServer::new(
                official::metadata::OfficialMetadataServiceImpl::new(engine.clone()),
            )
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            OfficialWorkflowServiceServer::new(
                official::workflow::OfficialWorkflowServiceImpl::new(engine.clone()),
            )
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            OfficialTaskServiceServer::new(official::tasks::OfficialTaskServiceImpl::new(
                engine.clone(),
            ))
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip),
        )
        .add_service(
            OfficialEventServiceServer::new(official::events::OfficialEventServiceImpl::new(
                engine,
            ))
            .send_compressed(CompressionEncoding::Gzip)
            .accept_compressed(CompressionEncoding::Gzip),
        );
    // gRPC reflection for dynamic client discovery (#164)
    #[cfg(feature = "grpc-reflection")]
    let server = server.add_service(reflection_service);

    server
}

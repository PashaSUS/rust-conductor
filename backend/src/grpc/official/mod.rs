pub mod events;
pub mod metadata;
pub mod tasks;
pub mod workflow;

pub mod pb {
    tonic::include_proto!("conductor.grpc");
}

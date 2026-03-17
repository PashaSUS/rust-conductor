pub mod metadata;
pub mod workflow;
pub mod tasks;
pub mod events;

pub mod pb {
    tonic::include_proto!("conductor.grpc");
}

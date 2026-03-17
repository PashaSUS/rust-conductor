fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["proto/conductor.proto", "proto/conductor_official.proto"],
            &["proto"],
        )?;
    Ok(())
}

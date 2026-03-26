fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Skip proto compilation if protoc is not available (e.g. in test-only builds).
    // The generated code lives in src/grpc/ and does not need regeneration
    // unless the .proto files change.
    let has_protoc = std::env::var("PROTOC")
        .map(|p| std::path::Path::new(&p).exists())
        .unwrap_or(false)
        || std::process::Command::new("protoc")
            .arg("--version")
            .output()
            .is_ok_and(|o| o.status.success());

    if std::env::var("SKIP_PROTO").is_ok() || !has_protoc {
        println!(
            "cargo:warning=Skipping protobuf compilation (protoc not found or SKIP_PROTO set)"
        );
        let out_dir = std::env::var("OUT_DIR").unwrap();
        let out_path = std::path::Path::new(&out_dir);
        for name in ["conductor", "conductor_official"] {
            let file = out_path.join(format!("{name}.rs"));
            if !file.exists() {
                std::fs::write(&file, "// proto stub — protoc not available\n")?;
            }
        }
        return Ok(());
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let descriptor_path = std::path::Path::new(&out_dir).join("conductor_descriptor.bin");

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(&descriptor_path)
        .compile_protos(
            &["proto/conductor.proto", "proto/conductor_official.proto"],
            &["proto"],
        )?;
    Ok(())
}

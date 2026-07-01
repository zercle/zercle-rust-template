fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = &["./proto/example/v1/example.proto"];
    let includes = &["./proto"];
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(proto_files, includes)?;
    println!("cargo::rerun-if-changed=proto/example/v1/example.proto");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_files = &[
        "./proto/catalog/v1/catalog.proto",
        "./proto/machines/v1/machines.proto",
        "./proto/sales/v1/sales.proto",
        "./proto/reporting/v1/reporting.proto",
    ];
    let includes = &["./proto"];
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(proto_files, includes)?;
    for proto in proto_files {
        println!("cargo::rerun-if-changed={proto}");
    }
    Ok(())
}

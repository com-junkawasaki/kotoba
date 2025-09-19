fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf files if they exist
    let proto_file = "src/cluster.proto";
    if std::path::Path::new(proto_file).exists() {
        println!("cargo:rerun-if-changed={}", proto_file);
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .compile(&[proto_file], &["src"])?;
    } else {
        println!("cargo:warning=cluster.proto not found, skipping protobuf generation");
    }

    Ok(())
}

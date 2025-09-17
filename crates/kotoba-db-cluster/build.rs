fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf files
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["src/cluster.proto"], &["src"])?;

    println!("cargo:rerun-if-changed=src/cluster.proto");
    Ok(())
}

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf files
    let proto_file = "src/cluster.proto";
    let out_dir = "src";

    if Path::new(proto_file).exists() {
        tonic_build::configure()
            .build_server(true)
            .build_client(true)
            .out_dir(out_dir)
            .compile(&[proto_file], &[out_dir])?;

        println!("cargo:rerun-if-changed={}", proto_file);
    }

    Ok(())
}

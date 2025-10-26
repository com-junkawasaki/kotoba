use fcdb_cas::PackCAS;
use fcdb_graph::GraphDB;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing FCDB API...");

    // Based on FCDB documentation, PackCAS likely needs a path
    println!("Testing PackCAS initialization with path...");

    let cas = PackCAS::new(std::path::PathBuf::from("./test_fcdb_db")).await?;
    println!("PackCAS created with path");

    // GraphDB::new should return GraphDB directly, not Result
    println!("Testing GraphDB initialization...");
    let graph = GraphDB::new(cas).await;
    println!("GraphDB created");

    // Test create_node - should return some kind of ID
    let data = b"Hello FCDB";
    let node_id = graph.create_node(data).await?;
    println!("Created node with ID: {:?}", node_id);

    Ok(())
}

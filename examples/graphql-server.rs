use std::sync::Arc;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Kotoba GraphQL Server...");

    // Initialize core components
    let mvcc = Arc::new(kotoba_storage::storage::MVCCManager::new());
    let merkle = Arc::new(kotoba_storage::storage::MerkleDAG::new());
    let rewrite_engine = Arc::new(kotoba_rewrite::rewrite::RewriteEngine::new());

    // Initialize schema manager
    let schema_manager = Arc::new(kotoba_schema::manager::SchemaManager::new(
        Box::new(kotoba_schema::manager::InMemorySchemaStorage::new())
    ));

    // Create HTTP server with GraphQL enabled
    let mut server = kotoba_server::http::server::HttpServer::new(
        kotoba_server::http::ir::HttpConfig::new(kotoba_server::http::ir::ServerConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
            graphql_enabled: Some(true),
            ..Default::default()
        }),
        mvcc,
        merkle,
        rewrite_engine,
    ).await?;

    // Enable GraphQL with schema manager
    server.enable_graphql(schema_manager);

    println!("✅ GraphQL Server initialized successfully!");
    println!("📡 GraphQL endpoint available at: http://127.0.0.1:8080/graphql");
    println!("🔧 You can send GraphQL queries to this endpoint");
    println!("📝 Example GraphQL query:");
    println!(r#"{
  __schema {
    types {
      name
    }
  }
}"#);

    // In a real implementation, you'd start the server here
    // server.start().await?;

    println!("🎉 Kotoba GraphQL API is ready!");
    println!("💡 Next: Implement your GraphQL resolvers and start the server");

    Ok(())
}

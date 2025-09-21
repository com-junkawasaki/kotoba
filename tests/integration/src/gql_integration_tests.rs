//! Integration test for GraphQL API with Kotoba DB
//!
//! This test verifies that GraphQL API works correctly with Kotoba DB components.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::test;

use kotoba_schema::{SchemaManager, GraphSchema};
use kotoba_server::http::graphql::{create_schema, graphql_handler, SchemaContext};

/// Test GraphQL API integration with Kotoba DB
#[tokio::test]
async fn test_graphql_api_integration() {
    println!("🚀 Testing GraphQL API integration with Kotoba DB...");

    // Create schema manager for testing
    let schema_manager = Arc::new(SchemaManager::new());

    // Create a test schema
    let mut test_schema = GraphSchema::new(
        "test_schema".to_string(),
        "Test Schema".to_string(),
        "1.0.0".to_string()
    );
    test_schema.description = Some("Test schema for GraphQL integration".to_string());

    schema_manager.register_schema(test_schema)
        .expect("Failed to register test schema");

    println!("✅ Test schema registered");

    // Setup GraphQL schema
    let graphql_schema = create_schema(schema_manager.clone());

    // Test GraphQL query: Get all schemas
    let query = r#"
    {
        schemas {
            id
            name
            version
            description
        }
    }
    "#;

    let response = graphql_handler(&graphql_schema, query.to_string()).await
        .expect("GraphQL query failed");

    println!("📄 GraphQL Query Response:");
    println!("{}", response);

    // Parse and verify response
    let response_json: serde_json::Value = serde_json::from_str(&response)
        .expect("Failed to parse GraphQL response");

    assert!(response_json.get("data").is_some(), "Response should contain data");
    assert!(response_json["data"].get("schemas").is_some(), "Response should contain schemas");

    let schemas = &response_json["data"]["schemas"];
    assert!(schemas.is_array(), "Schemas should be an array");
    assert_eq!(schemas.as_array().unwrap().len(), 1, "Should have one schema");

    let schema = &schemas[0];
    assert_eq!(schema["id"], "test_schema", "Schema ID should match");
    assert_eq!(schema["name"], "Test Schema", "Schema name should match");
    assert_eq!(schema["version"], "1.0.0", "Schema version should match");

    println!("✅ GraphQL query for schemas executed successfully");

    // Test GraphQL mutation: Create a new schema
    let mutation = r#"
    mutation {
        createSchema(
            id: "user_schema",
            name: "User Schema",
            version: "1.0.0",
            description: "Schema for user data"
        ) {
            id
            name
            version
            description
        }
    }
    "#;

    let mutation_response = graphql_handler(&graphql_schema, mutation.to_string()).await
        .expect("GraphQL mutation failed");

    println!("📝 GraphQL Mutation Response:");
    println!("{}", mutation_response);

    // Verify mutation response
    let mutation_json: serde_json::Value = serde_json::from_str(&mutation_response)
        .expect("Failed to parse GraphQL mutation response");

    assert!(mutation_json.get("data").is_some(), "Mutation response should contain data");

    let created_schema = &mutation_json["data"]["createSchema"];
    assert_eq!(created_schema["id"], "user_schema", "Created schema ID should match");
    assert_eq!(created_schema["name"], "User Schema", "Created schema name should match");

    println!("✅ GraphQL mutation for schema creation executed successfully");

    // Test GraphQL query: Get the newly created schema
    let get_schema_query = r#"
    {
        schema(id: "user_schema") {
            id
            name
            version
            description
        }
    }
    "#;

    let schema_response = graphql_handler(&graphql_schema, get_schema_query.to_string()).await
        .expect("GraphQL get schema query failed");

    let schema_json: serde_json::Value = serde_json::from_str(&schema_response)
        .expect("Failed to parse schema query response");

    assert!(schema_json.get("data").is_some(), "Schema query response should contain data");

    let retrieved_schema = &schema_json["data"]["schema"];
    assert_eq!(retrieved_schema["id"], "user_schema", "Retrieved schema ID should match");
    assert_eq!(retrieved_schema["name"], "User Schema", "Retrieved schema name should match");

    println!("✅ GraphQL query for specific schema executed successfully");

    // Test health check
    let health_query = r#"{ schemaHealth }"#;

    let health_response = graphql_handler(&graphql_schema, health_query.to_string()).await
        .expect("GraphQL health check failed");

    let health_json: serde_json::Value = serde_json::from_str(&health_response)
        .expect("Failed to parse health response");

    assert!(health_json.get("data").is_some(), "Health response should contain data");
    assert_eq!(health_json["data"]["schemaHealth"], "Schema system is healthy", "Health check should pass");

    println!("✅ GraphQL health check executed successfully");

    println!("🎉 GraphQL API integration test completed successfully!");
    println!("📊 GraphQL API is working correctly with Kotoba DB.");
}
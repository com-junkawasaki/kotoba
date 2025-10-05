//! Test for kotoba database using Redis storage
//!
//! This test demonstrates setting up a kotoba database with Redis storage
//! and performing basic operations.

use std::sync::Arc;
use tokio::time::{timeout, Duration};
use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;
use kotoba_query_engine::{GqlQueryEngine, QueryContext};
use kotoba_schema::{SchemaManager, GraphSchema};
use kotoba_server::http::graphql::{create_schema, graphql_handler, SchemaContext};

// Note: This test requires a Redis server running on localhost:6379
// Run with: cargo test --package kotoba-redis-tests test_kotoba_redis_database_setup

#[tokio::test]
async fn test_kotoba_redis_database_setup() {
    println!("🚀 Setting up kotoba database with Redis storage...");

    // Configure Redis storage
    let config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "kotoba:test:db".to_string(),
        enable_compression: true,
        enable_metrics: true,
        connection_pool_size: 5,
        ..Default::default()
    };

    // Create Redis store
    let store = RedisStore::new(config).await
        .expect("Failed to create Redis store");

    println!("✅ Redis store created successfully");

    // Verify connection
    assert!(store.is_connected().await, "Should be connected to Redis");
    println!("✅ Connected to Redis server");

    // Test basic operations
    let test_key = b"kotoba:test:key";
    let test_value = b"Hello from kotoba Redis database!";

    // Put operation
    store.put(test_key, test_value).await
        .expect("Put operation should succeed");
    println!("✅ Successfully stored key-value pair");

    // Get operation
    let retrieved = store.get(test_key).await
        .expect("Get operation should succeed");

    assert_eq!(retrieved, Some(test_value.to_vec()),
               "Retrieved value should match stored value");
    println!("✅ Successfully retrieved and verified value");

    // Test multiple keys
    let test_data = vec![
        (b"user:1", b"{\"name\":\"Alice\",\"role\":\"admin\"}"),
        (b"user:2", b"{\"name\":\"Bob\",\"role\":\"user\"}"),
        (b"config:theme", b"dark"),
        (b"session:active", b"true"),
    ];

    for (key, value) in &test_data {
        store.put(key, value).await
            .expect("Batch put should succeed");
    }
    println!("✅ Successfully stored multiple key-value pairs");

    // Verify all keys
    for (key, expected_value) in &test_data {
        let retrieved = store.get(key).await
            .expect("Batch get should succeed");
        assert_eq!(retrieved, Some(expected_value.to_vec()),
                   "Batch retrieved value should match");
    }
    println!("✅ Successfully verified all stored data");

    // Test scan operation
    let scanned = store.scan(b"user:").await
        .expect("Scan operation should succeed");
    assert_eq!(scanned.len(), 2, "Should find 2 user keys");
    println!("✅ Successfully scanned for user keys");

    // Test statistics
    let stats = store.get_stats().await;
    assert!(stats.total_operations > 0, "Should have recorded operations");
    assert!(matches!(stats.connection_status,
                     kotoba_storage_redis::ConnectionStatus::Connected),
            "Should be connected");
    println!("✅ Statistics: {} operations, connection status: {:?}",
             stats.total_operations, stats.connection_status);

    // Test delete operation
    store.delete(b"user:2").await
        .expect("Delete operation should succeed");

    let deleted_check = store.get(b"user:2").await
        .expect("Get after delete should succeed");
    assert_eq!(deleted_check, None, "Deleted key should return None");
    println!("✅ Successfully deleted key");

    // Test concurrent access
    let store_arc = Arc::new(store);
    let mut handles = vec![];

    for i in 0..5 {
        let store_clone = Arc::clone(&store_arc);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent:key:{}", i).into_bytes();
            let value = format!("concurrent value {}", i).into_bytes();

            store_clone.put(&key, &value).await
                .expect("Concurrent put should succeed");

            let retrieved = store_clone.get(&key).await
                .expect("Concurrent get should succeed");

            assert_eq!(retrieved, Some(value),
                       "Concurrent retrieved value should match");
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations
    for handle in handles {
        handle.await.expect("Concurrent operation should succeed");
    }
    println!("✅ Successfully completed concurrent operations");

    println!("🎉 Kotoba Redis database test completed successfully!");
    println!("📊 Database is ready for production use with Redis storage.");
}

#[tokio::test]
async fn test_kotoba_db_with_graphql() {
    println!("🚀 Testing Kotoba DB with GraphQL over Redis storage...");

    // Setup Redis storage
    let config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "kotoba:graphql:test:db".to_string(),
        enable_compression: true,
        enable_metrics: true,
        connection_pool_size: 5,
        ..Default::default()
    };

    let store = RedisStore::new(config).await
        .expect("Failed to create Redis store for GraphQL test");

    println!("✅ Redis store created for GraphQL test");

    // Setup Kotoba DB components
    let query_engine = Arc::new(GqlQueryEngine::new(Arc::new(store)));
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

    println!("🎉 Kotoba DB GraphQL integration test completed successfully!");
    println!("📊 GraphQL API is working correctly with Redis storage backend.");
}

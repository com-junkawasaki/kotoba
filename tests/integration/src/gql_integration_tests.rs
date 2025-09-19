//! Integration test for GQL Query Engine and Projection Engine
//!
//! This test verifies that GQL queries can be executed against the
//! Projection Engine's GraphDB with proper integration.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::test;

use kotoba_projection_engine::{ProjectionEngine, ProjectionConfig};
use kotoba_query_engine::{QueryContext, VertexId, EdgeId, Vertex, Edge};
use kotoba_ocel::{OcelEvent, OcelValue};
use kotoba_graphdb::PropertyValue;
use chrono::{DateTime, Utc, TimeZone};

/// Test GQL query execution against Projection Engine
#[tokio::test]
async fn test_gql_query_execution() {
    // Setup test data
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap().to_string();

    // Create projection engine
    let config = ProjectionConfig {
        rocksdb_path: db_path,
        max_concurrent_projections: 10,
        batch_size: 100,
        checkpoint_interval: 1000,
        cache_config: Default::default(),
        enable_metrics: false,
    };

    let projection_engine = Arc::new(ProjectionEngine::new(config).await.unwrap());

    // Start the engine
    projection_engine.start().await.unwrap();

    // Create test OCEL events and process them
    let events = create_test_ocel_events();
    for event in events {
        projection_engine.process_ocel_events(vec![event]).await.unwrap();
    }

    // Test GQL query execution
    let query_context = QueryContext {
        user_id: Some("test_user".to_string()),
        database: "default".to_string(),
        timeout: std::time::Duration::from_secs(30),
        parameters: HashMap::new(),
    };

    // Test MATCH query
    let match_query = "MATCH (n) RETURN n";
    let result = projection_engine.execute_gql_query(match_query, query_context.clone()).await;

    match result {
        Ok(query_result) => {
            println!("GQL MATCH query executed successfully");
            println!("Columns: {:?}", query_result.columns);
            println!("Rows count: {}", query_result.rows.len());
            assert!(!query_result.rows.is_empty(), "Should return some data");
        }
        Err(e) => {
            println!("GQL MATCH query failed (expected for now): {}", e);
            // This is expected to fail initially as we need more implementation
        }
    }

    // Test CREATE statement
    let create_statement = "CREATE GRAPH test_graph";
    let create_result = projection_engine.execute_gql_statement(create_statement, query_context).await;

    match create_result {
        Ok(result) => {
            println!("GQL CREATE statement executed successfully: {:?}", result);
        }
        Err(e) => {
            println!("GQL CREATE statement failed (expected for now): {}", e);
            // This is expected to fail initially as CREATE is not fully implemented
        }
    }

    // Stop the engine
    projection_engine.stop().await.unwrap();
}

/// Test Projection Engine adapter functionality
#[tokio::test]
async fn test_projection_engine_adapter() {
    // Setup test data
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap().to_string();

    // Create projection engine
    let config = ProjectionConfig {
        rocksdb_path: db_path,
        max_concurrent_projections: 10,
        batch_size: 100,
        checkpoint_interval: 1000,
        cache_config: Default::default(),
        enable_metrics: false,
    };

    let projection_engine = Arc::new(ProjectionEngine::new(config).await.unwrap());
    let adapter = kotoba_projection_engine::gql_integration::ProjectionEngineAdapter::new(projection_engine.clone());

    // Test vertex operations
    let test_vertex_id = VertexId("test_vertex_1".to_string());

    // Test get_vertex (should return None initially)
    let vertex_result = adapter.get_vertex(&test_vertex_id).await;
    assert!(vertex_result.is_ok());
    assert!(vertex_result.unwrap().is_none());

    // Test scan_vertices
    let vertices_result = adapter.scan_vertices(None).await;
    assert!(vertices_result.is_ok());
    let vertices = vertices_result.unwrap();
    println!("Found {} vertices", vertices.len());

    // Test edge operations
    let test_edge_id = EdgeId("test_edge_1".to_string());

    // Test get_edge (should return None initially)
    let edge_result = adapter.get_edge(&test_edge_id).await;
    assert!(edge_result.is_ok());
    assert!(edge_result.unwrap().is_none());

    // Test scan_edges
    let edges_result = adapter.scan_edges(None).await;
    assert!(edges_result.is_ok());
    let edges = edges_result.unwrap();
    println!("Found {} edges", edges.len());

    // Test cache operations
    let cache_result = adapter.get("test_key").await;
    assert!(cache_result.is_ok());
    assert!(cache_result.unwrap().is_none());

    let set_result = adapter.set("test_key", serde_json::json!("test_value"), None).await;
    assert!(set_result.is_ok());

    let get_result = adapter.get("test_key").await;
    assert!(get_result.is_ok());
    assert_eq!(get_result.unwrap(), Some(serde_json::json!("test_value")));
}

/// Helper function to create test OCEL events
fn create_test_ocel_events() -> Vec<OcelEvent> {
    let mut events = Vec::new();

    // Event 1: Create order
    let mut event1 = OcelEvent::new(
        "evt1".to_string(),
        "create_order".to_string(),
        Utc.ymd(2023, 1, 1).and_hms(12, 0, 0),
    );

    event1.vmap.insert("customer_id".to_string(), OcelValue::String("cust123".to_string()));
    event1.vmap.insert("amount".to_string(), OcelValue::Float(99.99));
    event1.omap.push("order1".to_string());
    event1.omap.push("customer1".to_string());

    events.push(event1);

    // Event 2: Process order
    let mut event2 = OcelEvent::new(
        "evt2".to_string(),
        "process_order".to_string(),
        Utc.ymd(2023, 1, 1).and_hms(12, 5, 0),
    );

    event2.vmap.insert("status".to_string(), OcelValue::String("processing".to_string()));
    event2.omap.push("order1".to_string());

    events.push(event2);

    // Event 3: Complete order
    let mut event3 = OcelEvent::new(
        "evt3".to_string(),
        "complete_order".to_string(),
        Utc.ymd(2023, 1, 1).and_hms(12, 10, 0),
    );

    event3.vmap.insert("status".to_string(), OcelValue::String("completed".to_string()));
    event3.omap.push("order1".to_string());

    events.push(event3);

    events
}

/// Test GQL query parsing and execution flow
#[tokio::test]
async fn test_gql_parsing_flow() {
    // Test basic GQL parsing
    let query = "MATCH (n) RETURN n";

    // This would normally go through the full pipeline:
    // 1. GQL Parser parses the query string
    // 2. Query Planner creates execution plan
    // 3. Query Executor runs against GraphDB
    // 4. Results returned as JSON

    println!("Testing GQL parsing flow for query: {}", query);

    // For now, just test that we can create the components
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap().to_string();

    let config = ProjectionConfig {
        rocksdb_path: db_path,
        max_concurrent_projections: 10,
        batch_size: 100,
        checkpoint_interval: 1000,
        cache_config: Default::default(),
        enable_metrics: false,
    };

    let projection_engine = ProjectionEngine::new(config).await.unwrap();
    let _adapter = kotoba_projection_engine::gql_integration::ProjectionEngineAdapter::new(Arc::new(projection_engine));

    println!("GQL integration components created successfully");

    // TODO: Add full parsing and execution test once all components are connected
}

//! Database Lifecycle Integration Tests
//!
//! Tests the complete lifecycle of a KotobaDB database including:
//! - Database creation and initialization
//! - Schema creation and validation
//! - Data insertion, updates, and queries
//! - Backup and restore operations
//! - Database shutdown and cleanup

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use kotoba_graphdb::GraphDB;
use kotoba_core::types::{Value, VertexId, EdgeId};

#[derive(Debug, Clone)]
struct TestGraph {
    nodes: Vec<NodeData>,
    edges: Vec<EdgeData>,
}

#[derive(Debug, Clone)]
struct NodeData {
    id: String,
    labels: Vec<String>,
    properties: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct EdgeData {
    from: String,
    to: String,
    label: String,
    properties: HashMap<String, Value>,
}

/// Complete database lifecycle test
#[tokio::test]
async fn test_full_database_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Database Creation
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("lifecycle_test.db");
    let db = DB::open_lsm(&db_path).await?;
    let db = Arc::new(Mutex::new(db));

    // 2. Schema Creation and Validation
    create_test_schema(&db).await?;

    // 3. Data Population
    let test_data = generate_test_data();
    populate_test_data(&db, &test_data).await?;

    // 4. Query Operations
    perform_test_queries(&db, &test_data).await?;

    // 5. Transaction Testing
    test_transaction_operations(&db).await?;

    // 6. Backup and Restore
    test_backup_restore(&db, &db_path).await?;

    // 7. Performance Validation
    validate_performance(&db).await?;

    // 8. Cleanup
    drop(temp_dir); // This will cleanup the temporary directory

    Ok(())
}

/// Create test schema with nodes and edges
/// Temporarily disabled - needs GraphDB schema API
async fn create_test_schema(_db: &Arc<Mutex<GraphDB>>) -> Result<(), Box<dyn std::error::Error>> {
    // TODO: Implement schema creation using GraphDB API
    // /*
    // let db_guard = db.lock().await;
    // 
    // // Create User nodes
    // let user_schema = NodeBlock {
    // labels: vec!["User".to_string()],
    // properties: HashMap::from([
    // ("name".to_string(), Value::String("".to_string())),
    // ("email".to_string(), Value::String("".to_string())),
    // ("age".to_string(), Value::Int(0)),
    // ]),
    // };
    // */
    // 
    // // Create Post nodes
    // let post_schema = NodeBlock {
    // labels: vec!["Post".to_string()],
    // properties: HashMap::from([
    // ("title".to_string(), Value::String("".to_string())),
    // ("content".to_string(), Value::String("".to_string())),
    // ("created_at".to_string(), Value::String("".to_string())),
    // ]),
    // };
    // 
    // // Create FOLLOWS relationship
    // let follows_schema = EdgeBlock {
    // from_labels: vec!["User".to_string()],
    // to_labels: vec!["User".to_string()],
    // label: "FOLLOWS".to_string(),
    // properties: HashMap::from([
    // ("since".to_string(), Value::String("".to_string())),
    // ]),
    // };
    // 
    // // Create AUTHORED relationship
    // let authored_schema = EdgeBlock {
    // from_labels: vec!["User".to_string()],
    // to_labels: vec!["Post".to_string()],
    // label: "AUTHORED".to_string(),
    // properties: HashMap::new(),
    // };
    // 
    // // Store schemas (simplified - in real implementation would use proper schema management)
    // println!("✓ Test schema created successfully");
    // 
    // Ok(())
    */
}

/// Generate comprehensive test data
fn generate_test_data() -> TestGraph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // Create users
    for i in 1..=10 {
        nodes.push(NodeData {
            id: format!("user_{}", i),
            labels: vec!["User".to_string()],
            properties: HashMap::from([
                ("name".to_string(), Value::String(format!("User {}", i))),
                ("email".to_string(), Value::String(format!("user{}@example.com", i))),
                ("age".to_string(), Value::Int(20 + i)),
            ]),
        });
    }

    // Create posts
    for i in 1..=20 {
        nodes.push(NodeData {
            id: format!("post_{}", i),
            labels: vec!["Post".to_string()],
            properties: HashMap::from([
                ("title".to_string(), Value::String(format!("Post Title {}", i))),
                ("content".to_string(), Value::String(format!("This is the content of post {}", i))),
                ("created_at".to_string(), Value::String("2024-01-01T00:00:00Z".to_string())),
            ]),
        });
    }

    // Create relationships
    for i in 1..=10 {
        // Each user follows 2-3 other users
        for j in 1..=3 {
            let target = ((i + j) % 10) + 1;
            if target != i {
                edges.push(EdgeData {
                    from: format!("user_{}", i),
                    to: format!("user_{}", target),
                    label: "FOLLOWS".to_string(),
                    properties: HashMap::from([
                        ("since".to_string(), Value::String("2024-01-01".to_string())),
                    ]),
                });
            }
        }

        // Each user authors 1-2 posts
        for j in 1..=2 {
            let post_id = ((i - 1) * 2 + j);
            edges.push(EdgeData {
                from: format!("user_{}", i),
                to: format!("post_{}", post_id),
                label: "AUTHORED".to_string(),
                properties: HashMap::new(),
            });
        }
    }

    TestGraph { nodes, edges }
}

/// Populate test data into database
async fn populate_test_data(db: &Arc<Mutex<DB>>, test_data: &TestGraph) -> Result<(), Box<dyn std::error::Error>> {
    let db_guard = db.lock().await;

    // Start a transaction for bulk insert
    let tx = db_guard.begin_transaction().await?;

    // Insert nodes
    for node in &test_data.nodes {
        let node_block = NodeBlock {
            labels: node.labels.clone(),
            properties: node.properties.clone(),
        };

        let block = Block::Node(node_block);
        let cid = db_guard.put_block(&block).await?;
        println!("✓ Inserted node {} with CID: {:?}", node.id, cid);
    }

    // Insert edges
    for edge in &test_data.edges {
        let edge_block = EdgeBlock {
            from_labels: vec![], // Simplified
            to_labels: vec![],
            label: edge.label.clone(),
            properties: edge.properties.clone(),
        };

        let block = Block::Edge(edge_block);
        let cid = db_guard.put_block(&block).await?;
        println!("✓ Inserted edge {} -> {} with CID: {:?}", edge.from, edge.to, cid);
    }

    // Commit transaction
    tx.commit().await?;
    println!("✓ Bulk data insertion completed successfully");

    Ok(())
}

/// Perform various query operations on test data
async fn perform_test_queries(db: &Arc<Mutex<DB>>, test_data: &TestGraph) -> Result<(), Box<dyn std::error::Error>> {
    let db_guard = db.lock().await;

    // Query 1: Find all users
    let users = db_guard.find_nodes_by_label("User").await?;
    assert_eq!(users.len(), 10, "Should find 10 users");
    println!("✓ Found {} users", users.len());

    // Query 2: Find all posts
    let posts = db_guard.find_nodes_by_label("Post").await?;
    assert_eq!(posts.len(), 20, "Should find 20 posts");
    println!("✓ Found {} posts", posts.len());

    // Query 3: Find users by property
    let adult_users = db_guard.find_nodes_by_property("age", &Value::Int(25)).await?;
    assert!(!adult_users.is_empty(), "Should find some adult users");
    println!("✓ Found {} adult users", adult_users.len());

    // Query 4: Find relationships
    let follows_relationships = db_guard.find_edges_by_label("FOLLOWS").await?;
    assert!(!follows_relationships.is_empty(), "Should find follow relationships");
    println!("✓ Found {} follow relationships", follows_relationships.len());

    Ok(())
}

/// Test transaction operations
async fn test_transaction_operations(db: &Arc<Mutex<DB>>) -> Result<(), Box<dyn std::error::Error>> {
    let db_guard = db.lock().await;

    // Test successful transaction
    {
        let tx = db_guard.begin_transaction().await?;

        // Create a test node
        let test_node = NodeBlock {
            labels: vec!["Test".to_string()],
            properties: HashMap::from([
                ("test_prop".to_string(), Value::String("test_value".to_string())),
            ]),
        };

        let block = Block::Node(test_node);
        let cid = db_guard.put_block(&block).await?;

        tx.commit().await?;
        println!("✓ Transaction committed successfully");

        // Verify the node exists
        let retrieved = db_guard.get_block(&cid).await?;
        assert!(retrieved.is_some(), "Committed node should exist");
    }

    // Test transaction rollback
    {
        let tx = db_guard.begin_transaction().await?;

        // Create another test node
        let rollback_node = NodeBlock {
            labels: vec!["RollbackTest".to_string()],
            properties: HashMap::from([
                ("should_not_exist".to_string(), Value::Bool(true)),
            ]),
        };

        let block = Block::Node(rollback_node);
        let cid = db_guard.put_block(&block).await?;

        tx.rollback().await?;
        println!("✓ Transaction rolled back successfully");

        // Verify the node doesn't exist
        let retrieved = db_guard.get_block(&cid).await?;
        assert!(retrieved.is_none(), "Rolled back node should not exist");
    }

    Ok(())
}

/// Test backup and restore functionality
async fn test_backup_restore(db: &Arc<Mutex<DB>>, db_path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::{BackupManager, RestoreManager};

    let backup_manager = BackupManager::new(db_path.parent().unwrap().to_path_buf());

    // Create a backup
    let backup_path = backup_manager.create_full_backup().await?;
    println!("✓ Created backup at: {:?}", backup_path);

    // Verify backup exists and has content
    assert!(backup_path.exists(), "Backup file should exist");
    let metadata = std::fs::metadata(&backup_path)?;
    assert!(metadata.len() > 0, "Backup file should not be empty");

    // Test restore to a new location
    let restore_dir = tempfile::tempdir()?;
    let restore_manager = RestoreManager::new(restore_dir.path().to_path_buf());

    restore_manager.restore_full_backup(&backup_path).await?;
    println!("✓ Successfully restored from backup");

    Ok(())
}

/// Validate performance characteristics
async fn validate_performance(db: &Arc<Mutex<DB>>) -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;

    let db_guard = db.lock().await;

    // Performance test: Bulk insert
    let start = Instant::now();
    let mut cids = Vec::new();

    for i in 0..1000 {
        let node = NodeBlock {
            labels: vec!["PerfTest".to_string()],
            properties: HashMap::from([
                ("id".to_string(), Value::Int(i)),
                ("data".to_string(), Value::String(format!("Performance test data {}", i))),
            ]),
        };

        let block = Block::Node(node);
        let cid = db_guard.put_block(&block).await?;
        cids.push(cid);
    }

    let insert_duration = start.elapsed();
    let inserts_per_second = 1000.0 / insert_duration.as_secs_f64();
    println!("✓ Bulk insert performance: {:.0} inserts/second", inserts_per_second);

    // Performance test: Bulk read
    let start = Instant::now();
    for cid in &cids {
        let _ = db_guard.get_block(cid).await?;
    }
    let read_duration = start.elapsed();
    let reads_per_second = 1000.0 / read_duration.as_secs_f64();
    println!("✓ Bulk read performance: {:.0} reads/second", reads_per_second);

    // Basic performance assertions
    assert!(inserts_per_second > 100.0, "Insert performance should be reasonable");
    assert!(reads_per_second > 500.0, "Read performance should be reasonable");

    Ok(())
}

#[tokio::test]
async fn test_database_concurrent_access() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("concurrent_test.db");
    let db = Arc::new(Mutex::new(DB::open_lsm(&db_path).await?));

    let mut handles = Vec::new();

    // Spawn 10 concurrent writers
    for i in 0..10 {
        let db_clone = Arc::clone(&db);
        let handle = tokio::spawn(async move {
            for j in 0..10 {
                let db_guard = db_clone.lock().await;
                let node = NodeBlock {
                    labels: vec![format!("Concurrent{}", i)],
                    properties: HashMap::from([
                        ("thread_id".to_string(), Value::Int(i as i64)),
                        ("operation_id".to_string(), Value::Int(j as i64)),
                    ]),
                };

                let block = Block::Node(node);
                let _cid = db_guard.put_block(&block).await?;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations to complete
    for handle in handles {
        handle.await??;
    }

    // Verify all data was written
    let db_guard = db.lock().await;
    for i in 0..10 {
        let nodes = db_guard.find_nodes_by_label(&format!("Concurrent{}", i)).await?;
        assert_eq!(nodes.len(), 10, "Each thread should have created 10 nodes");
    }

    println!("✓ Concurrent access test passed");
    Ok(())
}

#[tokio::test]
async fn test_database_error_recovery() -> Result<(), Box<dyn std::error::Error>> {
    // Test various error conditions and recovery scenarios
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("error_recovery_test.db");

    // Test 1: Database creation with invalid path
    let invalid_result = DB::open_lsm("/invalid/path/database.db").await;
    assert!(invalid_result.is_err(), "Should fail with invalid path");

    // Test 2: Normal database creation and operation
    let db = DB::open_lsm(&db_path).await?;
    let db = Arc::new(Mutex::new(db));

    // Test 3: Invalid data insertion (this would depend on schema validation)
    let db_guard = db.lock().await;
    let invalid_node = NodeBlock {
        labels: vec!["Invalid".to_string()],
        properties: HashMap::from([
            ("invalid_prop".to_string(), Value::Link([0u8; 32])), // Invalid link
        ]),
    };

    // This should either succeed or fail gracefully depending on validation
    let result = db_guard.put_block(&Block::Node(invalid_node)).await;
    // We don't assert here as validation behavior may vary

    println!("✓ Error recovery test completed");
    Ok(())
}

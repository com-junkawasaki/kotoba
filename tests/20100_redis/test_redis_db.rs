//! Test for kotoba database using Redis storage
//!
//! This test demonstrates setting up a kotoba database with Redis storage
//! and performing basic operations.

use std::sync::Arc;
use tokio::time::{timeout, Duration};
use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;

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

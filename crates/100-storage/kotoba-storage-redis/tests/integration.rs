//! Integration tests for kotoba-storage-redis
//!
//! These tests require a Redis server running on localhost:6379
//! To run: cargo test --test integration

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use kotoba_storage_redis::{RedisStore, RedisConfig, ConnectionStatus};
use kotoba_storage::KeyValueStore;

async fn create_test_store() -> RedisStore {
    let config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "test:integration".to_string(),
        enable_compression: true,
        enable_metrics: true,
        ..Default::default()
    };

    RedisStore::new(config).await.expect("Failed to create Redis store")
}

async fn cleanup_test_store(store: &RedisStore) {
    // Clean up test keys
    let pattern = format!("{}*", store.config.key_prefix);
    if let Ok(mut conn) = store.get_connection().await {
        let keys: Vec<String> = conn.keys(&pattern).await.unwrap_or_default();
        if !keys.is_empty() {
            let _: () = conn.del(&keys).await.unwrap_or(());
        }
        store.return_connection(conn).await;
    }
}

#[tokio::test]
async fn test_integration_basic_operations() {
    let store = create_test_store().await;

    // Ensure we're connected
    assert!(store.is_connected().await, "Should be connected to Redis");

    // Clean up first
    cleanup_test_store(&store).await;

    // Test put and get
    let test_key = b"integration_test_key";
    let test_value = b"integration test value";

    store.put(test_key, test_value).await.expect("Put should succeed");
    let retrieved = store.get(test_key).await.expect("Get should succeed");

    assert_eq!(retrieved, Some(test_value.to_vec()), "Retrieved value should match");

    // Test delete
    store.delete(test_key).await.expect("Delete should succeed");
    let after_delete = store.get(test_key).await.expect("Get after delete should succeed");
    assert_eq!(after_delete, None, "Value should be None after delete");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_multiple_keys() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Put multiple key-value pairs
    let test_data = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
    ];

    for (key, value) in &test_data {
        store.put(key, value).await.expect("Put should succeed");
    }

    // Verify all keys exist
    for (key, expected_value) in &test_data {
        let retrieved = store.get(key).await.expect("Get should succeed");
        assert_eq!(retrieved, Some(expected_value.to_vec()), "Value should match");
    }

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_scan() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Put keys with common prefix
    let keys_and_values = vec![
        (b"prefix_key1", b"value1"),
        (b"prefix_key2", b"value2"),
        (b"prefix_key3", b"value3"),
        (b"other_key", b"other_value"),
    ];

    for (key, value) in &keys_and_values {
        store.put(key, value).await.expect("Put should succeed");
    }

    // Scan with prefix
    let results = store.scan(b"prefix_").await.expect("Scan should succeed");
    assert_eq!(results.len(), 3, "Should find 3 keys with prefix");

    // Verify results are sorted
    assert_eq!(results[0].0, b"prefix_key1");
    assert_eq!(results[1].0, b"prefix_key2");
    assert_eq!(results[2].0, b"prefix_key3");

    // Scan with empty prefix should return all test keys
    let all_results = store.scan(b"").await.expect("Scan all should succeed");
    assert!(all_results.len() >= 4, "Should find at least our test keys");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_large_data() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Test with large key and value
    let large_key = vec![42u8; 1024]; // 1KB key
    let large_value = vec![255u8; 1024 * 1024]; // 1MB value

    store.put(&large_key, &large_value).await.expect("Put large data should succeed");
    let retrieved = store.get(&large_key).await.expect("Get large data should succeed");

    assert_eq!(retrieved, Some(large_value), "Large value should match");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_binary_data() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Test with binary data (null bytes, etc.)
    let binary_key = vec![0, 1, 2, 255, 0, 1, 2, 255];
    let binary_value = vec![255, 254, 253, 0, 1, 2, 3, 255, 254, 253];

    store.put(&binary_key, &binary_value).await.expect("Put binary data should succeed");
    let retrieved = store.get(&binary_key).await.expect("Get binary data should succeed");

    assert_eq!(retrieved, Some(binary_value), "Binary value should match");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_unicode_data() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Test Unicode keys and values
    let unicode_key = "🚀 котоба 🔥".as_bytes();
    let unicode_value = "Hello 世界 🌍".as_bytes();

    store.put(unicode_key, unicode_value).await.expect("Put unicode data should succeed");
    let retrieved = store.get(unicode_key).await.expect("Get unicode data should succeed");

    assert_eq!(retrieved, Some(unicode_value.to_vec()), "Unicode value should match");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_concurrent_access() {
    let store = Arc::new(create_test_store().await);
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    let mut handles = vec![];

    // Spawn multiple tasks to test concurrent access
    for i in 0..10 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i).into_bytes();
            let value = format!("concurrent_value_{}", i).into_bytes();

            // Put operation
            store_clone.put(&key, &value).await.expect("Concurrent put should succeed");

            // Get operation
            let retrieved = store_clone.get(&key).await.expect("Concurrent get should succeed");
            assert_eq!(retrieved, Some(value), "Concurrent retrieved value should match");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Concurrent task should succeed");
    }

    // Verify all keys were stored
    for i in 0..10 {
        let key = format!("concurrent_key_{}", i).into_bytes();
        let expected_value = format!("concurrent_value_{}", i).into_bytes();
        let retrieved = store.get(&key).await.expect("Final get should succeed");
        assert_eq!(retrieved, Some(expected_value), "Final value should match");
    }

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_ttl_behavior() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Test with short TTL
    let ttl_config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "test:ttl".to_string(),
        default_ttl_seconds: 2, // 2 seconds TTL
        enable_metrics: true,
        ..Default::default()
    };

    let ttl_store = RedisStore::new(ttl_config).await.expect("Should create TTL store");

    let test_key = b"ttl_test_key";
    let test_value = b"ttl test value";

    // Put value with TTL
    ttl_store.put(test_key, test_value).await.expect("Put with TTL should succeed");

    // Should be able to get immediately
    let retrieved = ttl_store.get(test_key).await.expect("Get should succeed immediately");
    assert_eq!(retrieved, Some(test_value.to_vec()), "Should retrieve value immediately");

    // Wait for TTL to expire
    sleep(Duration::from_secs(3)).await;

    // Should not be able to get after TTL expires
    let after_ttl = ttl_store.get(test_key).await.expect("Get after TTL should succeed");
    assert_eq!(after_ttl, None, "Value should be None after TTL expires");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_statistics() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Initial stats
    let initial_stats = store.get_stats().await;
    let initial_ops = initial_stats.total_operations;

    // Perform some operations
    store.put(b"stats_key1", b"stats_value1").await.expect("Put should succeed");
    store.get(b"stats_key1").await.expect("Get should succeed");
    store.put(b"stats_key2", b"stats_value2").await.expect("Put should succeed");
    store.delete(b"stats_key1").await.expect("Delete should succeed");

    // Check updated stats
    let updated_stats = store.get_stats().await;
    assert_eq!(updated_stats.total_operations, initial_ops + 4, "Should have 4 more operations");
    assert!(matches!(updated_stats.connection_status, ConnectionStatus::Connected), "Should be connected");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_overwrite() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    let test_key = b"overwrite_key";

    // Put initial value
    store.put(test_key, b"initial_value").await.expect("Initial put should succeed");
    let retrieved = store.get(test_key).await.expect("Initial get should succeed");
    assert_eq!(retrieved, Some(b"initial_value".to_vec()), "Should retrieve initial value");

    // Overwrite with new value
    store.put(test_key, b"updated_value").await.expect("Overwrite put should succeed");
    let retrieved = store.get(test_key).await.expect("Updated get should succeed");
    assert_eq!(retrieved, Some(b"updated_value".to_vec()), "Should retrieve updated value");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_empty_keys_values() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Test empty key
    store.put(b"", b"empty_key_value").await.expect("Put empty key should succeed");
    let retrieved = store.get(b"").await.expect("Get empty key should succeed");
    assert_eq!(retrieved, Some(b"empty_key_value".to_vec()), "Should retrieve empty key value");

    // Test empty value
    store.put(b"empty_value_key", b"").await.expect("Put empty value should succeed");
    let retrieved = store.get(b"empty_value_key").await.expect("Get empty value should succeed");
    assert_eq!(retrieved, Some(b"".to_vec()), "Should retrieve empty value");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_scan_ordering() {
    let store = create_test_store().await;
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    // Insert keys in reverse order
    store.put(b"scan_key3", b"value3").await.expect("Put key3 should succeed");
    store.put(b"scan_key1", b"value1").await.expect("Put key1 should succeed");
    store.put(b"scan_key2", b"value2").await.expect("Put key2 should succeed");

    // Scan should return results in sorted order
    let results = store.scan(b"scan_").await.expect("Scan should succeed");
    assert_eq!(results.len(), 3, "Should find 3 keys");
    assert_eq!(results[0].0, b"scan_key1", "First key should be scan_key1");
    assert_eq!(results[1].0, b"scan_key2", "Second key should be scan_key2");
    assert_eq!(results[2].0, b"scan_key3", "Third key should be scan_key3");

    // Clean up
    cleanup_test_store(&store).await;
}

#[tokio::test]
async fn test_integration_performance() {
    let store = Arc::new(create_test_store().await);
    assert!(store.is_connected().await);

    cleanup_test_store(&store).await;

    let num_operations = 1000;
    let start_time = std::time::Instant::now();

    // Perform many put operations
    for i in 0..num_operations {
        let key = format!("perf_key_{:04}", i).into_bytes();
        let value = format!("perf_value_{}", i).into_bytes();
        store.put(&key, &value).await.expect("Put should succeed");
    }

    let put_time = start_time.elapsed();

    // Perform many get operations
    let start_time = std::time::Instant::now();
    for i in 0..num_operations {
        let key = format!("perf_key_{:04}", i).into_bytes();
        let expected_value = format!("perf_value_{}", i).into_bytes();
        let retrieved = store.get(&key).await.expect("Get should succeed");
        assert_eq!(retrieved, Some(expected_value), "Value should match");
    }

    let get_time = start_time.elapsed();

    println!("Redis Performance Test:");
    println!("  Put {} entries in {:?}", num_operations, put_time);
    println!("  Get {} entries in {:?}", num_operations, get_time);

    // Basic performance assertions (these will depend on the system)
    assert!(put_time.as_millis() < 10000, "Put performance regression");
    assert!(get_time.as_millis() < 10000, "Get performance regression");

    // Clean up
    cleanup_test_store(&store).await;
}

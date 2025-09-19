//! Integration tests for all storage backends
//!
//! This test compares different storage backends:
//! - Memory
//! - RocksDB
//! - Redis
//!
//! Run with: cargo test --test storage_integration

use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use kotoba_storage::KeyValueStore;

#[cfg(feature = "memory")]
use kotoba_memory::MemoryKeyValueStore;

#[cfg(feature = "rocksdb")]
use kotoba_storage_rocksdb::RocksDbStore;

#[cfg(feature = "redis")]
use kotoba_storage_redis::{RedisStore, RedisConfig};

async fn create_memory_store() -> MemoryKeyValueStore {
    MemoryKeyValueStore::new()
}

#[cfg(feature = "rocksdb")]
async fn create_rocksdb_store() -> RocksDbStore {
    use tempfile::TempDir;
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("integration_test.db");

    // Note: This will leak the temp dir, but that's OK for tests
    std::mem::forget(temp_dir);

    RocksDbStore::new(&db_path).expect("Failed to create RocksDB store")
}

#[cfg(feature = "redis")]
async fn create_redis_store() -> RedisStore {
    let config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "integration:test".to_string(),
        enable_compression: true,
        ..Default::default()
    };

    RedisStore::new(config).await.expect("Failed to create Redis store")
}

async fn cleanup_redis_store(store: &RedisStore) {
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
async fn test_memory_storage_basic() {
    let store = create_memory_store().await;

    // Test basic operations
    let test_key = b"memory_test_key";
    let test_value = b"memory test value";

    store.put(test_key, test_value).await.expect("Memory put should succeed");
    let retrieved = store.get(test_key).await.expect("Memory get should succeed");
    assert_eq!(retrieved, Some(test_value.to_vec()), "Memory retrieved value should match");
}

#[cfg(feature = "rocksdb")]
#[tokio::test]
async fn test_rocksdb_storage_basic() {
    let store = create_rocksdb_store().await;

    // Test basic operations
    let test_key = b"rocksdb_test_key";
    let test_value = b"rocksdb test value";

    store.put(test_key, test_value).await.expect("RocksDB put should succeed");
    let retrieved = store.get(test_key).await.expect("RocksDB get should succeed");
    assert_eq!(retrieved, Some(test_value.to_vec()), "RocksDB retrieved value should match");
}

#[cfg(feature = "redis")]
#[tokio::test]
async fn test_redis_storage_basic() {
    let store = create_redis_store().await;
    cleanup_redis_store(&store).await;

    // Test basic operations
    let test_key = b"redis_test_key";
    let test_value = b"redis test value";

    store.put(test_key, test_value).await.expect("Redis put should succeed");
    let retrieved = store.get(test_key).await.expect("Redis get should succeed");
    assert_eq!(retrieved, Some(test_value.to_vec()), "Redis retrieved value should match");

    cleanup_redis_store(&store).await;
}

#[tokio::test]
async fn test_all_storages_consistency() {
    // Test that all storage backends behave consistently for basic operations

    let test_cases = vec![
        (b"key1", b"value1"),
        (b"key2", b"value2"),
        (b"key3", b"value3"),
    ];

    // Memory storage
    {
        let store = create_memory_store().await;
        for (key, value) in &test_cases {
            store.put(key, value).await.expect("Memory put should succeed");
        }

        for (key, expected_value) in &test_cases {
            let retrieved = store.get(key).await.expect("Memory get should succeed");
            assert_eq!(retrieved, Some(expected_value.to_vec()), "Memory consistency check failed");
        }
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;
        for (key, value) in &test_cases {
            store.put(key, value).await.expect("RocksDB put should succeed");
        }

        for (key, expected_value) in &test_cases {
            let retrieved = store.get(key).await.expect("RocksDB get should succeed");
            assert_eq!(retrieved, Some(expected_value.to_vec()), "RocksDB consistency check failed");
        }
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        for (key, value) in &test_cases {
            store.put(key, value).await.expect("Redis put should succeed");
        }

        for (key, expected_value) in &test_cases {
            let retrieved = store.get(key).await.expect("Redis get should succeed");
            assert_eq!(retrieved, Some(expected_value.to_vec()), "Redis consistency check failed");
        }

        cleanup_redis_store(&store).await;
    }
}

#[tokio::test]
async fn test_storage_scan_consistency() {
    // Test scan operations across different backends

    let prefix = b"scan_test_";
    let test_data = vec![
        (b"scan_test_key1", b"value1"),
        (b"scan_test_key2", b"value2"),
        (b"scan_test_key3", b"value3"),
        (b"other_key", b"other_value"), // Should not be included in scan
    ];

    // Memory storage
    {
        let store = create_memory_store().await;
        for (key, value) in &test_data {
            store.put(key, value).await.expect("Memory put should succeed");
        }

        let results = store.scan(prefix).await.expect("Memory scan should succeed");
        assert_eq!(results.len(), 3, "Memory scan should find 3 keys");

        // Verify results are sorted
        assert_eq!(results[0].0, b"scan_test_key1");
        assert_eq!(results[1].0, b"scan_test_key2");
        assert_eq!(results[2].0, b"scan_test_key3");
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;
        for (key, value) in &test_data {
            store.put(key, value).await.expect("RocksDB put should succeed");
        }

        let results = store.scan(prefix).await.expect("RocksDB scan should succeed");
        assert_eq!(results.len(), 3, "RocksDB scan should find 3 keys");
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        for (key, value) in &test_data {
            store.put(key, value).await.expect("Redis put should succeed");
        }

        let results = store.scan(prefix).await.expect("Redis scan should succeed");
        assert_eq!(results.len(), 3, "Redis scan should find 3 keys");

        cleanup_redis_store(&store).await;
    }
}

#[tokio::test]
async fn test_storage_delete_consistency() {
    // Test delete operations across different backends

    let test_key = b"delete_test_key";
    let test_value = b"delete test value";

    // Memory storage
    {
        let store = create_memory_store().await;
        store.put(test_key, test_value).await.expect("Memory put should succeed");

        let retrieved = store.get(test_key).await.expect("Memory get should succeed");
        assert_eq!(retrieved, Some(test_value.to_vec()), "Memory should have value before delete");

        store.delete(test_key).await.expect("Memory delete should succeed");

        let retrieved = store.get(test_key).await.expect("Memory get after delete should succeed");
        assert_eq!(retrieved, None, "Memory should not have value after delete");
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;
        store.put(test_key, test_value).await.expect("RocksDB put should succeed");

        let retrieved = store.get(test_key).await.expect("RocksDB get should succeed");
        assert_eq!(retrieved, Some(test_value.to_vec()), "RocksDB should have value before delete");

        store.delete(test_key).await.expect("RocksDB delete should succeed");

        let retrieved = store.get(test_key).await.expect("RocksDB get after delete should succeed");
        assert_eq!(retrieved, None, "RocksDB should not have value after delete");
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        store.put(test_key, test_value).await.expect("Redis put should succeed");

        let retrieved = store.get(test_key).await.expect("Redis get should succeed");
        assert_eq!(retrieved, Some(test_value.to_vec()), "Redis should have value before delete");

        store.delete(test_key).await.expect("Redis delete should succeed");

        let retrieved = store.get(test_key).await.expect("Redis get after delete should succeed");
        assert_eq!(retrieved, None, "Redis should not have value after delete");

        cleanup_redis_store(&store).await;
    }
}

#[tokio::test]
async fn test_storage_large_data() {
    // Test handling of large data across different backends

    let large_key = vec![42u8; 1024]; // 1KB key
    let large_value = vec![255u8; 64 * 1024]; // 64KB value

    // Memory storage
    {
        let store = create_memory_store().await;
        store.put(&large_key, &large_value).await.expect("Memory put large should succeed");
        let retrieved = store.get(&large_key).await.expect("Memory get large should succeed");
        assert_eq!(retrieved, Some(large_value.clone()), "Memory large value should match");
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;
        store.put(&large_key, &large_value).await.expect("RocksDB put large should succeed");
        let retrieved = store.get(&large_key).await.expect("RocksDB get large should succeed");
        assert_eq!(retrieved, Some(large_value.clone()), "RocksDB large value should match");
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        store.put(&large_key, &large_value).await.expect("Redis put large should succeed");
        let retrieved = store.get(&large_key).await.expect("Redis get large should succeed");
        assert_eq!(retrieved, Some(large_value.clone()), "Redis large value should match");

        cleanup_redis_store(&store).await;
    }
}

#[tokio::test]
async fn test_storage_concurrent_access() {
    // Test concurrent access across different backends

    let store = Arc::new(create_memory_store().await);
    let mut handles = vec![];

    // Spawn multiple tasks
    for i in 0..50 {
        let store_clone = Arc::clone(&store);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i).into_bytes();
            let value = format!("concurrent_value_{}", i).into_bytes();

            // Put operation
            store_clone.put(&key, &value).await.expect("Concurrent memory put should succeed");

            // Get operation
            let retrieved = store_clone.get(&key).await.expect("Concurrent memory get should succeed");
            assert_eq!(retrieved, Some(value), "Concurrent memory retrieved value should match");
        });
        handles.push(handle);
    }

    // Wait for all tasks to complete
    for handle in handles {
        handle.await.expect("Concurrent memory task should succeed");
    }

    // RocksDB concurrent access
    #[cfg(feature = "rocksdb")]
    {
        let store = Arc::new(create_rocksdb_store().await);
        let mut handles = vec![];

        for i in 0..50 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_rocks_key_{}", i).into_bytes();
                let value = format!("concurrent_rocks_value_{}", i).into_bytes();

                store_clone.put(&key, &value).await.expect("Concurrent RocksDB put should succeed");
                let retrieved = store_clone.get(&key).await.expect("Concurrent RocksDB get should succeed");
                assert_eq!(retrieved, Some(value), "Concurrent RocksDB retrieved value should match");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Concurrent RocksDB task should succeed");
        }
    }

    // Redis concurrent access
    #[cfg(feature = "redis")]
    {
        let store = Arc::new(create_redis_store().await);
        cleanup_redis_store(&store).await;
        let mut handles = vec![];

        for i in 0..50 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_redis_key_{}", i).into_bytes();
                let value = format!("concurrent_redis_value_{}", i).into_bytes();

                store_clone.put(&key, &value).await.expect("Concurrent Redis put should succeed");
                let retrieved = store_clone.get(&key).await.expect("Concurrent Redis get should succeed");
                assert_eq!(retrieved, Some(value), "Concurrent Redis retrieved value should match");
            });
            handles.push(handle);
        }

        for handle in handles {
            handle.await.expect("Concurrent Redis task should succeed");
        }

        cleanup_redis_store(&store).await;
    }
}

#[tokio::test]
async fn test_storage_binary_data() {
    // Test handling of binary data (null bytes, etc.)

    let binary_key = vec![0, 1, 2, 255, 0, 1, 2, 255];
    let binary_value = vec![255, 254, 253, 0, 1, 2, 3, 255, 254, 253];

    // Memory storage
    {
        let store = create_memory_store().await;
        store.put(&binary_key, &binary_value).await.expect("Memory put binary should succeed");
        let retrieved = store.get(&binary_key).await.expect("Memory get binary should succeed");
        assert_eq!(retrieved, Some(binary_value.clone()), "Memory binary value should match");
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;
        store.put(&binary_key, &binary_value).await.expect("RocksDB put binary should succeed");
        let retrieved = store.get(&binary_key).await.expect("RocksDB get binary should succeed");
        assert_eq!(retrieved, Some(binary_value.clone()), "RocksDB binary value should match");
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        store.put(&binary_key, &binary_value).await.expect("Redis put binary should succeed");
        let retrieved = store.get(&binary_key).await.expect("Redis get binary should succeed");
        assert_eq!(retrieved, Some(binary_value.clone()), "Redis binary value should match");

        cleanup_redis_store(&store).await;
    }
}

#[cfg(feature = "redis")]
#[tokio::test]
async fn test_redis_storage_ttl() {
    let store = create_redis_store().await;
    cleanup_redis_store(&store).await;

    let ttl_config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "ttl:test".to_string(),
        default_ttl_seconds: 2, // 2 seconds TTL
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

    cleanup_redis_store(&store).await;
}

#[tokio::test]
async fn test_storage_overwrite() {
    // Test overwriting existing keys

    let test_key = b"overwrite_key";

    // Memory storage
    {
        let store = create_memory_store().await;

        // Put initial value
        store.put(test_key, b"initial").await.expect("Memory initial put should succeed");
        let retrieved = store.get(test_key).await.expect("Memory initial get should succeed");
        assert_eq!(retrieved, Some(b"initial".to_vec()), "Memory should have initial value");

        // Overwrite
        store.put(test_key, b"updated").await.expect("Memory overwrite put should succeed");
        let retrieved = store.get(test_key).await.expect("Memory updated get should succeed");
        assert_eq!(retrieved, Some(b"updated".to_vec()), "Memory should have updated value");
    }

    // RocksDB storage
    #[cfg(feature = "rocksdb")]
    {
        let store = create_rocksdb_store().await;

        store.put(test_key, b"initial").await.expect("RocksDB initial put should succeed");
        let retrieved = store.get(test_key).await.expect("RocksDB initial get should succeed");
        assert_eq!(retrieved, Some(b"initial".to_vec()), "RocksDB should have initial value");

        store.put(test_key, b"updated").await.expect("RocksDB overwrite put should succeed");
        let retrieved = store.get(test_key).await.expect("RocksDB updated get should succeed");
        assert_eq!(retrieved, Some(b"updated".to_vec()), "RocksDB should have updated value");
    }

    // Redis storage
    #[cfg(feature = "redis")]
    {
        let store = create_redis_store().await;
        cleanup_redis_store(&store).await;

        store.put(test_key, b"initial").await.expect("Redis initial put should succeed");
        let retrieved = store.get(test_key).await.expect("Redis initial get should succeed");
        assert_eq!(retrieved, Some(b"initial".to_vec()), "Redis should have initial value");

        store.put(test_key, b"updated").await.expect("Redis overwrite put should succeed");
        let retrieved = store.get(test_key).await.expect("Redis updated get should succeed");
        assert_eq!(retrieved, Some(b"updated".to_vec()), "Redis should have updated value");

        cleanup_redis_store(&store).await;
    }
}

#[cfg(feature = "redis")]
#[tokio::test]
async fn test_redis_storage_compression() {
    let store = create_redis_store().await;
    cleanup_redis_store(&store).await;

    // Test with large data that should be compressed
    let large_value = vec![42u8; 2048]; // 2KB - should trigger compression

    let test_key = b"compression_test_key";

    store.put(test_key, &large_value).await.expect("Put with compression should succeed");
    let retrieved = store.get(test_key).await.expect("Get with compression should succeed");

    assert_eq!(retrieved, Some(large_value), "Compressed value should match original");

    cleanup_redis_store(&store).await;
}

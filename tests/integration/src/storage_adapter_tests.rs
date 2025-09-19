//! Storage Adapter Integration Tests
//!
//! This module provides comprehensive integration tests for storage adapters,
//! covering the Port/Adapter pattern implementation with different backends.
//!
//! Components tested:
//! - kotoba-storage (KeyValueStore trait)
//! - kotoba-memory (In-memory adapter)
//! - kotoba-graphdb (RocksDB adapter)

use std::sync::Arc;
use kotoba_storage::KeyValueStore;
use kotoba_errors::KotobaError;

/// Test fixture for storage adapter tests
pub struct StorageAdapterTestFixture {
    pub memory_adapter: Arc<dyn KeyValueStore + Send + Sync>,
    pub rocksdb_adapter: Option<Arc<dyn KeyValueStore + Send + Sync>>,
}

impl StorageAdapterTestFixture {
    pub async fn new() -> Result<Self, KotobaError> {
        // Always create memory adapter
        let memory_adapter = Arc::new(kotoba_memory::MemoryAdapter::new());

        // Try to create RocksDB adapter
        let rocksdb_adapter = if let Ok(adapter) = Self::create_rocksdb_adapter().await {
            Some(adapter)
        } else {
            println!("RocksDB adapter not available, skipping RocksDB tests");
            None
        };

        Ok(Self {
            memory_adapter,
            rocksdb_adapter,
        })
    }

    async fn create_rocksdb_adapter() -> Result<Arc<dyn KeyValueStore + Send + Sync>, KotobaError> {
        let temp_dir = tempfile::tempdir().map_err(|e| KotobaError::Storage(e.to_string()))?;
        let db_path = temp_dir.path().join("test_rocksdb");

        // Try to create RocksDB adapter
        match kotoba_graphdb::GraphDbAdapter::new(&db_path).await {
            Ok(adapter) => Ok(Arc::new(adapter)),
            Err(e) => {
                println!("Failed to create RocksDB adapter: {}", e);
                Err(KotobaError::Storage(format!("RocksDB initialization failed: {}", e)))
            }
        }
    }

    pub async fn cleanup(&self) -> Result<(), KotobaError> {
        // Memory adapter cleanup (if needed)
        if let Ok(keys) = self.memory_adapter.list_keys().await {
            for key in keys {
                let _ = self.memory_adapter.delete(key.as_bytes()).await;
            }
        }

        // RocksDB adapter cleanup (if available)
        if let Some(ref rocksdb) = self.rocksdb_adapter {
            if let Ok(keys) = rocksdb.list_keys().await {
                for key in keys {
                    let _ = rocksdb.delete(key.as_bytes()).await;
                }
            }
        }

        Ok(())
    }
}

/// Generic storage adapter test runner
pub async fn run_storage_adapter_tests(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("🧪 Running storage adapter tests for: {}", adapter_name);

    // Test 1: Basic CRUD operations
    test_basic_crud_operations(Arc::clone(&adapter), adapter_name).await?;

    // Test 2: Batch operations
    test_batch_operations(Arc::clone(&adapter), adapter_name).await?;

    // Test 3: Error handling
    test_error_handling(Arc::clone(&adapter), adapter_name).await?;

    // Test 4: Concurrent access
    test_concurrent_access(Arc::clone(&adapter), adapter_name).await?;

    // Test 5: Large data handling
    test_large_data_handling(Arc::clone(&adapter), adapter_name).await?;

    println!("✅ All storage adapter tests passed for: {}", adapter_name);
    Ok(())
}

async fn test_basic_crud_operations(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("  📝 Testing basic CRUD operations...");

    // Create test data
    let test_cases = vec![
        ("key1", "value1"),
        ("key2", "value2"),
        ("key3", "value3"),
    ];

    // Test CREATE (PUT)
    for (key, value) in &test_cases {
        adapter.put(key.as_bytes(), value.as_bytes()).await?;
    }

    // Test READ (GET)
    for (key, expected_value) in &test_cases {
        let retrieved = adapter.get(key.as_bytes()).await?
            .ok_or_else(|| KotobaError::Storage(format!("Key {} not found", key)))?;
        assert_eq!(retrieved, expected_value.as_bytes(), "Value mismatch for key {}", key);
    }

    // Test UPDATE
    adapter.put(b"key1", b"updated_value1").await?;
    let updated = adapter.get(b"key1").await?
        .ok_or_else(|| KotobaError::Storage("Updated key not found".to_string()))?;
    assert_eq!(updated, b"updated_value1");

    // Test DELETE
    adapter.delete(b"key1").await?;
    let deleted = adapter.get(b"key1").await?;
    assert!(deleted.is_none(), "Deleted key should not exist");

    // Cleanup
    for (key, _) in &test_cases {
        if key != &"key1" { // key1 already deleted
            let _ = adapter.delete(key.as_bytes()).await;
        }
    }

    println!("  ✅ Basic CRUD operations test passed");
    Ok(())
}

async fn test_batch_operations(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("  📦 Testing batch operations...");

    use std::collections::HashMap;

    // Create batch data
    let mut batch_data = HashMap::new();
    for i in 0..10 {
        let key = format!("batch_key_{}", i);
        let value = format!("batch_value_{}", i);
        batch_data.insert(key.into_bytes(), value.into_bytes());
    }

    // Test batch PUT
    adapter.batch_put(batch_data.clone()).await?;

    // Verify batch GET
    for (key, expected_value) in &batch_data {
        let retrieved = adapter.get(key).await?
            .ok_or_else(|| KotobaError::Storage("Batch key not found".to_string()))?;
        assert_eq!(retrieved, *expected_value);
    }

    // Cleanup
    for key in batch_data.keys() {
        let _ = adapter.delete(key).await;
    }

    println!("  ✅ Batch operations test passed");
    Ok(())
}

async fn test_error_handling(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("  ⚠️ Testing error handling...");

    // Test non-existent key
    let result = adapter.get(b"nonexistent_key").await?;
    assert!(result.is_none(), "Non-existent key should return None");

    // Test empty key (if supported)
    let empty_key_result = adapter.get(b"").await;
    match empty_key_result {
        Ok(None) => (), // Acceptable
        Ok(Some(_)) => (), // Also acceptable
        Err(_) => (), // Error is also acceptable for empty keys
    }

    // Test invalid UTF-8 keys (if applicable)
    let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
    let invalid_result = adapter.get(&invalid_utf8).await;
    match invalid_result {
        Ok(None) => (), // Acceptable
        Ok(Some(_)) => (), // Also acceptable
        Err(_) => (), // Error is acceptable for invalid keys
    }

    println!("  ✅ Error handling test passed");
    Ok(())
}

async fn test_concurrent_access(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("  🔄 Testing concurrent access...");

    let adapter = Arc::clone(&adapter);
    let mut handles = vec![];

    // Spawn concurrent operations
    for i in 0..20 {
        let adapter_clone = Arc::clone(&adapter);
        let handle = tokio::spawn(async move {
            let key = format!("concurrent_key_{}", i);
            let value = format!("concurrent_value_{}", i);

            // PUT operation
            adapter_clone.put(key.as_bytes(), value.as_bytes()).await?;

            // GET operation
            let retrieved = adapter_clone.get(key.as_bytes()).await?
                .ok_or_else(|| KotobaError::Storage("Concurrent key not found".to_string()))?;
            assert_eq!(retrieved, value.as_bytes());

            // DELETE operation
            adapter_clone.delete(key.as_bytes()).await?;

            Ok::<(), KotobaError>(())
        });
        handles.push(handle);
    }

    // Wait for all concurrent operations
    for handle in handles {
        handle.await??;
    }

    println!("  ✅ Concurrent access test passed");
    Ok(())
}

async fn test_large_data_handling(
    adapter: Arc<dyn KeyValueStore + Send + Sync>,
    adapter_name: &str
) -> Result<(), KotobaError> {
    println!("  📊 Testing large data handling...");

    // Test with moderately large data (1MB)
    let large_data_size = 1024 * 1024; // 1MB
    let large_value = vec![b'A'; large_data_size];

    let key = "large_data_key";

    // PUT large data
    adapter.put(key.as_bytes(), &large_value).await?;

    // GET large data
    let retrieved = adapter.get(key.as_bytes()).await?
        .ok_or_else(|| KotobaError::Storage("Large data key not found".to_string()))?;
    assert_eq!(retrieved.len(), large_data_size);
    assert_eq!(retrieved, large_value);

    // Test with multiple large entries
    for i in 0..5 {
        let key = format!("large_key_{}", i);
        let value = vec![b'B' + (i as u8); large_data_size / 10]; // 100KB each
        adapter.put(key.as_bytes(), &value).await?;
    }

    // Verify all large entries
    for i in 0..5 {
        let key = format!("large_key_{}", i);
        let expected_value = vec![b'B' + (i as u8); large_data_size / 10];
        let retrieved = adapter.get(key.as_bytes()).await?
            .ok_or_else(|| KotobaError::Storage(format!("Large key {} not found", i)))?;
        assert_eq!(retrieved, expected_value);
    }

    // Cleanup
    adapter.delete(key.as_bytes()).await?;
    for i in 0..5 {
        let key = format!("large_key_{}", i);
        let _ = adapter.delete(key.as_bytes()).await;
    }

    println!("  ✅ Large data handling test passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_adapter() {
        let fixture = StorageAdapterTestFixture::new().await.unwrap();

        // Run all storage adapter tests for memory adapter
        run_storage_adapter_tests(Arc::clone(&fixture.memory_adapter), "MemoryAdapter").await.unwrap();

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_rocksdb_adapter() {
        let fixture = StorageAdapterTestFixture::new().await.unwrap();

        if let Some(ref rocksdb_adapter) = fixture.rocksdb_adapter {
            // Run all storage adapter tests for RocksDB adapter
            run_storage_adapter_tests(Arc::clone(rocksdb_adapter), "RocksDbAdapter").await.unwrap();
        } else {
            println!("⚠️ RocksDB adapter not available, skipping RocksDB tests");
        }

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_adapter_consistency() {
        let fixture = StorageAdapterTestFixture::new().await.unwrap();

        // Test data consistency between adapters (if both available)
        let test_key = b"consistency_test";
        let test_value = b"consistency_value";

        // Test memory adapter
        fixture.memory_adapter.put(test_key, test_value).await.unwrap();
        let memory_result = fixture.memory_adapter.get(test_key).await.unwrap().unwrap();
        assert_eq!(memory_result, test_value);

        // Test RocksDB adapter (if available)
        if let Some(ref rocksdb_adapter) = fixture.rocksdb_adapter {
            rocksdb_adapter.put(test_key, test_value).await.unwrap();
            let rocksdb_result = rocksdb_adapter.get(test_key).await.unwrap().unwrap();
            assert_eq!(rocksdb_result, test_value);
        }

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_adapter_isolation() {
        let fixture = StorageAdapterTestFixture::new().await.unwrap();

        // Test that adapters are properly isolated
        let key1 = b"isolation_key_1";
        let key2 = b"isolation_key_2";
        let value1 = b"value_for_memory";
        let value2 = b"value_for_rocksdb";

        // Put different values in each adapter
        fixture.memory_adapter.put(key1, value1).await.unwrap();

        if let Some(ref rocksdb_adapter) = fixture.rocksdb_adapter {
            rocksdb_adapter.put(key2, value2).await.unwrap();

            // Verify isolation - each adapter should only have its own data
            let memory_result1 = fixture.memory_adapter.get(key1).await.unwrap().unwrap();
            assert_eq!(memory_result1, value1);

            let memory_result2 = fixture.memory_adapter.get(key2).await.unwrap();
            assert!(memory_result2.is_none()); // Memory adapter shouldn't have RocksDB data

            let rocksdb_result1 = rocksdb_adapter.get(key1).await.unwrap();
            assert!(rocksdb_result1.is_none()); // RocksDB adapter shouldn't have memory data

            let rocksdb_result2 = rocksdb_adapter.get(key2).await.unwrap().unwrap();
            assert_eq!(rocksdb_result2, value2);
        }

        fixture.cleanup().await.unwrap();
    }
}

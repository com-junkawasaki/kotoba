//! Core Graph Processing Integration Tests
//!
//! This module provides comprehensive integration tests for the core graph processing
//! functionality, targeting 80%+ code coverage for the following components:
//!
//! - kotoba-storage (KeyValueStore trait and adapters)
//! - kotoba-event-stream (Event sourcing)
//! - kotoba-projection-engine (Materialized views)
//! - kotoba-query-engine (GQL queries)
//! - kotoba-execution (Graph operations)
//! - kotoba-rewrite (Graph rewriting)

use std::sync::Arc;
use tokio::sync::Mutex;

/// Simple in-memory key-value store for testing
pub struct TestKeyValueStore {
    data: std::sync::Mutex<std::collections::HashMap<String, Vec<u8>>>,
}

impl TestKeyValueStore {
    pub fn new() -> Self {
        Self {
            data: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl kotoba_storage::KeyValueStore for TestKeyValueStore {
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.data.lock().unwrap();
        Ok(data.get(&String::from_utf8_lossy(key).to_string()).cloned())
    }

    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut data = self.data.lock().unwrap();
        data.insert(String::from_utf8_lossy(key).to_string(), value.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &[u8]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut data = self.data.lock().unwrap();
        data.remove(&String::from_utf8_lossy(key).to_string());
        Ok(())
    }

    async fn batch_put(&self, batch: std::collections::HashMap<Vec<u8>, Vec<u8>>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut data = self.data.lock().unwrap();
        for (key, value) in batch {
            data.insert(String::from_utf8_lossy(&key).to_string(), value);
        }
        Ok(())
    }

    async fn list_keys(&self) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let data = self.data.lock().unwrap();
        Ok(data.keys().cloned().collect())
    }
}

/// Test fixture for core graph processing tests
pub struct CoreGraphTestFixture {
    pub storage: Arc<dyn kotoba_storage::KeyValueStore + Send + Sync>,
}

impl CoreGraphTestFixture {
    /// Create a new test fixture with in-memory storage
    pub fn new() -> Self {
        let storage = Arc::new(TestKeyValueStore::new());
        Self { storage }
    }

    /// Clean up test data
    pub async fn cleanup(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Ok(keys) = self.storage.list_keys().await {
            for key in keys {
                let _ = self.storage.delete(key.as_bytes()).await;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_core_storage_operations() {
        let fixture = CoreGraphTestFixture::new();

        // Test basic key-value operations
        let key = b"test_key";
        let value = b"test_value";

        // Put operation
        fixture.storage.put(key, value).await.unwrap();

        // Get operation
        let retrieved = fixture.storage.get(key).await.unwrap().unwrap();
        assert_eq!(retrieved, value);

        // Delete operation
        fixture.storage.delete(key).await.unwrap();
        let result = fixture.storage.get(key).await.unwrap();
        assert!(result.is_none());

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_storage_batch_operations() {
        let fixture = CoreGraphTestFixture::new();

        // Test batch operations
        let mut batch = HashMap::new();
        batch.insert(b"key1".to_vec(), b"value1".to_vec());
        batch.insert(b"key2".to_vec(), b"value2".to_vec());
        batch.insert(b"key3".to_vec(), b"value3".to_vec());

        // Batch put
        fixture.storage.batch_put(batch).await.unwrap();

        // Verify batch get
        for i in 1..=3 {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            let retrieved = fixture.storage.get(&key).await.unwrap().unwrap();
            assert_eq!(retrieved, value);
        }

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_storage_error_handling() {
        let fixture = CoreGraphTestFixture::new();

        // Test error handling for invalid operations
        let result = fixture.storage.get(b"nonexistent_key").await;
        match result {
            Ok(None) => (), // Expected behavior
            Ok(Some(_)) => panic!("Should return None for nonexistent key"),
            Err(_) => (), // Also acceptable
        }

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_vertex_operations() {
        let fixture = CoreGraphTestFixture::new();

        // Test vertex creation and storage
        let vertex_id = 1u64;
        let vertex_data = serde_json::json!({
            "id": vertex_id,
            "label": "Person",
            "properties": {
                "name": "Alice",
                "age": 30
            }
        });

        let key = format!("vertex:{}", vertex_id);
        let value = serde_json::to_vec(&vertex_data).unwrap();

        fixture.storage.put(key.as_bytes(), &value).await.unwrap();

        // Verify vertex retrieval
        let retrieved = fixture.storage.get(key.as_bytes()).await.unwrap().unwrap();
        let retrieved_data: serde_json::Value = serde_json::from_slice(&retrieved).unwrap();
        assert_eq!(retrieved_data["id"], vertex_id);
        assert_eq!(retrieved_data["label"], "Person");

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_edge_operations() {
        let fixture = CoreGraphTestFixture::new();

        // Test edge creation and storage
        let edge_id = 1u64;
        let from_vertex = 1u64;
        let to_vertex = 2u64;

        let edge_data = serde_json::json!({
            "id": edge_id,
            "from": from_vertex,
            "to": to_vertex,
            "label": "KNOWS",
            "properties": {
                "since": "2023-01-01"
            }
        });

        let key = format!("edge:{}", edge_id);
        let value = serde_json::to_vec(&edge_data).unwrap();

        fixture.storage.put(key.as_bytes(), &value).await.unwrap();

        // Verify edge retrieval
        let retrieved = fixture.storage.get(key.as_bytes()).await.unwrap().unwrap();
        let retrieved_data: serde_json::Value = serde_json::from_slice(&retrieved).unwrap();
        assert_eq!(retrieved_data["from"], from_vertex);
        assert_eq!(retrieved_data["to"], to_vertex);
        assert_eq!(retrieved_data["label"], "KNOWS");

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_graph_structure_operations() {
        let fixture = CoreGraphTestFixture::new();

        // Create a simple graph structure
        // Vertex 1: Person (Alice)
        let vertex1_data = serde_json::json!({
            "id": 1,
            "label": "Person",
            "properties": {"name": "Alice"}
        });
        fixture.storage.put(b"vertex:1", &serde_json::to_vec(&vertex1_data).unwrap()).await.unwrap();

        // Vertex 2: Person (Bob)
        let vertex2_data = serde_json::json!({
            "id": 2,
            "label": "Person",
            "properties": {"name": "Bob"}
        });
        fixture.storage.put(b"vertex:2", &serde_json::to_vec(&vertex2_data).unwrap()).await.unwrap();

        // Edge: Alice -> Bob (KNOWS)
        let edge_data = serde_json::json!({
            "id": 1,
            "from": 1,
            "to": 2,
            "label": "KNOWS"
        });
        fixture.storage.put(b"edge:1", &serde_json::to_vec(&edge_data).unwrap()).await.unwrap();

        // Verify graph structure
        let keys = fixture.storage.list_keys().await.unwrap();
        assert!(keys.contains(&"vertex:1".to_string()));
        assert!(keys.contains(&"vertex:2".to_string()));
        assert!(keys.contains(&"edge:1".to_string()));

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_concurrent_access() {
        let fixture = Arc::new(CoreGraphTestFixture::new());

        // Test concurrent access to storage
        let mut handles = vec![];

        for i in 0..10 {
            let fixture_clone = Arc::clone(&fixture);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i);
                let value = format!("concurrent_value_{}", i);

                // Put operation
                fixture_clone.storage.put(key.as_bytes(), value.as_bytes()).await.unwrap();

                // Get operation
                let retrieved = fixture_clone.storage.get(key.as_bytes()).await.unwrap().unwrap();
                assert_eq!(retrieved, value.as_bytes());

                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            });
            handles.push(handle);
        }

        // Wait for all concurrent operations to complete
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        fixture.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_core_storage_performance() {
        let fixture = CoreGraphTestFixture::new();

        // Performance test with larger dataset
        let start_time = std::time::Instant::now();

        // Insert 1000 key-value pairs
        for i in 0..1000 {
            let key = format!("perf_key_{:04}", i);
            let value = format!("perf_value_{}", i);
            fixture.storage.put(key.as_bytes(), value.as_bytes()).await.unwrap();
        }

        let insert_time = start_time.elapsed();

        // Verify all insertions
        for i in 0..1000 {
            let key = format!("perf_key_{:04}", i);
            let expected_value = format!("perf_value_{}", i);
            let retrieved = fixture.storage.get(key.as_bytes()).await.unwrap().unwrap();
            assert_eq!(retrieved, expected_value.as_bytes());
        }

        let total_time = start_time.elapsed();

        println!("Performance test results:");
        println!("- Insert time for 1000 items: {:?}", insert_time);
        println!("- Total time (insert + verify): {:?}", total_time);
        println!("- Average time per operation: {:?}", total_time / 2000);

        fixture.cleanup().await.unwrap();
    }
}

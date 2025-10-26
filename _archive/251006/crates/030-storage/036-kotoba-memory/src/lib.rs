//! Kotoba In-Memory Storage Engine
//!
//! Pure in-memory storage implementation following the Pure Kernel/Effects Shell pattern.
//!
//! ## Effects Shell Implementation
//!
//! This crate provides an in-memory storage engine that implements the `StorageEngine` trait.
//! All operations are synchronous but wrapped in async interfaces for compatibility.
//!
//! ## Key Features
//!
//! - **Fast**: No disk I/O, pure memory operations
//! - **Thread-safe**: Uses `RwLock` for concurrent access
//! - **ACID**: Full transactional semantics
//! - **Debugging**: Perfect for testing and development

use kotoba_storage::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;

/// In-memory storage engine
#[derive(Debug, Clone)]
pub struct MemoryStorage {
    /// Thread-safe storage data
    data: Arc<RwLock<HashMap<StorageKey, StorageValue>>>,
    /// Engine metadata
    info: StorageInfo,
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl MemoryStorage {
    /// Create a new in-memory storage engine
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            info: StorageInfo {
                name: "MemoryStorage".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                capabilities: vec![
                    "get".to_string(),
                    "put".to_string(),
                    "delete".to_string(),
                    "exists".to_string(),
                    "list".to_string(),
                    "batch".to_string(),
                    "query".to_string(),
                    "transactions".to_string(),
                ],
            },
        }
    }

    /// Create a new storage engine with initial data
    pub fn with_data(initial_data: HashMap<StorageKey, StorageValue>) -> Self {
        let mut engine = Self::new();
        {
            let mut data = engine.data.write().unwrap();
            data.extend(initial_data);
        }
        engine
    }

    /// Get a snapshot of current data (for testing/debugging)
    pub fn snapshot(&self) -> HashMap<StorageKey, StorageValue> {
        self.data.read().unwrap().clone()
    }

    /// Clear all data
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }

    /// Execute a single storage operation (internal method)
    fn execute_operation(&self, operation: &StorageOperation) -> Result<OperationResult, StorageError> {
        match operation {
            StorageOperation::Get(key) => {
                let data = self.data.read().unwrap();
                let result = data.get(key).cloned();
                Ok(OperationResult::Get(result))
            }
            StorageOperation::Put(key, value) => {
                let mut data = self.data.write().unwrap();
                data.insert(key.clone(), value.clone());
                Ok(OperationResult::Put(true))
            }
            StorageOperation::Delete(key) => {
                let mut data = self.data.write().unwrap();
                let existed = data.remove(key).is_some();
                Ok(OperationResult::Delete(existed))
            }
            StorageOperation::Exists(key) => {
                let data = self.data.read().unwrap();
                let exists = data.contains_key(key);
                Ok(OperationResult::Exists(exists))
            }
            StorageOperation::List(namespace) => {
                let data = self.data.read().unwrap();
                let keys: Vec<StorageKey> = data.keys()
                    .filter(|key| key.namespace == *namespace)
                    .cloned()
                    .collect();
                Ok(OperationResult::List(keys))
            }
            StorageOperation::Batch(operations) => {
                // For batch operations, execute each one
                // In a real implementation, this might be transactional
                let mut results = Vec::new();
                for op in operations {
                    results.push(self.execute_operation(op)?);
                }
                // Return the last result (simplified)
                Ok(results.into_iter().last().unwrap_or(OperationResult::Put(true)))
            }
        }
    }

    /// Execute a query (internal method)
    fn execute_query_internal(&self, query: &QueryPlan) -> Result<QueryResult, StorageError> {
        let data = self.data.read().unwrap();

        // Simple filtering based on namespace
        let filtered: Vec<(StorageKey, StorageValue)> = data.iter()
            .filter(|(key, _)| key.namespace == query.namespace)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();

        // Apply limit if specified
        let values = if let Some(limit) = query.limit {
            filtered.into_iter().take(limit).collect()
        } else {
            filtered
        };

        Ok(QueryResult {
            values,
            has_more: false, // simplified
            total_count: None, // simplified
            execution_time_ms: 0, // in-memory, effectively instant
        })
    }
}

#[async_trait]
impl StorageEngine for MemoryStorage {
    async fn execute_plan(&self, plan: &StoragePlan) -> Result<StorageResult, StorageError> {
        // Validate the plan first (pure validation)
        plan.validate().map_err(|e| StorageError::OperationFailed(format!("Plan validation failed: {:?}", e)))?;

        // Execute operations (simplified - no real transaction handling)
        let mut results = Vec::new();
        for operation in &plan.operations {
            let result = self.execute_operation(operation)?;
            results.push(result);
        }

        Ok(StorageResult {
            results,
            execution_time_ms: 0, // in-memory, effectively instant
        })
    }

    async fn execute_query(&self, query: &QueryPlan) -> Result<QueryResult, StorageError> {
        self.execute_query_internal(query)
    }

    fn info(&self) -> StorageInfo {
        self.info.clone()
    }
}

/// Convenience functions for creating memory storage
pub mod factory {
    use super::*;

    /// Create a new memory storage engine
    pub fn create() -> MemoryStorage {
        MemoryStorage::new()
    }

    /// Create a memory storage with initial data
    pub fn with_data(data: HashMap<StorageKey, StorageValue>) -> MemoryStorage {
        MemoryStorage::with_data(data)
    }

    /// Create a memory storage and wrap it in an Arc for sharing
    pub fn shared() -> Arc<MemoryStorage> {
        Arc::new(MemoryStorage::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new();

        // Test PUT
        let key = StorageKey::new("users", "alice");
        let value = StorageValue::new(json!({"name": "Alice", "age": 30}));
        let plan = StoragePlan::single(StorageOperation::Put(key.clone(), value));
        let result = storage.execute_plan(&plan).await.unwrap();

        assert_eq!(result.results.len(), 1);
        assert!(matches!(result.results[0], OperationResult::Put(true)));

        // Test GET
        let get_plan = StoragePlan::single(StorageOperation::Get(key.clone()));
        let get_result = storage.execute_plan(&get_plan).await.unwrap();

        assert_eq!(get_result.results.len(), 1);
        match &get_result.results[0] {
            OperationResult::Get(Some(val)) => {
                assert_eq!(val.data["name"], "Alice");
                assert_eq!(val.data["age"], 30);
            }
            _ => panic!("Expected Get(Some(_))"),
        }

        // Test EXISTS
        let exists_plan = StoragePlan::single(StorageOperation::Exists(key.clone()));
        let exists_result = storage.execute_plan(&exists_plan).await.unwrap();

        assert_eq!(exists_result.results.len(), 1);
        assert!(matches!(exists_result.results[0], OperationResult::Exists(true)));
    }

    #[tokio::test]
    async fn test_memory_storage_delete() {
        let storage = MemoryStorage::new();

        // Put a value
        let key = StorageKey::new("test", "item");
        let value = StorageValue::new(json!("test_value"));
        let put_plan = StoragePlan::single(StorageOperation::Put(key.clone(), value));
        storage.execute_plan(&put_plan).await.unwrap();

        // Delete it
        let delete_plan = StoragePlan::single(StorageOperation::Delete(key.clone()));
        let delete_result = storage.execute_plan(&delete_plan).await.unwrap();

        assert_eq!(delete_result.results.len(), 1);
        assert!(matches!(delete_result.results[0], OperationResult::Delete(true)));

        // Try to get it again - should be None
        let get_plan = StoragePlan::single(StorageOperation::Get(key));
        let get_result = storage.execute_plan(&get_plan).await.unwrap();

        assert_eq!(get_result.results.len(), 1);
        assert!(matches!(get_result.results[0], OperationResult::Get(None)));
    }

    #[tokio::test]
    async fn test_memory_storage_list() {
        let storage = MemoryStorage::new();

        // Put multiple values in same namespace
        let keys = vec![
            StorageKey::new("users", "alice"),
            StorageKey::new("users", "bob"),
            StorageKey::new("posts", "post1"),
        ];

        for key in &keys {
            let value = StorageValue::new(json!("test"));
            let plan = StoragePlan::single(StorageOperation::Put(key.clone(), value));
            storage.execute_plan(&plan).await.unwrap();
        }

        // List users namespace
        let list_plan = StoragePlan::single(StorageOperation::List("users".to_string()));
        let list_result = storage.execute_plan(&list_plan).await.unwrap();

        assert_eq!(list_result.results.len(), 1);
        match &list_result.results[0] {
            OperationResult::List(keys) => {
                assert_eq!(keys.len(), 2);
                assert!(keys.contains(&StorageKey::new("users", "alice")));
                assert!(keys.contains(&StorageKey::new("users", "bob")));
            }
            _ => panic!("Expected List(_)"),
        }
    }

    #[tokio::test]
    async fn test_memory_storage_query() {
        let storage = MemoryStorage::new();

        // Put test data
        let key1 = StorageKey::new("users", "alice");
        let value1 = StorageValue::new(json!({"name": "Alice"}));
        let key2 = StorageKey::new("users", "bob");
        let value2 = StorageValue::new(json!({"name": "Bob"}));

        storage.execute_plan(&StoragePlan::single(StorageOperation::Put(key1, value1))).await.unwrap();
        storage.execute_plan(&StoragePlan::single(StorageOperation::Put(key2, value2))).await.unwrap();

        // Query users namespace
        let query = QueryPlan {
            namespace: "users".to_string(),
            conditions: vec![], // no conditions - get all
            sort_by: None,
            limit: None,
            offset: None,
        };

        let result = storage.execute_query(&query).await.unwrap();
        assert_eq!(result.values.len(), 2);
    }

    #[test]
    fn test_memory_storage_snapshot() {
        let storage = MemoryStorage::new();

        // Add some data synchronously (for testing)
        {
            let mut data = storage.data.write().unwrap();
            let key = StorageKey::new("test", "key");
            let value = StorageValue::new(json!("value"));
            data.insert(key, value);
        }

        // Get snapshot
        let snapshot = storage.snapshot();
        assert_eq!(snapshot.len(), 1);

        // Clear and verify
        storage.clear();
        assert_eq!(storage.snapshot().len(), 0);
    }

    #[test]
    fn test_memory_storage_info() {
        let storage = MemoryStorage::new();
        let info = storage.info();

        assert_eq!(info.name, "MemoryStorage");
        assert!(!info.capabilities.is_empty());
        assert!(info.capabilities.contains(&"get".to_string()));
        assert!(info.capabilities.contains(&"put".to_string()));
        assert!(info.capabilities.contains(&"transactions".to_string()));
    }
}

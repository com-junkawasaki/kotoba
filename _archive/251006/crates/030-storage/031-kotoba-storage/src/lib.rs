//! Kotoba Storage Interface
//!
//! Core storage abstractions following the Pure Kernel/Effects Shell pattern.
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate defines the storage abstractions:
//!
//! - **Pure Kernel**: `StoragePlan`, `QueryPlan` - pure data structures and planning
//! - **Effects Shell**: `StorageEngine` - handles actual I/O operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pure representation of a storage key
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StorageKey {
    /// Key namespace/category
    pub namespace: String,
    /// Primary key identifier
    pub key: String,
    /// Optional sub-key for composite keys
    pub sub_key: Option<String>,
}

impl StorageKey {
    /// Create a simple key
    pub fn new(namespace: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
            sub_key: None,
        }
    }

    /// Create a composite key
    pub fn with_sub_key(
        namespace: impl Into<String>,
        key: impl Into<String>,
        sub_key: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
            sub_key: Some(sub_key.into()),
        }
    }

    /// Get the full key path
    pub fn full_path(&self) -> String {
        match &self.sub_key {
            Some(sub) => format!("{}:{}:{}", self.namespace, self.key, sub),
            None => format!("{}:{}", self.namespace, self.key),
        }
    }
}

/// Pure representation of a storage value
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StorageValue {
    /// Actual data content
    pub data: serde_json::Value,
    /// Optional metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Version/timestamp for optimistic concurrency
    pub version: Option<u64>,
}

impl StorageValue {
    /// Create a simple value
    pub fn new(data: serde_json::Value) -> Self {
        Self {
            data,
            metadata: HashMap::new(),
            version: None,
        }
    }

    /// Create a value with metadata
    pub fn with_metadata(data: serde_json::Value, metadata: HashMap<String, serde_json::Value>) -> Self {
        Self {
            data,
            metadata,
            version: None,
        }
    }
}

/// Pure storage operation plan
#[derive(Debug, Clone, PartialEq)]
pub enum StorageOperation {
    /// Get a single value
    Get(StorageKey),
    /// Put a value
    Put(StorageKey, StorageValue),
    /// Delete a value
    Delete(StorageKey),
    /// Check if key exists
    Exists(StorageKey),
    /// List keys in namespace
    List(String), // namespace
    /// Batch operations
    Batch(Vec<StorageOperation>),
}

impl StorageOperation {
    /// Check if operation is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, StorageOperation::Get(_) | StorageOperation::Exists(_) | StorageOperation::List(_))
    }

    /// Get keys affected by this operation
    pub fn affected_keys(&self) -> Vec<&StorageKey> {
        match self {
            StorageOperation::Get(key) => vec![key],
            StorageOperation::Put(key, _) => vec![key],
            StorageOperation::Delete(key) => vec![key],
            StorageOperation::Exists(key) => vec![key],
            StorageOperation::List(_) => vec![], // affects all keys in namespace
            StorageOperation::Batch(ops) => ops.iter().flat_map(|op| op.affected_keys()).collect(),
        }
    }
}

/// Pure query plan
#[derive(Debug, Clone, PartialEq)]
pub struct QueryPlan {
    /// Target namespace
    pub namespace: String,
    /// Query conditions (pure predicates)
    pub conditions: Vec<QueryCondition>,
    /// Sort specification
    pub sort_by: Option<SortSpec>,
    /// Limit specification
    pub limit: Option<usize>,
}

/// Pure query condition
#[derive(Debug, Clone, PartialEq)]
pub enum QueryCondition {
    /// Equality condition
    Equals(String, serde_json::Value), // field, value
    /// Range condition
    Range(String, serde_json::Value), // field, range (simplified)
    /// Logical AND
    And(Vec<QueryCondition>),
    /// Logical OR
    Or(Vec<QueryCondition>),
}

/// Sort specification
#[derive(Debug, Clone, PartialEq)]
pub struct SortSpec {
    pub field: String,
    pub ascending: bool,
}

/// Pure storage plan - represents a complete storage transaction plan
#[derive(Debug, Clone, PartialEq)]
pub struct StoragePlan {
    /// Operations to execute
    pub operations: Vec<StorageOperation>,
    /// Expected version for optimistic concurrency
    pub expected_version: Option<u64>,
    /// Whether this plan can be executed in parallel with others
    pub is_commutative: bool,
}

impl StoragePlan {
    /// Create a simple plan with single operation
    pub fn single(operation: StorageOperation) -> Self {
        Self {
            operations: vec![operation],
            expected_version: None,
            is_commutative: operation.is_read_only(),
        }
    }

    /// Create a batch plan
    pub fn batch(operations: Vec<StorageOperation>) -> Self {
        let is_commutative = operations.iter().all(|op| op.is_read_only());
        Self {
            operations,
            expected_version: None,
            is_commutative,
        }
    }

    /// Check if plan conflicts with another plan
    pub fn conflicts_with(&self, other: &StoragePlan) -> bool {
        if self.is_commutative && other.is_commutative {
            return false; // read-only plans don't conflict
        }

        let self_keys: std::collections::HashSet<_> = self.operations
            .iter()
            .flat_map(|op| op.affected_keys())
            .collect();

        let other_keys: std::collections::HashSet<_> = other.operations
            .iter()
            .flat_map(|op| op.affected_keys())
            .collect();

        // Check for key conflicts
        !self_keys.is_disjoint(&other_keys)
    }

    /// Validate the plan (pure validation)
    pub fn validate(&self) -> Result<(), StoragePlanError> {
        if self.operations.is_empty() {
            return Err(StoragePlanError::EmptyPlan);
        }

        for operation in &self.operations {
            match operation {
                StorageOperation::Batch(nested) => {
                    if nested.is_empty() {
                        return Err(StoragePlanError::EmptyBatch);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }
}

/// Errors that can occur during plan validation
#[derive(Debug, Clone, PartialEq)]
pub enum StoragePlanError {
    EmptyPlan,
    EmptyBatch,
    InvalidOperation(String),
}

/// Effects Shell trait for storage engines
#[async_trait::async_trait]
pub trait StorageEngine: Send + Sync {
    /// Execute a storage plan (effects: I/O operations)
    async fn execute_plan(&self, plan: &StoragePlan) -> Result<StorageResult, StorageError>;

    /// Execute a query plan (effects: I/O operations)
    async fn execute_query(&self, query: &QueryPlan) -> Result<QueryResult, StorageError>;

    /// Get storage engine information
    fn info(&self) -> StorageInfo;
}

/// Result of storage operations
#[derive(Debug, Clone)]
pub struct StorageResult {
    /// Results for each operation
    pub results: Vec<OperationResult>,
    /// Execution time
    pub execution_time_ms: u64,
}

/// Result of a single operation
#[derive(Debug, Clone)]
pub enum OperationResult {
    /// Get operation result
    Get(Option<StorageValue>),
    /// Put operation result
    Put(bool), // success
    /// Delete operation result
    Delete(bool), // existed
    /// Exists operation result
    Exists(bool),
    /// List operation result
    List(Vec<StorageKey>),
}

/// Result of query operations
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Matching values
    pub values: Vec<(StorageKey, StorageValue)>,
    /// Whether there are more results
    pub has_more: bool,
    /// Total count (if available)
    pub total_count: Option<usize>,
    /// Execution time
    pub execution_time_ms: u64,
}

/// Storage engine information
#[derive(Debug, Clone)]
pub struct StorageInfo {
    pub name: String,
    pub version: String,
    pub capabilities: Vec<String>,
}

/// Storage operation errors
#[derive(Debug, Clone)]
pub enum StorageError {
    ConnectionFailed(String),
    OperationFailed(String),
    SerializationError(String),
    VersionConflict(String),
    KeyNotFound(StorageKey),
    IoError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_key_creation() {
        let key = StorageKey::new("users", "alice");
        assert_eq!(key.namespace, "users");
        assert_eq!(key.key, "alice");
        assert!(key.sub_key.is_none());
        assert_eq!(key.full_path(), "users:alice");
    }

    #[test]
    fn test_storage_key_with_sub_key() {
        let key = StorageKey::with_sub_key("users", "alice", "profile");
        assert_eq!(key.full_path(), "users:alice:profile");
    }

    #[test]
    fn test_storage_operation_read_only() {
        let get_op = StorageOperation::Get(StorageKey::new("test", "key"));
        let put_op = StorageOperation::Put(StorageKey::new("test", "key"), StorageValue::new(serde_json::Value::Null));

        assert!(get_op.is_read_only());
        assert!(!put_op.is_read_only());
    }

    #[test]
    fn test_storage_plan_validation() {
        let valid_plan = StoragePlan::single(StorageOperation::Get(StorageKey::new("test", "key")));
        assert!(valid_plan.validate().is_ok());

        let invalid_plan = StoragePlan {
            operations: vec![],
            expected_version: None,
            is_commutative: true,
        };
        assert!(matches!(invalid_plan.validate(), Err(StoragePlanError::EmptyPlan)));
    }

    #[test]
    fn test_storage_plan_conflicts() {
        let read_plan = StoragePlan::single(StorageOperation::Get(StorageKey::new("test", "key")));
        let write_plan = StoragePlan::single(StorageOperation::Put(
            StorageKey::new("test", "key"),
            StorageValue::new(serde_json::Value::Null)
        ));

        // Read-only plans don't conflict with each other
        assert!(!read_plan.conflicts_with(&read_plan));

        // Write plans conflict with read plans on same key
        assert!(read_plan.conflicts_with(&write_plan));
        assert!(write_plan.conflicts_with(&read_plan));

        // Write plans conflict with each other
        assert!(write_plan.conflicts_with(&write_plan));
    }

    #[test]
    fn test_storage_value_creation() {
        let data = serde_json::json!({"name": "Alice", "age": 30});
        let value = StorageValue::new(data.clone());

        assert_eq!(value.data, data);
        assert!(value.metadata.is_empty());
        assert!(value.version.is_none());
    }

    #[test]
    fn test_storage_value_with_metadata() {
        let data = serde_json::json!({"name": "Bob"});
        let mut metadata = HashMap::new();
        metadata.insert("created_at".to_string(), serde_json::json!("2024-01-01"));

        let value = StorageValue::with_metadata(data.clone(), metadata.clone());

        assert_eq!(value.data, data);
        assert_eq!(value.metadata, metadata);
        assert!(value.version.is_none());
    }
}

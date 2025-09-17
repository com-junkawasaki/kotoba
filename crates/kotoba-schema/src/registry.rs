//! Schema Registry
//!
//! This module provides a centralized registry for managing schema collections
//! and provides high-level operations for schema management.

use crate::schema::*;
use crate::manager::*;
use kotoba_core::types::*;
use kotoba_errors::KotobaError;
use std::collections::HashMap;
use std::sync::Arc;

/// Schema registry for managing multiple schemas
pub struct SchemaRegistry {
    /// Schema managers organized by namespace
    managers: HashMap<String, Arc<dyn SchemaStorage>>,
    /// Default namespace
    default_namespace: String,
}

impl SchemaRegistry {
    /// Create a new schema registry
    pub fn new() -> Self {
        Self {
            managers: HashMap::new(),
            default_namespace: "default".to_string(),
        }
    }

    /// Create a registry with a default namespace
    pub fn with_default_namespace(namespace: String) -> Self {
        Self {
            managers: HashMap::new(),
            default_namespace: namespace,
        }
    }

    /// Register a storage backend for a namespace
    pub fn register_namespace(&mut self, namespace: String, storage: Arc<dyn SchemaStorage>) {
        self.managers.insert(namespace, storage);
    }

    /// Get a schema manager for a namespace
    pub fn get_manager(&self, namespace: &str) -> Result<SchemaManager> {
        let storage = self.managers.get(namespace)
            .ok_or_else(|| KotobaError::Storage(format!("Namespace '{}' not found", namespace)))?;

        Ok(SchemaManager::new(Arc::clone(storage)))
    }

    /// Get the default schema manager
    pub fn get_default_manager(&self) -> Result<SchemaManager> {
        self.get_manager(&self.default_namespace)
    }

    /// Create a new namespace with in-memory storage
    pub fn create_namespace(&mut self, namespace: String) {
        let storage = Arc::new(InMemorySchemaStorage::new());
        self.register_namespace(namespace, storage);
    }

    /// List all namespaces
    pub fn list_namespaces(&self) -> Vec<String> {
        self.managers.keys().cloned().collect()
    }

    /// Check if a namespace exists
    pub fn namespace_exists(&self, namespace: &str) -> bool {
        self.managers.contains_key(namespace)
    }

    /// Remove a namespace
    pub fn remove_namespace(&mut self, namespace: &str) -> Result<()> {
        if namespace == self.default_namespace {
            return Err(KotobaError::Storage("Cannot remove default namespace".to_string()));
        }

        self.managers.remove(namespace)
            .ok_or_else(|| KotobaError::Storage(format!("Namespace '{}' not found", namespace)))?;

        Ok(())
    }

    /// Cross-namespace schema operations
    pub async fn copy_schema(
        &mut self,
        source_namespace: &str,
        source_schema_id: &str,
        target_namespace: &str,
        target_schema_id: &str,
    ) -> Result<()> {
        // Get source schema
        let mut source_manager = self.get_manager(source_namespace)?;
        let schema = source_manager.get_schema(source_schema_id)?
            .ok_or_else(|| KotobaError::Storage(format!("Schema '{}' not found in namespace '{}'",
                                                      source_schema_id, source_namespace)))?;

        // Create target manager
        let mut target_manager = self.get_manager(target_namespace)?;

        // Modify schema for target
        let mut target_schema = schema;
        target_schema.id = target_schema_id.to_string();
        target_schema.name = format!("{} (from {})", target_schema.name, source_namespace);

        // Register in target namespace
        target_manager.register_schema(target_schema)?;

        Ok(())
    }

    /// Bulk schema operations
    pub fn bulk_register_schemas(
        &mut self,
        namespace: &str,
        schemas: Vec<GraphSchema>,
    ) -> Result<BulkOperationResult> {
        let mut manager = self.get_manager(namespace)?;
        let mut results = BulkOperationResult::default();

        for schema in schemas {
            match manager.register_schema(schema.clone()) {
                Ok(_) => {
                    results.successful.push(schema.id);
                },
                Err(e) => {
                    results.failed.push((schema.id, e.to_string()));
                }
            }
        }

        Ok(results)
    }

    /// Get registry statistics
    pub fn get_statistics(&self) -> Result<RegistryStatistics> {
        let mut stats = RegistryStatistics::default();

        for (namespace, storage) in &self.managers {
            let schema_count = storage.list_schemas()?.len();
            stats.namespaces.push(NamespaceStatistics {
                name: namespace.clone(),
                schema_count,
            });
            stats.total_schemas += schema_count;
        }

        Ok(stats)
    }

    /// Health check for all namespaces
    pub async fn health_check(&self) -> Result<Vec<NamespaceHealth>> {
        let mut results = Vec::new();

        for (namespace, storage) in &self.managers {
            let healthy = storage.list_schemas().is_ok();
            results.push(NamespaceHealth {
                namespace: namespace.clone(),
                healthy,
            });
        }

        Ok(results)
    }
}

/// Result of bulk operations
#[derive(Debug, Clone, Default)]
pub struct BulkOperationResult {
    pub successful: Vec<String>,
    pub failed: Vec<(String, String)>,
}

impl BulkOperationResult {
    /// Check if all operations were successful
    pub fn is_complete_success(&self) -> bool {
        self.failed.is_empty()
    }

    /// Get success rate as percentage
    pub fn success_rate(&self) -> f64 {
        let total = self.successful.len() + self.failed.len();
        if total == 0 {
            0.0
        } else {
            (self.successful.len() as f64 / total as f64) * 100.0
        }
    }
}

/// Registry statistics
#[derive(Debug, Clone, Default)]
pub struct RegistryStatistics {
    pub total_schemas: usize,
    pub namespaces: Vec<NamespaceStatistics>,
}

/// Namespace statistics
#[derive(Debug, Clone)]
pub struct NamespaceStatistics {
    pub name: String,
    pub schema_count: usize,
}

/// Namespace health status
#[derive(Debug, Clone)]
pub struct NamespaceHealth {
    pub namespace: String,
    pub healthy: bool,
}

/// Schema collection for batch operations
pub struct SchemaCollection {
    schemas: Vec<GraphSchema>,
}

impl SchemaCollection {
    /// Create a new empty collection
    pub fn new() -> Self {
        Self {
            schemas: Vec::new(),
        }
    }

    /// Add a schema to the collection
    pub fn add_schema(&mut self, schema: GraphSchema) {
        self.schemas.push(schema);
    }

    /// Add multiple schemas to the collection
    pub fn add_schemas(&mut self, schemas: Vec<GraphSchema>) {
        self.schemas.extend(schemas);
    }

    /// Get all schemas
    pub fn get_schemas(&self) -> &[GraphSchema] {
        &self.schemas
    }

    /// Get schemas by ID
    pub fn get_schema(&self, id: &str) -> Option<&GraphSchema> {
        self.schemas.iter().find(|s| s.id == id)
    }

    /// Remove a schema by ID
    pub fn remove_schema(&mut self, id: &str) -> Option<GraphSchema> {
        let position = self.schemas.iter().position(|s| s.id == id)?;
        Some(self.schemas.remove(position))
    }

    /// Get collection statistics
    pub fn statistics(&self) -> CollectionStatistics {
        let mut vertex_types = 0;
        let mut edge_types = 0;
        let mut total_properties = 0;

        for schema in &self.schemas {
            vertex_types += schema.vertex_types.len();
            edge_types += schema.edge_types.len();
            total_properties += schema.vertex_types.values()
                .map(|vt| vt.properties.len())
                .sum::<usize>() + schema.edge_types.values()
                .map(|et| et.properties.len())
                .sum::<usize>();
        }

        CollectionStatistics {
            schema_count: self.schemas.len(),
            total_vertex_types: vertex_types,
            total_edge_types: edge_types,
            total_properties,
        }
    }

    /// Validate all schemas in the collection
    pub fn validate_all(&self) -> ValidationSummary {
        let mut valid_schemas = Vec::new();
        let mut invalid_schemas = Vec::new();
        let mut total_errors = 0;

        for schema in &self.schemas {
            let validation = schema.validate_schema();
            if validation.is_valid {
                valid_schemas.push(schema.id.clone());
            } else {
                invalid_schemas.push((schema.id.clone(), validation.errors.len()));
                total_errors += validation.errors.len();
            }
        }

        ValidationSummary {
            valid_schemas,
            invalid_schemas,
            total_errors,
        }
    }
}

/// Collection statistics
#[derive(Debug, Clone)]
pub struct CollectionStatistics {
    pub schema_count: usize,
    pub total_vertex_types: usize,
    pub total_edge_types: usize,
    pub total_properties: usize,
}

/// Validation summary for a collection
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    pub valid_schemas: Vec<String>,
    pub invalid_schemas: Vec<(String, usize)>, // (schema_id, error_count)
    pub total_errors: usize,
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = SchemaRegistry::new();
        assert!(registry.list_namespaces().is_empty());
    }

    #[test]
    fn test_namespace_operations() {
        let mut registry = SchemaRegistry::new();

        // Create namespace
        registry.create_namespace("test_namespace".to_string());
        assert!(registry.namespace_exists("test_namespace"));
        assert!(!registry.namespace_exists("nonexistent"));

        // List namespaces
        let namespaces = registry.list_namespaces();
        assert_eq!(namespaces.len(), 1);
        assert!(namespaces.contains(&"test_namespace".to_string()));
    }

    #[test]
    fn test_schema_collection() {
        let mut collection = SchemaCollection::new();

        // Add schemas
        let schema1 = GraphSchema::new("schema1".to_string(), "Schema 1".to_string(), "1.0.0".to_string());
        let schema2 = GraphSchema::new("schema2".to_string(), "Schema 2".to_string(), "1.0.0".to_string());

        collection.add_schema(schema1);
        collection.add_schema(schema2);

        // Test statistics
        let stats = collection.statistics();
        assert_eq!(stats.schema_count, 2);

        // Test validation
        let validation = collection.validate_all();
        assert_eq!(validation.valid_schemas.len(), 2);
        assert_eq!(validation.invalid_schemas.len(), 0);
    }

    #[tokio::test]
    fn test_cross_namespace_operations() {
        let mut registry = SchemaRegistry::new();

        // Create two namespaces
        registry.create_namespace("source".to_string());
        registry.create_namespace("target".to_string());

        // Register schema in source
        let mut source_manager = registry.get_manager("source").unwrap();
        let schema = GraphSchema::new("test_schema".to_string(), "Test Schema".to_string(), "1.0.0".to_string());
        source_manager.register_schema(schema).unwrap();

        // Copy to target namespace
        registry.copy_schema("source", "test_schema", "target", "copied_schema").unwrap();

        // Verify in target
        let mut target_manager = registry.get_manager("target").unwrap();
        let copied = target_manager.get_schema("copied_schema").unwrap().unwrap();
        assert_eq!(copied.name, "Test Schema (from source)");
    }

    #[tokio::test]
    fn test_bulk_operations() {
        let mut registry = SchemaRegistry::new();
        registry.create_namespace("bulk_test".to_string());

        // Create multiple schemas
        let schemas = vec![
            GraphSchema::new("bulk1".to_string(), "Bulk Schema 1".to_string(), "1.0.0".to_string()),
            GraphSchema::new("bulk2".to_string(), "Bulk Schema 2".to_string(), "1.0.0".to_string()),
            GraphSchema::new("bulk3".to_string(), "Bulk Schema 3".to_string(), "1.0.0".to_string()),
        ];

        // Bulk register
        let result = registry.bulk_register_schemas("bulk_test", schemas).unwrap();
        assert!(result.is_complete_success());
        assert_eq!(result.successful.len(), 3);
        assert_eq!(result.failed.len(), 0);
    }

    #[tokio::test]
    fn test_registry_statistics() {
        let mut registry = SchemaRegistry::new();

        // Create namespace with schemas
        registry.create_namespace("stats_test".to_string());
        let mut manager = registry.get_manager("stats_test").unwrap();

        let schema = GraphSchema::new("stats_schema".to_string(), "Stats Schema".to_string(), "1.0.0".to_string());
        manager.register_schema(schema).unwrap();

        // Get statistics
        let stats = registry.get_statistics().unwrap();
        assert_eq!(stats.total_schemas, 1);
        assert_eq!(stats.namespaces.len(), 1);
        assert_eq!(stats.namespaces[0].schema_count, 1);
    }
}

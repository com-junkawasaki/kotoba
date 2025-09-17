//! Schema Manager
//!
//! This module provides the main interface for managing graph schemas,
//! including registration, retrieval, validation, and caching.

use crate::schema::*;
use crate::validator::*;
use kotoba_core::types::*;
use kotoba_errors::KotobaError;
use std::collections::HashMap;
use std::sync::Arc;

/// Schema manager for handling graph schemas
#[derive(Clone)]
pub struct SchemaManager {
    /// Storage backend for persisting schemas
    storage: Arc<dyn SchemaStorage>,
    /// Cache of loaded schemas
    cache: HashMap<String, GraphSchema>,
    /// Whether caching is enabled
    cache_enabled: bool,
}

/// Storage backend trait for schema persistence
pub trait SchemaStorage: Send + Sync {
    /// Store a schema
    fn store_schema(&self, schema_id: &str, schema: &GraphSchema) -> Result<()>;

    /// Load a schema by ID
    fn load_schema(&self, schema_id: &str) -> Result<Option<GraphSchema>>;

    /// Delete a schema
    fn delete_schema(&self, schema_id: &str) -> Result<()>;

    /// List all schema IDs
    fn list_schemas(&self) -> Result<Vec<String>>;

    /// Check if a schema exists
    fn schema_exists(&self, schema_id: &str) -> Result<bool> {
        Ok(self.load_schema(schema_id)?.is_some())
    }
}

impl SchemaManager {
    /// Create a new schema manager with the specified storage backend
    pub fn new(storage: Arc<dyn SchemaStorage>) -> Self {
        Self {
            storage,
            cache: HashMap::new(),
            cache_enabled: true,
        }
    }

    /// Create a schema manager with caching disabled
    pub fn without_cache(storage: Arc<dyn SchemaStorage>) -> Self {
        Self {
            storage,
            cache: HashMap::new(),
            cache_enabled: false,
        }
    }

    /// Register a new schema
    pub fn register_schema(&mut self, schema: GraphSchema) -> Result<()> {
        // Validate the schema
        let validation = schema.validate_schema();
        if !validation.is_valid {
            return Err(KotobaError::Storage(format!(
                "Schema validation failed: {:?}",
                validation.errors
            )));
        }

        // Store the schema
        self.storage.store_schema(&schema.id, &schema)?;

        // Cache the schema if caching is enabled
        if self.cache_enabled {
            self.cache.insert(schema.id.clone(), schema);
        }

        Ok(())
    }

    /// Get a schema by ID
    pub fn get_schema(&mut self, schema_id: &str) -> Result<Option<GraphSchema>> {
        // Check cache first
        if self.cache_enabled {
            if let Some(schema) = self.cache.get(schema_id) {
                return Ok(Some(schema.clone()));
            }
        }

        // Load from storage
        match self.storage.load_schema(schema_id)? {
            Some(schema) => {
                // Cache the schema if caching is enabled
                if self.cache_enabled {
                    self.cache.insert(schema_id.to_string(), schema.clone());
                }
                Ok(Some(schema))
            },
            None => Ok(None),
        }
    }

    /// Update an existing schema
    pub fn update_schema(&mut self, schema: GraphSchema) -> Result<()> {
        // Validate the updated schema
        let validation = schema.validate_schema();
        if !validation.is_valid {
            return Err(KotobaError::Storage(format!(
                "Schema validation failed: {:?}",
                validation.errors
            )));
        }

        // Check if schema exists
        if !self.storage.schema_exists(&schema.id)? {
            return Err(KotobaError::Storage(format!(
                "Schema '{}' does not exist",
                schema.id
            )));
        }

        // Store the updated schema
        self.storage.store_schema(&schema.id, &schema)?;

        // Update cache if caching is enabled
        if self.cache_enabled {
            self.cache.insert(schema.id.clone(), schema);
        }

        Ok(())
    }

    /// Delete a schema
    pub fn delete_schema(&mut self, schema_id: &str) -> Result<()> {
        // Delete from storage
        self.storage.delete_schema(schema_id)?;

        // Remove from cache
        if self.cache_enabled {
            self.cache.remove(schema_id);
        }

        Ok(())
    }

    /// List all registered schemas
    pub fn list_schemas(&self) -> Result<Vec<String>> {
        self.storage.list_schemas()
    }

    /// Validate graph data against a schema
    pub fn validate_graph_data(
        &mut self,
        schema_id: &str,
        graph_data: &serde_json::Value
    ) -> Result<ValidationResult> {
        let schema = match self.get_schema(schema_id)? {
            Some(s) => s,
            None => {
                return Ok(ValidationResult {
                    is_valid: false,
                    errors: vec![ValidationError {
                        error_type: ValidationErrorType::SchemaNotFound,
                        message: format!("Schema '{}' not found", schema_id),
                        element_id: None,
                        property: None,
                    }],
                    warnings: vec![],
                });
            }
        };

        let validator = GraphValidator::new(schema)
            .map_err(|e| KotobaError::Storage(format!("Failed to create validator: {}", e)))?;

        Ok(validator.validate_graph(graph_data))
    }

    /// Create a validator for a specific schema
    pub fn create_validator(&mut self, schema_id: &str) -> Result<GraphValidator> {
        let schema = self.get_schema(schema_id)?
            .ok_or_else(|| KotobaError::Storage(format!("Schema '{}' not found", schema_id)))?;

        GraphValidator::new(schema)
            .map_err(|e| KotobaError::Storage(format!("Failed to create validator: {}", e)))
    }

    /// Get schema statistics
    pub fn get_schema_statistics(&mut self, schema_id: &str) -> Result<Option<SchemaStatistics>> {
        match self.get_schema(schema_id)? {
            Some(schema) => Ok(Some(schema.statistics())),
            None => Ok(None),
        }
    }

    /// Clear the schema cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_statistics(&self) -> CacheStatistics {
        CacheStatistics {
            cached_schemas: self.cache.len(),
            cache_enabled: self.cache_enabled,
        }
    }

    /// Check if a schema exists
    pub fn schema_exists(&self, schema_id: &str) -> Result<bool> {
        // Check cache first if enabled
        if self.cache_enabled && self.cache.contains_key(schema_id) {
            return Ok(true);
        }

        // Check storage
        self.storage.schema_exists(schema_id)
    }

    /// Clone a schema with a new ID
    pub fn clone_schema(
        &mut self,
        source_schema_id: &str,
        new_schema_id: &str
    ) -> Result<()> {
        let mut source_schema = self.get_schema(source_schema_id)?
            .ok_or_else(|| KotobaError::Storage(format!("Source schema '{}' not found", source_schema_id)))?;

        // Update schema ID and metadata
        source_schema.id = new_schema_id.to_string();
        source_schema.name = format!("{} (Clone)", source_schema.name);
        source_schema.version = "1.0.0".to_string();

        // Register the cloned schema
        self.register_schema(source_schema)
    }

    /// Get schema metadata
    pub fn get_schema_metadata(&mut self, schema_id: &str) -> Result<Option<HashMap<String, Value>>> {
        match self.get_schema(schema_id)? {
            Some(schema) => Ok(Some(schema.metadata.clone())),
            None => Ok(None),
        }
    }

    /// Update schema metadata
    pub fn update_schema_metadata(
        &mut self,
        schema_id: &str,
        metadata: HashMap<String, Value>
    ) -> Result<()> {
        let mut schema = self.get_schema(schema_id)?
            .ok_or_else(|| KotobaError::Storage(format!("Schema '{}' not found", schema_id)))?;

        schema.metadata = metadata;
        self.update_schema(schema)
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    pub cached_schemas: usize,
    pub cache_enabled: bool,
}

/// In-memory schema storage implementation for testing and simple use cases
pub struct InMemorySchemaStorage {
    schemas: std::sync::RwLock<HashMap<String, GraphSchema>>,
}

impl InMemorySchemaStorage {
    /// Create a new in-memory schema storage
    pub fn new() -> Self {
        Self {
            schemas: std::sync::RwLock::new(HashMap::new()),
        }
    }
}

impl SchemaStorage for InMemorySchemaStorage {
    fn store_schema(&self, schema_id: &str, schema: &GraphSchema) -> Result<()> {
        let mut schemas = self.schemas.write().unwrap();
        schemas.insert(schema_id.to_string(), schema.clone());
        Ok(())
    }

    fn load_schema(&self, schema_id: &str) -> Result<Option<GraphSchema>> {
        let schemas = self.schemas.read().unwrap();
        Ok(schemas.get(schema_id).cloned())
    }

    fn delete_schema(&self, schema_id: &str) -> Result<()> {
        let mut schemas = self.schemas.write().unwrap();
        schemas.remove(schema_id);
        Ok(())
    }

    fn list_schemas(&self) -> Result<Vec<String>> {
        let schemas = self.schemas.read().unwrap();
        Ok(schemas.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

fn create_test_manager() -> SchemaManager {
    let storage = Arc::new(InMemorySchemaStorage::new());
    SchemaManager::new(storage)
}

    fn create_test_schema() -> GraphSchema {
        let mut schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

        let mut user_props = std::collections::HashMap::new();
        user_props.insert(
            "name".to_string(),
            PropertySchema {
                name: "name".to_string(),
                property_type: PropertyType::String,
                description: Some("User name".to_string()),
                required: true,
                default_value: None,
                constraints: vec![PropertyConstraint::MinLength(1)],
            },
        );

        let user_vertex = VertexTypeSchema {
            name: "User".to_string(),
            description: Some("User vertex type".to_string()),
            required_properties: vec!["name".to_string()],
            properties: user_props,
            inherits: vec![],
            constraints: vec![],
        };

        schema.add_vertex_type(user_vertex);
        schema
    }

    #[test]
    fn test_schema_registration() {
        let mut manager = create_test_manager();
        let schema = create_test_schema();

        // Register schema
        manager.register_schema(schema.clone()).unwrap();

        // Retrieve schema
        let retrieved = manager.get_schema("test_schema").unwrap().unwrap();
        assert_eq!(retrieved.id, "test_schema");
        assert_eq!(retrieved.name, "Test Schema");
    }

    #[tokio::test]
    fn test_schema_validation() {
        let mut manager = create_test_manager();
        let schema = create_test_schema();

        // Register schema
        manager.register_schema(schema).unwrap();

        // Test valid graph data
        let valid_graph = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {
                    "name": "John Doe"
                }
            }]
        });

        let result = manager.validate_graph_data("test_schema", &valid_graph).unwrap();
        assert!(result.is_valid);

        // Test invalid graph data
        let invalid_graph = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {}
            }]
        });

        let result = manager.validate_graph_data("test_schema", &invalid_graph).unwrap();
        assert!(!result.is_valid);
    }

    #[test]
    fn test_schema_deletion() {
        let mut manager = create_test_manager();
        let schema = create_test_schema();

        // Register and then delete schema
        manager.register_schema(schema).unwrap();
        assert!(manager.get_schema("test_schema").unwrap().is_some());

        manager.delete_schema("test_schema").unwrap();
        assert!(manager.get_schema("test_schema").unwrap().is_none());
    }

    #[test]
    fn test_list_schemas() {
        let mut manager = create_test_manager();

        // Register multiple schemas
        let schema1 = GraphSchema::new("schema1".to_string(), "Schema 1".to_string(), "1.0.0".to_string());
        let schema2 = GraphSchema::new("schema2".to_string(), "Schema 2".to_string(), "1.0.0".to_string());

        manager.register_schema(schema1).unwrap();
        manager.register_schema(schema2).unwrap();

        let schemas = manager.list_schemas().unwrap();
        assert_eq!(schemas.len(), 2);
        assert!(schemas.contains(&"schema1".to_string()));
        assert!(schemas.contains(&"schema2".to_string()));
    }

    #[test]
    fn test_cache_statistics() {
        let manager = create_test_manager();
        let stats = manager.cache_statistics();
        assert_eq!(stats.cached_schemas, 0);
        assert!(stats.cache_enabled);
    }

    #[test]
    fn test_clone_schema() {
        let mut manager = create_test_manager();
        let schema = create_test_schema();

        // Register original schema
        manager.register_schema(schema).unwrap();

        // Clone schema
        manager.clone_schema("test_schema", "cloned_schema").unwrap();

        // Verify both schemas exist
        let original = manager.get_schema("test_schema").unwrap().unwrap();
        let cloned = manager.get_schema("cloned_schema").unwrap().unwrap();

        assert_eq!(original.name, "Test Schema");
        assert_eq!(cloned.name, "Test Schema (Clone)");
        assert_eq!(cloned.id, "cloned_schema");
    }
}

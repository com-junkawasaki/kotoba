//! # kotoba-schema-registry
//!
//! A schema registry for managing and evolving schemas in Kotoba.
//! プロセスネットワーク as GTS(DPO)+OpenGraph with Merkle DAG & PG view

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;
use jsonschema::JSONSchema;
use thiserror::Error;
use anyhow::Result;

pub mod compatibility;
pub use compatibility::CompatibilityMode;


/// Represents a schema in the registry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Schema {
    pub id: String,
    pub name: String,
    pub version: u32,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(with = "serde_json::value")]
    pub body: serde_json::Value,
    #[serde(skip)]
    compiled_schema: Option<JSONSchema>,
}

impl Schema {
    /// Compiles the JSON schema body for validation.
    pub fn compile(&mut self) -> Result<()> {
        let compiled = JSONSchema::compile(&self.body)
            .map_err(|e| SchemaError::CompilationError(e.to_string()))?;
        self.compiled_schema = Some(compiled);
        Ok(())
    }

    /// Validates a JSON value against the schema.
    pub fn validate(&self, instance: &serde_json::Value) -> Result<(), Vec<String>> {
        if let Some(ref compiled) = self.compiled_schema {
            compiled.validate(instance)
                .map_err(|errors| errors.map(|e| e.to_string()).collect())
        } else {
            Err(vec!["Schema is not compiled".to_string()])
        }
    }
}


/// Errors that can occur within the schema registry.
#[derive(Error, Debug, PartialEq)]
pub enum SchemaError {
    #[error("Schema with ID '{0}' already exists")]
    SchemaAlreadyExists(String),
    #[error("Schema with ID '{0}' not found")]
    SchemaNotFound(String),
    #[error("Version '{1}' for schema '{0}' already exists")]
    VersionAlreadyExists(String, u32),
    #[error("Invalid schema format: {0}")]
    InvalidSchema(String),
    #[error("Failed to compile schema: {0}")]
    CompilationError(String),
    #[error("Schema compatibility check failed for version {1} of schema '{0}': {2}")]
    CompatibilityError(String, u32, String),
}


/// The schema registry.
#[derive(Debug)]
pub struct SchemaRegistry {
    /// Storage for schemas, mapping a schema ID to a map of versions and schemas.
    schemas: HashMap<String, HashMap<u32, Schema>>,
    /// Default compatibility mode for this registry.
    compatibility_mode: CompatibilityMode,
}

impl SchemaRegistry {
    /// Creates a new, empty schema registry with a specific compatibility mode.
    pub fn new(compatibility_mode: CompatibilityMode) -> Self {
        Self {
            schemas: HashMap::new(),
            compatibility_mode,
        }
    }

    /// Registers a new schema or a new version of an existing schema.
    ///
    /// If the schema ID does not exist, it will be created.
    /// If the schema ID exists, it will add a new version if the version number is unique
    /// and it passes the compatibility check.
    pub fn register_schema(&mut self, mut schema: Schema) -> Result<Schema, SchemaError> {
        schema.compile().map_err(|e| SchemaError::InvalidSchema(e.to_string()))?;
        
        if let Some(versions) = self.schemas.get(&schema.id) {
            if let Some(latest_version) = versions.keys().max() {
                if schema.version > *latest_version {
                    let latest_schema = &versions[latest_version];
                    self.check_compatibility(latest_schema, &schema)?;
                }
            }
        }
        
        let versions = self.schemas.entry(schema.id.clone()).or_insert_with(HashMap::new);

        if versions.contains_key(&schema.version) {
            return Err(SchemaError::VersionAlreadyExists(schema.id, schema.version));
        }

        versions.insert(schema.version, schema.clone());
        Ok(schema)
    }

    /// Retrieves a specific version of a schema by its ID.
    pub fn get_schema(&self, id: &str, version: u32) -> Option<&Schema> {
        self.schemas.get(id).and_then(|versions| versions.get(&version))
    }

    /// Retrieves the latest version of a schema by its ID.
    pub fn get_latest_schema(&self, id: &str) -> Option<&Schema> {
        self.schemas.get(id).and_then(|versions| {
            versions
                .keys()
                .max()
                .and_then(|latest_version| versions.get(latest_version))
        })
    }
    
    /// Checks compatibility between an old and a new schema.
    fn check_compatibility(&self, old_schema: &Schema, new_schema: &Schema) -> Result<(), SchemaError> {
        compatibility::check(&old_schema.body, &new_schema.body, self.compatibility_mode).map_err(
            |e| {
                SchemaError::CompatibilityError(
                    new_schema.id.clone(),
                    new_schema.version,
                    e,
                )
            },
        )
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new(CompatibilityMode::None)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_schema(id: &str, version: u32) -> Schema {
        Schema {
            id: id.to_string(),
            name: format!("Test Schema {}", id),
            version,
            description: Some("A schema for testing".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            body: json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            }),
            compiled_schema: None,
        }
    }

    #[test]
    fn test_register_and_get_schema() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::None);
        let schema = create_test_schema("user", 1);

        let registered_schema = registry.register_schema(schema.clone()).unwrap();
        assert_eq!(registered_schema, schema);

        let retrieved_schema = registry.get_schema("user", 1).unwrap();
        assert_eq!(retrieved_schema, &schema);
    }

    #[test]
    fn test_register_duplicate_version() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::None);
        let schema1 = create_test_schema("user", 1);
        let schema2 = create_test_schema("user", 1);

        registry.register_schema(schema1).unwrap();
        let result = registry.register_schema(schema2);

        assert_eq!(result, Err(SchemaError::VersionAlreadyExists("user".to_string(), 1)));
    }

    #[test]
    fn test_get_non_existent_schema() {
        let registry = SchemaRegistry::new(CompatibilityMode::None);
        assert!(registry.get_schema("user", 1).is_none());
    }

    #[test]
    fn test_get_latest_schema() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::None);
        let schema1 = create_test_schema("user", 1);
        let schema2 = create_test_schema("user", 2);

        registry.register_schema(schema1).unwrap();
        registry.register_schema(schema2.clone()).unwrap();

        let latest = registry.get_latest_schema("user").unwrap();
        assert_eq!(latest.version, 2);
    }

    #[test]
    fn test_schema_validation() {
        let mut schema = create_test_schema("user", 1);
        schema.compile().unwrap();

        let valid_instance = json!({ "name": "Alice" });
        assert!(schema.validate(&valid_instance).is_ok());

        let invalid_instance = json!({ "age": 30 });
        assert!(schema.validate(&invalid_instance).is_err());
    }

    #[test]
    fn test_invalid_schema_compilation() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::None);
        let mut schema = create_test_schema("user", 1);
        schema.body = json!({ "type": "invalid" }); // Invalid schema type

        let result = registry.register_schema(schema);
        assert!(matches!(result, Err(SchemaError::InvalidSchema(_))));
    }

    #[test]
    fn test_backward_compatibility_ok() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::Backward);
        let schema1 = create_test_schema("user", 1);
        registry.register_schema(schema1.clone()).unwrap();

        // Add a new optional field `email`, which is backward compatible.
        let mut schema2 = create_test_schema("user", 2);
        schema2.body = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "email": { "type": "string" }
            },
            "required": ["name"]
        });
        
        assert!(registry.register_schema(schema2).is_ok());
    }

    #[test]
    fn test_backward_compatibility_fail() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::Backward);
        let schema1 = create_test_schema("user", 1);
        registry.register_schema(schema1.clone()).unwrap();

        // Add a new required field `age`, which is NOT backward compatible.
        let mut schema2 = create_test_schema("user", 2);
        schema2.body = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "age": { "type": "integer" }
            },
            "required": ["name", "age"]
        });

        let result = registry.register_schema(schema2);
        assert!(matches!(result, Err(SchemaError::CompatibilityError(_, _, _))));
        assert!(result.unwrap_err().to_string().contains("Optional properties {\"age\"} were made required."));
    }

    #[test]
    fn test_forward_compatibility_fail() {
        let mut registry = SchemaRegistry::new(CompatibilityMode::Forward);
        let mut schema1 = create_test_schema("user", 1);
        schema1.body = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            },
            "required": ["name"],
            "additionalProperties": false
        });
        registry.register_schema(schema1.clone()).unwrap();

        // Add a new optional field `email`. This is not forward compatible
        // if the old schema has `additionalProperties: false`.
        let mut schema2 = create_test_schema("user", 2);
        schema2.body = json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "email": { "type": "string" }
            },
            "required": ["name"]
        });
        
        let result = registry.register_schema(schema2);
        assert!(matches!(result, Err(SchemaError::CompatibilityError(_, _, _))));
        assert!(result.unwrap_err().to_string().contains("New properties {\"email\"} added, but old schema does not allow additional properties."));
    }
}

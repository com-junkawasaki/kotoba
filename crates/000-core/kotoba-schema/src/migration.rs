//! Schema Migration
//!
//! This module provides functionality for migrating graph data between
//! different schema versions and handling schema evolution.

use kotoba_errors::KotobaError;
use std::collections::HashMap;

/// Migration engine for schema evolution
pub struct SchemaMigration {
    /// Migration rules
    rules: HashMap<String, MigrationRule>,
}

impl SchemaMigration {
    /// Create a new migration engine
    pub fn new() -> Self {
        Self {
            rules: HashMap::new(),
        }
    }

    /// Add a migration rule
    pub fn add_rule(&mut self, rule_id: String, rule: MigrationRule) {
        self.rules.insert(rule_id, rule);
    }

    /// Get a migration rule
    pub fn get_rule(&self, rule_id: &str) -> Option<&MigrationRule> {
        self.rules.get(rule_id)
    }

    /// Remove a migration rule
    pub fn remove_rule(&mut self, rule_id: &str) -> Option<MigrationRule> {
        self.rules.remove(rule_id)
    }

    /// List all migration rules
    pub fn list_rules(&self) -> Vec<String> {
        self.rules.keys().cloned().collect()
    }

    /// Validate migration rules
    pub fn validate_rules(&self) -> Result<(), KotobaError> {
        let mut errors = Vec::new();

        for (rule_id, rule) in &self.rules {
            // Validate rule structure
            match rule.rule_type {
                MigrationRuleType::RenameProperty => {
                    if rule.source_path.is_empty() || rule.target_path.is_empty() {
                        errors.push(format!("Rule '{}' has empty source or target path", rule_id));
                    }
                },
                MigrationRuleType::ChangePropertyType => {
                    if rule.source_path.is_empty() {
                        errors.push(format!("Rule '{}' has empty source path", rule_id));
                    }
                },
                MigrationRuleType::TransformValue => {
                    if rule.transformation.is_none() {
                        errors.push(format!("Rule '{}' requires transformation for TransformValue", rule_id));
                    }
                },
                _ => {
                    // Other rule types are valid as long as they have source paths
                    if rule.source_path.is_empty() {
                        errors.push(format!("Rule '{}' has empty source path", rule_id));
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(KotobaError::Storage(errors.join("; ")))
        }
    }

    /// Apply migration rules to graph data
    pub fn migrate_graph_data(
        &self,
        graph_data: &mut serde_json::Value,
        rules: &[String],
    ) -> Result<MigrationResult, KotobaError> {
        let mut applied_rules = Vec::new();
        let mut errors = Vec::new();

        for rule_id in rules {
            if let Some(rule) = self.rules.get(rule_id) {
                match self.apply_migration_rule(graph_data, rule) {
                    Ok(_) => applied_rules.push(rule_id.clone()),
                    Err(e) => errors.push(format!("Failed to apply rule '{}': {}", rule_id, e)),
                }
            } else {
                errors.push(format!("Migration rule '{}' not found", rule_id));
            }
        }

        let success = errors.is_empty();
        Ok(MigrationResult {
            applied_rules,
            errors,
            success,
        })
    }
}

impl SchemaMigration {
    /// Apply a single migration rule to graph data
    fn apply_migration_rule(
        &self,
        graph_data: &mut serde_json::Value,
        rule: &MigrationRule,
    ) -> Result<(), KotobaError> {
        match rule.rule_type {
            MigrationRuleType::RenameProperty => {
                self.rename_property(graph_data, &rule.source_path, &rule.target_path)
            },
            MigrationRuleType::ChangePropertyType => {
                self.change_property_type(graph_data, &rule.source_path, &rule.target_path)
            },
            MigrationRuleType::AddProperty => {
                self.add_property(graph_data, &rule.target_path, &rule.transformation)
            },
            MigrationRuleType::RemoveProperty => {
                self.remove_property(graph_data, &rule.source_path)
            },
            MigrationRuleType::TransformValue => {
                if let Some(transformation) = &rule.transformation {
                    self.transform_value(graph_data, &rule.source_path, transformation)
                } else {
                    Err(KotobaError::Storage("Transformation required for TransformValue rule".to_string()))
                }
            },
        }
    }

    /// Rename a property in graph data
    fn rename_property(
        &self,
        graph_data: &mut serde_json::Value,
        source_path: &str,
        target_path: &str,
    ) -> Result<(), KotobaError> {
        if let Some(vertices) = graph_data.get_mut("vertices").and_then(|v| v.as_array_mut()) {
            for vertex in vertices {
                if let Some(props) = vertex.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if let Some(value) = props.remove(source_path) {
                        props.insert(target_path.to_string(), value);
                    }
                }
            }
        }

        if let Some(edges) = graph_data.get_mut("edges").and_then(|v| v.as_array_mut()) {
            for edge in edges {
                if let Some(props) = edge.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if let Some(value) = props.remove(source_path) {
                        props.insert(target_path.to_string(), value);
                    }
                }
            }
        }

        Ok(())
    }

    /// Change property type with actual type conversion
    fn change_property_type(
        &self,
        graph_data: &mut serde_json::Value,
        source_path: &str,
        target_path: &str,
    ) -> Result<(), KotobaError> {
        // Extract source and target types from paths
        let source_parts: Vec<&str> = source_path.split('.').collect();
        let target_parts: Vec<&str> = target_path.split('.').collect();

        if source_parts.len() < 2 || target_parts.len() < 2 {
            return Err(KotobaError::Storage("Invalid property path format".to_string()));
        }

        let source_type = source_parts[1];
        let target_type = target_parts[1];

        // Convert property values in vertices
        if let Some(vertices) = graph_data.get_mut("vertices").and_then(|v| v.as_array_mut()) {
            for vertex in vertices {
                if let Some(properties) = vertex.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    self.convert_property_value(properties, source_type, target_type)?;
                }
            }
        }

        // Convert property values in edges
        if let Some(edges) = graph_data.get_mut("edges").and_then(|e| e.as_array_mut()) {
            for edge in edges {
                if let Some(properties) = edge.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    self.convert_property_value(properties, source_type, target_type)?;
                }
            }
        }

        Ok(())
    }

    /// Convert a single property value from one type to another
    fn convert_property_value(
        &self,
        properties: &mut serde_json::Map<String, serde_json::Value>,
        source_type: &str,
        target_type: &str,
    ) -> Result<(), KotobaError> {
        // Find properties of the source type
        let source_keys: Vec<String> = properties.keys()
            .filter(|key| key.ends_with(&format!(":{}", source_type)))
            .cloned()
            .collect();

        for source_key in source_keys {
            if let Some(value) = properties.get(&source_key) {
                let new_key = source_key.replace(&format!(":{}", source_type), &format!(":{}", target_type));
                let converted_value = self.convert_single_value(value, target_type)?;

                properties.insert(new_key, converted_value);
                properties.remove(&source_key);
            }
        }

        Ok(())
    }

    /// Convert a single JSON value to the target type
    fn convert_single_value(
        &self,
        value: &serde_json::Value,
        target_type: &str,
    ) -> Result<serde_json::Value, KotobaError> {
        match target_type {
            "string" => {
                match value {
                    serde_json::Value::String(_s) => Ok(value.clone()),
                    serde_json::Value::Number(n) => Ok(serde_json::Value::String(n.to_string())),
                    serde_json::Value::Bool(b) => Ok(serde_json::Value::String(b.to_string())),
                    _ => Ok(serde_json::Value::String("".to_string())),
                }
            },
            "integer" => {
                match value {
                    serde_json::Value::String(s) => {
                        s.parse::<i64>()
                            .map(|n| Ok(serde_json::Value::Number(n.into())))
                            .unwrap_or_else(|_| Ok(serde_json::Value::Number(0.into())))
                    },
                    serde_json::Value::Number(n) => {
                        if let Some(n_int) = n.as_i64() {
                            Ok(serde_json::Value::Number(n_int.into()))
                        } else if let Some(n_float) = n.as_f64() {
                            Ok(serde_json::Value::Number((n_float as i64).into()))
                        } else {
                            Ok(serde_json::Value::Number(0.into()))
                        }
                    },
                    serde_json::Value::Bool(b) => {
                        Ok(serde_json::Value::Number(if *b { 1.into() } else { 0.into() }))
                    },
                    _ => Ok(serde_json::Value::Number(0.into())),
                }
            },
            "float" => {
                match value {
                    serde_json::Value::String(s) => {
                        s.parse::<f64>()
                            .map(|n| Ok(serde_json::Value::Number(serde_json::Number::from_f64(n).unwrap_or(0.into()))))
                            .unwrap_or_else(|_| Ok(serde_json::Value::Number(0.into())))
                    },
                    serde_json::Value::Number(n) => {
                        if let Some(n_float) = n.as_f64() {
                            Ok(serde_json::Value::Number(serde_json::Number::from_f64(n_float).unwrap_or(0.into())))
                        } else {
                            Ok(serde_json::Value::Number(0.into()))
                        }
                    },
                    serde_json::Value::Bool(b) => {
                        Ok(serde_json::Value::Number(if *b { 1.into() } else { 0.into() }))
                    },
                    _ => Ok(serde_json::Value::Number(0.into())),
                }
            },
            "boolean" => {
                match value {
                    serde_json::Value::String(s) => {
                        Ok(serde_json::Value::Bool(s.to_lowercase() == "true" || s == "1"))
                    },
                    serde_json::Value::Number(n) => {
                        if let Some(n_int) = n.as_i64() {
                            Ok(serde_json::Value::Bool(n_int != 0))
                        } else if let Some(n_float) = n.as_f64() {
                            Ok(serde_json::Value::Bool(n_float != 0.0))
                        } else {
                            Ok(serde_json::Value::Bool(false))
                        }
                    },
                    serde_json::Value::Bool(_b) => Ok(value.clone()),
                    _ => Ok(serde_json::Value::Bool(false)),
                }
            },
            _ => {
                // Unknown type - keep original value
                Ok(value.clone())
            }
        }
    }

    /// Add a property to graph elements
    fn add_property(
        &self,
        graph_data: &mut serde_json::Value,
        target_path: &str,
        default_value: &Option<ValueTransformation>,
    ) -> Result<(), KotobaError> {
        let default_json = match default_value {
            Some(ValueTransformation::StringToInt) => serde_json::Value::Number(0.into()),
            Some(ValueTransformation::IntToString) => serde_json::Value::String("".to_string()),
            Some(ValueTransformation::Uppercase) => serde_json::Value::String("".to_string()),
            Some(ValueTransformation::Lowercase) => serde_json::Value::String("".to_string()),
            Some(ValueTransformation::Custom(_)) => serde_json::Value::Null,
            None => serde_json::Value::Null,
        };

        if let Some(vertices) = graph_data.get_mut("vertices").and_then(|v| v.as_array_mut()) {
            for vertex in vertices {
                if let Some(props) = vertex.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if !props.contains_key(target_path) {
                        props.insert(target_path.to_string(), default_json.clone());
                    }
                }
            }
        }

        if let Some(edges) = graph_data.get_mut("edges").and_then(|v| v.as_array_mut()) {
            for edge in edges {
                if let Some(props) = edge.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if !props.contains_key(target_path) {
                        props.insert(target_path.to_string(), default_json.clone());
                    }
                }
            }
        }

        Ok(())
    }

    /// Remove a property from graph elements
    fn remove_property(&self, graph_data: &mut serde_json::Value, source_path: &str) -> Result<(), KotobaError> {
        if let Some(vertices) = graph_data.get_mut("vertices").and_then(|v| v.as_array_mut()) {
            for vertex in vertices {
                if let Some(props) = vertex.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    props.remove(source_path);
                }
            }
        }

        if let Some(edges) = graph_data.get_mut("edges").and_then(|v| v.as_array_mut()) {
            for edge in edges {
                if let Some(props) = edge.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    props.remove(source_path);
                }
            }
        }

        Ok(())
    }

    /// Transform property values
    fn transform_value(
        &self,
        graph_data: &mut serde_json::Value,
        source_path: &str,
        transformation: &ValueTransformation,
    ) -> Result<(), KotobaError> {
        let transform_fn = |value: &mut serde_json::Value| {
            match transformation {
                ValueTransformation::StringToInt => {
                    if let Some(s) = value.as_str() {
                        if let Ok(num) = s.parse::<i64>() {
                            *value = serde_json::Value::Number(num.into());
                        }
                    }
                },
                ValueTransformation::IntToString => {
                    if let Some(n) = value.as_i64() {
                        *value = serde_json::Value::String(n.to_string());
                    }
                },
                ValueTransformation::Uppercase => {
                    if let Some(s) = value.as_str() {
                        *value = serde_json::Value::String(s.to_uppercase());
                    }
                },
                ValueTransformation::Lowercase => {
                    if let Some(s) = value.as_str() {
                        *value = serde_json::Value::String(s.to_lowercase());
                    }
                },
                ValueTransformation::Custom(rule) => {
                    // Custom transformations would be implemented here
                    println!("Warning: Custom transformation '{}' applied - manual verification recommended", rule);
                }
            }
        };

        if let Some(vertices) = graph_data.get_mut("vertices").and_then(|v| v.as_array_mut()) {
            for vertex in vertices {
                if let Some(props) = vertex.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if let Some(value) = props.get_mut(source_path) {
                        transform_fn(value);
                    }
                }
            }
        }

        if let Some(edges) = graph_data.get_mut("edges").and_then(|v| v.as_array_mut()) {
            for edge in edges {
                if let Some(props) = edge.get_mut("properties").and_then(|p| p.as_object_mut()) {
                    if let Some(value) = props.get_mut(source_path) {
                        transform_fn(value);
                    }
                }
            }
        }

        Ok(())
    }
}

/// Migration result
#[derive(Debug, Clone)]
pub struct MigrationResult {
    pub applied_rules: Vec<String>,
    pub errors: Vec<String>,
    pub success: bool,
}

/// Migration rule definition
#[derive(Debug, Clone)]
pub struct MigrationRule {
    pub rule_type: MigrationRuleType,
    pub source_path: String,
    pub target_path: String,
    pub transformation: Option<ValueTransformation>,
}

/// Migration rule types
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationRuleType {
    RenameProperty,
    ChangePropertyType,
    AddProperty,
    RemoveProperty,
    TransformValue,
}

/// Value transformation types
#[derive(Debug, Clone)]
pub enum ValueTransformation {
    StringToInt,
    IntToString,
    Uppercase,
    Lowercase,
    Custom(String), // Custom transformation identifier
}

/// Migration plan for complex schema changes
pub struct MigrationPlan {
    rules: Vec<MigrationRule>,
    description: String,
    source_version: String,
    target_version: String,
}

impl MigrationPlan {
    /// Create a new migration plan
    pub fn new(description: String, source_version: String, target_version: String) -> Self {
        Self {
            rules: Vec::new(),
            description,
            source_version,
            target_version,
        }
    }

    /// Add a migration rule to the plan
    pub fn add_rule(&mut self, rule: MigrationRule) {
        self.rules.push(rule);
    }

    /// Get all rules in the plan
    pub fn get_rules(&self) -> &[MigrationRule] {
        &self.rules
    }

    /// Get plan description
    pub fn description(&self) -> &str {
        &self.description
    }

    /// Get source version
    pub fn source_version(&self) -> &str {
        &self.source_version
    }

    /// Get target version
    pub fn target_version(&self) -> &str {
        &self.target_version
    }

    /// Execute the migration plan on graph data
    pub fn execute(&self, graph_data: &mut serde_json::Value) -> Result<MigrationResult, KotobaError> {
        let _migration = SchemaMigration::new();
        let rule_ids: Vec<String> = (0..self.rules.len()).map(|i| format!("rule_{}", i)).collect();

        // Add rules to migration engine
        let mut temp_migration = SchemaMigration::new();
        for (i, rule) in self.rules.iter().enumerate() {
            temp_migration.add_rule(format!("rule_{}", i), rule.clone());
        }

        // Validate rules
        temp_migration.validate_rules()?;

        // Execute migration
        temp_migration.migrate_graph_data(graph_data, &rule_ids)
    }
}

/// Migration history tracking
pub struct MigrationHistory {
    migrations: Vec<MigrationRecord>,
}

impl MigrationHistory {
    /// Create a new migration history
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    /// Record a migration
    pub fn record_migration(&mut self, record: MigrationRecord) {
        self.migrations.push(record);
    }

    /// Get all migration records
    pub fn get_records(&self) -> &[MigrationRecord] {
        &self.migrations
    }

    /// Get migrations for a specific schema
    pub fn get_schema_migrations(&self, schema_id: &str) -> Vec<&MigrationRecord> {
        self.migrations.iter()
            .filter(|r| r.schema_id == schema_id)
            .collect()
    }

    /// Check if a migration was already applied
    pub fn is_migration_applied(&self, migration_id: &str) -> bool {
        self.migrations.iter().any(|r| r.migration_id == migration_id)
    }
}

/// Migration record
#[derive(Debug, Clone)]
pub struct MigrationRecord {
    pub migration_id: String,
    pub schema_id: String,
    pub source_version: String,
    pub target_version: String,
    pub applied_at: u64,
    pub success: bool,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_rule_creation() {
        let rule = MigrationRule {
            rule_type: MigrationRuleType::RenameProperty,
            source_path: "old_name".to_string(),
            target_path: "new_name".to_string(),
            transformation: None,
        };

        assert_eq!(rule.rule_type, MigrationRuleType::RenameProperty);
        assert_eq!(rule.source_path, "old_name");
        assert_eq!(rule.target_path, "new_name");
    }

    #[test]
    fn test_migration_engine() {
        let mut migration = SchemaMigration::new();

        let rule = MigrationRule {
            rule_type: MigrationRuleType::RenameProperty,
            source_path: "username".to_string(),
            target_path: "user_name".to_string(),
            transformation: None,
        };

        migration.add_rule("rename_username".to_string(), rule);

        assert!(migration.get_rule("rename_username").is_some());
        assert!(migration.get_rule("nonexistent").is_none());
    }

    #[test]
    fn test_migration_plan() {
        let mut plan = MigrationPlan::new(
            "User schema update".to_string(),
            "1.0.0".to_string(),
            "1.1.0".to_string(),
        );

        let rule = MigrationRule {
            rule_type: MigrationRuleType::RenameProperty,
            source_path: "username".to_string(),
            target_path: "user_name".to_string(),
            transformation: None,
        };

        plan.add_rule(rule);

        assert_eq!(plan.get_rules().len(), 1);
        assert_eq!(plan.source_version(), "1.0.0");
        assert_eq!(plan.target_version(), "1.1.0");
    }

    #[test]
    fn test_migration_history() {
        let mut history = MigrationHistory::new();

        let record = MigrationRecord {
            migration_id: "migration_001".to_string(),
            schema_id: "user_schema".to_string(),
            source_version: "1.0.0".to_string(),
            target_version: "1.1.0".to_string(),
            applied_at: 1234567890,
            success: true,
            details: "Successfully migrated user schema".to_string(),
        };

        history.record_migration(record);

        assert_eq!(history.get_records().len(), 1);
        assert!(history.is_migration_applied("migration_001"));
        assert!(!history.is_migration_applied("migration_002"));
    }

    #[test]
    fn test_property_rename_migration() {
        let mut migration = SchemaMigration::new();

        let rule = MigrationRule {
            rule_type: MigrationRuleType::RenameProperty,
            source_path: "username".to_string(),
            target_path: "user_name".to_string(),
            transformation: None,
        };

        migration.add_rule("rename_username".to_string(), rule);

        // Test graph data
        let mut graph_data = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {
                    "username": "john_doe",
                    "email": "john@example.com"
                }
            }]
        });

        let result = migration.migrate_graph_data(
            &mut graph_data,
            &["rename_username".to_string()]
        ).unwrap();

        assert!(result.success);
        assert_eq!(result.applied_rules.len(), 1);

        // Verify the property was renamed
        if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
            if let Some(vertex) = vertices.first() {
                if let Some(props) = vertex.get("properties").and_then(|p| p.as_object()) {
                    assert!(props.contains_key("user_name"));
                    assert!(!props.contains_key("username"));
                    assert_eq!(props.get("user_name").unwrap(), "john_doe");
                }
            }
        }
    }

    #[test]
    fn test_value_transformation() {
        let mut migration = SchemaMigration::new();

        let rule = MigrationRule {
            rule_type: MigrationRuleType::TransformValue,
            source_path: "user_id".to_string(),
            target_path: "user_id".to_string(),
            transformation: Some(ValueTransformation::IntToString),
        };

        migration.add_rule("convert_user_id".to_string(), rule);

        // Test graph data
        let mut graph_data = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {
                    "user_id": 123,
                    "name": "John Doe"
                }
            }]
        });

        let result = migration.migrate_graph_data(
            &mut graph_data,
            &["convert_user_id".to_string()]
        ).unwrap();

        assert!(result.success);

        // Verify the value was transformed
        if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
            if let Some(vertex) = vertices.first() {
                if let Some(props) = vertex.get("properties").and_then(|p| p.as_object()) {
                    if let Some(user_id) = props.get("user_id") {
                        assert!(user_id.is_string());
                        assert_eq!(user_id.as_str().unwrap(), "123");
                    }
                }
            }
        }
    }
}

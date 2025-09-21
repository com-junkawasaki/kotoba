//! Schema Validation Engine
//!
//! This module provides comprehensive validation functionality for graph data
//! against defined schemas.

use crate::schema::*;
use kotoba_errors::KotobaError;
use regex::Regex;
use std::collections::HashMap;

/// Helper function to extract string value from JSON
fn get_string_value(value: &serde_json::Value) -> Option<&str> {
    value.as_str()
}

/// Helper function to extract number value from JSON
fn get_number_value(value: &serde_json::Value) -> Option<f64> {
    value.as_f64()
}

/// Validate email format
fn is_valid_email(email: &str) -> bool {
    let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$");
    match email_regex {
        Ok(regex) => regex.is_match(email),
        Err(_) => false,
    }
}

/// Validate URL format
fn is_valid_url(url: &str) -> bool {
    url::Url::parse(url).is_ok()
}

/// Graph data validator
#[derive(Debug)]
pub struct GraphValidator {
    schema: GraphSchema,
    compiled_constraints: HashMap<String, Regex>,
}

impl GraphValidator {
    /// Create a new validator for the given schema
    pub fn new(schema: GraphSchema) -> Result<Self, KotobaError> {
        let mut validator = Self {
            schema,
            compiled_constraints: HashMap::new(),
        };

        // Pre-compile regex patterns
        validator.compile_constraints()?;

        Ok(validator)
    }

    /// Validate graph data against the schema
    pub fn validate_graph(&self, graph_data: &serde_json::Value) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Extract vertices and edges from graph data
        if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
            for (i, vertex) in vertices.iter().enumerate() {
                let Ok((warns, vertex_errors)) = self.validate_vertex(vertex) else {
                // Skip this vertex if validation setup failed
                continue;
            };
            warnings.extend(warns);
            for mut error in vertex_errors {
                if error.element_id.is_none() {
                    error.element_id = vertex.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| format!("vertex[{}]:{}", i, s));
                }
                errors.push(error);
            }
            }
        }

        if let Some(edges) = graph_data.get("edges").and_then(|v| v.as_array()) {
            for (i, edge) in edges.iter().enumerate() {
                let Ok((warns, edge_errors)) = self.validate_edge(edge) else {
                    // Skip this edge if validation setup failed
                    continue;
                };
            warnings.extend(warns);
            for mut error in edge_errors {
                if error.element_id.is_none() {
                    error.element_id = edge.get("id")
                        .and_then(|id| id.as_str())
                        .map(|s| format!("edge[{}]:{}", i, s));
                }
                errors.push(error);
            }
            }
        }

        // Validate global constraints
        let constraint_errors = self.validate_global_constraints(graph_data);
        errors.extend(constraint_errors);

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Validate a single vertex
    pub fn validate_vertex(&self, vertex: &serde_json::Value) -> Result<(Vec<String>, Vec<ValidationError>), KotobaError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Get vertex type from labels (assuming first label is the type)
        let vertex_type = match self.extract_vertex_type(vertex) {
            Ok(vt) => vt,
            Err(e) => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::TypeMismatch,
                    message: format!("Failed to extract vertex type: {}", e),
                    element_id: None,
                    property: Some("labels".to_string()),
                });
                return Ok((warnings, errors));
            }
        };

        // Get vertex type schema
        let vertex_schema = match self.schema.get_vertex_type(&vertex_type) {
            Some(vs) => vs,
            None => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::TypeMismatch,
                    message: format!("Unknown vertex type: {}", vertex_type),
                    element_id: None,
                    property: Some("labels".to_string()),
                });
                return Ok((warnings, errors));
            }
        };

        // Validate properties
        if let Some(properties) = vertex.get("properties").and_then(|p| p.as_object()) {
            // Check required properties
            for required_prop in &vertex_schema.required_properties {
                if !properties.contains_key(required_prop) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::MissingRequiredProperty,
                        message: format!("Missing required property: {}", required_prop),
                        element_id: None,
                        property: Some(required_prop.clone()),
                    });
                }
            }

            // Validate each property
            for (prop_name, prop_value) in properties {
                if let Some(prop_schema) = vertex_schema.properties.get(prop_name) {
                    let Ok((warns, prop_errors)) = self.validate_property(prop_schema, prop_value) else {
                        // Skip this property if validation setup failed
                        continue;
                    };
                    warnings.extend(warns);
                    errors.extend(prop_errors);
                } else {
                    warnings.push(format!("Unknown property '{}' in vertex type '{}'", prop_name, vertex_type));
                }
            }
        } else if !vertex_schema.required_properties.is_empty() {
            errors.push(ValidationError {
                error_type: ValidationErrorType::MissingRequiredProperty,
                message: "Vertex has no properties but schema requires some".to_string(),
                element_id: None,
                property: None,
            });
        }

        Ok((warnings, errors))
    }

    /// Validate a single edge
    pub fn validate_edge(&self, edge: &serde_json::Value) -> Result<(Vec<String>, Vec<ValidationError>), KotobaError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Get edge type from label
        let edge_type = match self.extract_edge_type(edge) {
            Ok(et) => et,
            Err(e) => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::TypeMismatch,
                    message: format!("Failed to extract edge type: {}", e),
                    element_id: None,
                    property: Some("label".to_string()),
                });
                return Ok((warnings, errors));
            }
        };

        // Get edge type schema
        let edge_schema = match self.schema.get_edge_type(&edge_type) {
            Some(es) => es,
            None => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::TypeMismatch,
                    message: format!("Unknown edge type: {}", edge_type),
                    element_id: None,
                    property: Some("label".to_string()),
                });
                return Ok((warnings, errors));
            }
        };

        // Validate source and target types (if specified in the edge data)
        // Note: This would require access to the full graph to validate vertex types
        // For now, we skip this validation

        // Validate properties
        if let Some(properties) = edge.get("properties").and_then(|p| p.as_object()) {
            // Check required properties
            for required_prop in &edge_schema.required_properties {
                if !properties.contains_key(required_prop) {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::MissingRequiredProperty,
                        message: format!("Missing required property: {}", required_prop),
                        element_id: None,
                        property: Some(required_prop.clone()),
                    });
                }
            }

            // Validate each property
            for (prop_name, prop_value) in properties {
                if let Some(prop_schema) = edge_schema.properties.get(prop_name) {
                    let Ok((warns, prop_errors)) = self.validate_property(prop_schema, prop_value) else {
                        // Skip this property if validation setup failed
                        continue;
                    };
                    warnings.extend(warns);
                    errors.extend(prop_errors);
                } else {
                    warnings.push(format!("Unknown property '{}' in edge type '{}'", prop_name, edge_type));
                }
            }
        } else if !edge_schema.required_properties.is_empty() {
            errors.push(ValidationError {
                error_type: ValidationErrorType::MissingRequiredProperty,
                message: "Edge has no properties but schema requires some".to_string(),
                element_id: None,
                property: None,
            });
        }

        Ok((warnings, errors))
    }

    /// Validate a property value against its schema
    pub fn validate_property(&self, prop_schema: &PropertySchema, value: &serde_json::Value) -> Result<(Vec<String>, Vec<ValidationError>), KotobaError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Type validation
        let type_errors = self.validate_property_type(&prop_schema.property_type, value);
        errors.extend(type_errors);

        // Constraint validation
        for constraint in &prop_schema.constraints {
            let constraint_errors = self.validate_constraint(constraint, value, &prop_schema.name, value, &mut warnings);
            errors.extend(constraint_errors);
        }

        Ok((warnings, errors))
    }

    /// Validate property type
    fn validate_property_type(&self, expected_type: &PropertyType, value: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        let type_matches = match (expected_type, value) {
            (PropertyType::String, serde_json::Value::String(_)) => true,
            (PropertyType::Integer, serde_json::Value::Number(n)) if n.is_i64() => true,
            (PropertyType::Float, serde_json::Value::Number(_)) => true,
            (PropertyType::Boolean, serde_json::Value::Bool(_)) => true,
            (PropertyType::DateTime, serde_json::Value::String(s)) => {
                // Basic ISO 8601 date validation
                s.len() >= 10 && s.contains('T')
            },
            (PropertyType::Json, _) => true, // Accept any JSON value
            (PropertyType::Array(element_type), serde_json::Value::Array(arr)) => {
                // Validate each element in the array
                for element in arr {
                    let element_errors = self.validate_property_type(element_type, element);
                    errors.extend(element_errors);
                }
                errors.is_empty()
            },
            (PropertyType::Map(_), serde_json::Value::Object(_)) => true,
            _ => false,
        };

        if !type_matches && errors.is_empty() {
            errors.push(ValidationError {
                error_type: ValidationErrorType::InvalidPropertyType,
                message: format!("Property type mismatch for {:?}", expected_type),
                element_id: None,
                property: None,
            });
        }

        errors
    }

    /// Validate a constraint
    fn validate_constraint(&self, constraint: &PropertyConstraint, value: &serde_json::Value, property_name: &str, property_value: &serde_json::Value, warnings: &mut Vec<String>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match constraint {
            PropertyConstraint::MinLength(min) => {
                if let Some(s) = value.as_str() {
                    if s.len() < *min {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::ConstraintViolation,
                            message: format!("String length {} is less than minimum {}", s.len(), min),
                            element_id: None,
                            property: None,
                        });
                    }
                }
            },
            PropertyConstraint::MaxLength(max) => {
                if let Some(s) = value.as_str() {
                    if s.len() > *max {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::ConstraintViolation,
                            message: format!("String length {} exceeds maximum {}", s.len(), max),
                            element_id: None,
                            property: None,
                        });
                    }
                }
            },
            PropertyConstraint::MinValue(min) => {
                if let Some(n) = value.as_i64() {
                    if n < *min {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::ConstraintViolation,
                            message: format!("Value {} is less than minimum {}", n, min),
                            element_id: None,
                            property: None,
                        });
                    }
                }
            },
            PropertyConstraint::MaxValue(max) => {
                if let Some(n) = value.as_i64() {
                    if n > *max {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::ConstraintViolation,
                            message: format!("Value {} exceeds maximum {}", n, max),
                            element_id: None,
                            property: None,
                        });
                    }
                }
            },
            PropertyConstraint::Pattern(pattern) => {
                if let Some(regex) = self.compiled_constraints.get(pattern) {
                    if let Some(s) = value.as_str() {
                        if !regex.is_match(s) {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::ConstraintViolation,
                                message: format!("String '{}' does not match pattern '{}'", s, pattern),
                                element_id: None,
                                property: None,
                            });
                        }
                    }
                }
            },
            PropertyConstraint::Enum(_allowed_values) => {
                // Enum validation is not fully implemented in this simplified version
                // In a full implementation, we would compare the JSON value against allowed values
                warnings.push("Enum constraint validation not fully implemented".to_string());
            },
            PropertyConstraint::Custom(rule_name) => {
                // Custom validation rules implementation
                match rule_name.as_str() {
                    "email" => {
                        if let Some(value_str) = get_string_value(property_value) {
                            if !is_valid_email(value_str) {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Property '{}' must be a valid email address", property_name),
                                    element_id: None,
                                    property: Some(property_name.to_string()),
                                });
                            }
                        }
                    },
                    "url" => {
                        if let Some(value_str) = get_string_value(property_value) {
                            if !is_valid_url(value_str) {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Property '{}' must be a valid URL", property_name),
                                    element_id: None,
                                    property: Some(property_name.to_string()),
                                });
                            }
                        }
                    },
                    "positive_number" => {
                        if let Some(value_num) = get_number_value(property_value) {
                            if value_num <= 0.0 {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Property '{}' must be a positive number", property_name),
                                    element_id: None,
                                    property: Some(property_name.to_string()),
                                });
                            }
                        }
                    },
                    "future_date" => {
                        if let Some(value_str) = get_string_value(property_value) {
                            if let Ok(date) = chrono::DateTime::parse_from_rfc3339(value_str) {
                                if date <= chrono::Utc::now() {
                                    errors.push(ValidationError {
                                        error_type: ValidationErrorType::ConstraintViolation,
                                        message: format!("Property '{}' must be a future date", property_name),
                                        element_id: None,
                                        property: Some(property_name.to_string()),
                                    });
                                }
                            } else {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Property '{}' must be a valid RFC3339 date", property_name),
                                    element_id: None,
                                    property: Some(property_name.to_string()),
                                });
                            }
                        }
                    },
                    _ => {
                        warnings.push(format!("Unknown custom validation rule '{}'", rule_name));
                    }
                }
            }
        }

        errors
    }

    /// Validate global schema constraints
    fn validate_global_constraints(&self, graph_data: &serde_json::Value) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for constraint in &self.schema.constraints {
            match constraint {
                SchemaConstraint::UniqueProperty { vertex_type, property } => {
                    let unique_errors = self.validate_unique_property(graph_data, vertex_type, property);
                    errors.extend(unique_errors);
                },
                SchemaConstraint::Cardinality { edge_type, min, max } => {
                    let cardinality_errors = self.validate_cardinality(graph_data, edge_type, *min, *max);
                    errors.extend(cardinality_errors);
                },
                SchemaConstraint::PathConstraint { pattern, description } => {
                    let path_errors = self.validate_path_constraint(graph_data, pattern, description);
                    errors.extend(path_errors);
                },
                SchemaConstraint::Custom { name, parameters } => {
                    let custom_errors = self.validate_custom_constraint(graph_data, name, &parameters.clone());
                    errors.extend(custom_errors);
                }
            }
        }

        errors
    }

    /// Validate unique property constraint
    fn validate_unique_property(&self, graph_data: &serde_json::Value, vertex_type: &str, property: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();
        let mut seen_values = std::collections::HashSet::new();

        if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
            for vertex in vertices {
                // Check if this vertex is of the specified type
                if let Ok(vt) = self.extract_vertex_type(vertex) {
                    if vt == vertex_type {
                        // Extract the property value
                        if let Some(props) = vertex.get("properties").and_then(|p| p.as_object()) {
                            if let Some(value) = props.get(property) {
                                let value_key = format!("{:?}", value);
                                if !seen_values.insert(value_key.clone()) {
                                    errors.push(ValidationError {
                                        error_type: ValidationErrorType::ConstraintViolation,
                                        message: format!("Duplicate value for unique property '{}' in vertex type '{}': {}",
                                                       property, vertex_type, value_key),
                                        element_id: vertex.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()),
                                        property: Some(property.to_string()),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        errors
    }

    /// Validate cardinality constraint
    fn validate_cardinality(&self, graph_data: &serde_json::Value, edge_type: &str, min: usize, max: Option<usize>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        if let Some(edges) = graph_data.get("edges").and_then(|v| v.as_array()) {
            let mut edge_counts = std::collections::HashMap::new();

            // Count edges of this type
            for edge in edges {
                if let Ok(et) = self.extract_edge_type(edge) {
                    if et == edge_type {
                        // For cardinality, we need to count edges between the same source-target pairs
                        // This is a simplified implementation
                        let source = edge.get("src").and_then(|s| s.as_str()).unwrap_or("unknown");
                        let target = edge.get("tgt").and_then(|s| s.as_str()).unwrap_or("unknown");
                        let key = format!("{}->{}", source, target);

                        *edge_counts.entry(key).or_insert(0) += 1;
                    }
                }
            }

            // Check cardinality constraints
            for (edge_key, count) in edge_counts {
                if count < min {
                    errors.push(ValidationError {
                        error_type: ValidationErrorType::ConstraintViolation,
                        message: format!("Edge '{}' has {} instances, minimum required is {}", edge_key, count, min),
                        element_id: Some(edge_key.clone()),
                        property: None,
                    });
                }

                if let Some(max_val) = max {
                    if count > max_val {
                        errors.push(ValidationError {
                            error_type: ValidationErrorType::ConstraintViolation,
                            message: format!("Edge '{}' has {} instances, maximum allowed is {}", edge_key, count, max_val),
                            element_id: Some(edge_key.clone()),
                            property: None,
                        });
                    }
                }
            }
        }

        errors
    }

    /// Extract vertex type from vertex data
    fn extract_vertex_type(&self, vertex: &serde_json::Value) -> Result<String, KotobaError> {
        match vertex.get("labels").and_then(|l| l.as_array()).and_then(|arr| arr.first()) {
            Some(label) if label.is_string() => Ok(label.as_str().unwrap().to_string()),
            _ => Ok("Unknown".to_string()), // Return default instead of error
        }
    }

    /// Extract edge type from edge data
    fn extract_edge_type(&self, edge: &serde_json::Value) -> Result<String, KotobaError> {
        match edge.get("label").and_then(|l| l.as_str()) {
            Some(label) => Ok(label.to_string()),
            None => Ok("Unknown".to_string()), // Return default instead of error
        }
    }

    /// Pre-compile regex constraints
    fn compile_constraints(&mut self) -> Result<(), KotobaError> {
        for vertex_type in self.schema.vertex_types.values() {
            for property in vertex_type.properties.values() {
                for constraint in &property.constraints {
                    if let PropertyConstraint::Pattern(pattern) = constraint {
                        if !self.compiled_constraints.contains_key(pattern) {
                            match Regex::new(pattern) {
                                Ok(regex) => {
                                    self.compiled_constraints.insert(pattern.clone(), regex);
                                },
                                Err(e) => {
                                    return Err(KotobaError::Storage(format!("Invalid regex pattern '{}': {}", pattern, e)));
                                }
                            }
                        }
                    }
                }
            }
        }

        for edge_type in self.schema.edge_types.values() {
            for property in edge_type.properties.values() {
                for constraint in &property.constraints {
                    if let PropertyConstraint::Pattern(pattern) = constraint {
                        if !self.compiled_constraints.contains_key(pattern) {
                            match Regex::new(pattern) {
                                Ok(regex) => {
                                    self.compiled_constraints.insert(pattern.clone(), regex);
                                },
                                Err(e) => {
                                    return Err(KotobaError::Storage(format!("Invalid regex pattern '{}': {}", pattern, e)));
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Validate path constraints in the graph
    fn validate_path_constraint(&self, graph_data: &serde_json::Value, pattern: &str, description: &str) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        // Simple path constraint validation - check for basic patterns
        // This is a simplified implementation for demonstration
        match pattern {
            "no_self_loops" => {
                if let Some(edges) = graph_data.get("edges").and_then(|v| v.as_array()) {
                    for edge in edges {
                        if let (Some(source), Some(target)) = (
                            edge.get("source").and_then(|s| s.as_str()),
                            edge.get("target").and_then(|t| t.as_str())
                        ) {
                            if source == target {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Self-loop detected: {}", description),
                                    element_id: edge.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()),
                                    property: None,
                                });
                            }
                        }
                    }
                }
            },
            "no_isolated_vertices" => {
                if let (Some(vertices), Some(edges)) = (
                    graph_data.get("vertices").and_then(|v| v.as_array()),
                    graph_data.get("edges").and_then(|e| e.as_array())
                ) {
                    let mut connected_vertices = std::collections::HashSet::new();

                    for edge in edges {
                        if let (Some(source), Some(target)) = (
                            edge.get("source").and_then(|s| s.as_str()),
                            edge.get("target").and_then(|t| t.as_str())
                        ) {
                            connected_vertices.insert(source.to_string());
                            connected_vertices.insert(target.to_string());
                        }
                    }

                    for vertex in vertices {
                        if let Some(vertex_id) = vertex.get("id").and_then(|id| id.as_str()) {
                            if !connected_vertices.contains(vertex_id) {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Isolated vertex detected: {}", description),
                                    element_id: Some(vertex_id.to_string()),
                                    property: None,
                                });
                            }
                        }
                    }
                }
            },
            _ => {
                // Unknown pattern - this is expected for custom patterns
                // In a real implementation, you might want to support more patterns
                // or provide a way to define custom pattern validators
            }
        }

        errors
    }

    /// Validate custom schema constraints
    fn validate_custom_constraint(&self, graph_data: &serde_json::Value, name: &str, parameters: &HashMap<String, serde_json::Value>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        match name {
            "max_vertices" => {
                if let Some(max_count) = parameters.get("count").and_then(|v| v.as_u64()) {
                    if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
                        if vertices.len() > max_count as usize {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::ConstraintViolation,
                                message: format!("Too many vertices: maximum allowed is {}", max_count),
                                element_id: None,
                                property: None,
                            });
                        }
                    }
                }
            },
            "max_edges" => {
                if let Some(max_count) = parameters.get("count").and_then(|v| v.as_u64()) {
                    if let Some(edges) = graph_data.get("edges").and_then(|e| e.as_array()) {
                        if edges.len() > max_count as usize {
                            errors.push(ValidationError {
                                error_type: ValidationErrorType::ConstraintViolation,
                                message: format!("Too many edges: maximum allowed is {}", max_count),
                                element_id: None,
                                property: None,
                            });
                        }
                    }
                }
            },
            "required_vertex_types" => {
                if let Some(required_types) = parameters.get("types").and_then(|v| v.as_array()) {
                    let mut found_types = std::collections::HashSet::new();

                    if let Some(vertices) = graph_data.get("vertices").and_then(|v| v.as_array()) {
                        for vertex in vertices {
                            if let Some(labels) = vertex.get("labels").and_then(|l| l.as_array()) {
                                for label in labels {
                                    if let Some(label_str) = label.as_str() {
                                        found_types.insert(label_str.to_string());
                                    }
                                }
                            }
                        }
                    }

                    for required_type in required_types {
                        if let Some(type_str) = required_type.as_str() {
                            if !found_types.contains(type_str) {
                                errors.push(ValidationError {
                                    error_type: ValidationErrorType::ConstraintViolation,
                                    message: format!("Required vertex type '{}' not found in graph", type_str),
                                    element_id: None,
                                    property: None,
                                });
                            }
                        }
                    }
                }
            },
            _ => {
                errors.push(ValidationError {
                    error_type: ValidationErrorType::ConstraintViolation,
                    message: format!("Unknown custom constraint: {}", name),
                    element_id: None,
                    property: None,
                });
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_schema() -> GraphSchema {
        let mut schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

        // Add User vertex type
        let mut user_props = HashMap::new();
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

        user_props.insert(
            "age".to_string(),
            PropertySchema {
                name: "age".to_string(),
                property_type: PropertyType::Integer,
                description: Some("User age".to_string()),
                required: false,
                default_value: None,
                constraints: vec![PropertyConstraint::MinValue(0), PropertyConstraint::MaxValue(150)],
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
    fn test_validator_creation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema);
        assert!(validator.is_ok());
    }

    #[test]
    fn test_valid_vertex_validation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        let valid_vertex = serde_json::json!({
            "id": "user1",
            "labels": ["User"],
            "properties": {
                "name": "John Doe",
                "age": 25
            }
        });

        let result = validator.validate_vertex(&valid_vertex);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty()); // No warnings
    }

    #[test]
    fn test_invalid_vertex_validation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        // Missing required property
        let invalid_vertex = serde_json::json!({
            "id": "user1",
            "labels": ["User"],
            "properties": {
                "age": 25
            }
        });

        let result = validator.validate_vertex(&invalid_vertex);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());
        assert!(matches!(errors[0].error_type, ValidationErrorType::MissingRequiredProperty));
    }

    #[test]
    fn test_property_type_validation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        // Test invalid type
        let invalid_vertex = serde_json::json!({
            "id": "user1",
            "labels": ["User"],
            "properties": {
                "name": "John Doe",
                "age": "25"  // Should be integer, not string
            }
        });

        let result = validator.validate_vertex(&invalid_vertex);
        assert!(result.is_err());
    }

    #[test]
    fn test_constraint_validation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        // Test constraint violation (age too high)
        let invalid_vertex = serde_json::json!({
            "id": "user1",
            "labels": ["User"],
            "properties": {
                "name": "John Doe",
                "age": 200  // Exceeds max value of 150
            }
        });

        let result = validator.validate_vertex(&invalid_vertex);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(matches!(errors[0].error_type, ValidationErrorType::ConstraintViolation));
    }

    #[test]
    fn test_graph_validation() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        let graph_data = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {
                    "name": "John Doe",
                    "age": 25
                }
            }, {
                "id": "user2",
                "labels": ["User"],
                "properties": {
                    "name": "Jane Smith"
                }
            }],
            "edges": []
        });

        let result = validator.validate_graph(&graph_data);
        assert!(result.is_valid);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_graph_validation_with_errors() {
        let schema = create_test_schema();
        let validator = GraphValidator::new(schema).unwrap();

        let graph_data = serde_json::json!({
            "vertices": [{
                "id": "user1",
                "labels": ["User"],
                "properties": {
                    "age": 25  // Missing required name property
                }
            }],
            "edges": []
        });

        let result = validator.validate_graph(&graph_data);
        assert!(!result.is_valid);
        assert!(!result.errors.is_empty());
    }
}

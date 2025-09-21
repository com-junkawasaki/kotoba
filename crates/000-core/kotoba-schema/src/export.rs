//! Schema Export/Import
//!
//! This module provides functionality for exporting schemas to various formats
//! and importing schemas from external sources.

use crate::schema::*;
use kotoba_errors::KotobaError;
use std::collections::HashMap;

/// Schema exporter for different formats
pub struct SchemaExporter;

impl SchemaExporter {
    /// Export schema as JSON Schema
    pub fn to_json_schema(schema: &GraphSchema) -> Result<serde_json::Value, KotobaError> {
        let mut json_schema = serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "$id": format!("https://kotoba.dev/schemas/{}", schema.id),
            "title": schema.name,
            "description": schema.description,
            "type": "object",
            "properties": {
                "vertices": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "labels"],
                        "properties": {
                            "id": { "type": "string" },
                            "labels": {
                                "type": "array",
                                "items": { "type": "string" },
                                "minItems": 1
                            },
                            "properties": {
                                "type": "object",
                                "additionalProperties": false
                            }
                        }
                    }
                },
                "edges": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["id", "label", "src", "tgt"],
                        "properties": {
                            "id": { "type": "string" },
                            "label": { "type": "string" },
                            "src": { "type": "string" },
                            "tgt": { "type": "string" },
                            "properties": {
                                "type": "object",
                                "additionalProperties": false
                            }
                        }
                    }
                }
            }
        });

        // Add vertex type schemas
        if let Some(properties) = json_schema.get_mut("properties") {
            if let Some(vertices) = properties.get_mut("vertices") {
                if let Some(items) = vertices.get_mut("items") {
                    if let Some(vertex_props) = items.get_mut("properties") {
                        if let Some(props_schema) = vertex_props.get_mut("properties") {
                            Self::add_vertex_type_schemas(props_schema, schema);
                        }
                    }
                }
            }

            if let Some(edges) = properties.get_mut("edges") {
                if let Some(items) = edges.get_mut("items") {
                    if let Some(edge_props) = items.get_mut("properties") {
                        if let Some(props_schema) = edge_props.get_mut("properties") {
                            Self::add_edge_type_schemas(props_schema, schema);
                        }
                    }
                }
            }
        }

        Ok(json_schema)
    }

    /// Export schema as GraphQL schema
    pub fn to_graphql_schema(schema: &GraphSchema) -> Result<String, KotobaError> {
        let mut gql_schema = String::new();

        // Add header
        gql_schema.push_str(&format!("# GraphQL Schema for {}\n", schema.name));
        gql_schema.push_str(&format!("# Generated from Kotoba Schema {}\n\n", schema.id));

        // Add vertex types
        for (type_name, vertex_type) in &schema.vertex_types {
            gql_schema.push_str(&format!("type {} {{\n", type_name));
            gql_schema.push_str("  id: ID!\n");
            gql_schema.push_str("  labels: [String!]!\n");

            for (prop_name, prop_schema) in &vertex_type.properties {
                let gql_type = Self::property_type_to_graphql(&prop_schema.property_type);
                let required = if prop_schema.required { "!" } else { "" };
                gql_schema.push_str(&format!("  {}: {}{}\n", prop_name, gql_type, required));
            }

            gql_schema.push_str("}\n\n");
        }

        // Add edge types as connections
        for (type_name, edge_type) in &schema.edge_types {
            gql_schema.push_str(&format!("type {}Connection {{\n", type_name));
            gql_schema.push_str("  id: ID!\n");
            gql_schema.push_str("  label: String!\n");
            gql_schema.push_str("  src: ID!\n");
            gql_schema.push_str("  tgt: ID!\n");

            for (prop_name, prop_schema) in &edge_type.properties {
                let gql_type = Self::property_type_to_graphql(&prop_schema.property_type);
                let required = if prop_schema.required { "!" } else { "" };
                gql_schema.push_str(&format!("  {}: {}{}\n", prop_name, gql_type, required));
            }

            gql_schema.push_str("}\n\n");
        }

        // Add query type
        gql_schema.push_str("type Query {\n");
        for type_name in schema.vertex_types.keys() {
            gql_schema.push_str(&format!("  {}s: [{}]!\n", type_name.to_lowercase(), type_name));
            gql_schema.push_str(&format!("  {}(id: ID!): {}\n", type_name.to_lowercase(), type_name));
        }
        gql_schema.push_str("}\n");

        Ok(gql_schema)
    }

    /// Export schema as OpenAPI specification
    pub fn to_openapi_schema(schema: &GraphSchema) -> Result<serde_json::Value, KotobaError> {
        let openapi = serde_json::json!({
            "openapi": "3.0.3",
            "info": {
                "title": schema.name,
                "description": schema.description,
                "version": schema.version
            },
            "components": {
                "schemas": {}
            }
        });

        // This would be expanded to include detailed OpenAPI schemas
        // For now, it's a basic structure

        Ok(openapi)
    }

    /// Export schema as SQL DDL
    pub fn to_sql_ddl(schema: &GraphSchema) -> Result<String, KotobaError> {
        let mut ddl = String::new();

        ddl.push_str(&format!("-- SQL DDL for {}\n", schema.name));
        ddl.push_str(&format!("-- Generated from Kotoba Schema {}\n\n", schema.id));

        // Create vertex tables
        for (type_name, vertex_type) in &schema.vertex_types {
            ddl.push_str(&format!("CREATE TABLE {}_vertices (\n", type_name.to_lowercase()));
            ddl.push_str("  id VARCHAR(255) PRIMARY KEY,\n");
            ddl.push_str("  labels JSON,\n");

            for (prop_name, prop_schema) in &vertex_type.properties {
                let sql_type = Self::property_type_to_sql(&prop_schema.property_type);
                let nullable = if prop_schema.required { " NOT NULL" } else { "" };
                ddl.push_str(&format!("  {} {}{},\n", prop_name, sql_type, nullable));
            }

            ddl.push_str("  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n");
            ddl.push_str("  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP\n");
            ddl.push_str(");\n\n");
        }

        // Create edge tables
        for (type_name, edge_type) in &schema.edge_types {
            ddl.push_str(&format!("CREATE TABLE {}_edges (\n", type_name.to_lowercase()));
            ddl.push_str("  id VARCHAR(255) PRIMARY KEY,\n");
            ddl.push_str("  label VARCHAR(255) NOT NULL,\n");
            ddl.push_str("  src VARCHAR(255) NOT NULL,\n");
            ddl.push_str("  tgt VARCHAR(255) NOT NULL,\n");

            for (prop_name, prop_schema) in &edge_type.properties {
                let sql_type = Self::property_type_to_sql(&prop_schema.property_type);
                let nullable = if prop_schema.required { " NOT NULL" } else { "" };
                ddl.push_str(&format!("  {} {}{},\n", prop_name, sql_type, nullable));
            }

            ddl.push_str("  created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,\n");
            ddl.push_str("  updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,\n");

            // Add foreign key constraints
            ddl.push_str(&format!("  FOREIGN KEY (src) REFERENCES vertices(id),\n"));
            ddl.push_str(&format!("  FOREIGN KEY (tgt) REFERENCES vertices(id)\n"));

            ddl.push_str(");\n\n");
        }

        Ok(ddl)
    }

    /// Helper: Add vertex type schemas to JSON Schema
    fn add_vertex_type_schemas(props_schema: &mut serde_json::Value, schema: &GraphSchema) {
        if let Some(obj) = props_schema.as_object_mut() {
            for (type_name, vertex_type) in &schema.vertex_types {
                let mut vertex_schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "labels": {
                            "type": "array",
                            "items": { "type": "string" },
                            "contains": { "const": type_name }
                        },
                        "properties": {
                            "type": "object",
                            "properties": {},
                            "required": vertex_type.required_properties.clone()
                        }
                    },
                    "required": ["id", "labels"]
                });

                if let Some(props) = vertex_schema.pointer_mut("/properties/properties/properties") {
                    if let Some(props_obj) = props.as_object_mut() {
                        for (prop_name, prop_schema) in &vertex_type.properties {
                            let prop_def = Self::property_schema_to_json_schema(prop_schema);
                            props_obj.insert(prop_name.clone(), prop_def);
                        }
                    }
                }

                obj.insert(format!("{}_vertex", type_name.to_lowercase()), vertex_schema);
            }
        }
    }

    /// Helper: Add edge type schemas to JSON Schema
    fn add_edge_type_schemas(props_schema: &mut serde_json::Value, schema: &GraphSchema) {
        if let Some(obj) = props_schema.as_object_mut() {
            for (type_name, edge_type) in &schema.edge_types {
                let mut edge_schema = serde_json::json!({
                    "type": "object",
                    "properties": {
                        "id": { "type": "string" },
                        "label": { "const": type_name },
                        "src": { "type": "string" },
                        "tgt": { "type": "string" },
                        "properties": {
                            "type": "object",
                            "properties": {},
                            "required": edge_type.required_properties.clone()
                        }
                    },
                    "required": ["id", "label", "src", "tgt"]
                });

                if let Some(props) = edge_schema.pointer_mut("/properties/properties/properties") {
                    if let Some(props_obj) = props.as_object_mut() {
                        for (prop_name, prop_schema) in &edge_type.properties {
                            let prop_def = Self::property_schema_to_json_schema(prop_schema);
                            props_obj.insert(prop_name.clone(), prop_def);
                        }
                    }
                }

                obj.insert(format!("{}_edge", type_name.to_lowercase()), edge_schema);
            }
        }
    }

    /// Helper: Convert property schema to JSON Schema
    fn property_schema_to_json_schema(prop_schema: &PropertySchema) -> serde_json::Value {
        let mut schema = match &prop_schema.property_type {
            PropertyType::String => serde_json::json!({ "type": "string" }),
            PropertyType::Integer => serde_json::json!({ "type": "integer" }),
            PropertyType::Float => serde_json::json!({ "type": "number" }),
            PropertyType::Boolean => serde_json::json!({ "type": "boolean" }),
            PropertyType::DateTime => serde_json::json!({
                "type": "string",
                "format": "date-time"
            }),
            PropertyType::Json => serde_json::json!({}), // Allow any JSON
            PropertyType::Array(element_type) => {
                let item_schema = match element_type.as_ref() {
                    PropertyType::String => serde_json::json!({ "type": "string" }),
                    PropertyType::Integer => serde_json::json!({ "type": "integer" }),
                    PropertyType::Float => serde_json::json!({ "type": "number" }),
                    PropertyType::Boolean => serde_json::json!({ "type": "boolean" }),
                    _ => serde_json::json!({}), // Allow any for complex types
                };
                serde_json::json!({
                    "type": "array",
                    "items": item_schema
                })
            },
            PropertyType::Map(_) => serde_json::json!({
                "type": "object",
                "additionalProperties": true
            }),
        };

        // Add constraints
        if let Some(obj) = schema.as_object_mut() {
            for constraint in &prop_schema.constraints {
                match constraint {
                    PropertyConstraint::MinLength(min) => {
                        obj.insert("minLength".to_string(), serde_json::json!(min));
                    },
                    PropertyConstraint::MaxLength(max) => {
                        obj.insert("maxLength".to_string(), serde_json::json!(max));
                    },
                    PropertyConstraint::MinValue(min) => {
                        obj.insert("minimum".to_string(), serde_json::json!(min));
                    },
                    PropertyConstraint::MaxValue(max) => {
                        obj.insert("maximum".to_string(), serde_json::json!(max));
                    },
                    PropertyConstraint::Pattern(pattern) => {
                        obj.insert("pattern".to_string(), serde_json::json!(pattern));
                    },
                    PropertyConstraint::Enum(values) => {
                        let enum_values: Vec<serde_json::Value> = values.iter()
                            .map(|v| serde_json::to_value(v).unwrap_or(serde_json::Value::Null))
                            .collect();
                        obj.insert("enum".to_string(), serde_json::json!(enum_values));
                    },
                    _ => {} // Other constraints not directly supported in JSON Schema
                }
            }
        }

        schema
    }

    /// Helper: Convert property type to GraphQL type
    fn property_type_to_graphql(prop_type: &PropertyType) -> &'static str {
        match prop_type {
            PropertyType::String => "String",
            PropertyType::Integer => "Int",
            PropertyType::Float => "Float",
            PropertyType::Boolean => "Boolean",
            PropertyType::DateTime => "String",
            PropertyType::Json => "String", // JSON as string in GraphQL
            PropertyType::Array(_) => "[String]", // Simplified
            PropertyType::Map(_) => "String", // JSON as string
        }
    }

    /// Helper: Convert property type to SQL type
    fn property_type_to_sql(prop_type: &PropertyType) -> &'static str {
        match prop_type {
            PropertyType::String => "VARCHAR(255)",
            PropertyType::Integer => "BIGINT",
            PropertyType::Float => "DOUBLE PRECISION",
            PropertyType::Boolean => "BOOLEAN",
            PropertyType::DateTime => "TIMESTAMP",
            PropertyType::Json => "JSON",
            PropertyType::Array(_) => "JSON", // Arrays as JSON
            PropertyType::Map(_) => "JSON", // Maps as JSON
        }
    }
}

/// Schema importer for different formats
pub struct SchemaImporter;

impl SchemaImporter {
    /// Import schema from JSON Schema
    pub fn from_json_schema(json_schema: &serde_json::Value) -> Result<GraphSchema, KotobaError> {
        let id = json_schema.get("$id")
            .and_then(|id| id.as_str())
            .and_then(|id_str| id_str.split('/').last())
            .unwrap_or("imported_schema")
            .to_string();

        let name = json_schema.get("title")
            .and_then(|t| t.as_str())
            .unwrap_or("Imported Schema")
            .to_string();

        let description = json_schema.get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());

        let mut schema = GraphSchema::new(id, name, "1.0.0".to_string());
        if let Some(desc) = description {
            schema.description = Some(desc);
        }

        // This is a simplified implementation
        // A full implementation would parse the JSON Schema structure
        // and convert it to Kotoba schema types

        Ok(schema)
    }

    /// Import schema from GraphQL schema string
    pub fn from_graphql_schema(gql_schema: &str) -> Result<GraphSchema, KotobaError> {
        let mut schema = GraphSchema::new(
            "graphql_imported".to_string(),
            "Imported from GraphQL".to_string(),
            "1.0.0".to_string(),
        );

        // Parse GraphQL schema and convert to Kotoba schema types
        Self::parse_graphql_types(gql_schema, &mut schema)?;

        Ok(schema)
    }

    /// Parse GraphQL types and convert to Kotoba schema types
    fn parse_graphql_types(gql_schema: &str, schema: &mut GraphSchema) -> Result<(), KotobaError> {
        let lines: Vec<&str> = gql_schema.lines().collect();

        for line in lines {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse type definitions
            if line.starts_with("type ") {
                Self::parse_type_definition(line, schema)?;
            } else if line.starts_with("enum ") {
                Self::parse_enum_definition(line, schema)?;
            } else if line.starts_with("interface ") {
                Self::parse_interface_definition(line, schema)?;
            }
        }

        Ok(())
    }

    /// Parse GraphQL type definition
    fn parse_type_definition(line: &str, schema: &mut GraphSchema) -> Result<(), KotobaError> {
        let parts: Vec<&str> = line.split('{').collect();
        if parts.len() < 2 {
            return Ok(()); // Skip incomplete definitions
        }

        let type_def = parts[0].trim();
        let type_parts: Vec<&str> = type_def.split_whitespace().collect();
        if type_parts.len() < 2 {
            return Ok(());
        }

        let type_name = type_parts[1];

        // Create vertex type schema
        let mut vertex_type = VertexTypeSchema::new(type_name.to_string());
        vertex_type.description = Some(format!("GraphQL type {}", type_name));

        // Parse fields (simplified implementation)
        let fields_part = parts[1].trim();
        if fields_part.ends_with('}') {
            let fields_str = &fields_part[..fields_part.len() - 1];
            for field_line in fields_str.lines() {
                let field_line = field_line.trim();
                if !field_line.is_empty() {
                    if let Some(field_name) = field_line.split(':').next() {
                        let field_name = field_name.trim();
                        if field_name != "id" && field_name != "labels" {
                            // Add as optional string property by default
                            let mut props = HashMap::new();
                            props.insert(
                                field_name.to_string(),
                                PropertySchema {
                                    name: field_name.to_string(),
                                    property_type: PropertyType::String,
                                    description: Some(format!("Field from GraphQL type {}", type_name)),
                                    required: false,
                                    default_value: None,
                                    constraints: vec![],
                                },
                            );
                            vertex_type.properties = props;
                        }
                    }
                }
            }
        }

        schema.add_vertex_type(vertex_type);
        Ok(())
    }

    /// Parse GraphQL enum definition
    fn parse_enum_definition(line: &str, schema: &mut GraphSchema) -> Result<(), KotobaError> {
        let parts: Vec<&str> = line.split('{').collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let enum_def = parts[0].trim();
        let enum_parts: Vec<&str> = enum_def.split_whitespace().collect();
        if enum_parts.len() < 2 {
            return Ok(());
        }

        let enum_name = enum_parts[1];

        // For enums, we create a vertex type with an enum property
        let mut vertex_type = VertexTypeSchema::new(enum_name.to_string());
        vertex_type.description = Some(format!("GraphQL enum {}", enum_name));

        let mut props = HashMap::new();
        props.insert(
            "value".to_string(),
            PropertySchema {
                name: "value".to_string(),
                property_type: PropertyType::String,
                description: Some(format!("Enum value for {}", enum_name)),
                required: true,
                default_value: None,
                constraints: vec![],
            },
        );

        vertex_type.properties = props;
        schema.add_vertex_type(vertex_type);
        Ok(())
    }

    /// Parse GraphQL interface definition
    fn parse_interface_definition(line: &str, schema: &mut GraphSchema) -> Result<(), KotobaError> {
        let parts: Vec<&str> = line.split('{').collect();
        if parts.len() < 2 {
            return Ok(());
        }

        let interface_def = parts[0].trim();
        let interface_parts: Vec<&str> = interface_def.split_whitespace().collect();
        if interface_parts.len() < 2 {
            return Ok(());
        }

        let interface_name = interface_parts[1];

        // Create an interface type (similar to regular type but marked as interface)
        let mut vertex_type = VertexTypeSchema::new(format!("I{}", interface_name));
        vertex_type.description = Some(format!("GraphQL interface {}", interface_name));

        // Parse fields (simplified implementation)
        let fields_part = parts[1].trim();
        if fields_part.ends_with('}') {
            let fields_str = &fields_part[..fields_part.len() - 1];
            for field_line in fields_str.lines() {
                let field_line = field_line.trim();
                if !field_line.is_empty() {
                    if let Some(field_name) = field_line.split(':').next() {
                        let field_name = field_name.trim();
                        if field_name != "id" && field_name != "labels" {
                            let mut props = HashMap::new();
                            props.insert(
                                field_name.to_string(),
                                PropertySchema {
                                    name: field_name.to_string(),
                                    property_type: PropertyType::String,
                                    description: Some(format!("Field from GraphQL interface {}", interface_name)),
                                    required: false,
                                    default_value: None,
                                    constraints: vec![],
                                },
                            );
                            vertex_type.properties = props;
                        }
                    }
                }
            }
        }

        schema.add_vertex_type(vertex_type);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_schema_export() {
        let mut schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

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

        let user_vertex = VertexTypeSchema {
            name: "User".to_string(),
            description: Some("User vertex type".to_string()),
            required_properties: vec!["name".to_string()],
            properties: user_props,
            inherits: vec![],
            constraints: vec![],
        };

        schema.add_vertex_type(user_vertex);

        let json_schema = SchemaExporter::to_json_schema(&schema);
        assert!(json_schema.is_ok());

        let schema_obj = json_schema.unwrap();
        assert_eq!(schema_obj.get("title").unwrap(), "Test Schema");
        assert!(schema_obj.get("properties").unwrap().get("vertices").is_some());
    }

    #[test]
    fn test_graphql_schema_export() {
        let mut schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

        let mut user_props = HashMap::new();
        user_props.insert(
            "name".to_string(),
            PropertySchema {
                name: "name".to_string(),
                property_type: PropertyType::String,
                description: Some("User name".to_string()),
                required: true,
                default_value: None,
                constraints: vec![],
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

        let gql_schema = SchemaExporter::to_graphql_schema(&schema);
        assert!(gql_schema.is_ok());

        let schema_str = gql_schema.unwrap();
        assert!(schema_str.contains("type User"));
        assert!(schema_str.contains("name: String!"));
    }

    #[test]
    fn test_sql_ddl_export() {
        let mut schema = GraphSchema::new(
            "test_schema".to_string(),
            "Test Schema".to_string(),
            "1.0.0".to_string(),
        );

        let mut user_props = HashMap::new();
        user_props.insert(
            "name".to_string(),
            PropertySchema {
                name: "name".to_string(),
                property_type: PropertyType::String,
                description: Some("User name".to_string()),
                required: true,
                default_value: None,
                constraints: vec![],
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

        let ddl = SchemaExporter::to_sql_ddl(&schema);
        assert!(ddl.is_ok());

        let ddl_str = ddl.unwrap();
        assert!(ddl.contains("CREATE TABLE user_vertices"));
        assert!(ddl.contains("name VARCHAR(255) NOT NULL"));
    }

    #[test]
    fn test_json_schema_import() {
        let json_schema = serde_json::json!({
            "$id": "https://example.com/schemas/test",
            "title": "Test Schema",
            "description": "A test schema",
            "type": "object"
        });

        let schema = SchemaImporter::from_json_schema(&json_schema);
        assert!(schema.is_ok());

        let imported = schema.unwrap();
        assert_eq!(imported.id, "test");
        assert_eq!(imported.name, "Test Schema");
        assert_eq!(imported.description, Some("A test schema".to_string()));
    }
}

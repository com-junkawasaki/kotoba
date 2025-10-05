//! Graph Integration for kotoba-schema
//!
//! This module provides integration between schema validation and graph operations,
//! allowing seamless validation of graph data against registered schemas.

#[cfg(feature = "graph")]
use kotoba_graph::prelude::*;
#[cfg(feature = "graph")]
use crate::schema::*;
#[cfg(feature = "graph")]
use crate::manager::*;
#[cfg(feature = "graph")]
use std::sync::Arc;

/// Graph validator that integrates with kotoba-graph
#[cfg(feature = "graph")]
pub struct GraphSchemaValidator {
    schema_manager: SchemaManager,
    default_schema_id: Option<String>,
}

#[cfg(feature = "graph")]
impl GraphSchemaValidator {
    /// Create a new graph schema validator
    pub fn new(schema_manager: SchemaManager) -> Self {
        Self {
            schema_manager,
            default_schema_id: None,
        }
    }

    /// Create validator with default schema
    pub fn with_default_schema(schema_manager: SchemaManager, schema_id: String) -> Self {
        Self {
            schema_manager,
            default_schema_id: Some(schema_id),
        }
    }

    /// Validate a graph against a schema
    pub async fn validate_graph(&mut self, graph: &Graph, schema_id: Option<&str>) -> Result<ValidationResult> {
        let schema_id = schema_id
            .or_else(|| self.default_schema_id.as_deref())
            .ok_or_else(|| KotobaError::Storage("No schema ID provided and no default schema set".to_string()))?;

        // Convert graph to JSON representation for validation
        let graph_json = self.graph_to_json(graph)?;
        self.schema_manager.validate_graph_data(schema_id, &graph_json).await
    }

    /// Validate a vertex before adding to graph
    pub async fn validate_vertex(&mut self, vertex: &VertexData, schema_id: Option<&str>) -> Result<ValidationResult> {
        let schema_id = schema_id
            .or_else(|| self.default_schema_id.as_deref())
            .ok_or_else(|| KotobaError::Storage("No schema ID provided and no default schema set".to_string()))?;

        let vertex_json = serde_json::json!({
            "id": vertex.id.to_string(),
            "labels": vertex.labels,
            "properties": vertex.props
        });

        let validator = self.schema_manager.create_validator(schema_id).await?;
        Ok(validator.validate_vertex(&vertex_json))
    }

    /// Validate an edge before adding to graph
    pub async fn validate_edge(&mut self, edge: &EdgeData, schema_id: Option<&str>) -> Result<ValidationResult> {
        let schema_id = schema_id
            .or_else(|| self.default_schema_id.as_deref())
            .ok_or_else(|| KotobaError::Storage("No schema ID provided and no default schema set".to_string()))?;

        let edge_json = serde_json::json!({
            "id": edge.id.to_string(),
            "label": edge.label,
            "src": edge.src.to_string(),
            "tgt": edge.dst.to_string(),
            "properties": edge.props
        });

        let validator = self.schema_manager.create_validator(schema_id).await?;
        Ok(validator.validate_edge(&edge_json))
    }

    /// Set default schema for this validator
    pub fn set_default_schema(&mut self, schema_id: String) {
        self.default_schema_id = Some(schema_id);
    }

    /// Clear default schema
    pub fn clear_default_schema(&mut self) {
        self.default_schema_id = None;
    }

    /// Get current default schema ID
    pub fn default_schema_id(&self) -> Option<&str> {
        self.default_schema_id.as_deref()
    }

    /// Convert a graph to JSON representation for validation
    fn graph_to_json(&self, graph: &Graph) -> Result<serde_json::Value> {
        let mut vertices = Vec::new();
        let mut edges = Vec::new();

        // Convert vertices
        for vertex in graph.vertices.values() {
            let vertex_json = serde_json::json!({
                "id": vertex.id.to_string(),
                "labels": vertex.labels.clone(),
                "properties": vertex.props.clone()
            });
            vertices.push(vertex_json);
        }

        // Convert edges
        for edge in graph.edges.values() {
            let edge_json = serde_json::json!({
                "id": edge.id.to_string(),
                "label": edge.label.clone(),
                "src": edge.src.to_string(),
                "tgt": edge.dst.to_string(),
                "properties": edge.props.clone()
            });
            edges.push(edge_json);
        }

        Ok(serde_json::json!({
            "vertices": vertices,
            "edges": edges
        }))
    }
}

/// Schema-aware graph builder
#[cfg(feature = "graph")]
pub struct SchemaAwareGraphBuilder {
    validator: GraphSchemaValidator,
    graph: Graph,
}

#[cfg(feature = "graph")]
impl SchemaAwareGraphBuilder {
    /// Create a new schema-aware graph builder
    pub fn new(validator: GraphSchemaValidator) -> Self {
        Self {
            validator,
            graph: Graph::empty(),
        }
    }

    /// Add a vertex with schema validation
    pub async fn add_vertex(&mut self, vertex: VertexData, schema_id: Option<&str>) -> Result<()> {
        // Validate vertex before adding
        let validation = self.validator.validate_vertex(&vertex, schema_id).await?;
        if !validation.is_valid {
            return Err(KotobaError::Storage(format!(
                "Vertex validation failed: {:?}",
                validation.errors
            )));
        }

        self.graph.add_vertex(vertex);
        Ok(())
    }

    /// Add an edge with schema validation
    pub async fn add_edge(&mut self, edge: EdgeData, schema_id: Option<&str>) -> Result<()> {
        // Validate edge before adding
        let validation = self.validator.validate_edge(&edge, schema_id).await?;
        if !validation.is_valid {
            return Err(KotobaError::Storage(format!(
                "Edge validation failed: {:?}",
                validation.errors
            )));
        }

        self.graph.add_edge(edge);
        Ok(())
    }

    /// Build the final graph
    pub fn build(self) -> Graph {
        self.graph
    }

    /// Get current graph (for inspection)
    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    /// Validate entire graph
    pub async fn validate_graph(&mut self, schema_id: Option<&str>) -> Result<ValidationResult> {
        self.validator.validate_graph(&self.graph, schema_id).await
    }
}

/// Schema migration helper for graphs
#[cfg(feature = "graph")]
pub struct GraphSchemaMigration {
    validator: GraphSchemaValidator,
}

#[cfg(feature = "graph")]
impl GraphSchemaMigration {
    /// Create a new graph schema migration helper
    pub fn new(validator: GraphSchemaValidator) -> Self {
        Self { validator }
    }

    /// Migrate a graph from one schema to another
    pub async fn migrate_graph(
        &mut self,
        graph: &mut Graph,
        from_schema_id: &str,
        to_schema_id: &str,
        migration_rules: Vec<MigrationRule>,
    ) -> Result<()> {
        // Convert graph to JSON
        let graph_json = self.graph_to_json_for_migration(graph)?;

        // Apply migration rules
        let mut migration = SchemaMigration::new();
        let rule_ids: Vec<String> = (0..migration_rules.len())
            .map(|i| format!("migration_rule_{}", i))
            .collect();

        for (i, rule) in migration_rules.into_iter().enumerate() {
            migration.add_rule(format!("migration_rule_{}", i), rule);
        }

        let mut migrated_json = graph_json;
        let result = migration.migrate_graph_data(&mut migrated_json, &rule_ids)?;

        if !result.success {
            return Err(KotobaError::Storage(format!(
                "Migration failed: {:?}",
                result.errors
            )));
        }

        // Validate migrated data against target schema
        let validation = self.validator.schema_manager
            .validate_graph_data(to_schema_id, &migrated_json).await?;

        if !validation.is_valid {
            return Err(KotobaError::Storage(format!(
                "Migration validation failed: {:?}",
                validation.errors
            )));
        }

        // Convert back to graph
        *graph = self.json_to_graph(&migrated_json)?;

        Ok(())
    }

    /// Convert graph to JSON for migration
    fn graph_to_json_for_migration(&self, graph: &Graph) -> Result<serde_json::Value> {
        let mut vertices = Vec::new();
        let mut edges = Vec::new();

        // Convert vertices
        for vertex in graph.vertices.values() {
            let vertex_json = serde_json::json!({
                "id": vertex.id.to_string(),
                "labels": vertex.labels.clone(),
                "properties": vertex.props.clone()
            });
            vertices.push(vertex_json);
        }

        // Convert edges
        for edge in graph.edges.values() {
            let edge_json = serde_json::json!({
                "id": edge.id.to_string(),
                "label": edge.label.clone(),
                "src": edge.src.to_string(),
                "tgt": edge.dst.to_string(),
                "properties": edge.props.clone()
            });
            edges.push(edge_json);
        }

        Ok(serde_json::json!({
            "vertices": vertices,
            "edges": edges
        }))
    }

    /// Convert JSON back to graph
    fn json_to_graph(&self, json: &serde_json::Value) -> Result<Graph> {
        let mut graph = Graph::empty();

        // Convert vertices
        if let Some(vertices) = json.get("vertices").and_then(|v| v.as_array()) {
            for vertex_json in vertices {
                let id = vertex_json.get("id")
                    .and_then(|id| id.as_str())
                    .ok_or_else(|| KotobaError::Storage("Vertex missing id".to_string()))?;

                let vertex_id = VertexId::new_v4(); // In a real implementation, you'd preserve original IDs

                let labels = vertex_json.get("labels")
                    .and_then(|l| l.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect()
                    )
                    .unwrap_or_default();

                let props = vertex_json.get("properties")
                    .and_then(|p| p.as_object())
                    .map(|obj| obj.iter()
                        .map(|(k, v)| (k.clone(), Value::from(v.clone())))
                        .collect()
                    )
                    .unwrap_or_default();

                let vertex_data = VertexData {
                    id: vertex_id,
                    labels,
                    props,
                };

                graph.add_vertex(vertex_data);
            }
        }

        // Convert edges
        if let Some(edges) = json.get("edges").and_then(|v| v.as_array()) {
            for edge_json in edges {
                let id = edge_json.get("id")
                    .and_then(|id| id.as_str())
                    .ok_or_else(|| KotobaError::Storage("Edge missing id".to_string()))?;

                let edge_id = EdgeId::new_v4();

                let label = edge_json.get("label")
                    .and_then(|l| l.as_str())
                    .unwrap_or("UNKNOWN")
                    .to_string();

                let src_str = edge_json.get("src")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| KotobaError::Storage("Edge missing src".to_string()))?;

                let tgt_str = edge_json.get("tgt")
                    .and_then(|s| s.as_str())
                    .ok_or_else(|| KotobaError::Storage("Edge missing tgt".to_string()))?;

                // In a real implementation, you'd need to map string IDs back to VertexIds
                // This is a simplified version
                let src = VertexId::new_v4();
                let tgt = VertexId::new_v4();

                let props = edge_json.get("properties")
                    .and_then(|p| p.as_object())
                    .map(|obj| obj.iter()
                        .map(|(k, v)| (k.clone(), Value::from(v.clone())))
                        .collect()
                    )
                    .unwrap_or_default();

                let edge_data = EdgeData {
                    id: edge_id,
                    src,
                    dst: tgt,
                    label,
                    props,
                };

                graph.add_edge(edge_data);
            }
        }

        Ok(graph)
    }
}

/// Schema-aware graph operations
#[cfg(feature = "graph")]
pub struct SchemaAwareGraphOps {
    validator: GraphSchemaValidator,
}

#[cfg(feature = "graph")]
impl SchemaAwareGraphOps {
    /// Create new schema-aware graph operations
    pub fn new(validator: GraphSchemaValidator) -> Self {
        Self { validator }
    }

    /// Perform schema-validated graph operations
    pub async fn execute_operation(
        &mut self,
        graph: &mut Graph,
        operation: GraphOperation,
        schema_id: Option<&str>,
    ) -> Result<()> {
        match operation {
            GraphOperation::AddVertex(vertex) => {
                self.validator.validate_vertex(&vertex, schema_id).await?;
                graph.add_vertex(vertex);
            },
            GraphOperation::AddEdge(edge) => {
                self.validator.validate_edge(&edge, schema_id).await?;
                graph.add_edge(edge);
            },
            GraphOperation::UpdateVertex(id, updates) => {
                // This would require more complex validation
                // For now, just apply the updates
                if let Some(vertex) = graph.vertices.get_mut(&id) {
                    for (key, value) in updates {
                        vertex.props.insert(key, value);
                    }
                }
            },
            GraphOperation::UpdateEdge(id, updates) => {
                // This would require more complex validation
                if let Some(edge) = graph.edges.get_mut(&id) {
                    for (key, value) in updates {
                        edge.props.insert(key, value);
                    }
                }
            },
        }

        Ok(())
    }
}

/// Graph operation types
#[cfg(feature = "graph")]
pub enum GraphOperation {
    AddVertex(VertexData),
    AddEdge(EdgeData),
    UpdateVertex(VertexId, HashMap<String, Value>),
    UpdateEdge(EdgeId, HashMap<String, Value>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[cfg(feature = "graph")]
    #[tokio::test]
    async fn test_graph_validator() {
        // Create in-memory storage
        let storage = Arc::new(InMemorySchemaStorage::new());
        let manager = SchemaManager::new(storage);

        let mut validator = GraphSchemaValidator::new(manager);

        // Create a test schema
        let schema = create_test_schema();
        validator.schema_manager.register_schema(schema.clone()).await.unwrap();

        // Create a test graph
        let mut graph = Graph::empty();
        let vertex_id = VertexId::new_v4();
        let vertex = VertexData {
            id: vertex_id,
            labels: vec!["User".to_string()],
            props: {
                let mut props = HashMap::new();
                props.insert("name".to_string(), Value::String("John Doe".to_string()));
                props
            },
        };
        graph.add_vertex(vertex);

        // Validate graph
        let result = validator.validate_graph(&graph, Some("test_schema")).await.unwrap();
        assert!(result.is_valid);
    }

    #[cfg(feature = "graph")]
    fn create_test_schema() -> GraphSchema {
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
        schema
    }
}

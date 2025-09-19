//! GQL Engine Integration
//!
//! Integration layer between Projection Engine and GQL Query Engine.

use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use serde_json;

use crate::{ProjectionEngine, ProjectionDefinition, EventEnvelope};
use kotoba_ocel::OcelEvent;
use kotoba_graphdb::{GraphDB, GraphQuery};
use kotoba_cache::CacheLayer;
use kotoba_query_engine::{
    GqlQueryEngine, QueryContext, ProjectionPort, IndexManagerPort, CachePort,
    Vertex, Edge, VertexId, EdgeId, VertexFilter, EdgeFilter, Path, PathPattern, QueryResult
};

/// Projection Engine adapter for GQL engine
pub struct ProjectionEngineAdapter {
    projection_engine: Arc<ProjectionEngine>,
}

impl ProjectionEngineAdapter {
    /// Create new adapter
    pub fn new(projection_engine: Arc<ProjectionEngine>) -> Self {
        Self { projection_engine }
    }

    /// Execute GQL query
    pub async fn execute_gql_query(
        &self,
        query: &str,
        context: QueryContext,
    ) -> Result<QueryResult> {
        // Create GQL engine components
        let gql_engine = GqlQueryEngine::new(
            Arc::new(self.clone()),
            Arc::new(self.clone()),
            Arc::new(self.clone()),
        );

        // Execute query
        gql_engine.execute_query(query, context).await
    }

    /// Execute GQL statement
    pub async fn execute_gql_statement(
        &self,
        statement: &str,
        context: QueryContext,
    ) -> Result<serde_json::Value> {
        // Create GQL engine components
        let gql_engine = GqlQueryEngine::new(
            Arc::new(self.clone()),
            Arc::new(self.clone()),
            Arc::new(self.clone()),
        );

        // Execute statement and convert result
        let result = gql_engine.execute_statement(statement, context).await?;
        Ok(serde_json::json!({
            "success": true,
            "affected_rows": 1, // Placeholder
            "message": "Statement executed successfully"
        }))
    }
}

impl Clone for ProjectionEngineAdapter {
    fn clone(&self) -> Self {
        Self {
            projection_engine: self.projection_engine.clone(),
        }
    }
}

#[async_trait]
impl ProjectionPort for ProjectionEngineAdapter {
    async fn get_vertex(&self, id: &VertexId) -> Result<Option<Vertex>> {
        // Query GraphDB directly with string ID
        if let Some(node) = self.projection_engine.graphdb.get_node(id).await? {
            Ok(Some(Vertex {
                id: node.id.clone(),
                labels: node.labels,
                properties: node.properties.into_iter()
                    .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                    .collect(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>> {
        // Query GraphDB directly with string ID
        if let Some(edge) = self.projection_engine.graphdb.get_edge(id).await? {
            Ok(Some(Edge {
                id: edge.id.clone(),
                from_vertex: edge.from_node.clone(),
                to_vertex: edge.to_node.clone(),
                label: edge.label,
                properties: edge.properties.into_iter()
                    .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                    .collect(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn scan_vertices(&self, filter: Option<VertexFilter>) -> Result<Vec<Vertex>> {
        // Get all vertices from GraphDB
        let nodes = self.projection_engine.graphdb.scan_nodes().await?;

        let mut vertices = Vec::new();
        for node in nodes {
            // Apply filter if provided
            if let Some(ref filter) = filter {
                // Check labels filter
                if let Some(ref required_labels) = filter.labels {
                    if !required_labels.iter().any(|label| node.labels.contains(label)) {
                        continue;
                    }
                }

                // Check property filters
                let mut matches = true;
                for (prop_key, filter_value) in &filter.property_filters {
                    if let Some(node_value) = node.properties.get(prop_key) {
                        // Simple equality check for now
                        let node_json = serde_json::to_value(node_value).unwrap_or(serde_json::Value::Null);
                        if node_json != serde_json::to_value(filter_value.value.clone()).unwrap_or(serde_json::Value::Null) {
                            matches = false;
                            break;
                        }
                    } else {
                        matches = false;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            }

            // Convert to Vertex
            let vertex = Vertex {
                id: node.id.clone(),
                labels: node.labels,
                properties: node.properties.into_iter()
                    .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                    .collect(),
            };
            vertices.push(vertex);
        }

        Ok(vertices)
    }

    async fn scan_edges(&self, filter: Option<EdgeFilter>) -> Result<Vec<Edge>> {
        // Get all edges from GraphDB
        let graph_edges = self.projection_engine.graphdb.scan_edges().await?;

        let mut edges = Vec::new();
        for edge in graph_edges {
            // Apply filter if provided
            if let Some(ref filter) = filter {
                // Check labels filter
                if let Some(ref required_labels) = filter.labels {
                    if !required_labels.contains(&edge.label) {
                        continue;
                    }
                }

                // Check from_vertex filter
                if let Some(ref from_vertex) = filter.from_vertex {
                    if edge.from_node != *from_vertex {
                        continue;
                    }
                }

                // Check to_vertex filter
                if let Some(ref to_vertex) = filter.to_vertex {
                    if edge.to_node != *to_vertex {
                        continue;
                    }
                }

                // Check property filters
                let mut matches = true;
                for (prop_key, filter_value) in &filter.property_filters {
                    if let Some(edge_value) = edge.properties.get(prop_key) {
                        // Simple equality check for now
                        let edge_json = serde_json::to_value(edge_value).unwrap_or(serde_json::Value::Null);
                        if edge_json != serde_json::to_value(filter_value.value.clone()).unwrap_or(serde_json::Value::Null) {
                            matches = false;
                            break;
                        }
                    } else {
                        matches = false;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            }

            // Convert to Edge
            let gql_edge = Edge {
                id: edge.id.clone(),
                from_vertex: edge.from_node.clone(),
                to_vertex: edge.to_node.clone(),
                label: edge.label,
                properties: edge.properties.into_iter()
                    .map(|(k, v)| (k, serde_json::to_value(v).unwrap_or(serde_json::Value::Null)))
                    .collect(),
            };
            edges.push(gql_edge);
        }

        Ok(edges)
    }

    async fn traverse(&self, start: &VertexId, pattern: &PathPattern) -> Result<Vec<Path>> {
        // TODO: Implement graph traversal
        Ok(Vec::new())
    }
}

#[async_trait]
impl IndexManagerPort for ProjectionEngineAdapter {
    async fn lookup_vertices(&self, property: &str, value: &serde_json::Value) -> Result<Vec<VertexId>> {
        // Use GraphDB's property index to lookup vertices
        // For now, scan all vertices and filter (implement proper indexing later)
        let nodes = self.projection_engine.graphdb.scan_nodes().await?;

        let mut vertex_ids = Vec::new();
        for node in nodes {
            if let Some(node_value) = node.properties.get(property) {
                let node_json = serde_json::to_value(node_value).unwrap_or(serde_json::Value::Null);
                if node_json == *value {
                    vertex_ids.push(node.id.clone());
                }
            }
        }

        Ok(vertex_ids)
    }

    async fn lookup_edges(&self, property: &str, value: &serde_json::Value) -> Result<Vec<EdgeId>> {
        // Use GraphDB's property index to lookup edges
        // For now, scan all edges and filter (implement proper indexing later)
        let edges = self.projection_engine.graphdb.scan_edges().await?;

        let mut edge_ids = Vec::new();
        for edge in edges {
            if let Some(edge_value) = edge.properties.get(property) {
                let edge_json = serde_json::to_value(edge_value).unwrap_or(serde_json::Value::Null);
                if edge_json == *value {
                    edge_ids.push(edge.id.clone());
                }
            }
        }

        Ok(edge_ids)
    }

    async fn range_scan(&self, property: &str, start: &serde_json::Value, end: &serde_json::Value) -> Result<Vec<VertexId>> {
        // Implement range scan for vertices
        // For now, scan all vertices and filter by range
        let nodes = self.projection_engine.graphdb.scan_nodes().await?;

        let mut vertex_ids = Vec::new();
        for node in nodes {
            if let Some(node_value) = node.properties.get(property) {
                let node_json = serde_json::to_value(node_value).unwrap_or(serde_json::Value::Null);
                // Simple range check for numbers (implement proper range logic later)
                // For now, only check numeric values
                match (&node_json, start, end) {
                    (serde_json::Value::Number(n), serde_json::Value::Number(s), serde_json::Value::Number(e)) => {
                        if n.as_f64() >= s.as_f64() && n.as_f64() <= e.as_f64() {
                            vertex_ids.push(node.id.clone());
                        }
                    }
                    _ => {
                        // For non-numeric values, skip (implement proper comparison later)
                    }
                }
            }
        }

        Ok(vertex_ids)
    }

    async fn has_vertex_index(&self, property: &str) -> Result<bool> {
        // For now, assume no indexes exist (implement proper index checking later)
        // TODO: Check if vertex index exists for the property
        Ok(false)
    }

    async fn has_edge_index(&self, property: &str) -> Result<bool> {
        // For now, assume no indexes exist (implement proper index checking later)
        // TODO: Check if edge index exists for the property
        Ok(false)
    }
}

#[async_trait]
impl CachePort for ProjectionEngineAdapter {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>> {
        // Convert CacheError to anyhow::Error
        self.projection_engine.cache_layer.get(key).await
            .map_err(|e| anyhow::anyhow!("Cache error: {}", e))
    }

    async fn set(&self, key: &str, value: serde_json::Value, ttl: Option<std::time::Duration>) -> Result<()> {
        let ttl_seconds = ttl.map(|d| d.as_secs() as u64);
        self.projection_engine.cache_layer.set(key, value, ttl_seconds).await
            .map_err(|e| anyhow::anyhow!("Cache error: {}", e))
    }

    async fn delete(&self, key: &str) -> Result<()> {
        self.projection_engine.cache_layer.delete(key).await
            .map_err(|e| anyhow::anyhow!("Cache error: {}", e))?;
        Ok(())
    }
}


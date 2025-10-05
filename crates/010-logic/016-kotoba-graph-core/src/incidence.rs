//! # Incidence Bipartite Graph Implementation
//!
//! This module provides an implementation of incidence bipartite graphs
//! for representing attributed graphs in a canonical form.

use super::*;
use crate::graph::Graph;
use kotoba_types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, BTreeMap, BTreeSet};

/// Incidence bipartite graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidenceGraph {
    /// Left vertices (entities/nodes)
    pub left_vertices: BTreeMap<String, IncidenceVertex>,
    /// Right vertices (attributes/relations)
    pub right_vertices: BTreeMap<String, IncidenceVertex>,
    /// Incidence edges between left and right vertices
    pub edges: BTreeMap<String, IncidenceEdge>,
    /// Metadata
    pub metadata: IncidenceMetadata,
}

impl IncidenceGraph {
    /// Create a new incidence graph
    pub fn new() -> Self {
        Self {
            left_vertices: BTreeMap::new(),
            right_vertices: BTreeMap::new(),
            edges: BTreeMap::new(),
            metadata: IncidenceMetadata::default(),
        }
    }

    /// Add a left vertex (entity/node)
    pub fn add_left_vertex(&mut self, vertex: IncidenceVertex) {
        self.left_vertices.insert(vertex.id.clone(), vertex);
    }

    /// Add a right vertex (attribute/relation)
    pub fn add_right_vertex(&mut self, vertex: IncidenceVertex) {
        self.right_vertices.insert(vertex.id.clone(), vertex);
    }

    /// Add an incidence edge
    pub fn add_edge(&mut self, edge: IncidenceEdge) {
        self.edges.insert(edge.id.clone(), edge);
    }

    /// Get all left vertices
    pub fn left_vertices(&self) -> &BTreeMap<String, IncidenceVertex> {
        &self.left_vertices
    }

    /// Get all right vertices
    pub fn right_vertices(&self) -> &BTreeMap<String, IncidenceVertex> {
        &self.right_vertices
    }

    /// Get all edges
    pub fn edges(&self) -> &BTreeMap<String, IncidenceEdge> {
        &self.edges
    }

    /// Get edges incident to a left vertex
    pub fn edges_for_left(&self, left_id: &str) -> Vec<&IncidenceEdge> {
        self.edges.values()
            .filter(|edge| edge.left == left_id)
            .collect()
    }

    /// Get edges incident to a right vertex
    pub fn edges_for_right(&self, right_id: &str) -> Vec<&IncidenceEdge> {
        self.edges.values()
            .filter(|edge| edge.right == right_id)
            .collect()
    }

    /// Compute canonical ordering of vertices
    pub fn canonical_vertex_ordering(&self) -> CanonicalOrdering {
        let mut ordering = CanonicalOrdering::new();

        // Order left vertices by their canonical signatures
        let mut left_order: Vec<(String, String)> = self.left_vertices.iter()
            .map(|(id, vertex)| (id.clone(), vertex.canonical_signature()))
            .collect();
        left_order.sort_by(|a, b| a.1.cmp(&b.1));

        for (id, _) in left_order {
            ordering.left_order.push(id);
        }

        // Order right vertices by their canonical signatures
        let mut right_order: Vec<(String, String)> = self.right_vertices.iter()
            .map(|(id, vertex)| (id.clone(), vertex.canonical_signature()))
            .collect();
        right_order.sort_by(|a, b| a.1.cmp(&b.1));

        for (id, _) in right_order {
            ordering.right_order.push(id);
        }

        ordering
    }

    /// Compute canonical ordering of edges
    pub fn canonical_edge_ordering(&self) -> CanonicalOrdering {
        let mut ordering = CanonicalOrdering::new();

        // Order edges by their canonical signatures
        let mut edge_order: Vec<(String, String)> = self.edges.iter()
            .map(|(id, edge)| (id.clone(), edge.canonical_signature()))
            .collect();
        edge_order.sort_by(|a, b| a.1.cmp(&b.1));

        for (id, _) in edge_order {
            ordering.edge_order.push(id);
        }

        ordering
    }

    /// Convert to canonical form
    pub fn to_canonical_form(&self) -> CanonicalIncidenceGraph {
        let vertex_ordering = self.canonical_vertex_ordering();
        let edge_ordering = self.canonical_edge_ordering();

        CanonicalIncidenceGraph {
            left_vertices: self.reorder_vertices(&self.left_vertices, &vertex_ordering.left_order),
            right_vertices: self.reorder_vertices(&self.right_vertices, &vertex_ordering.right_order),
            edges: self.reorder_edges(&edge_ordering.edge_order),
            ordering: vertex_ordering,
        }
    }

    /// Reorder vertices according to canonical ordering
    fn reorder_vertices(
        &self,
        vertices: &BTreeMap<String, IncidenceVertex>,
        ordering: &[String],
    ) -> Vec<(String, IncidenceVertex)> {
        ordering.iter()
            .filter_map(|id| vertices.get(id).map(|v| (id.clone(), v.clone())))
            .collect()
    }

    /// Reorder edges according to canonical ordering
    fn reorder_edges(&self, ordering: &[String]) -> Vec<(String, IncidenceEdge)> {
        ordering.iter()
            .filter_map(|id| self.edges.get(id).map(|e| (id.clone(), e.clone())))
            .collect()
    }

    /// Compute hash of the incidence graph
    pub fn hash(&self) -> Hash {
        let canonical = self.to_canonical_form();
        Hash::from_sha256(&serde_json::to_vec(&canonical).expect("Failed to serialize"))
    }
}

/// Incidence vertex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidenceVertex {
    /// Vertex ID
    pub id: String,
    /// Vertex type
    pub vertex_type: String,
    /// Properties
    pub properties: HashMap<String, Value>,
    /// Degree (for ordering)
    pub degree: usize,
    /// Canonical signature for ordering
    pub canonical_signature: String,
}

impl IncidenceVertex {
    /// Create a new incidence vertex
    pub fn new(id: String, vertex_type: String) -> Self {
        Self {
            id,
            vertex_type,
            properties: HashMap::new(),
            degree: 0,
            canonical_signature: String::new(),
        }
    }

    /// Set properties
    pub fn with_properties(mut self, properties: HashMap<String, Value>) -> Self {
        self.properties = properties;
        self
    }

    /// Compute canonical signature
    pub fn canonical_signature(&self) -> String {
        let mut signature = format!("{}:{}", self.vertex_type, self.degree);

        // Add properties in sorted order
        let mut props: Vec<_> = self.properties.iter().collect();
        props.sort_by(|a, b| a.0.cmp(b.0));

        for (key, value) in props {
            signature.push_str(&format!(":{}={}", key, value_to_canonical_string(value)));
        }

        signature
    }
}

/// Incidence edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidenceEdge {
    /// Edge ID
    pub id: String,
    /// Left vertex (entity)
    pub left: String,
    /// Right vertex (attribute/relation)
    pub right: String,
    /// Edge type
    pub edge_type: String,
    /// Properties
    pub properties: HashMap<String, Value>,
    /// Multiplicity (for ordering)
    pub multiplicity: usize,
}

impl IncidenceEdge {
    /// Create a new incidence edge
    pub fn new(left: String, right: String, edge_type: String) -> Self {
        Self {
            id: format!("{}_{}_{}", left, edge_type, right),
            left,
            right,
            edge_type,
            properties: HashMap::new(),
            multiplicity: 1,
        }
    }

    /// Set properties
    pub fn with_properties(mut self, properties: HashMap<String, Value>) -> Self {
        self.properties = properties;
        self
    }

    /// Compute canonical signature
    pub fn canonical_signature(&self) -> String {
        let mut signature = format!("{}:{}:{}", self.left, self.edge_type, self.right);

        // Add properties in sorted order
        let mut props: Vec<_> = self.properties.iter().collect();
        props.sort_by(|a, b| a.0.cmp(b.0));

        for (key, value) in props {
            signature.push_str(&format!(":{}={}", key, value_to_canonical_string(value)));
        }

        signature
    }
}

/// Canonical ordering of graph elements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalOrdering {
    /// Ordering of left vertices
    pub left_order: Vec<String>,
    /// Ordering of right vertices
    pub right_order: Vec<String>,
    /// Ordering of edges
    pub edge_order: Vec<String>,
}

impl CanonicalOrdering {
    /// Create a new canonical ordering
    pub fn new() -> Self {
        Self {
            left_order: Vec::new(),
            right_order: Vec::new(),
            edge_order: Vec::new(),
        }
    }

    /// Get the position of a left vertex in the ordering
    pub fn left_position(&self, vertex_id: &str) -> Option<usize> {
        self.left_order.iter().position(|id| id == vertex_id)
    }

    /// Get the position of a right vertex in the ordering
    pub fn right_position(&self, vertex_id: &str) -> Option<usize> {
        self.right_order.iter().position(|id| id == vertex_id)
    }

    /// Get the position of an edge in the ordering
    pub fn edge_position(&self, edge_id: &str) -> Option<usize> {
        self.edge_order.iter().position(|id| id == edge_id)
    }
}

/// Canonical incidence graph (ordered)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalIncidenceGraph {
    /// Left vertices in canonical order
    pub left_vertices: Vec<(String, IncidenceVertex)>,
    /// Right vertices in canonical order
    pub right_vertices: Vec<(String, IncidenceVertex)>,
    /// Edges in canonical order
    pub edges: Vec<(String, IncidenceEdge)>,
    /// Canonical ordering used
    pub ordering: CanonicalOrdering,
}

impl CanonicalIncidenceGraph {
    /// Create a new canonical incidence graph
    pub fn new() -> Self {
        Self {
            left_vertices: Vec::new(),
            right_vertices: Vec::new(),
            edges: Vec::new(),
            ordering: CanonicalOrdering::new(),
        }
    }

    /// Compute hash of the canonical form
    pub fn hash(&self) -> Hash {
        Hash::from_sha256(&serde_json::to_vec(self).expect("Failed to serialize"))
    }

    /// Get adjacency list for left vertices
    pub fn left_adjacency(&self) -> HashMap<String, Vec<String>> {
        let mut adj = HashMap::new();

        for (edge_id, edge) in &self.edges {
            adj.entry(edge.left.clone())
                .or_insert_with(Vec::new)
                .push(edge.right.clone());
        }

        adj
    }

    /// Get adjacency list for right vertices
    pub fn right_adjacency(&self) -> HashMap<String, Vec<String>> {
        let mut adj = HashMap::new();

        for (edge_id, edge) in &self.edges {
            adj.entry(edge.right.clone())
                .or_insert_with(Vec::new)
                .push(edge.left.clone());
        }

        adj
    }
}

/// Incidence metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidenceMetadata {
    /// Graph type
    pub graph_type: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Number of left vertices
    pub left_count: usize,
    /// Number of right vertices
    pub right_count: usize,
    /// Number of edges
    pub edge_count: usize,
    /// Properties
    pub properties: HashMap<String, Value>,
}

impl Default for IncidenceMetadata {
    fn default() -> Self {
        Self {
            graph_type: "incidence_bipartite".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            left_count: 0,
            right_count: 0,
            edge_count: 0,
            properties: HashMap::new(),
        }
    }
}

/// Converter from regular graphs to incidence graphs
#[derive(Debug, Clone)]
pub struct IncidenceConverter {
    /// Configuration
    pub config: ConverterConfig,
}

impl IncidenceConverter {
    /// Create a new converter
    pub fn new() -> Self {
        Self {
            config: ConverterConfig::default(),
        }
    }

    /// Convert a regular graph to incidence bipartite graph
    pub fn convert(&self, graph: &Graph) -> IncidenceGraph {
        let mut incidence_graph = IncidenceGraph::new();

        // Convert vertices to left vertices
        for (vertex_id, vertex_data) in &graph.vertices {
            let mut properties = HashMap::new();
            properties.insert("labels".to_string(), Value::Array(
                vertex_data.labels.iter().cloned().map(Value::String).collect()
            ));

            for (key, value) in &vertex_data.props {
                properties.insert(key.clone(), value.clone());
            }

            let incidence_vertex = IncidenceVertex::new(
                vertex_id.to_string(),
                "entity".to_string(),
            ).with_properties(properties);

            incidence_graph.add_left_vertex(incidence_vertex);
        }

        // Convert edges to right vertices and incidence edges
        for (edge_id, edge_data) in &graph.edges {
            // Create right vertex for the edge label
            let label_vertex_id = format!("attr_{}", edge_data.label);
            let mut label_properties = HashMap::new();
            label_properties.insert("type".to_string(), Value::String("edge_label".to_string()));

            let label_vertex = IncidenceVertex::new(
                label_vertex_id.clone(),
                "attribute".to_string(),
            ).with_properties(label_properties);

            incidence_graph.add_right_vertex(label_vertex);

            // Create incidence edge
            let incidence_edge = IncidenceEdge::new(
                edge_data.src.to_string(),
                label_vertex_id,
                "incidence".to_string(),
            );

            incidence_graph.add_edge(incidence_edge);
        }

        // Update metadata
        incidence_graph.metadata.left_count = incidence_graph.left_vertices.len();
        incidence_graph.metadata.right_count = incidence_graph.right_vertices.len();
        incidence_graph.metadata.edge_count = incidence_graph.edges.len();

        incidence_graph
    }
}

/// Converter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConverterConfig {
    /// Include vertex properties as attributes
    pub include_vertex_properties: bool,
    /// Include edge properties as attributes
    pub include_edge_properties: bool,
    /// Prefix for generated attribute vertices
    pub attribute_prefix: String,
}

impl Default for ConverterConfig {
    fn default() -> Self {
        Self {
            include_vertex_properties: true,
            include_edge_properties: true,
            attribute_prefix: "attr_".to_string(),
        }
    }
}

/// Utility function to convert values to canonical strings
fn value_to_canonical_string(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => format!("bool:{}", b),
        Value::Number(n) => format!("number:{}", n),
        Value::String(s) => format!("string:{}", s),
        Value::Array(arr) => format!("array:{}", arr.len()),
        Value::Object(obj) => format!("object:{}", obj.len()),
        _ => "unknown".to_string(),
    }
}

//! # Graph Data Structures and Algorithms
//!
//! This module provides comprehensive graph data structures and algorithms
//! for the Kotoba ecosystem, including column-oriented graph storage,
//! traversal algorithms, and graph transformation utilities.

use kotoba_types::*;
use kotoba_errors::KotobaError;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

/// Vertex data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    pub id: VertexId,
    pub labels: Vec<Label>,
    pub props: Properties,
}

impl VertexData {
    /// Create a new vertex with the given ID
    pub fn new(id: VertexId) -> Self {
        Self {
            id,
            labels: Vec::new(),
            props: HashMap::new(),
        }
    }

    /// Add a label to this vertex
    pub fn with_label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Add a property to this vertex
    pub fn with_property(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Check if vertex has a specific label
    pub fn has_label(&self, label: &Label) -> bool {
        self.labels.contains(label)
    }

    /// Get a property value
    pub fn get_property(&self, key: &PropertyKey) -> Option<&Value> {
        self.props.get(key)
    }

    /// Get all property keys
    pub fn property_keys(&self) -> Vec<&PropertyKey> {
        self.props.keys().collect()
    }
}

/// Edge data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: EdgeId,
    pub src: VertexId,
    pub dst: VertexId,
    pub label: Label,
    pub props: Properties,
}

impl EdgeData {
    /// Create a new edge
    pub fn new(src: VertexId, dst: VertexId, label: Label) -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            src,
            dst,
            label,
            props: HashMap::new(),
        }
    }

    /// Create a new edge with specific ID
    pub fn with_id(mut self, id: EdgeId) -> Self {
        self.id = id;
        self
    }

    /// Add a property to this edge
    pub fn with_property(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Get a property value
    pub fn get_property(&self, key: &PropertyKey) -> Option<&Value> {
        self.props.get(key)
    }

    /// Get the opposite vertex ID given one vertex ID
    pub fn opposite(&self, vertex_id: &VertexId) -> Option<&VertexId> {
        if &self.src == vertex_id {
            Some(&self.dst)
        } else if &self.dst == vertex_id {
            Some(&self.src)
        } else {
            None
        }
    }
}

/// Column-oriented graph representation
#[derive(Debug, Clone)]
pub struct Graph {
    /// Vertex data (ID -> data)
    pub vertices: HashMap<VertexId, VertexData>,

    /// Edge data (ID -> data)
    pub edges: HashMap<EdgeId, EdgeData>,

    /// Outgoing adjacency list (src -> [dst])
    pub adj_out: HashMap<VertexId, HashSet<VertexId>>,

    /// Incoming adjacency list (dst -> [src])
    pub adj_in: HashMap<VertexId, HashSet<VertexId>>,

    /// Label-based vertex index (label -> [vertex ID])
    pub vertex_labels: HashMap<Label, HashSet<VertexId>>,

    /// Label-based edge index (label -> [edge ID])
    pub edge_labels: HashMap<Label, HashSet<EdgeId>>,
}

impl Graph {
    /// Create an empty graph
    pub fn empty() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: HashMap::new(),
            adj_out: HashMap::new(),
            adj_in: HashMap::new(),
            vertex_labels: HashMap::new(),
            edge_labels: HashMap::new(),
        }
    }

    /// Add a vertex to the graph
    pub fn add_vertex(&mut self, vertex: VertexData) -> VertexId {
        let id = vertex.id;
        for label in &vertex.labels {
            self.vertex_labels.entry(label.clone()).or_insert_with(HashSet::new).insert(id);
        }
        self.vertices.insert(id, vertex);
        id
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: EdgeData) -> EdgeId {
        let id = edge.id;
        let src = edge.src;
        let dst = edge.dst;

        // Update adjacency lists
        self.adj_out.entry(src).or_insert_with(HashSet::new).insert(dst);
        self.adj_in.entry(dst).or_insert_with(HashSet::new).insert(src);

        // Update label index
        self.edge_labels.entry(edge.label.clone()).or_insert_with(HashSet::new).insert(id);

        self.edges.insert(id, edge);
        id
    }

    /// Remove a vertex from the graph
    pub fn remove_vertex(&mut self, id: &VertexId) -> bool {
        if let Some(vertex) = self.vertices.remove(id) {
            // Remove from label indices
            for label in &vertex.labels {
                if let Some(vertices) = self.vertex_labels.get_mut(label) {
                    vertices.remove(id);
                    if vertices.is_empty() {
                        self.vertex_labels.remove(label);
                    }
                }
            }

            // Remove related edges
            let mut edges_to_remove = Vec::new();
            for (edge_id, edge) in &self.edges {
                if edge.src == *id || edge.dst == *id {
                    edges_to_remove.push(*edge_id);
                }
            }
            for edge_id in edges_to_remove {
                self.remove_edge(&edge_id);
            }

            // Remove from adjacency lists
            self.adj_out.remove(id);
            self.adj_in.remove(id);

            // Remove from other vertices' adjacency lists
            for adj in self.adj_out.values_mut() {
                adj.remove(id);
            }
            for adj in self.adj_in.values_mut() {
                adj.remove(id);
            }

            true
        } else {
            false
        }
    }

    /// Remove an edge from the graph
    pub fn remove_edge(&mut self, id: &EdgeId) -> bool {
        if let Some(edge) = self.edges.remove(id) {
            // Update adjacency lists
            if let Some(out) = self.adj_out.get_mut(&edge.src) {
                out.remove(&edge.dst);
                if out.is_empty() {
                    self.adj_out.remove(&edge.src);
                }
            }
            if let Some(in_) = self.adj_in.get_mut(&edge.dst) {
                in_.remove(&edge.src);
                if in_.is_empty() {
                    self.adj_in.remove(&edge.dst);
                }
            }

            // Update label index
            if let Some(edges) = self.edge_labels.get_mut(&edge.label) {
                edges.remove(id);
                if edges.is_empty() {
                    self.edge_labels.remove(&edge.label);
                }
            }

            true
        } else {
            false
        }
    }

    /// Get a vertex by ID
    pub fn get_vertex(&self, id: &VertexId) -> Option<&VertexData> {
        self.vertices.get(id)
    }

    /// Get an edge by ID
    pub fn get_edge(&self, id: &EdgeId) -> Option<&EdgeData> {
        self.edges.get(id)
    }

    /// Get vertices by label
    pub fn vertices_by_label(&self, label: &Label) -> HashSet<VertexId> {
        self.vertex_labels.get(label).cloned().unwrap_or_default()
    }

    /// Get edges by label
    pub fn edges_by_label(&self, label: &Label) -> HashSet<EdgeId> {
        self.edge_labels.get(label).cloned().unwrap_or_default()
    }

    /// Get vertex degree
    pub fn degree(&self, id: &VertexId) -> usize {
        let out_degree = self.adj_out.get(id).map(|s| s.len()).unwrap_or(0);
        let in_degree = self.adj_in.get(id).map(|s| s.len()).unwrap_or(0);
        out_degree + in_degree
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Get all vertex IDs
    pub fn vertex_ids(&self) -> Vec<VertexId> {
        self.vertices.keys().cloned().collect()
    }

    /// Get all edge IDs
    pub fn edge_ids(&self) -> Vec<EdgeId> {
        self.edges.keys().cloned().collect()
    }

    /// Check if graph contains vertex
    pub fn contains_vertex(&self, id: &VertexId) -> bool {
        self.vertices.contains_key(id)
    }

    /// Check if graph contains edge
    pub fn contains_edge(&self, id: &EdgeId) -> bool {
        self.edges.contains_key(id)
    }

    /// Get neighbors of a vertex
    pub fn neighbors(&self, id: &VertexId) -> Vec<VertexId> {
        self.adj_out.get(id).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// Get incoming neighbors of a vertex
    pub fn incoming_neighbors(&self, id: &VertexId) -> Vec<VertexId> {
        self.adj_in.get(id).map(|s| s.iter().cloned().collect()).unwrap_or_default()
    }

    /// Convert from GraphInstance
    pub fn from_graph_instance(graph_instance: &GraphInstance) -> KotobaResult<Self> {
        let mut graph = Graph::empty();
        let mut vertex_id_map = HashMap::new();

        // Convert Nodes to Vertices
        for node in &graph_instance.core.nodes {
            let vertex_id = uuid::Uuid::new_v4(); // Should be deterministically generated from CID

            let props = if let Some(attrs) = &node.attrs {
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            };

            let vertex_data = VertexData {
                id: vertex_id,
                labels: node.labels.clone(),
                props,
            };

            graph.add_vertex(vertex_data);
            vertex_id_map.insert(node.cid.as_str(), vertex_id);
        }

        // Convert Edges to Edges
        for edge in &graph_instance.core.edges {
            let edge_id = uuid::Uuid::new_v4(); // Should be deterministically generated from CID

            let src_vertex_id = *vertex_id_map.get(edge.src.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Source node not found: {}", edge.src)))?;

            let dst_vertex_id = *vertex_id_map.get(edge.tgt.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Target node not found: {}", edge.tgt)))?;

            let props = if let Some(attrs) = &edge.attrs {
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            };

            let edge_data = EdgeData {
                id: edge_id,
                src: src_vertex_id,
                dst: dst_vertex_id,
                label: edge.r#type.clone(),
                props,
            };

            graph.add_edge(edge_data);
        }

        Ok(graph)
    }

    /// Convert to GraphInstance
    pub fn to_graph_instance(&self, graph_cid: Cid) -> GraphInstance {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Convert Vertices to Nodes
        for (vertex_id, vertex_data) in &self.vertices {
            let node = Node {
                cid: generate_cid(&format!("node_{}", vertex_id)),
                labels: vertex_data.labels.clone(),
                r#type: vertex_data.labels.first()
                    .cloned()
                    .unwrap_or_else(|| "Node".to_string()),
                ports: vec![],
                attrs: if vertex_data.props.is_empty() {
                    None
                } else {
                    Some(vertex_data.props.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>())
                },
                component_ref: None,
            };
            nodes.push(node);
        }

        // Convert Edges to Edges
        for (edge_id, edge_data) in &self.edges {
            let edge = Edge {
                cid: generate_cid(&format!("edge_{}", edge_id)),
                label: Some(edge_data.label.clone()),
                r#type: edge_data.label.clone(),
                src: format!("#{}", edge_data.src),
                tgt: format!("#{}", edge_data.dst),
                attrs: if edge_data.props.is_empty() {
                    None
                } else {
                    Some(edge_data.props.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>())
                },
            };
            edges.push(edge);
        }

        let graph_core = GraphCore {
            nodes,
            edges,
            boundary: None,
            attrs: None,
        };

        GraphInstance {
            core: graph_core,
            kind: GraphKind::Graph,
            cid: graph_cid,
            typing: None,
        }
    }

    /// Convert to GraphInstance with computed CID
    pub fn to_graph_instance_with_cid(&self) -> KotobaResult<GraphInstance> {
        let graph_instance = self.to_graph_instance(generate_cid("temp"));
        Ok(GraphInstance {
            cid: generate_cid("graph"),
            ..graph_instance
        })
    }
}

/// Thread-safe graph reference
#[derive(Debug, Clone)]
pub struct GraphRef {
    inner: Arc<RwLock<Graph>>,
}

impl GraphRef {
    /// Create a new graph reference
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: Arc::new(RwLock::new(graph)),
        }
    }

    /// Get read access to the graph
    pub fn read(&self) -> RwLockReadGuard<'_, Graph> {
        self.inner.read()
    }

    /// Get write access to the graph
    pub fn write(&self) -> RwLockWriteGuard<'_, Graph> {
        self.inner.write()
    }

    /// Convert from GraphInstance
    pub fn from_graph_instance(graph_instance: &GraphInstance) -> KotobaResult<Self> {
        let graph = Graph::from_graph_instance(graph_instance)?;
        Ok(Self::new(graph))
    }

    /// Convert to GraphInstance
    pub fn to_graph_instance(&self, graph_cid: Cid) -> GraphInstance {
        let graph = self.read();
        graph.to_graph_instance(graph_cid)
    }

    /// Convert to GraphInstance with computed CID
    pub fn to_graph_instance_with_cid(&self) -> KotobaResult<GraphInstance> {
        let graph = self.read();
        graph.to_graph_instance_with_cid()
    }
}

/// CID generation helper
fn generate_cid(data: &str) -> Cid {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result[..32]);
    Cid(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        let graph = Graph::empty();
        assert_eq!(graph.vertex_count(), 0);
        assert_eq!(graph.edge_count(), 0);
        assert!(graph.is_connected);
    }

    #[test]
    fn test_vertex_operations() {
        let mut graph = Graph::empty();

        let vertex = VertexData::new(uuid::Uuid::new_v4())
            .with_label("Person".to_string())
            .with_property("name".to_string(), Value::String("Alice".to_string()));

        let vertex_id = graph.add_vertex(vertex);
        assert!(graph.contains_vertex(&vertex_id));
        assert_eq!(graph.vertex_count(), 1);
    }

    #[test]
    fn test_edge_operations() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        let edge = EdgeData::new(v1, v2, "knows".to_string());
        let edge_id = graph.add_edge(edge);

        assert!(graph.contains_edge(&edge_id));
        assert_eq!(graph.edge_count(), 1);
        assert_eq!(graph.degree(&v1), 1);
        assert_eq!(graph.degree(&v2), 1);
    }

    #[test]
    fn test_vertex_removal() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));
        let v2 = graph.add_vertex(VertexData::new(uuid::Uuid::new_v4()));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        graph.remove_vertex(&v1);

        assert_eq!(graph.vertex_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }
}

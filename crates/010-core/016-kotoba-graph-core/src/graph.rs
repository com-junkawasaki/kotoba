//! # Graph Data Structures and Algorithms
//!
//! This module provides comprehensive graph data structures and algorithms
//! for the Kotoba ecosystem, including column-oriented graph storage,
//! traversal algorithms, and graph transformation utilities.

use kotoba_types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use parking_lot::{RwLock, RwLockReadGuard, RwLockWriteGuard};

// Re-export types from kotoba-types
pub use kotoba_types::{VertexId, EdgeId, Label, Properties, PropertyKey, Value, GraphInstance, GraphCore, Node, Edge, GraphKind, Typing, Boundary, Port, PortDirection, Attrs};

/// Generate a deterministic CID from vertex data
fn generate_vertex_cid(labels: &[Label], props: &Properties) -> VertexId {
    let mut data = Vec::new();
    data.extend_from_slice(&serde_json::to_vec(labels).unwrap());
    data.extend_from_slice(&serde_json::to_vec(props).unwrap());
    VertexId::from(data.as_slice())
}

/// Generate a deterministic CID from edge data
fn generate_edge_cid(source: &VertexId, target: &VertexId, label: &Label, props: &Properties) -> EdgeId {
    let mut data = Vec::new();
    data.extend_from_slice(source.as_ref().as_bytes());
    data.extend_from_slice(target.as_ref().as_bytes());
    data.extend_from_slice(label.as_bytes());
    data.extend_from_slice(&serde_json::to_vec(props).unwrap());
    EdgeId::from(data.as_slice())
}

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
            id: generate_vertex_cid(&[], &HashMap::new()),
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

    /// Check if graph is connected
    pub fn is_connected(&self) -> bool {
        if self.vertices.is_empty() {
            return true;
        }

        let mut visited = HashSet::new();
        let mut to_visit = VecDeque::new();

        // Start from first vertex
        if let Some(first_vertex) = self.vertices.keys().next() {
            to_visit.push_back(*first_vertex);
            visited.insert(*first_vertex);
        }

        // BFS traversal
        while let Some(current) = to_visit.pop_front() {
            if let Some(neighbors) = self.adj_out.get(&current) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        visited.insert(*neighbor);
                        to_visit.push_back(*neighbor);
                    }
                }
            }
        }

        visited.len() == self.vertices.len()
    }

    /// Check if graph contains vertex
    pub fn contains_vertex(&self, id: &VertexId) -> bool {
        self.vertices.contains_key(id)
    }

    /// Check if graph contains edge
    pub fn contains_edge(&self, id: &EdgeId) -> bool {
        self.edges.contains_key(id)
    }
}

/// Vertex builder for fluent API
#[derive(Debug, Clone)]
pub struct VertexBuilder {
    id: Option<VertexId>,
    labels: Vec<Label>,
    props: Properties,
}

impl VertexBuilder {
    /// Create a new vertex builder
    pub fn new() -> Self {
        Self {
            id: None,
            labels: Vec::new(),
            props: HashMap::new(),
        }
    }

    /// Set vertex ID
    pub fn id(mut self, id: VertexId) -> Self {
        self.id = Some(id);
        self
    }

    /// Add a label
    pub fn label(mut self, label: Label) -> Self {
        self.labels.push(label);
        self
    }

    /// Set labels
    pub fn labels(mut self, labels: Vec<Label>) -> Self {
        self.labels = labels;
        self
    }

    /// Add a property
    pub fn prop(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Set properties
    pub fn props(mut self, props: Properties) -> Self {
        self.props = props;
        self
    }

    /// Build the vertex
    pub fn build(self) -> VertexData {
        VertexData {
            id: self.id.unwrap_or_else(|| generate_vertex_cid(&[], &HashMap::new())),
            labels: self.labels,
            props: self.props,
        }
    }
}

/// Edge builder for fluent API
#[derive(Debug, Clone)]
pub struct EdgeBuilder {
    id: Option<EdgeId>,
    src: Option<VertexId>,
    dst: Option<VertexId>,
    label: Option<Label>,
    props: Properties,
}

impl EdgeBuilder {
    /// Create a new edge builder
    pub fn new() -> Self {
        Self {
            id: None,
            src: None,
            dst: None,
            label: None,
            props: HashMap::new(),
        }
    }

    /// Set edge ID
    pub fn id(mut self, id: EdgeId) -> Self {
        self.id = Some(id);
        self
    }

    /// Set source vertex
    pub fn src(mut self, src: VertexId) -> Self {
        self.src = Some(src);
        self
    }

    /// Set destination vertex
    pub fn dst(mut self, dst: VertexId) -> Self {
        self.dst = Some(dst);
        self
    }

    /// Set label
    pub fn label(mut self, label: Label) -> Self {
        self.label = Some(label);
        self
    }

    /// Add a property
    pub fn prop(mut self, key: PropertyKey, value: Value) -> Self {
        self.props.insert(key, value);
        self
    }

    /// Set properties
    pub fn props(mut self, props: Properties) -> Self {
        self.props = props;
        self
    }

    /// Build the edge
    pub fn build(self) -> Result<EdgeData, String> {
        let src = self.src.ok_or("Source vertex not set")?;
        let dst = self.dst.ok_or("Destination vertex not set")?;
        let label = self.label.ok_or("Label not set")?;

        Ok(EdgeData {
            id: self.id.unwrap_or_else(|| generate_vertex_cid(&[], &HashMap::new())),
            src,
            dst,
            label,
            props: self.props,
        })
    }
}

/// Graph builder for fluent API
#[derive(Debug, Clone)]
pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: Graph::empty(),
        }
    }

    /// Add a vertex
    pub fn vertex(mut self, vertex: VertexData) -> Self {
        self.graph.add_vertex(vertex);
        self
    }

    /// Add an edge
    pub fn edge(mut self, edge: EdgeData) -> Self {
        self.graph.add_edge(edge);
        self
    }

    /// Build the graph
    pub fn build(self) -> Graph {
        self.graph
    }
}

/// Graph traversal algorithms
#[derive(Debug, Clone)]
pub struct GraphTraversal<'a> {
    graph: &'a Graph,
    visited: HashSet<VertexId>,
    queue: VecDeque<VertexId>,
    stack: Vec<VertexId>,
}

impl<'a> GraphTraversal<'a> {
    /// Create a new traversal
    pub fn new(graph: &'a Graph) -> Self {
        Self {
            graph,
            visited: HashSet::new(),
            queue: VecDeque::new(),
            stack: Vec::new(),
        }
    }

    /// BFS (Breadth-First Search)
    pub fn bfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.queue.clear();
        let mut result = Vec::new();

        self.visited.insert(start);
        self.queue.push_back(start);

        while let Some(current) = self.queue.pop_front() {
            result.push(current);

            // Explore neighbors
            if let Some(neighbors) = self.graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if !self.visited.contains(&neighbor) {
                        self.visited.insert(neighbor);
                        self.queue.push_back(neighbor);
                    }
                }
            }
        }

        result
    }

    /// DFS (Depth-First Search)
    pub fn dfs(&mut self, start: VertexId) -> Vec<VertexId> {
        self.visited.clear();
        self.stack.clear();
        let mut result = Vec::new();

        self.stack.push(start);

        while let Some(current) = self.stack.pop() {
            if !self.visited.contains(&current) {
                self.visited.insert(current);
                result.push(current);

                // Explore neighbors in reverse order for correct DFS
                if let Some(neighbors) = self.graph.adj_out.get(&current) {
                    let mut neighbors_vec: Vec<_> = neighbors.iter().collect();
                    neighbors_vec.reverse();

                    for &neighbor in &neighbors_vec {
                        if !self.visited.contains(&neighbor) {
                            self.stack.push(neighbor);
                        }
                    }
                }
            }
        }

        result
    }

    /// Check if graph is connected
    pub fn is_connected(&mut self) -> bool {
        if self.graph.vertices.is_empty() {
            return true;
        }

        // Start BFS from first vertex
        if let Some(first_vertex) = self.graph.vertices.keys().next() {
            let reachable = self.bfs(*first_vertex);
            reachable.len() == self.graph.vertices.len()
        } else {
            false
        }
    }

    /// Find shortest path between two vertices
    pub fn shortest_path(&mut self, start: VertexId, end: VertexId) -> Option<Vec<VertexId>> {
        let mut parent: HashMap<VertexId, VertexId> = HashMap::new();
        let mut queue = VecDeque::new();

        queue.push_back(start);
        parent.insert(start, start);

        while let Some(current) = queue.pop_front() {
            if current == end {
                // Reconstruct path
                let mut path = Vec::new();
                let mut current = end;

                while current != start {
                    path.push(current);
                    if let Some(&next) = parent.get(&current) {
                        current = next;
                    } else {
                        return None;
                    }
                }
                path.push(start);
                path.reverse();
                return Some(path);
            }

            if let Some(neighbors) = self.graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if !parent.contains_key(&neighbor) {
                        parent.insert(neighbor, current);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        None
    }

    /// Find all cycles in the graph
    pub fn find_cycles(&mut self) -> Vec<Vec<VertexId>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for &vertex in self.graph.vertices.keys() {
            if !visited.contains(&vertex) {
                self.find_cycles_dfs(vertex, &mut visited, &mut rec_stack, &mut cycles, vertex);
            }
        }

        cycles
    }

    /// DFS helper for cycle detection
    fn find_cycles_dfs(
        &mut self,
        vertex: VertexId,
        visited: &mut HashSet<VertexId>,
        rec_stack: &mut HashSet<VertexId>,
        cycles: &mut Vec<Vec<VertexId>>,
        start: VertexId,
    ) {
        visited.insert(vertex);
        rec_stack.insert(vertex);

        if let Some(neighbors) = self.graph.adj_out.get(&vertex) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    self.find_cycles_dfs(neighbor, visited, rec_stack, cycles, start);
                } else if rec_stack.contains(&neighbor) {
                    // Cycle found - but we need to extract the actual cycle
                    cycles.push(vec![vertex, neighbor]);
                }
            }
        }

        rec_stack.remove(&vertex);
    }

    /// Compute graph diameter (longest shortest path)
    pub fn diameter(&mut self) -> Option<usize> {
        if !self.is_connected() {
            return None;
        }

        let mut max_distance = 0;

        for &vertex in self.graph.vertices.keys() {
            if let Some(distances) = self.compute_distances(vertex) {
                if let Some(&max_dist) = distances.values().max() {
                    max_distance = max_distance.max(max_dist);
                }
            }
        }

        Some(max_distance)
    }

    /// Compute distances from a source vertex
    fn compute_distances(&mut self, source: VertexId) -> Option<HashMap<VertexId, usize>> {
        let mut distances = HashMap::new();
        let mut queue = VecDeque::new();

        distances.insert(source, 0);
        queue.push_back(source);

        while let Some(current) = queue.pop_front() {
            let current_dist = distances[&current];

            if let Some(neighbors) = self.graph.adj_out.get(&current) {
                for &neighbor in neighbors {
                    if !distances.contains_key(&neighbor) {
                        distances.insert(neighbor, current_dist + 1);
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        Some(distances)
    }

    /// Find strongly connected components (Kosaraju's algorithm)
    pub fn strongly_connected_components(&mut self) -> Vec<Vec<VertexId>> {
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        // First DFS pass: get finishing times
        for &vertex in self.graph.vertices.keys() {
            if !visited.contains(&vertex) {
                self.scc_dfs1(vertex, &mut visited, &mut stack);
            }
        }

        // Transpose the graph
        let transposed = self.transpose_graph();

        // Second DFS pass: get SCCs
        let mut visited = HashSet::new();
        let mut sccs = Vec::new();

        while let Some(vertex) = stack.pop() {
            if !visited.contains(&vertex) {
                let mut scc = Vec::new();
                self.scc_dfs2(vertex, &transposed, &mut visited, &mut scc);
                sccs.push(scc);
            }
        }

        sccs
    }

    /// First DFS pass for SCC
    fn scc_dfs1(&mut self, vertex: VertexId, visited: &mut HashSet<VertexId>, stack: &mut Vec<VertexId>) {
        visited.insert(vertex);

        if let Some(neighbors) = self.graph.adj_out.get(&vertex) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    self.scc_dfs1(neighbor, visited, stack);
                }
            }
        }

        stack.push(vertex);
    }

    /// Second DFS pass for SCC
    fn scc_dfs2(
        &mut self,
        vertex: VertexId,
        transposed: &HashMap<VertexId, HashSet<VertexId>>,
        visited: &mut HashSet<VertexId>,
        scc: &mut Vec<VertexId>,
    ) {
        visited.insert(vertex);
        scc.push(vertex);

        if let Some(neighbors) = transposed.get(&vertex) {
            for &neighbor in neighbors {
                if !visited.contains(&neighbor) {
                    self.scc_dfs2(neighbor, transposed, visited, scc);
                }
            }
        }
    }

    /// Transpose the graph (reverse all edges)
    fn transpose_graph(&self) -> HashMap<VertexId, HashSet<VertexId>> {
        let mut transposed = HashMap::new();

        for (src, dsts) in &self.graph.adj_out {
            for dst in dsts {
                transposed.entry(*dst).or_insert_with(HashSet::new).insert(*src);
            }
        }

        transposed
    }
}

/// Graph analysis utilities
#[derive(Debug, Clone)]
pub struct GraphAnalysis;

impl GraphAnalysis {
    /// Compute graph statistics
    pub fn statistics(graph: &Graph) -> GraphStatistics {
        let vertex_count = graph.vertex_count();
        let edge_count = graph.edge_count();

        let degrees: Vec<usize> = graph.vertices.keys()
            .map(|v| graph.degree(v))
            .collect();

        let average_degree = if vertex_count > 0 {
            degrees.iter().sum::<usize>() as f64 / vertex_count as f64
        } else {
            0.0
        };

        let max_degree = degrees.iter().max().copied().unwrap_or(0);

        GraphStatistics {
            vertex_count,
            edge_count,
            average_degree,
            max_degree,
            density: if vertex_count > 1 {
                2.0 * edge_count as f64 / (vertex_count as f64 * (vertex_count - 1) as f64)
            } else {
                0.0
            },
        }
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
            let vertex_id = generate_vertex_cid(&[], &HashMap::new()); // Deterministically generated from CID

            let props = if let Some(attrs) = &node.attrs {
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<_, _>>()
            } else {
                HashMap::new()
            };

            graph.vertices.insert(
                vertex_id,
                VertexData {
                    id: vertex_id,
                    label: node.label.clone(),
                    properties: props,
                },
            );
            vertex_id_map.insert(&node.id, vertex_id);
        }

        // Convert Edges
        for edge in &graph_instance.core.edges {
            if let (Some(from_id), Some(to_id)) = (
                vertex_id_map.get(&edge.source),
                vertex_id_map.get(&edge.target),
            ) {
                let edge_id = generate_edge_cid(&v1, &v2, &"default".to_string(), &HashMap::new()); // Deterministically generated

                let props = if let Some(attrs) = &edge.attrs {
                    attrs.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>()
                } else {
                    HashMap::new()
                };

                graph.edges.insert(
                    edge_id,
                    EdgeData {
                        id: edge_id,
                        source: *from_id,
                        target: *to_id,
                        label: edge.label.clone(),
                        properties: props,
                    },
                );

                graph.adj_out.entry(*from_id).or_insert_with(HashSet::new).insert(*to_id);
                graph.adj_in.entry(*to_id).or_insert_with(HashSet::new).insert(*from_id);
            }
        }

        Ok(graph)
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

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStatistics {
    /// Number of vertices
    pub vertex_count: usize,
    /// Number of edges
    pub edge_count: usize,
    /// Average degree
    pub average_degree: f64,
    /// Maximum degree
    pub max_degree: usize,
    /// Graph density
    pub density: f64,
}

impl GraphStatistics {
    /// Check if graph is dense (density > 0.5)
    pub fn is_dense(&self) -> bool {
        self.density > 0.5
    }

    /// Check if graph is sparse (density < 0.1)
    pub fn is_sparse(&self) -> bool {
        self.density < 0.1
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
            let vertex_id = generate_vertex_cid(&[], &HashMap::new()); // Deterministically generated from CID

            let props = if let Some(attrs) = &node.attrs {
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<_, _>>()
            } else {
                HashMap::new()
            };

            graph.vertices.insert(
                vertex_id,
                VertexData {
                    id: vertex_id,
                    label: node.label.clone(),
                    properties: props,
                },
            );
            vertex_id_map.insert(&node.id, vertex_id);
        }

        // Convert Edges
        for edge in &graph_instance.core.edges {
            if let (Some(from_id), Some(to_id)) = (
                vertex_id_map.get(&edge.source),
                vertex_id_map.get(&edge.target),
            ) {
                let edge_id = generate_edge_cid(&v1, &v2, &"default".to_string(), &HashMap::new()); // Deterministically generated

                let props = if let Some(attrs) = &edge.attrs {
                    attrs.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>()
                } else {
                    HashMap::new()
                };

                graph.edges.insert(
                    edge_id,
                    EdgeData {
                        id: edge_id,
                        source: *from_id,
                        target: *to_id,
                        label: edge.label.clone(),
                        properties: props,
                    },
                );

                graph.adj_out.entry(*from_id).or_insert_with(HashSet::new).insert(*to_id);
                graph.adj_in.entry(*to_id).or_insert_with(HashSet::new).insert(*from_id);
            }
        }

        Ok(graph)
    }

    /// Convert to GraphInstance with computed CID
    pub fn to_graph_instance_with_cid(&self) -> KotobaResult<GraphInstance> {
        let graph_instance = self.to_graph_instance(generate_cid("temp"));
        Ok(GraphInstance {
            cid: generate_cid("graph"),
            ..graph_instance
        })
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
            let vertex_id = generate_vertex_cid(&[], &HashMap::new()); // Deterministically generated from CID

            let props = if let Some(attrs) = &node.attrs {
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect::<HashMap<_, _>>()
            } else {
                HashMap::new()
            };

            graph.vertices.insert(
                vertex_id,
                VertexData {
                    id: vertex_id,
                    label: node.label.clone(),
                    properties: props,
                },
            );
            vertex_id_map.insert(&node.id, vertex_id);
        }

        // Convert Edges
        for edge in &graph_instance.core.edges {
            if let (Some(from_id), Some(to_id)) = (
                vertex_id_map.get(&edge.source),
                vertex_id_map.get(&edge.target),
            ) {
                let edge_id = generate_edge_cid(&v1, &v2, &"default".to_string(), &HashMap::new()); // Deterministically generated

                let props = if let Some(attrs) = &edge.attrs {
                    attrs.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>()
                } else {
                    HashMap::new()
                };

                graph.edges.insert(
                    edge_id,
                    EdgeData {
                        id: edge_id,
                        source: *from_id,
                        target: *to_id,
                        label: edge.label.clone(),
                        properties: props,
                    },
                );

                graph.adj_out.entry(*from_id).or_insert_with(HashSet::new).insert(*to_id);
                graph.adj_in.entry(*to_id).or_insert_with(HashSet::new).insert(*from_id);
            }
        }

        Ok(graph)
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
            let vertex_id = generate_vertex_cid(&[], &HashMap::new()); // Deterministically generated from CID

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

        let vertex = VertexData::new(generate_vertex_cid(&[], &HashMap::new()))
            .with_label("Person".to_string())
            .with_property("name".to_string(), Value::String("Alice".to_string()));

        let vertex_id = graph.add_vertex(vertex);
        assert!(graph.contains_vertex(&vertex_id));
        assert_eq!(graph.vertex_count(), 1);
    }

    #[test]
    fn test_edge_operations() {
        let mut graph = Graph::empty();

        let v1 = graph.add_vertex(VertexData::new(generate_vertex_cid(&[], &HashMap::new())));
        let v2 = graph.add_vertex(VertexData::new(generate_vertex_cid(&[], &HashMap::new())));

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

        let v1 = graph.add_vertex(VertexData::new(generate_vertex_cid(&[], &HashMap::new())));
        let v2 = graph.add_vertex(VertexData::new(generate_vertex_cid(&[], &HashMap::new())));

        graph.add_edge(EdgeData::new(v1, v2, "knows".to_string()));

        assert_eq!(graph.vertex_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        graph.remove_vertex(&v1);

        assert_eq!(graph.vertex_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }
}

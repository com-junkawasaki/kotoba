//! # Core Graph Implementation
//!
//! This module provides the core graph implementation with canonicalization
//! and merkle tree support for content-addressed graphs.

use super::*;
use kotoba_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use parking_lot::RwLock;

/// Content-addressed graph with canonicalization and merkle support
#[derive(Debug, Clone)]
pub struct Graph {
    /// Inner graph data
    pub inner: GraphData,
    /// Canonicalization result
    pub canonicalization: Option<CanonicalizationResult>,
    /// Merkle tree
    pub merkle_tree: Option<MerkleTree>,
    /// Graph reference
    pub graph_ref: Option<GraphRef>,
}

impl Graph {
    /// Create a new empty graph
    pub fn new() -> Self {
        Self {
            inner: GraphData::empty(),
            canonicalization: None,
            merkle_tree: None,
            graph_ref: None,
        }
    }

    /// Create from inner data
    pub fn from_data(data: GraphData) -> Self {
        Self {
            inner: data,
            canonicalization: None,
            merkle_tree: None,
            graph_ref: None,
        }
    }

    /// Add a vertex
    pub fn add_vertex(&mut self, vertex: VertexData) -> VertexId {
        let id = self.inner.add_vertex(vertex);
        self.invalidate_canonicalization();
        id
    }

    /// Add an edge
    pub fn add_edge(&mut self, edge: EdgeData) -> EdgeId {
        let id = self.inner.add_edge(edge);
        self.invalidate_canonicalization();
        id
    }

    /// Get a vertex
    pub fn get_vertex(&self, id: &VertexId) -> Option<&VertexData> {
        self.inner.get_vertex(id)
    }

    /// Get an edge
    pub fn get_edge(&self, id: &EdgeId) -> Option<&EdgeData> {
        self.inner.get_edge(id)
    }

    /// Remove a vertex
    pub fn remove_vertex(&mut self, id: &VertexId) -> bool {
        let removed = self.inner.remove_vertex(id);
        if removed {
            self.invalidate_canonicalization();
        }
        removed
    }

    /// Remove an edge
    pub fn remove_edge(&mut self, id: &EdgeId) -> bool {
        let removed = self.inner.remove_edge(id);
        if removed {
            self.invalidate_canonicalization();
        }
        removed
    }

    /// Canonicalize the graph
    pub fn canonicalize(&mut self, canonicalizer: &GraphCanonicalizer) {
        if self.canonicalization.is_none() {
            self.canonicalization = Some(canonicalizer.canonicalize(&self.inner));
        }
    }

    /// Build merkle tree
    pub fn build_merkle_tree(&mut self, builder: &MerkleTreeBuilder) {
        if let Some(ref canonicalization) = self.canonicalization {
            if self.merkle_tree.is_none() {
                let chunks = builder.split_graph_into_chunks(&self.inner);
                self.merkle_tree = Some(MerkleTree::new(chunks));
            }
        }
    }

    /// Get or compute graph reference
    pub fn graph_ref(&mut self) -> &GraphRef {
        if self.graph_ref.is_none() {
            if let Some(ref canonicalization) = self.canonicalization {
                self.graph_ref = Some(GraphRef::new(
                    canonicalization.hash.clone(),
                    canonicalization.canonical_graph.clone(),
                ));
            } else {
                // Fallback: compute from inner graph
                let hash = Hash::from_sha256(&serde_json::to_vec(&self.inner).unwrap());
                self.graph_ref = Some(GraphRef::new(hash, Vec::new()));
            }
        }
        self.graph_ref.as_ref().unwrap()
    }

    /// Invalidate cached canonicalization and merkle tree
    pub fn invalidate_canonicalization(&mut self) {
        self.canonicalization = None;
        self.merkle_tree = None;
        self.graph_ref = None;
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.inner.vertex_count()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.inner.edge_count()
    }

    /// Get degree of a vertex
    pub fn degree(&self, vertex_id: &VertexId) -> usize {
        self.inner.degree(vertex_id)
    }

    /// Get adjacency list
    pub fn adjacency(&self) -> &HashMap<VertexId, HashSet<VertexId>> {
        &self.inner.adj_out
    }

    /// Convert to inner graph
    pub fn into_inner(self) -> GraphData {
        self.inner
    }

    /// Get inner graph reference
    pub fn inner(&self) -> &GraphData {
        &self.inner
    }

    /// Get inner graph mutable reference
    pub fn inner_mut(&mut self) -> &mut GraphData {
        &mut self.inner
    }
}

/// Inner graph data (same as original)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphData {
    /// 頂点データ（ID→データ）
    pub vertices: HashMap<VertexId, VertexData>,

    /// エッジデータ（ID→データ）
    pub edges: HashMap<EdgeId, EdgeData>,

    /// 外向き隣接リスト（src→[dst])
    pub adj_out: HashMap<VertexId, HashSet<VertexId>>,

    /// 内向き隣接リスト（dst→[src])
    pub adj_in: HashMap<VertexId, HashSet<VertexId>>,

    /// ラベル別頂点インデックス（ラベル→[頂点ID]）
    pub vertex_labels: HashMap<Label, HashSet<VertexId>>,

    /// ラベル別エッジインデックス（ラベル→[エッジID]）
    pub edge_labels: HashMap<Label, HashSet<EdgeId>>,
}

impl GraphData {
    /// 空のグラフを作成
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

    /// 頂点を追加
    pub fn add_vertex(&mut self, vertex: VertexData) -> VertexId {
        let id = vertex.id;
        for label in &vertex.labels {
            self.vertex_labels.entry(label.clone()).or_insert(HashSet::new()).insert(id);
        }
        self.vertices.insert(id, vertex);
        id
    }

    /// エッジを追加
    pub fn add_edge(&mut self, edge: EdgeData) -> EdgeId {
        let id = edge.id;
        let src = edge.src;
        let dst = edge.dst;

        // 隣接リスト更新
        self.adj_out.entry(src).or_insert(HashSet::new()).insert(dst);
        self.adj_in.entry(dst).or_insert(HashSet::new()).insert(src);

        // ラベルインデックス更新
        self.edge_labels.entry(edge.label.clone()).or_insert(HashSet::new()).insert(id);

        self.edges.insert(id, edge);
        id
    }

    /// 頂点を取得
    pub fn get_vertex(&self, id: &VertexId) -> Option<&VertexData> {
        self.vertices.get(id)
    }

    /// エッジを取得
    pub fn get_edge(&self, id: &EdgeId) -> Option<&EdgeData> {
        self.edges.get(id)
    }

    /// 頂点を削除
    pub fn remove_vertex(&mut self, id: &VertexId) -> bool {
        if let Some(vertex) = self.vertices.remove(id) {
            // ラベルインデックスから削除
            for label in &vertex.labels {
                if let Some(vertices) = self.vertex_labels.get_mut(label) {
                    vertices.remove(id);
                    if vertices.is_empty() {
                        self.vertex_labels.remove(label);
                    }
                }
            }

            // 関連エッジを削除
            let mut edges_to_remove = Vec::new();
            for (edge_id, edge) in &self.edges {
                if edge.src == *id || edge.dst == *id {
                    edges_to_remove.push(*edge_id);
                }
            }
            for edge_id in edges_to_remove {
                self.remove_edge(&edge_id);
            }

            // 隣接リストから削除
            self.adj_out.remove(id);
            self.adj_in.remove(id);

            // 他の頂点の隣接リストから削除
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

    /// エッジを削除
    pub fn remove_edge(&mut self, id: &EdgeId) -> bool {
        if let Some(edge) = self.edges.remove(id) {
            // 隣接リスト更新
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

            // ラベルインデックス更新
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

    /// ラベル別頂点を取得
    pub fn vertices_by_label(&self, label: &Label) -> HashSet<VertexId> {
        self.vertex_labels.get(label).cloned().unwrap_or_default()
    }

    /// ラベル別エッジを取得
    pub fn edges_by_label(&self, label: &Label) -> HashSet<EdgeId> {
        self.edge_labels.get(label).cloned().unwrap_or_default()
    }

    /// 頂点の次数を取得
    pub fn degree(&self, id: &VertexId) -> usize {
        let out_degree = self.adj_out.get(id).map(|s| s.len()).unwrap_or(0);
        let in_degree = self.adj_in.get(id).map(|s| s.len()).unwrap_or(0);
        out_degree + in_degree
    }

    /// 頂点数を取得
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// エッジ数を取得
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Thread-safe graph reference
#[derive(Debug, Clone)]
pub struct GraphRef {
    inner: std::sync::Arc<RwLock<Graph>>,
}

impl GraphRef {
    /// Create a new graph reference
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: std::sync::Arc::new(RwLock::new(graph)),
        }
    }

    /// Create from graph data
    pub fn from_data(data: GraphData) -> Self {
        Self::new(Graph::from_data(data))
    }

    /// Read access
    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, Graph> {
        self.inner.read()
    }

    /// Write access
    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, Graph> {
        self.inner.write()
    }

    /// Canonicalize the graph
    pub fn canonicalize(&self, canonicalizer: &GraphCanonicalizer) {
        let mut graph = self.write();
        graph.canonicalize(canonicalizer);
    }

    /// Build merkle tree
    pub fn build_merkle_tree(&self, builder: &MerkleTreeBuilder) {
        let mut graph = self.write();
        graph.build_merkle_tree(builder);
    }

    /// Get graph reference
    pub fn graph_ref(&self) -> GraphRef {
        let graph = self.read();
        graph.graph_ref().clone()
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.read().vertex_count()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.read().edge_count()
    }

    /// Add vertex
    pub fn add_vertex(&self, vertex: VertexData) -> VertexId {
        let mut graph = self.write();
        graph.add_vertex(vertex)
    }

    /// Add edge
    pub fn add_edge(&self, edge: EdgeData) -> EdgeId {
        let mut graph = self.write();
        graph.add_edge(edge)
    }

    /// Get vertex
    pub fn get_vertex(&self, id: &VertexId) -> Option<VertexData> {
        self.read().get_vertex(id).cloned()
    }

    /// Get edge
    pub fn get_edge(&self, id: &EdgeId) -> Option<EdgeData> {
        self.read().get_edge(id).cloned()
    }
}

/// Graph builder for constructing graphs
#[derive(Debug, Clone)]
pub struct GraphBuilder {
    graph: Graph,
}

impl GraphBuilder {
    /// Create a new graph builder
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    /// Add vertex
    pub fn add_vertex(mut self, vertex: VertexData) -> Self {
        self.graph.add_vertex(vertex);
        self
    }

    /// Add edge
    pub fn add_edge(mut self, edge: EdgeData) -> Self {
        self.graph.add_edge(edge);
        self
    }

    /// Build the graph
    pub fn build(self) -> Graph {
        self.graph
    }
}

/// Graph operations
impl Graph {
    /// Compute graph hash
    pub fn hash(&self) -> Hash {
        if let Some(ref canonicalization) = self.canonicalization {
            canonicalization.hash.clone()
        } else {
            Hash::from_sha256(&serde_json::to_vec(&self.inner).unwrap())
        }
    }

    /// Check graph isomorphism
    pub fn is_isomorphic(&self, other: &Graph, checker: &GraphIsomorphismChecker) -> bool {
        checker.are_isomorphic(&self.inner, &other.inner)
    }

    /// Get canonical form
    pub fn canonical_form(&self) -> Option<&CanonicalizationResult> {
        self.canonicalization.as_ref()
    }

    /// Get merkle tree
    pub fn merkle_tree(&self) -> Option<&MerkleTree> {
        self.merkle_tree.as_ref()
    }

    /// Create subgraph
    pub fn subgraph(&self, vertices: &HashSet<VertexId>) -> Graph {
        let mut subgraph = Graph::new();
        let mut subgraph_data = GraphData::empty();

        // Add selected vertices
        for vertex_id in vertices {
            if let Some(vertex) = self.get_vertex(vertex_id) {
                subgraph_data.add_vertex(vertex.clone());
            }
        }

        // Add edges between selected vertices
        for (edge_id, edge) in &self.inner.edges {
            if vertices.contains(&edge.src) && vertices.contains(&edge.dst) {
                subgraph_data.add_edge(edge.clone());
            }
        }

        subgraph.inner = subgraph_data;
        subgraph
    }

    /// Get connected components
    pub fn connected_components(&self) -> Vec<Graph> {
        let mut visited = HashSet::new();
        let mut components = Vec::new();

        for (vertex_id, _) in &self.inner.vertices {
            if !visited.contains(vertex_id) {
                let component_vertices = self.dfs_collect(vertex_id, &mut visited);
                let component = self.subgraph(&component_vertices);
                components.push(component);
            }
        }

        components
    }

    /// DFS to collect connected component
    fn dfs_collect(&self, start: &VertexId, visited: &mut HashSet<VertexId>) -> HashSet<VertexId> {
        let mut component = HashSet::new();
        let mut stack = vec![*start];

        while let Some(vertex) = stack.pop() {
            if !visited.contains(&vertex) {
                visited.insert(vertex);
                component.insert(vertex);

                // Add neighbors
                if let Some(neighbors) = self.inner.adj_out.get(&vertex) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            stack.push(*neighbor);
                        }
                    }
                }
            }
        }

        component
    }
}

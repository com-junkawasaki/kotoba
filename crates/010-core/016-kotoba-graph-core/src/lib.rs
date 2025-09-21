//! # Kotoba Graph Core
//!
//! Incidence bipartite graph + canonicalization + merkle for content-addressed graphs.
//!
//! This crate provides the foundation for content-addressed graphs using
//! incidence bipartite representation with canonicalization and merkle hashing.

pub mod incidence;
pub mod canonical;
pub mod merkle;
pub mod graph;

use kotoba_types::*;
use kotoba_codebase::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Graph ID type for content addressing
pub type GraphId = Hash;

/// Content-addressed graph reference
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GraphRef {
    /// Content hash of the graph
    pub hash: Hash,
    /// Canonical form of the graph
    pub canonical_form: Vec<u8>,
}

impl GraphRef {
    /// Create a new graph reference
    pub fn new(hash: Hash, canonical_form: Vec<u8>) -> Self {
        Self { hash, canonical_form }
    }

    /// Create from graph content
    pub fn from_graph<T: AsRef<[u8]>>(content: T) -> Self {
        let hash = Hash::from_sha256(content.as_ref());
        let canonical_form = content.as_ref().to_vec();
        Self::new(hash, canonical_form)
    }
}

/// Graph canonicalization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizationResult {
    /// Canonical graph representation
    pub canonical_graph: Vec<u8>,
    /// Hash of the canonical form
    pub hash: Hash,
    /// Isomorphism class ID
    pub isomorphism_class: String,
    /// Canonical ordering of nodes
    pub node_ordering: Vec<usize>,
    /// Canonical ordering of edges
    pub edge_ordering: Vec<usize>,
}

/// Merkle tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash of this node
    pub hash: Hash,
    /// Left child
    pub left: Option<Box<MerkleNode>>,
    /// Right child
    pub right: Option<Box<MerkleNode>>,
    /// Data at this node (for leaf nodes)
    pub data: Option<Vec<u8>>,
}

/// Merkle tree for graph integrity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleTree {
    /// Root node
    pub root: Option<MerkleNode>,
    /// Height of the tree
    pub height: usize,
    /// Number of leaves
    pub leaf_count: usize,
}

impl MerkleTree {
    /// Create a new merkle tree from data
    pub fn new(data: Vec<Vec<u8>>) -> Self {
        let leaf_count = data.len();
        let height = if leaf_count == 0 { 0 } else { (leaf_count as f64).log2().ceil() as usize + 1 };

        let mut tree = Self {
            root: None,
            height,
            leaf_count,
        };

        tree.root = Some(tree.build_tree(data, 0, leaf_count, 0));
        tree
    }

    /// Build the merkle tree recursively
    fn build_tree(&self, data: Vec<Vec<u8>>, start: usize, end: usize, level: usize) -> MerkleNode {
        if start == end {
            // Leaf node
            let hash = Hash::from_sha256(&data[start]);
            MerkleNode {
                hash,
                left: None,
                right: None,
                data: Some(data[start].clone()),
            }
        } else if start + 1 == end {
            // Single node at this level
            let hash = Hash::from_sha256(&data[start]);
            MerkleNode {
                hash,
                left: None,
                right: None,
                data: Some(data[start].clone()),
            }
        } else {
            // Internal node
            let mid = (start + end) / 2;
            let left = self.build_tree(data.clone(), start, mid, level + 1);
            let right = self.build_tree(data, mid, end, level + 1);

            let mut combined = Vec::new();
            combined.extend_from_slice(&left.hash.0);
            combined.extend_from_slice(&right.hash.0);

            let hash = Hash::from_sha256(&combined);

            MerkleNode {
                hash,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
                data: None,
            }
        }
    }

    /// Get root hash
    pub fn root_hash(&self) -> Option<Hash> {
        self.root.as_ref().map(|node| node.hash.clone())
    }

    /// Verify merkle proof
    pub fn verify_proof(&self, proof: &[Hash], leaf_index: usize, leaf_hash: Hash) -> bool {
        if let Some(root) = &self.root {
            self.verify_proof_recursive(root, proof, 0, leaf_index, leaf_hash)
        } else {
            false
        }
    }

    /// Verify merkle proof recursively
    fn verify_proof_recursive(
        &self,
        node: &MerkleNode,
        proof: &[Hash],
        proof_index: usize,
        leaf_index: usize,
        leaf_hash: Hash,
    ) -> bool {
        if proof_index >= proof.len() {
            return node.hash == leaf_hash;
        }

        if let (Some(left), Some(right)) = (&node.left, &node.right) {
            let mid = self.leaf_count / 2;
            let (expected_hash, next_proof_index) = if leaf_index < mid {
                // Left subtree
                let computed_hash = Hash::from_sha256({
                    let mut combined = Vec::new();
                    combined.extend_from_slice(&left.hash.0);
                    combined.extend_from_slice(&proof[proof_index].0);
                    combined
                });
                (computed_hash, proof_index + 1)
            } else {
                // Right subtree
                let computed_hash = Hash::from_sha256({
                    let mut combined = Vec::new();
                    combined.extend_from_slice(&proof[proof_index].0);
                    combined.extend_from_slice(&right.hash.0);
                    combined
                });
                (computed_hash, proof_index + 1)
            };

            self.verify_proof_recursive(&expected_hash.to_string(), proof, next_proof_index, leaf_index, leaf_hash)
        } else {
            false
        }
    }
}

/// Graph canonicalizer for computing canonical forms
#[derive(Debug, Clone)]
pub struct GraphCanonicalizer {
    /// Canonicalization configuration
    pub config: CanonicalizationConfig,
}

impl GraphCanonicalizer {
    /// Create a new graph canonicalizer
    pub fn new() -> Self {
        Self {
            config: CanonicalizationConfig::default(),
        }
    }

    /// Canonicalize a graph
    pub fn canonicalize(&self, graph: &Graph) -> CanonicalizationResult {
        // 1. Convert to incidence bipartite representation
        let incidence_graph = self.build_incidence_graph(graph);

        // 2. Compute canonical ordering
        let (node_ordering, edge_ordering) = self.compute_canonical_ordering(&incidence_graph);

        // 3. Create canonical graph
        let canonical_graph = self.create_canonical_graph(graph, &node_ordering, &edge_ordering);

        // 4. Compute hash
        let hash = Hash::from_sha256(&canonical_graph);

        CanonicalizationResult {
            canonical_graph,
            hash,
            isomorphism_class: format!("iso_{}", hash),
            node_ordering,
            edge_ordering,
        }
    }

    /// Build incidence bipartite graph
    fn build_incidence_graph(&self, _graph: &Graph) -> IncidenceGraph {
        // Implementation would create incidence bipartite representation
        IncidenceGraph::default()
    }

    /// Compute canonical ordering
    fn compute_canonical_ordering(&self, _incidence_graph: &IncidenceGraph) -> (Vec<usize>, Vec<usize>) {
        // Implementation would use canonical labeling algorithm
        (Vec::new(), Vec::new())
    }

    /// Create canonical graph
    fn create_canonical_graph(
        &self,
        _graph: &Graph,
        _node_ordering: &[usize],
        _edge_ordering: &[usize],
    ) -> Vec<u8> {
        // Implementation would reorder graph according to canonical ordering
        Vec::new()
    }
}

/// Canonicalization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalizationConfig {
    /// Algorithm to use for canonicalization
    pub algorithm: CanonicalizationAlgorithm,
    /// Maximum graph size for canonicalization
    pub max_size: Option<usize>,
    /// Enable optimizations
    pub enable_optimizations: bool,
}

impl Default for CanonicalizationConfig {
    fn default() -> Self {
        Self {
            algorithm: CanonicalizationAlgorithm::Bliss,
            max_size: Some(10000),
            enable_optimizations: true,
        }
    }
}

/// Canonicalization algorithm
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CanonicalizationAlgorithm {
    /// Bliss canonical labeling
    Bliss,
    /// Nauty canonical labeling
    Nauty,
    /// Custom algorithm
    Custom(String),
}

/// Incidence bipartite graph representation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncidenceGraph {
    /// Left vertices (entities)
    pub left_vertices: Vec<IncidenceVertex>,
    /// Right vertices (attributes/relations)
    pub right_vertices: Vec<IncidenceVertex>,
    /// Edges between left and right
    pub edges: Vec<IncidenceEdge>,
}

impl IncidenceGraph {
    /// Create a new incidence graph
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a left vertex
    pub fn add_left_vertex(&mut self, vertex: IncidenceVertex) {
        self.left_vertices.push(vertex);
    }

    /// Add a right vertex
    pub fn add_right_vertex(&mut self, vertex: IncidenceVertex) {
        self.right_vertices.push(vertex);
    }

    /// Add an edge
    pub fn add_edge(&mut self, edge: IncidenceEdge) {
        self.edges.push(edge);
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
}

/// Incidence edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidenceEdge {
    /// Source vertex (left)
    pub source: String,
    /// Target vertex (right)
    pub target: String,
    /// Edge type
    pub edge_type: String,
    /// Properties
    pub properties: HashMap<String, Value>,
}

/// Graph processor for processing graphs with canonicalization and merkle
#[derive(Debug, Clone)]
pub struct GraphProcessor {
    /// Canonicalizer
    pub canonicalizer: GraphCanonicalizer,
    /// Merkle tree builder
    pub merkle_builder: MerkleTreeBuilder,
}

impl GraphProcessor {
    /// Create a new graph processor
    pub fn new() -> Self {
        Self {
            canonicalizer: GraphCanonicalizer::new(),
            merkle_builder: MerkleTreeBuilder::new(),
        }
    }

    /// Process a graph: canonicalize and compute merkle tree
    pub fn process_graph(&self, graph: &Graph) -> GraphProcessingResult {
        // Canonicalize the graph
        let canonicalization = self.canonicalizer.canonicalize(graph);

        // Build merkle tree from canonical form
        let merkle_tree = self.merkle_builder.build_tree_from_graph(&canonicalization);

        GraphProcessingResult {
            canonicalization,
            merkle_tree,
            graph_ref: GraphRef::new(
                canonicalization.hash.clone(),
                canonicalization.canonical_graph.clone(),
            ),
        }
    }
}

/// Merkle tree builder
#[derive(Debug, Clone)]
pub struct MerkleTreeBuilder {
    /// Configuration
    pub config: MerkleConfig,
}

impl MerkleTreeBuilder {
    /// Create a new merkle tree builder
    pub fn new() -> Self {
        Self {
            config: MerkleConfig::default(),
        }
    }

    /// Build merkle tree from canonical graph
    pub fn build_tree_from_graph(&self, canonicalization: &CanonicalizationResult) -> MerkleTree {
        // Split canonical graph into chunks for merkle tree
        let chunks = self.split_into_chunks(&canonicalization.canonical_graph);
        MerkleTree::new(chunks)
    }

    /// Split data into chunks for merkle tree
    fn split_into_chunks(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let chunk_size = self.config.chunk_size;
        data.chunks(chunk_size)
            .map(|chunk| chunk.to_vec())
            .collect()
    }
}

/// Merkle configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleConfig {
    /// Chunk size for merkle tree
    pub chunk_size: usize,
    /// Hash algorithm
    pub hash_algorithm: HashAlgorithm,
}

impl Default for MerkleConfig {
    fn default() -> Self {
        Self {
            chunk_size: 1024,
            hash_algorithm: HashAlgorithm::Sha256,
        }
    }
}

/// Hash algorithm for merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HashAlgorithm {
    /// SHA256
    Sha256,
    /// Blake3
    Blake3,
}

/// Graph processing result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphProcessingResult {
    /// Canonicalization result
    pub canonicalization: CanonicalizationResult,
    /// Merkle tree
    pub merkle_tree: MerkleTree,
    /// Graph reference
    pub graph_ref: GraphRef,
}

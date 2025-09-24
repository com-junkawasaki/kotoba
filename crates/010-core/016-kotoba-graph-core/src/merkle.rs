//! # Merkle Tree Implementation for Graph Integrity
//!
//! This module provides merkle tree implementation for verifying
//! graph integrity and enabling efficient proof generation.

use super::*;
use crate::graph::Graph;
use kotoba_types::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

impl MerkleTree {
    /// Create a new merkle tree from leaves
    pub fn from_leaves(leaves: Vec<Hash>) -> Self {
        let leaf_count = leaves.len();
        let height = if leaf_count == 0 {
            0
        } else {
            (leaf_count as f64).log2().ceil() as usize + 1
        };

        let mut tree = Self {
            root: None,
            height,
            leaf_count,
        };

        if !leaves.is_empty() {
            let leaf_nodes = leaves.into_iter()
                .map(|hash| MerkleNode {
                    hash,
                    left: None,
                    right: None,
                    data: None,
                })
                .collect();

            tree.root = Some(tree.build_tree_from_nodes(leaf_nodes));
        }

        tree
    }

    /// Build tree from node vector
    fn build_tree_from_nodes(&self, mut nodes: Vec<MerkleNode>) -> MerkleNode {
        while nodes.len() > 1 {
            let mut new_level = Vec::new();

            for i in (0..nodes.len()).step_by(2) {
                if i + 1 < nodes.len() {
                    // Internal node
                    let left = nodes[i].clone();
                    let right = nodes[i + 1].clone();

                    let mut combined = Vec::new();
                    combined.extend_from_slice(&left.hash.0);
                    combined.extend_from_slice(&right.hash.0);

                    let hash = Hash::from_sha256(&combined);

                    new_level.push(MerkleNode {
                        hash,
                        left: Some(Box::new(left)),
                        right: Some(Box::new(right)),
                        data: None,
                    });
                } else {
                    // Odd number of nodes, promote the last one
                    new_level.push(nodes[i].clone());
                }
            }

            nodes = new_level;
        }

        nodes.into_iter().next().unwrap_or_else(|| {
            MerkleNode {
                hash: Hash::from_sha256(&[]),
                left: None,
                right: None,
                data: None,
            }
        })
    }

    /// Get root hash
    pub fn root_hash(&self) -> Option<Hash> {
        self.root.as_ref().map(|node| node.hash.clone())
    }

    /// Generate merkle proof for a leaf
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaf_count {
            return None;
        }

        let mut proof = Vec::new();
        let mut current_index = leaf_index;

        // Traverse from leaf to root
        let mut current_level = 0;
        let mut node = &self.root;

        while let Some(n) = node {
            if current_level == 0 {
                // At leaf level, add sibling
                if let (Some(left), Some(right)) = (&n.left, &n.right) {
                    if current_index % 2 == 0 {
                        proof.push(right.hash.clone());
                    } else {
                        proof.push(left.hash.clone());
                    }
                }
            } else {
                // At internal levels
                if current_index % 2 == 0 {
                    if let Some(right) = &n.right {
                        proof.push(right.hash.clone());
                    }
                } else {
                    if let Some(left) = &n.left {
                        proof.push(left.hash.clone());
                    }
                }
            }

            // Move to parent level
            current_index /= 2;
            current_level += 1;

            if current_index == 0 && current_level == 1 {
                break;
            }

            // In a real implementation, we'd need to traverse the tree properly
            // This is a simplified version
            break;
        }

        Some(MerkleProof {
            leaf_index,
            proof_hashes: proof,
            root_hash: self.root_hash().unwrap_or_default(),
        })
    }

    /// Verify a merkle proof
    pub fn verify_proof(&self, proof: &MerkleProof, leaf_hash: Hash) -> bool {
        if proof.leaf_index >= self.leaf_count {
            return false;
        }

        let computed_root = self.compute_root_from_proof(proof, leaf_hash);
        computed_root == proof.root_hash
    }

    /// Compute root hash from proof
    fn compute_root_from_proof(&self, proof: &MerkleProof, leaf_hash: Hash) -> Hash {
        let mut current_hash = leaf_hash;
        let mut proof_index = 0;

        // Reconstruct the path from leaf to root
        for level in 0..self.height {
            if proof_index >= proof.proof_hashes.len() {
                break;
            }

            let sibling_hash = &proof.proof_hashes[proof_index];
            proof_index += 1;

            // Determine if we're left or right child
            let is_left = (proof.leaf_index >> level) % 2 == 0;

            let combined = if is_left {
                let mut data = Vec::new();
                data.extend_from_slice(&current_hash.0);
                data.extend_from_slice(&sibling_hash.0);
                data
            } else {
                let mut data = Vec::new();
                data.extend_from_slice(&sibling_hash.0);
                data.extend_from_slice(&current_hash.0);
                data
            };

            current_hash = Hash::from_sha256(&combined);
        }

        current_hash
    }

    /// Get tree height
    pub fn height(&self) -> usize {
        self.height
    }

    /// Get leaf count
    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }

    /// Get all leaf hashes
    pub fn leaf_hashes(&self) -> Vec<Hash> {
        if let Some(root) = &self.root {
            self.collect_leaves(root)
        } else {
            Vec::new()
        }
    }

    /// Collect all leaf hashes recursively
    fn collect_leaves(&self, node: &MerkleNode) -> Vec<Hash> {
        if node.left.is_none() && node.right.is_none() {
            vec![node.hash.clone()]
        } else {
            let mut leaves = Vec::new();
            if let Some(left) = &node.left {
                leaves.extend(self.collect_leaves(left));
            }
            if let Some(right) = &node.right {
                leaves.extend(self.collect_leaves(right));
            }
            leaves
        }
    }
}

/// Merkle proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Index of the leaf being proven
    pub leaf_index: usize,
    /// Hashes needed for verification
    pub proof_hashes: Vec<Hash>,
    /// Root hash of the tree
    pub root_hash: Hash,
}

impl MerkleProof {
    /// Create a new merkle proof
    pub fn new(leaf_index: usize, proof_hashes: Vec<Hash>, root_hash: Hash) -> Self {
        Self {
            leaf_index,
            proof_hashes,
            root_hash,
        }
    }

    /// Verify the proof
    pub fn verify(&self, leaf_hash: Hash, root_hash: Hash) -> bool {
        let computed_root = self.compute_root(leaf_hash);
        computed_root == root_hash
    }

    /// Compute root from leaf and proof
    pub fn compute_root(&self, leaf_hash: Hash) -> Hash {
        let mut current_hash = leaf_hash;
        let mut proof_iter = self.proof_hashes.iter();

        for level in 0.. {
            if let Some(sibling_hash) = proof_iter.next() {
                let is_left = (self.leaf_index >> level) % 2 == 0;

                let combined = if is_left {
                    let mut data = Vec::new();
                    data.extend_from_slice(&current_hash.0);
                    data.extend_from_slice(&sibling_hash.0);
                    data
                } else {
                    let mut data = Vec::new();
                    data.extend_from_slice(&sibling_hash.0);
                    data.extend_from_slice(&current_hash.0);
                    data
                };

                current_hash = Hash::from_sha256(&combined);
            } else {
                break;
            }
        }

        current_hash
    }
}

/// Merkle tree builder with configuration
#[derive(Debug, Clone)]
pub struct MerkleTreeBuilder {
    /// Configuration
    pub config: MerkleConfig,
    /// Cache for computed trees
    pub tree_cache: HashMap<String, MerkleTree>,
}

impl MerkleTreeBuilder {
    /// Create a new merkle tree builder
    pub fn new(config: MerkleConfig) -> Self {
        Self {
            config,
            tree_cache: HashMap::new(),
        }
    }

    /// Build merkle tree from graph data
    pub fn build_from_graph(&self, graph: &Graph) -> MerkleTree {
        let cache_key = self.compute_graph_cache_key(graph);

        if let Some(cached_tree) = self.tree_cache.get(&cache_key) {
            return cached_tree.clone();
        }

        let chunks = self.split_graph_into_chunks(graph);
        let tree = MerkleTree::new(chunks);

        self.tree_cache.insert(cache_key, tree.clone());
        tree
    }

    /// Split graph into chunks for merkle tree
    fn split_graph_into_chunks(&self, graph: &Graph) -> Vec<Vec<u8>> {
        let mut chunks = Vec::new();

        // Add vertices chunk
        let vertex_data: Vec<_> = graph.vertices.values().collect();
        let vertex_chunk = serde_json::to_vec(&vertex_data).unwrap();
        chunks.push(vertex_chunk);

        // Add edges chunk
        let edge_data: Vec<_> = graph.edges.values().collect();
        let edge_chunk = serde_json::to_vec(&edge_data).unwrap();
        chunks.push(edge_chunk);

        // Add adjacency chunks
        let adj_out_chunk = serde_json::to_vec(&graph.adj_out).unwrap();
        chunks.push(adj_out_chunk);

        let adj_in_chunk = serde_json::to_vec(&graph.adj_in).unwrap();
        chunks.push(adj_in_chunk);

        // Add index chunks
        let vertex_labels_chunk = serde_json::to_vec(&graph.vertex_labels).unwrap();
        chunks.push(vertex_labels_chunk);

        let edge_labels_chunk = serde_json::to_vec(&graph.edge_labels).unwrap();
        chunks.push(edge_labels_chunk);

        chunks
    }

    /// Compute cache key for graph
    fn compute_graph_cache_key(&self, graph: &Graph) -> String {
        let mut key_data = Vec::new();
        key_data.extend_from_slice(&graph.vertex_count().to_be_bytes());
        key_data.extend_from_slice(&graph.edge_count().to_be_bytes());

        // Add hashes of all components
        let vertex_hash = Hash::from_sha256(&serde_json::to_vec(&graph.vertices).unwrap());
        let edge_hash = Hash::from_sha256(&serde_json::to_vec(&graph.edges).unwrap());
        let adj_out_hash = Hash::from_sha256(&serde_json::to_vec(&graph.adj_out).unwrap());
        let adj_in_hash = Hash::from_sha256(&serde_json::to_vec(&graph.adj_in).unwrap());

        key_data.extend_from_slice(&vertex_hash.0);
        key_data.extend_from_slice(&edge_hash.0);
        key_data.extend_from_slice(&adj_out_hash.0);
        key_data.extend_from_slice(&adj_in_hash.0);

        hex::encode(key_data)
    }

    /// Clear cache (returns new builder)
    pub fn with_cleared_cache(self) -> Self {
        let mut new_builder = self.clone();
        new_builder.tree_cache.clear();
        new_builder
    }
}

/// Incremental merkle tree for dynamic updates
#[derive(Debug, Clone)]
pub struct IncrementalMerkleTree {
    /// Current tree
    pub tree: MerkleTree,
    /// Leaf update tracker
    pub leaf_updates: HashMap<usize, Hash>,
    /// Dirty flag for root
    pub root_dirty: bool,
}

impl IncrementalMerkleTree {
    /// Create a new incremental merkle tree
    pub fn new(initial_leaves: Vec<Hash>) -> Self {
        let tree = MerkleTree::from_leaves(initial_leaves);

        Self {
            tree,
            leaf_updates: HashMap::new(),
            root_dirty: false,
        }
    }

    /// Update a leaf
    pub fn update_leaf(&mut self, leaf_index: usize, new_hash: Hash) {
        if leaf_index < self.tree.leaf_count {
            self.leaf_updates.insert(leaf_index, new_hash);
            self.root_dirty = true;
        }
    }

    /// Recompute root if dirty
    pub fn ensure_root_updated(&mut self) {
        if self.root_dirty {
            self.recompute_root();
            self.root_dirty = false;
        }
    }

    /// Recompute root from updates
    fn recompute_root(&mut self) {
        // Implementation would recompute only the affected parts of the tree
        // For now, rebuild the entire tree
        let mut leaves = Vec::new();

        for i in 0..self.tree.leaf_count {
            if let Some(updated_hash) = self.leaf_updates.get(&i) {
                leaves.push(updated_hash.clone());
            } else {
                // Get original leaf hash
                leaves.push(Hash::from_sha256(&[])); // Placeholder
            }
        }

        self.tree = MerkleTree::from_leaves(leaves);
        self.leaf_updates.clear();
    }

    /// Get current root hash
    pub fn root_hash(&mut self) -> Hash {
        self.ensure_root_updated();
        self.tree.root_hash().unwrap_or_default()
    }
}

/// Merkle tree path for verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePath {
    /// Leaf index
    pub leaf_index: usize,
    /// Path from leaf to root
    pub path: Vec<MerklePathNode>,
}

impl MerklePath {
    /// Create a new merkle path
    pub fn new(leaf_index: usize, path: Vec<MerklePathNode>) -> Self {
        Self { leaf_index, path }
    }

    /// Verify the path
    pub fn verify(&self, leaf_hash: Hash, root_hash: Hash) -> bool {
        let computed_hash = self.compute_hash(leaf_hash);
        computed_hash == root_hash
    }

    /// Compute hash along the path
    fn compute_hash(&self, leaf_hash: Hash) -> Hash {
        let mut current_hash = leaf_hash;

        for node in &self.path {
            let combined = match node.side {
                PathSide::Left => {
                    let mut data = Vec::new();
                    data.extend_from_slice(&current_hash.0);
                    data.extend_from_slice(&node.sibling_hash.0);
                    data
                },
                PathSide::Right => {
                    let mut data = Vec::new();
                    data.extend_from_slice(&node.sibling_hash.0);
                    data.extend_from_slice(&current_hash.0);
                    data
                },
            };

            current_hash = Hash::from_sha256(&combined);
        }

        current_hash
    }
}

/// Merkle path node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerklePathNode {
    /// Sibling hash
    pub sibling_hash: Hash,
    /// Side of the path (left or right)
    pub side: PathSide,
}

/// Path side enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathSide {
    /// Left side
    Left,
    /// Right side
    Right,
}

/// Graph merkle tree for content-addressed graphs
#[derive(Debug, Clone)]
pub struct GraphMerkleTree {
    /// Base merkle tree
    pub tree: MerkleTree,
    /// Graph reference
    pub graph_ref: GraphRef,
    /// Chunk mapping
    pub chunk_mapping: HashMap<Hash, String>,
}

impl GraphMerkleTree {
    /// Create a new graph merkle tree
    pub fn new(graph: &Graph, canonicalization: &CanonicalizationResult) -> Self {
        let builder = MerkleTreeBuilder::new(MerkleConfig::default());
        let tree = builder.build_from_graph(graph);

        let graph_ref = GraphRef::new(
            canonicalization.hash.clone(),
            canonicalization.canonical_graph.clone(),
        );

        let mut chunk_mapping = HashMap::new();
        // Implementation would map chunks to graph components

        Self {
            tree,
            graph_ref,
            chunk_mapping,
        }
    }

    /// Verify graph integrity
    pub fn verify_integrity(&self, graph: &Graph) -> bool {
        let current_tree = MerkleTreeBuilder::new(MerkleConfig::default())
            .build_from_graph(graph);

        self.tree.root_hash() == current_tree.root_hash()
    }

    /// Get proof for a specific graph component
    pub fn get_component_proof(&self, component_id: &str) -> Option<MerkleProof> {
        // Implementation would find the component's leaf index and generate proof
        None
    }
}

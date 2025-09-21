//! # Composition DAG
//!
//! This module provides a directed acyclic graph representation of function compositions.

use super::*;
use std::collections::{HashMap, HashSet};

impl CompositionDAG {
    /// Create a new empty composition DAG
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            root: 0,
        }
    }

    /// Add a function application node
    pub fn add_application(&mut self, function: DefRef, arguments: Vec<usize>) -> usize {
        let node = CompositionNode::Function { function, arguments };
        self.add_node(node)
    }

    /// Add a variable node
    pub fn add_variable(&mut self, variable: String) -> usize {
        let node = CompositionNode::Variable(variable);
        self.add_node(node)
    }

    /// Add a literal node
    pub fn add_literal(&mut self, value: Value) -> usize {
        let node = CompositionNode::Literal(value);
        self.add_node(node)
    }

    /// Add a composition node
    pub fn add_composition(&mut self, subexpressions: Vec<usize>) -> usize {
        let node = CompositionNode::Composition(subexpressions);
        self.add_node(node)
    }

    /// Add a node to the DAG
    fn add_node(&mut self, node: CompositionNode) -> usize {
        let id = self.nodes.len();
        self.nodes.push(node);
        id
    }

    /// Set the root node
    pub fn set_root(&mut self, root: usize) {
        self.root = root;
    }

    /// Get the root node ID
    pub fn root(&self) -> usize {
        self.root
    }

    /// Compute the canonical DefRef for this composition
    pub fn canonical_def_ref(&self) -> DefRef {
        // Serialize the entire composition and hash it
        let content = serde_json::to_vec(self).expect("Failed to serialize composition");
        DefRef::new(&content, DefType::Function)
    }

    /// Optimize the composition DAG
    pub fn optimize(&mut self) {
        // Remove redundant nodes
        self.remove_redundant_nodes();

        // Apply common subexpression elimination
        self.eliminate_common_subexpressions();

        // Apply function-specific optimizations
        self.apply_function_optimizations();
    }

    /// Remove nodes that are not reachable from the root
    pub fn remove_unreachable_nodes(&mut self) {
        let reachable = self.compute_reachable_nodes();
        let mut new_nodes = Vec::new();
        let mut old_to_new = HashMap::new();

        for (i, node) in self.nodes.iter().enumerate() {
            if reachable.contains(&i) {
                let new_id = new_nodes.len();
                old_to_new.insert(i, new_id);
                new_nodes.push(node.clone());
            }
        }

        // Update edges
        let mut new_edges = Vec::new();
        for edge in &self.edges {
            if let (Some(&new_source), Some(&new_target)) =
                (old_to_new.get(&edge.source), old_to_new.get(&edge.target)) {
                new_edges.push(CompositionEdge {
                    source: new_source,
                    target: new_target,
                    label: edge.label.clone(),
                });
            }
        }

        // Update node references in the new nodes
        for node in &mut new_nodes {
            match node {
                CompositionNode::Function { arguments, .. } => {
                    for arg in arguments {
                        if let Some(&new_arg) = old_to_new.get(arg) {
                            *arg = new_arg;
                        }
                    }
                },
                CompositionNode::Composition(subexpressions) => {
                    for subexpr in subexpressions {
                        if let Some(&new_subexpr) = old_to_new.get(subexpr) {
                            *subexpr = new_subexpr;
                        }
                    }
                },
                _ => {}
            }
        }

        self.nodes = new_nodes;
        self.edges = new_edges;

        // Update root
        if let Some(&new_root) = old_to_new.get(&self.root) {
            self.root = new_root;
        }
    }

    /// Compute reachable nodes from root
    fn compute_reachable_nodes(&self) -> HashSet<usize> {
        let mut reachable = HashSet::new();
        let mut to_visit = vec![self.root];

        while let Some(node_id) = to_visit.pop() {
            if reachable.insert(node_id) {
                // Find all nodes that this node references
                match &self.nodes[node_id] {
                    CompositionNode::Function { arguments, .. } => {
                        for &arg in arguments {
                            to_visit.push(arg);
                        }
                    },
                    CompositionNode::Composition(subexpressions) => {
                        for &subexpr in subexpressions {
                            to_visit.push(subexpr);
                        }
                    },
                    _ => {}
                }

                // Find all nodes that reference this node (reverse edges)
                for edge in &self.edges {
                    if edge.target == node_id {
                        to_visit.push(edge.source);
                    }
                }
            }
        }

        reachable
    }

    /// Remove redundant nodes (simple case)
    fn remove_redundant_nodes(&mut self) {
        // Simple redundancy elimination
        let mut node_hash = HashMap::new();
        let mut to_remove = HashSet::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let hash = self.compute_node_hash(node);
            if let Some(&existing) = node_hash.get(&hash) {
                to_remove.insert(i);
            } else {
                node_hash.insert(hash, i);
            }
        }

        // Remove redundant nodes and update references
        if !to_remove.is_empty() {
            self.compact_nodes(&to_remove);
        }
    }

    /// Compute hash for a node
    fn compute_node_hash(&self, node: &CompositionNode) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        match node {
            CompositionNode::Function { function, arguments } => {
                function.hash(&mut hasher);
                for &arg in arguments {
                    arg.hash(&mut hasher);
                }
            },
            CompositionNode::Variable(var) => {
                var.hash(&mut hasher);
            },
            CompositionNode::Literal(value) => {
                // Simple hash for value
                match value {
                    Value::String(s) => s.hash(&mut hasher),
                    Value::I64(i) => i.hash(&mut hasher),
                    Value::F64(f) => f.to_bits().hash(&mut hasher),
                    Value::Bool(b) => b.hash(&mut hasher),
                    Value::Bytes(b) => b.hash(&mut hasher),
                    Value::Hash(h) => h.0.hash(&mut hasher),
                    _ => 0u64.hash(&mut hasher),
                }
            },
            CompositionNode::Composition(subexpressions) => {
                for &subexpr in subexpressions {
                    subexpr.hash(&mut hasher);
                }
            }
        }
        format!("{:x}", hasher.finish())
    }

    /// Compact nodes by removing redundant ones
    fn compact_nodes(&mut self, to_remove: &HashSet<usize>) {
        let mut new_nodes = Vec::new();
        let mut old_to_new = HashMap::new();

        for (i, node) in self.nodes.iter().enumerate() {
            if !to_remove.contains(&i) {
                let new_id = new_nodes.len();
                old_to_new.insert(i, new_id);
                new_nodes.push(node.clone());
            }
        }

        // Update all references
        for node in &mut new_nodes {
            match node {
                CompositionNode::Function { arguments, .. } => {
                    for arg in arguments {
                        if let Some(&new_arg) = old_to_new.get(arg) {
                            *arg = new_arg;
                        }
                    }
                },
                CompositionNode::Composition(subexpressions) => {
                    for subexpr in subexpressions {
                        if let Some(&new_subexpr) = old_to_new.get(subexpr) {
                            *subexpr = new_subexpr;
                        }
                    }
                },
                _ => {}
            }
        }

        self.nodes = new_nodes;

        // Update edges
        let mut new_edges = Vec::new();
        for edge in &self.edges {
            if let (Some(&new_source), Some(&new_target)) =
                (old_to_new.get(&edge.source), old_to_new.get(&edge.target)) {
                new_edges.push(CompositionEdge {
                    source: new_source,
                    target: new_target,
                    label: edge.label.clone(),
                });
            }
        }
        self.edges = new_edges;

        // Update root
        if let Some(&new_root) = old_to_new.get(&self.root) {
            self.root = new_root;
        }
    }

    /// Eliminate common subexpressions
    fn eliminate_common_subexpressions(&mut self) {
        // Simple CSE implementation
        let mut expression_map = HashMap::new();
        let mut cse_nodes = HashMap::new();

        // This is a simplified implementation
        // A full implementation would use value numbering or similar techniques
    }

    /// Apply function-specific optimizations
    fn apply_function_optimizations(&mut self) {
        // Apply optimizations based on function properties
        // This would use the function metadata from DefRef
    }

    /// Validate the DAG structure
    pub fn validate(&self) -> Result<(), CompositionError> {
        // Check that root exists
        if self.root >= self.nodes.len() {
            return Err(CompositionError::InvalidRoot(self.root));
        }

        // Check that all node references are valid
        for (i, node) in self.nodes.iter().enumerate() {
            match node {
                CompositionNode::Function { arguments, .. } => {
                    for &arg in arguments {
                        if arg >= self.nodes.len() {
                            return Err(CompositionError::InvalidNodeReference(i, arg));
                        }
                    }
                },
                CompositionNode::Composition(subexpressions) => {
                    for &subexpr in subexpressions {
                        if subexpr >= self.nodes.len() {
                            return Err(CompositionError::InvalidNodeReference(i, subexpr));
                        }
                    }
                },
                _ => {}
            }
        }

        // Check for cycles (simple check)
        if self.has_cycles() {
            return Err(CompositionError::HasCycles);
        }

        Ok(())
    }

    /// Check if the DAG has cycles
    fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        self.dfs_cycle_check(self.root, &mut visited, &mut rec_stack)
    }

    /// DFS cycle detection
    fn dfs_cycle_check(&self, node: usize, visited: &mut HashSet<usize>, rec_stack: &mut HashSet<usize>) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        // Check all outgoing edges
        for edge in &self.edges {
            if edge.source == node {
                let neighbor = edge.target;
                if !visited.contains(&neighbor) {
                    if self.dfs_cycle_check(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(&neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }
}

/// Composition error
#[derive(Debug, Clone)]
pub enum CompositionError {
    /// Invalid root node
    InvalidRoot(usize),
    /// Invalid node reference
    InvalidNodeReference(usize, usize),
    /// DAG contains cycles
    HasCycles,
}

impl std::fmt::Display for CompositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompositionError::InvalidRoot(root) => write!(f, "Invalid root node: {}", root),
            CompositionError::InvalidNodeReference(node, reference) => {
                write!(f, "Node {} has invalid reference to node {}", node, reference)
            },
            CompositionError::HasCycles => write!(f, "Composition DAG contains cycles"),
        }
    }
}

impl std::error::Error for CompositionError {}

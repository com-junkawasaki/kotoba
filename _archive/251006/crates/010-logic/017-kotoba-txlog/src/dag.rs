//! # Transaction DAG for Causal Ordering
//!
//! This module provides a directed acyclic graph implementation
//! for managing transaction causal ordering.

use super::*;
use kotoba_types::*;
use kotoba_codebase::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Transaction DAG for causal ordering
#[derive(Debug, Clone)]
pub struct TransactionDAG {
    /// Transaction storage
    pub transactions: HashMap<TransactionRef, Transaction>,
    /// Adjacency list (transaction -> children)
    pub children: HashMap<TransactionRef, HashSet<TransactionRef>>,
    /// Reverse adjacency list (transaction -> parents)
    pub parents: HashMap<TransactionRef, HashSet<TransactionRef>>,
    /// Root transactions (no parents)
    pub roots: HashSet<TransactionRef>,
    /// Transaction order (topological)
    pub topological_order: Vec<TransactionRef>,
    /// Configuration
    pub config: DAGConfig,
}

impl TransactionDAG {
    /// Create a new transaction DAG
    pub fn new() -> Self {
        Self {
            transactions: HashMap::new(),
            children: HashMap::new(),
            parents: HashMap::new(),
            roots: HashSet::new(),
            topological_order: Vec::new(),
            config: DAGConfig::default(),
        }
    }

    /// Add a transaction to the DAG
    pub fn add_transaction(&mut self, mut tx: Transaction) -> Result<TransactionRef, TxLogError> {
        let tx_ref = TransactionRef::from_transaction(&tx);

        // Check if transaction already exists
        if self.transactions.contains_key(&tx_ref) {
            return Ok(tx_ref);
        }

        // Verify transaction integrity
        tx.verify_integrity()?;

        // Check parent existence
        for parent_ref in &tx.parents {
            if !self.transactions.contains_key(parent_ref) {
                return Err(TxLogError::ParentNotFound(parent_ref.clone()));
            }
        }

        // Add transaction
        self.transactions.insert(tx_ref.clone(), tx);

        // Update relationships
        if tx.parents.is_empty() {
            self.roots.insert(tx_ref.clone());
        } else {
            for parent_ref in &tx.parents {
                self.children
                    .entry(parent_ref.clone())
                    .or_insert_with(HashSet::new)
                    .insert(tx_ref.clone());

                self.parents
                    .entry(tx_ref.clone())
                    .or_insert_with(HashSet::new)
                    .insert(parent_ref.clone());
            }
        }

        // Update topological order
        self.update_topological_order();

        Ok(tx_ref)
    }

    /// Get a transaction by reference
    pub fn get_transaction(&self, tx_ref: &TransactionRef) -> Option<&Transaction> {
        self.transactions.get(tx_ref)
    }

    /// Check if transaction exists
    pub fn contains_transaction(&self, tx_ref: &TransactionRef) -> bool {
        self.transactions.contains_key(tx_ref)
    }

    /// Get transaction children
    pub fn get_children(&self, tx_ref: &TransactionRef) -> Option<&HashSet<TransactionRef>> {
        self.children.get(tx_ref)
    }

    /// Get transaction parents
    pub fn get_parents(&self, tx_ref: &TransactionRef) -> Option<&HashSet<TransactionRef>> {
        self.parents.get(tx_ref)
    }

    /// Get all descendants of a transaction
    pub fn get_descendants(&self, tx_ref: &TransactionRef) -> HashSet<TransactionRef> {
        let mut descendants = HashSet::new();
        let mut to_visit = vec![tx_ref.clone()];

        while let Some(current) = to_visit.pop() {
            if let Some(children) = self.children.get(&current) {
                for child in children {
                    if descendants.insert(child.clone()) {
                        to_visit.push(child.clone());
                    }
                }
            }
        }

        descendants
    }

    /// Get all ancestors of a transaction
    pub fn get_ancestors(&self, tx_ref: &TransactionRef) -> HashSet<TransactionRef> {
        let mut ancestors = HashSet::new();
        let mut to_visit = vec![tx_ref.clone()];

        while let Some(current) = to_visit.pop() {
            if let Some(parents) = self.parents.get(&current) {
                for parent in parents {
                    if ancestors.insert(parent.clone()) {
                        to_visit.push(parent.clone());
                    }
                }
            }
        }

        ancestors
    }

    /// Get transactions in causal order (ancestors first)
    pub fn get_causal_order(&self, tx_ref: &TransactionRef) -> Vec<TransactionRef> {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let ancestors = self.get_ancestors(tx_ref);

        // Add ancestors in topological order
        for ancestor in &ancestors {
            if !visited.contains(ancestor) {
                let ancestor_order = self.get_transaction_order(ancestor);
                order.extend(ancestor_order);
                for tx in &ancestor_order {
                    visited.insert(tx.clone());
                }
            }
        }

        // Add the transaction itself
        if !visited.contains(tx_ref) {
            order.push(tx_ref.clone());
        }

        order
    }

    /// Get transactions reachable from roots to a specific transaction
    pub fn get_transactions_to(&self, tx_ref: &TransactionRef) -> Vec<TransactionRef> {
        let mut reachable = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start from roots
        for root in &self.roots {
            if !visited.contains(root) {
                queue.push_back(root.clone());
                visited.insert(root.clone());
            }
        }

        while let Some(current) = queue.pop_front() {
            reachable.push(current.clone());

            // Add children
            if let Some(children) = self.children.get(&current) {
                for child in children {
                    if !visited.contains(child) {
                        visited.insert(child.clone());
                        queue.push_back(child.clone());
                    }
                }
            }
        }

        reachable
    }

    /// Update topological order
    fn update_topological_order(&mut self) {
        let mut order = Vec::new();
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();

        // Perform topological sort starting from roots
        for root in &self.roots {
            self.topological_sort_visit(root, &mut order, &mut visited, &mut visiting);
        }

        self.topological_order = order;
    }

    /// Topological sort visit (DFS with cycle detection)
    fn topological_sort_visit(
        &self,
        tx_ref: &TransactionRef,
        order: &mut Vec<TransactionRef>,
        visited: &mut HashSet<TransactionRef>,
        visiting: &mut HashSet<TransactionRef>,
    ) {
        if visiting.contains(tx_ref) {
            // Cycle detected
            return;
        }

        if visited.contains(tx_ref) {
            return;
        }

        visiting.insert(tx_ref.clone());

        // Visit children
        if let Some(children) = self.children.get(tx_ref) {
            for child in children {
                self.topological_sort_visit(child, order, visited, visiting);
            }
        }

        visiting.remove(tx_ref);
        visited.insert(tx_ref.clone());
        order.push(tx_ref.clone());
    }

    /// Get transaction order (ancestors first)
    fn get_transaction_order(&self, tx_ref: &TransactionRef) -> Vec<TransactionRef> {
        let ancestors = self.get_ancestors(tx_ref);
        let mut order = Vec::new();
        let mut visited = HashSet::new();

        for ancestor in &ancestors {
            if !visited.contains(ancestor) {
                let mut stack = vec![ancestor.clone()];
                while let Some(current) = stack.pop() {
                    if visited.contains(&current) {
                        continue;
                    }

                    visited.insert(current.clone());

                    // Add parents first
                    if let Some(parents) = self.parents.get(&current) {
                        for parent in parents {
                            if !visited.contains(parent) {
                                stack.push(parent.clone());
                            }
                        }
                    }

                    order.push(current);
                }
            }
        }

        order
    }

    /// Verify DAG integrity
    pub fn verify_integrity(&self) -> Result<bool, TxLogError> {
        let mut is_valid = true;
        let mut errors = Vec::new();

        // Check that all parents exist
        for (tx_ref, parents) in &self.parents {
            for parent_ref in parents {
                if !self.transactions.contains_key(parent_ref) {
                    errors.push(format!("Transaction {} has non-existent parent {}", tx_ref.tx_id, parent_ref.tx_id));
                    is_valid = false;
                }
            }
        }

        // Check that all children exist
        for (tx_ref, children) in &self.children {
            for child_ref in children {
                if !self.transactions.contains_key(child_ref) {
                    errors.push(format!("Transaction {} has non-existent child {}", tx_ref.tx_id, child_ref.tx_id));
                    is_valid = false;
                }
            }
        }

        // Check topological order
        let mut position = HashMap::new();
        for (i, tx_ref) in self.topological_order.iter().enumerate() {
            position.insert(tx_ref.clone(), i);
        }

        for tx_ref in self.transactions.keys() {
            if let Some(&pos) = position.get(tx_ref) {
                // Check that all parents come before this transaction
                if let Some(parents) = self.parents.get(tx_ref) {
                    for parent_ref in parents {
                        if let Some(&parent_pos) = position.get(parent_ref) {
                            if parent_pos >= pos {
                                errors.push(format!(
                                    "Parent {} (pos {}) comes after child {} (pos {})",
                                    parent_ref.tx_id, parent_pos, tx_ref.tx_id, pos
                                ));
                                is_valid = false;
                            }
                        }
                    }
                }
            } else {
                errors.push(format!("Transaction {} not in topological order", tx_ref.tx_id));
                is_valid = false;
            }
        }

        if !is_valid {
            return Err(TxLogError::DAGInconsistency(errors.join("; ")));
        }

        Ok(is_valid)
    }

    /// Get DAG statistics
    pub fn get_stats(&self) -> DAGStats {
        DAGStats {
            transaction_count: self.transactions.len(),
            root_count: self.roots.len(),
            max_depth: self.compute_max_depth(),
            avg_children: if self.transactions.is_empty() {
                0.0
            } else {
                self.children.values().map(|c| c.len()).sum::<usize>() as f64 / self.transactions.len() as f64
            },
            avg_parents: if self.transactions.is_empty() {
                0.0
            } else {
                self.parents.values().map(|p| p.len()).sum::<usize>() as f64 / self.transactions.len() as f64
            },
        }
    }

    /// Compute maximum depth of the DAG
    fn compute_max_depth(&self) -> usize {
        let mut max_depth = 0;
        let mut depths = HashMap::new();

        for tx_ref in &self.roots {
            let depth = self.compute_depth(tx_ref, &mut depths);
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    /// Compute depth of a transaction
    fn compute_depth(&self, tx_ref: &TransactionRef, depths: &mut HashMap<TransactionRef, usize>) -> usize {
        if let Some(&depth) = depths.get(tx_ref) {
            return depth;
        }

        let depth = if let Some(parents) = self.parents.get(tx_ref) {
            let max_parent_depth = parents.iter()
                .map(|parent| self.compute_depth(parent, depths))
                .max()
                .unwrap_or(0);
            max_parent_depth + 1
        } else {
            1
        };

        depths.insert(tx_ref.clone(), depth);
        depth
    }

    /// Find common ancestors of two transactions
    pub fn find_common_ancestors(&self, tx1: &TransactionRef, tx2: &TransactionRef) -> HashSet<TransactionRef> {
        let ancestors1 = self.get_ancestors(tx1);
        let ancestors2 = self.get_ancestors(tx2);

        ancestors1.intersection(&ancestors2).cloned().collect()
    }

    /// Find lowest common ancestors
    pub fn find_lowest_common_ancestors(&self, tx1: &TransactionRef, tx2: &TransactionRef) -> HashSet<TransactionRef> {
        let common = self.find_common_ancestors(tx1, tx2);
        let mut lcas = HashSet::new();

        for ancestor in &common {
            // Check if this ancestor is a lowest common ancestor
            let descendants1 = self.get_descendants(ancestor);
            let descendants2 = self.get_descendants(ancestor);

            let mut is_lca = true;
            for other in &common {
                if other == ancestor {
                    continue;
                }

                // If there's another common ancestor that is a descendant of this one,
                // then this is not a lowest common ancestor
                if descendants1.contains(other) || descendants2.contains(other) {
                    is_lca = false;
                    break;
                }
            }

            if is_lca {
                lcas.insert(ancestor.clone());
            }
        }

        lcas
    }

    /// Get transactions in a time range
    pub fn get_transactions_in_range(&self, start_time: u64, end_time: u64) -> Vec<TransactionRef> {
        self.transactions.values()
            .filter(|tx| tx.hlc.physical >= start_time && tx.hlc.physical <= end_time)
            .map(|tx| TransactionRef::from_transaction(tx))
            .collect()
    }

    /// Export DAG as DOT format for visualization
    pub fn export_dot(&self) -> String {
        let mut dot = String::from("digraph TransactionDAG {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n");

        // Add nodes
        for (tx_ref, tx) in &self.transactions {
            let label = format!("{} ({})", tx.tx_id, tx.author);
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", tx_ref.tx_id, label));
        }

        // Add edges
        for (parent, children) in &self.children {
            for child in children {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", parent.tx_id, child.tx_id));
            }
        }

        dot.push_str("}\n");
        dot
    }
}

/// DAG configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGConfig {
    /// Maximum number of parents per transaction
    pub max_parents: usize,
    /// Enable cycle detection
    pub enable_cycle_detection: bool,
    /// Enable automatic cleanup
    pub enable_cleanup: bool,
    /// Cleanup threshold (number of transactions)
    pub cleanup_threshold: usize,
}

impl Default for DAGConfig {
    fn default() -> Self {
        Self {
            max_parents: 10,
            enable_cycle_detection: true,
            enable_cleanup: true,
            cleanup_threshold: 10000,
        }
    }
}

/// DAG statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DAGStats {
    /// Total number of transactions
    pub transaction_count: usize,
    /// Number of root transactions
    pub root_count: usize,
    /// Maximum depth of the DAG
    pub max_depth: usize,
    /// Average number of children per transaction
    pub avg_children: f64,
    /// Average number of parents per transaction
    pub avg_parents: f64,
}

impl Default for DAGStats {
    fn default() -> Self {
        Self {
            transaction_count: 0,
            root_count: 0,
            max_depth: 0,
            avg_children: 0.0,
            avg_parents: 0.0,
        }
    }
}

/// Transaction DAG builder for fluent API
#[derive(Debug, Clone)]
pub struct TransactionDAGBuilder {
    dag: TransactionDAG,
}

impl TransactionDAGBuilder {
    /// Create a new DAG builder
    pub fn new() -> Self {
        Self {
            dag: TransactionDAG::new(),
        }
    }

    /// Add a transaction
    pub fn add_transaction(mut self, tx: Transaction) -> Result<Self, TxLogError> {
        self.dag.add_transaction(tx)?;
        Ok(self)
    }

    /// Build the DAG
    pub fn build(self) -> TransactionDAG {
        self.dag
    }
}

/// Cycle detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleDetectionResult {
    /// Whether a cycle was detected
    pub has_cycle: bool,
    /// Cycle path if detected
    pub cycle_path: Option<Vec<TransactionRef>>,
    /// Cycle length if detected
    pub cycle_length: Option<usize>,
}

impl CycleDetectionResult {
    /// Create a positive result
    pub fn cycle_found(path: Vec<TransactionRef>) -> Self {
        Self {
            has_cycle: true,
            cycle_path: Some(path),
            cycle_length: Some(path.len()),
        }
    }

    /// Create a negative result
    pub fn no_cycle() -> Self {
        Self {
            has_cycle: false,
            cycle_path: None,
            cycle_length: None,
        }
    }
}

/// DAG cleaner for removing old transactions
#[derive(Debug, Clone)]
pub struct DAGCleaner {
    /// Transactions to keep (by reference)
    pub keep_transactions: HashSet<TransactionRef>,
    /// Depth limit for cleaning
    pub depth_limit: Option<usize>,
}

impl DAGCleaner {
    /// Create a new DAG cleaner
    pub fn new(keep_transactions: HashSet<TransactionRef>) -> Self {
        Self {
            keep_transactions,
            depth_limit: None,
        }
    }

    /// Clean the DAG, keeping only reachable transactions
    pub fn clean(&self, dag: &TransactionDAG) -> TransactionDAG {
        let mut cleaned_dag = TransactionDAG::new();
        let mut to_process = VecDeque::new();
        let mut processed = HashSet::new();

        // Start with transactions to keep
        for tx_ref in &self.keep_transactions {
            if let Some(tx) = dag.get_transaction(tx_ref) {
                to_process.push_back(tx.clone());
            }
        }

        // Process transactions and their ancestors
        while let Some(tx) = to_process.pop_front() {
            let tx_ref = TransactionRef::from_transaction(&tx);

            if processed.contains(&tx_ref) {
                continue;
            }

            // Add transaction to cleaned DAG
            cleaned_dag.transactions.insert(tx_ref.clone(), tx.clone());

            // Add ancestors
            if let Some(parents) = dag.get_parents(&tx_ref) {
                for parent_ref in parents {
                    if let Some(parent_tx) = dag.get_transaction(parent_ref) {
                        to_process.push_back(parent_tx.clone());
                    }
                }
            }

            processed.insert(tx_ref);
        }

        // Rebuild relationships in cleaned DAG
        cleaned_dag.rebuild_relationships();

        cleaned_dag
    }

    /// Rebuild relationships in the cleaned DAG
    fn rebuild_relationships(&self, _dag: &mut TransactionDAG) {
        // Implementation would rebuild parent-child relationships
        // based on the transaction parent references
    }
}

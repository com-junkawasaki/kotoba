//! # Provenance Tracking and Queries
//!
//! This module provides provenance tracking and query functionality
//! for understanding the causal history of definitions.

use super::*;
use kotoba_types::*;
use kotoba_codebase::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Provenance tracker for managing causal relationships
#[derive(Debug, Clone)]
pub struct ProvenanceTracker {
    /// DefRef to provenance mapping
    pub provenance_map: HashMap<DefRef, ProvenanceEntry>,
    /// Transaction to DefRef mapping
    pub transaction_outputs: HashMap<TransactionRef, Vec<DefRef>>,
    /// Reverse mapping: DefRef to creating transaction
    pub defref_creators: HashMap<DefRef, TransactionRef>,
    /// Dependency graph between DefRefs
    pub dependency_graph: HashMap<DefRef, HashSet<DefRef>>,
    /// Configuration
    pub config: ProvenanceConfig,
}

impl ProvenanceTracker {
    /// Create a new provenance tracker
    pub fn new() -> Self {
        Self {
            provenance_map: HashMap::new(),
            transaction_outputs: HashMap::new(),
            defref_creators: HashMap::new(),
            dependency_graph: HashMap::new(),
            config: ProvenanceConfig::default(),
        }
    }

    /// Track a transaction and its provenance
    pub fn track_transaction(&mut self, tx_ref: &TransactionRef, tx: &Transaction) {
        let outputs = tx.outputs();

        // Record transaction outputs
        self.transaction_outputs.insert(tx_ref.clone(), outputs.clone());

        // Track provenance for each output
        for output_ref in &outputs {
            self.track_defref_creation(tx_ref, tx, output_ref);
        }

        // Update dependency graph
        let inputs = tx.dependencies();
        for output_ref in &outputs {
            let input_deps: HashSet<DefRef> = inputs.iter().cloned().collect();
            self.dependency_graph.insert(output_ref.clone(), input_deps);
        }
    }

    /// Track the creation of a DefRef
    fn track_defref_creation(&mut self, tx_ref: &TransactionRef, tx: &Transaction, def_ref: &DefRef) {
        let entry = ProvenanceEntry {
            def_ref: def_ref.clone(),
            created_by: tx_ref.clone(),
            created_at: tx.hlc.clone(),
            operation: tx.operation.clone(),
            inputs: tx.dependencies(),
            metadata: tx.metadata.clone(),
        };

        self.provenance_map.insert(def_ref.clone(), entry);
        self.defref_creators.insert(def_ref.clone(), tx_ref.clone());
    }

    /// Query provenance: why does this value exist?
    pub fn why(&self, def_ref: &DefRef) -> Result<ProvenanceChain, TxLogError> {
        let mut chain = ProvenanceChain::new(def_ref.clone(), def_ref.clone());
        let mut visited = HashSet::new();
        let mut current_ref = def_ref.clone();

        while !visited.contains(&current_ref) {
            visited.insert(current_ref.clone());

            if let Some(entry) = self.provenance_map.get(&current_ref) {
                let link = ProvenanceLink {
                    transaction_ref: entry.created_by.clone(),
                    operation: entry.operation.clone(),
                    inputs: entry.inputs.clone(),
                    output: current_ref.clone(),
                    timestamp: entry.created_at.clone(),
                };

                chain.add_link(link);

                // Continue with the first input (most direct dependency)
                if let Some(first_input) = entry.inputs.first() {
                    current_ref = first_input.clone();
                } else {
                    break;
                }
            } else {
                break;
            }

            // Prevent infinite loops
            if chain.length() > self.config.max_chain_length {
                return Err(TxLogError::ChainTooLong);
            }
        }

        Ok(chain)
    }

    /// Get all DefRefs created by a transaction
    pub fn get_transaction_outputs(&self, tx_ref: &TransactionRef) -> Option<&Vec<DefRef>> {
        self.transaction_outputs.get(tx_ref)
    }

    /// Get the transaction that created a DefRef
    pub fn get_defref_creator(&self, def_ref: &DefRef) -> Option<&TransactionRef> {
        self.defref_creators.get(def_ref)
    }

    /// Get dependencies of a DefRef
    pub fn get_defref_dependencies(&self, def_ref: &DefRef) -> Option<&HashSet<DefRef>> {
        self.dependency_graph.get(def_ref)
    }

    /// Get all DefRefs that depend on a given DefRef
    pub fn get_defref_dependents(&self, def_ref: &DefRef) -> HashSet<DefRef> {
        let mut dependents = HashSet::new();

        for (output_ref, inputs) in &self.dependency_graph {
            if inputs.contains(def_ref) {
                dependents.insert(output_ref.clone());
            }
        }

        dependents
    }

    /// Find the root cause of a DefRef (original inputs)
    pub fn find_root_cause(&self, def_ref: &DefRef) -> Result<Vec<DefRef>, TxLogError> {
        let chain = self.why(def_ref)?;
        Ok(chain.chain.iter()
            .filter_map(|link| link.inputs.first())
            .cloned()
            .collect())
    }

    /// Get provenance summary for a DefRef
    pub fn get_provenance_summary(&self, def_ref: &DefRef) -> Result<ProvenanceSummary, TxLogError> {
        let chain = self.why(def_ref)?;

        let mut operation_counts = HashMap::new();
        let mut author_counts = HashMap::new();
        let mut total_operations = 0;

        for link in &chain.chain {
            let op_type = link.operation.operation_type();
            *operation_counts.entry(op_type).or_insert(0) += 1;

            // Extract author from transaction (would need transaction lookup)
            // For now, use placeholder
            *author_counts.entry("unknown".to_string()).or_insert(0) += 1;

            total_operations += 1;
        }

        Ok(ProvenanceSummary {
            def_ref: def_ref.clone(),
            chain_length: chain.length(),
            operation_counts,
            author_counts,
            total_operations,
            is_valid: chain.is_valid(),
        })
    }

    /// Verify provenance integrity
    pub fn verify_integrity(&self) -> bool {
        let mut is_valid = true;

        // Check all provenance entries
        for (def_ref, entry) in &self.provenance_map {
            // Verify transaction exists
            // (would need access to transaction log)

            // Verify inputs are properly tracked
            for input_ref in &entry.inputs {
                if !self.provenance_map.contains_key(input_ref) {
                    // Input doesn't have provenance - might be external
                }
            }

            // Verify dependency graph consistency
            if let Some(deps) = self.dependency_graph.get(def_ref) {
                if *deps != entry.inputs.iter().cloned().collect() {
                    is_valid = false;
                }
            }
        }

        // Check dependency graph completeness
        for (output_ref, inputs) in &self.dependency_graph {
            if !self.provenance_map.contains_key(output_ref) {
                is_valid = false;
            }

            for input_ref in inputs {
                if let Some(entry) = self.provenance_map.get(input_ref) {
                    if !entry.inputs.contains(output_ref) {
                        is_valid = false;
                    }
                }
            }
        }

        is_valid
    }

    /// Get provenance statistics
    pub fn get_stats(&self) -> ProvenanceStats {
        let total_entries = self.provenance_map.len();
        let mut max_depth = 0;
        let mut total_operations = 0;
        let mut operation_types = HashMap::new();

        for entry in self.provenance_map.values() {
            total_operations += 1;

            let op_type = entry.operation.operation_type();
            *operation_types.entry(op_type).or_insert(0) += 1;

            // Calculate depth (simplified)
            max_depth = max_depth.max(entry.inputs.len());
        }

        ProvenanceStats {
            total_entries,
            max_depth,
            total_operations,
            operation_types,
            avg_operations_per_defref: if total_entries > 0 {
                total_operations as f64 / total_entries as f64
            } else {
                0.0
            },
        }
    }

    /// Export provenance graph as DOT format
    pub fn export_provenance_graph(&self) -> String {
        let mut dot = String::from("digraph ProvenanceGraph {\n");
        dot.push_str("  rankdir=TB;\n");
        dot.push_str("  node [shape=box];\n");

        // Add DefRef nodes
        for (def_ref, entry) in &self.provenance_map {
            let label = format!("{} ({})", def_ref.def_type, def_ref.name.as_deref().unwrap_or("unnamed"));
            dot.push_str(&format!("  \"{}\" [label=\"{}\"];\n", def_ref.hash, label));
        }

        // Add dependency edges
        for (output_ref, inputs) in &self.dependency_graph {
            for input_ref in inputs {
                dot.push_str(&format!("  \"{}\" -> \"{}\";\n", input_ref.hash, output_ref.hash));
            }
        }

        dot.push_str("}\n");
        dot
    }
}

/// Provenance entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceEntry {
    /// DefRef that was created
    pub def_ref: DefRef,
    /// Transaction that created it
    pub created_by: TransactionRef,
    /// When it was created
    pub created_at: HLC,
    /// Operation that created it
    pub operation: TransactionOperation,
    /// Input DefRefs used in creation
    pub inputs: Vec<DefRef>,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

/// Provenance chain for tracking causality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceChain {
    /// Starting DefRef
    pub start_ref: DefRef,
    /// Chain of provenance links
    pub chain: Vec<ProvenanceLink>,
    /// Final DefRef
    pub end_ref: DefRef,
}

impl ProvenanceChain {
    /// Create a new provenance chain
    pub fn new(start_ref: DefRef, end_ref: DefRef) -> Self {
        Self {
            start_ref,
            chain: Vec::new(),
            end_ref,
        }
    }

    /// Add a link to the chain
    pub fn add_link(&mut self, link: ProvenanceLink) {
        self.chain.push(link);
    }

    /// Get the length of the chain
    pub fn length(&self) -> usize {
        self.chain.len()
    }

    /// Get all operations in the chain
    pub fn operations(&self) -> Vec<&TransactionOperation> {
        self.chain.iter().map(|link| &link.operation).collect()
    }

    /// Get all transactions in the chain
    pub fn transactions(&self) -> Vec<&TransactionRef> {
        self.chain.iter().map(|link| &link.transaction_ref).collect()
    }

    /// Check if the chain is valid
    pub fn is_valid(&self) -> bool {
        if self.chain.is_empty() {
            return self.start_ref == self.end_ref;
        }

        // Check that each link connects properly
        for i in 0..self.chain.len() - 1 {
            let current_link = &self.chain[i];
            let next_link = &self.chain[i + 1];

            // The output of one link should be the input of the next
            if !next_link.inputs.contains(&current_link.output) {
                return false;
            }
        }

        true
    }

    /// Get the root causes (original inputs)
    pub fn root_causes(&self) -> Vec<DefRef> {
        let mut root_causes = Vec::new();
        let mut seen_inputs = HashSet::new();

        for link in &self.chain {
            for input in &link.inputs {
                if !seen_inputs.contains(input) && !self.contains_as_output(input) {
                    root_causes.push(input.clone());
                    seen_inputs.insert(input.clone());
                }
            }
        }

        root_causes
    }

    /// Check if a DefRef appears as an output in the chain
    fn contains_as_output(&self, def_ref: &DefRef) -> bool {
        self.chain.iter().any(|link| link.output == *def_ref)
    }
}

/// Provenance link in the chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceLink {
    /// Transaction that performed the operation
    pub transaction_ref: TransactionRef,
    /// Operation performed
    pub operation: TransactionOperation,
    /// Input DefRefs
    pub inputs: Vec<DefRef>,
    /// Output DefRef
    pub output: DefRef,
    /// Timestamp of the operation
    pub timestamp: HLC,
}

/// Provenance summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceSummary {
    /// DefRef being summarized
    pub def_ref: DefRef,
    /// Length of the provenance chain
    pub chain_length: usize,
    /// Count of operations by type
    pub operation_counts: HashMap<String, usize>,
    /// Count of operations by author
    pub author_counts: HashMap<String, usize>,
    /// Total number of operations
    pub total_operations: usize,
    /// Whether the provenance is valid
    pub is_valid: bool,
}

/// Provenance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceConfig {
    /// Maximum chain length to traverse
    pub max_chain_length: usize,
    /// Enable detailed provenance tracking
    pub enable_detailed_tracking: bool,
    /// Cache provenance chains
    pub cache_chains: bool,
    /// Maximum cache size
    pub max_cache_size: usize,
}

impl Default for ProvenanceConfig {
    fn default() -> Self {
        Self {
            max_chain_length: 100,
            enable_detailed_tracking: true,
            cache_chains: true,
            max_cache_size: 1000,
        }
    }
}

/// Provenance statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceStats {
    /// Total number of provenance entries
    pub total_entries: usize,
    /// Maximum depth of provenance chains
    pub max_depth: usize,
    /// Total number of operations tracked
    pub total_operations: usize,
    /// Operations by type
    pub operation_types: HashMap<String, usize>,
    /// Average operations per DefRef
    pub avg_operations_per_defref: f64,
}

/// Provenance query interface
#[derive(Debug, Clone)]
pub struct ProvenanceQuery {
    /// Tracker to query
    pub tracker: ProvenanceTracker,
}

impl ProvenanceQuery {
    /// Create a new provenance query
    pub fn new(tracker: ProvenanceTracker) -> Self {
        Self { tracker }
    }

    /// Query provenance for a DefRef
    pub fn query(&self, def_ref: &DefRef) -> Result<ProvenanceChain, TxLogError> {
        self.tracker.why(def_ref)
    }

    /// Query provenance for multiple DefRefs
    pub fn query_multiple(&self, def_refs: &[DefRef]) -> Result<Vec<ProvenanceChain>, TxLogError> {
        let mut results = Vec::new();

        for def_ref in def_refs {
            results.push(self.query(def_ref)?);
        }

        Ok(results)
    }

    /// Find common provenance between DefRefs
    pub fn find_common_provenance(&self, def_refs: &[DefRef]) -> Result<Option<ProvenanceChain>, TxLogError> {
        if def_refs.is_empty() {
            return Ok(None);
        }

        let mut chains = Vec::new();
        for def_ref in def_refs {
            chains.push(self.query(def_ref)?);
        }

        // Find common ancestors
        let common_chain = self.find_intersection(&chains);
        Ok(common_chain)
    }

    /// Find intersection of multiple provenance chains
    fn find_intersection(&self, chains: &[ProvenanceChain]) -> Option<ProvenanceChain> {
        if chains.is_empty() {
            return None;
        }

        let first_chain = &chains[0];
        let mut common_links = Vec::new();

        for link in &first_chain.chain {
            let mut is_common = true;

            for chain in &chains[1..] {
                if !chain.chain.iter().any(|l| l.transaction_ref == link.transaction_ref) {
                    is_common = false;
                    break;
                }
            }

            if is_common {
                common_links.push(link.clone());
            } else {
                break;
            }
        }

        if common_links.is_empty() {
            None
        } else {
            Some(ProvenanceChain {
                start_ref: first_chain.start_ref.clone(),
                chain: common_links,
                end_ref: first_chain.end_ref.clone(),
            })
        }
    }

    /// Get provenance impact analysis
    pub fn impact_analysis(&self, def_ref: &DefRef) -> Result<ImpactAnalysis, TxLogError> {
        let dependents = self.tracker.get_defref_dependents(def_ref);
        let provenance = self.tracker.why(def_ref)?;

        let mut affected_operations = HashMap::new();
        for link in &provenance.chain {
            let op_type = link.operation.operation_type();
            *affected_operations.entry(op_type).or_insert(0) += 1;
        }

        Ok(ImpactAnalysis {
            def_ref: def_ref.clone(),
            dependent_count: dependents.len(),
            dependents: dependents.into_iter().collect(),
            provenance_chain_length: provenance.length(),
            affected_operations,
        })
    }
}

/// Impact analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAnalysis {
    /// DefRef being analyzed
    pub def_ref: DefRef,
    /// Number of dependent DefRefs
    pub dependent_count: usize,
    /// Dependent DefRefs
    pub dependents: Vec<DefRef>,
    /// Length of provenance chain
    pub provenance_chain_length: usize,
    /// Count of affected operations by type
    pub affected_operations: HashMap<String, usize>,
}

impl ImpactAnalysis {
    /// Get risk score (higher = more impact)
    pub fn risk_score(&self) -> f64 {
        self.dependent_count as f64 * 0.5 + self.provenance_chain_length as f64 * 0.3
    }

    /// Check if impact is significant
    pub fn is_significant(&self) -> bool {
        self.risk_score() > 10.0 || self.dependent_count > 50
    }
}

/// Provenance validator
#[derive(Debug, Clone)]
pub struct ProvenanceValidator {
    /// Tracker to validate
    pub tracker: ProvenanceTracker,
}

impl ProvenanceValidator {
    /// Create a new validator
    pub fn new(tracker: ProvenanceTracker) -> Self {
        Self { tracker }
    }

    /// Validate all provenance entries
    pub fn validate_all(&self) -> ValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check each entry
        for (def_ref, entry) in &self.tracker.provenance_map {
            if let Err(e) = self.validate_entry(def_ref, entry) {
                errors.push(e);
            }

            if let Some(warning) = self.check_entry_warnings(def_ref, entry) {
                warnings.push(warning);
            }
        }

        ValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            total_entries: self.tracker.provenance_map.len(),
        }
    }

    /// Validate a single provenance entry
    fn validate_entry(&self, _def_ref: &DefRef, entry: &ProvenanceEntry) -> Result<(), String> {
        // Check timestamp validity
        if !entry.created_at.is_valid() {
            return Err("Invalid HLC timestamp".to_string());
        }

        // Check operation validity
        // (implementation would verify operation-specific constraints)

        Ok(())
    }

    /// Check for warnings in a provenance entry
    fn check_entry_warnings(&self, _def_ref: &DefRef, entry: &ProvenanceEntry) -> Option<String> {
        // Check for long chains
        if entry.inputs.len() > 10 {
            return Some("Provenance chain is very long".to_string());
        }

        // Check for missing metadata
        if entry.metadata.is_empty() {
            return Some("Provenance entry has no metadata".to_string());
        }

        None
    }
}

/// Validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether all provenance is valid
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Total number of entries checked
    pub total_entries: usize,
}

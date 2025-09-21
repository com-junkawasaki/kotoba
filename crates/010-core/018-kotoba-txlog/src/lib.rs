//! # Kotoba TxLog
//!
//! Tx DAG + provenance + replay for causal tracking and audit trails.
//!
//! This crate provides transaction DAG management with provenance tracking
//! and replay functionality for maintaining causal relationships and audit trails.

pub mod tx;
pub mod dag;
pub mod provenance;
pub mod replay;
pub mod witness;
pub mod topology;

use kotoba_types::*;
use kotoba_codebase::*;
use kotoba_graph_core::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction log for managing causal relationships
#[derive(Debug, Clone)]
pub struct TxLog {
    /// Transaction DAG
    pub dag: TransactionDAG,
    /// Provenance tracker
    pub provenance: ProvenanceTracker,
    /// Replay manager
    pub replay: ReplayManager,
    /// Witness manager
    pub witness: WitnessManager,
    /// Configuration
    pub config: TxLogConfig,
}

impl TxLog {
    /// Create a new transaction log
    pub fn new(config: TxLogConfig) -> Self {
        Self {
            dag: TransactionDAG::new(),
            provenance: ProvenanceTracker::new(),
            replay: ReplayManager::new(),
            witness: WitnessManager::new(),
            config,
        }
    }

    /// Add a new transaction
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<TransactionRef, TxLogError> {
        // Validate transaction
        self.validate_transaction(&tx)?;

        // Add to DAG
        let tx_ref = self.dag.add_transaction(tx.clone())?;

        // Track provenance
        self.provenance.track_transaction(&tx_ref, &tx);

        // Add witnesses
        self.witness.add_witnesses(&tx_ref, &tx.witnesses);

        Ok(tx_ref)
    }

    /// Get transaction by reference
    pub fn get_transaction(&self, tx_ref: &TransactionRef) -> Option<&Transaction> {
        self.dag.get_transaction(tx_ref)
    }

    /// Query provenance: why does this value exist?
    pub fn why(&self, def_ref: &DefRef) -> Result<ProvenanceChain, TxLogError> {
        self.provenance.why(def_ref)
    }

    /// Replay transactions from a specific point
    pub fn replay_from(&self, from_tx: &TransactionRef) -> Result<Vec<Transaction>, TxLogError> {
        self.replay.replay_from(from_tx)
    }

    /// Verify transaction chain integrity
    pub fn verify_integrity(&self) -> Result<IntegrityReport, TxLogError> {
        let dag_integrity = self.dag.verify_integrity()?;
        let provenance_integrity = self.provenance.verify_integrity();
        let witness_integrity = self.witness.verify_integrity();

        Ok(IntegrityReport {
            dag_integrity,
            provenance_integrity,
            witness_integrity,
            overall_integrity: dag_integrity && provenance_integrity && witness_integrity,
        })
    }

    /// Validate a transaction before adding
    fn validate_transaction(&self, tx: &Transaction) -> Result<(), TxLogError> {
        // Check HLC ordering
        if !tx.hlc.is_valid() {
            return Err(TxLogError::InvalidHLC);
        }

        // Check parent references
        for parent_ref in &tx.parents {
            if !self.dag.contains_transaction(parent_ref) {
                return Err(TxLogError::ParentNotFound(parent_ref.clone()));
            }
        }

        // Verify signatures if required
        if self.config.require_signatures {
            tx.verify_signatures()?;
        }

        Ok(())
    }
}

/// Transaction reference for content addressing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionRef {
    /// Hash of the transaction
    pub hash: Hash,
    /// Transaction ID
    pub tx_id: String,
}

impl TransactionRef {
    /// Create a new transaction reference
    pub fn new(hash: Hash, tx_id: String) -> Self {
        Self { hash, tx_id }
    }

    /// Create from transaction
    pub fn from_transaction(tx: &Transaction) -> Self {
        let content = serde_json::to_vec(tx).expect("Failed to serialize transaction");
        let hash = Hash::from_sha256(&content);
        Self::new(hash, tx.tx_id.clone())
    }
}

/// Transaction representing a single operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction ID
    pub tx_id: String,
    /// Hybrid Logical Clock timestamp
    pub hlc: HLC,
    /// Parent transaction references
    pub parents: Vec<TransactionRef>,
    /// Author of the transaction
    pub author: String,
    /// Signature of the transaction
    pub signature: Option<String>,
    /// Witness references for audit
    pub witnesses: Vec<DefRef>,
    /// Operation performed
    pub operation: TransactionOperation,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(
        tx_id: String,
        hlc: HLC,
        parents: Vec<TransactionRef>,
        author: String,
        operation: TransactionOperation,
    ) -> Self {
        Self {
            tx_id,
            hlc,
            parents,
            author,
            signature: None,
            witnesses: Vec::new(),
            operation,
            metadata: HashMap::new(),
        }
    }

    /// Add signature
    pub fn with_signature(mut self, signature: String) -> Self {
        self.signature = Some(signature);
        self
    }

    /// Add witnesses
    pub fn with_witnesses(mut self, witnesses: Vec<DefRef>) -> Self {
        self.witnesses = witnesses;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Verify signatures
    pub fn verify_signatures(&self) -> Result<(), TxLogError> {
        if let Some(ref signature) = self.signature {
            // Implementation would verify the signature
            // For now, just check if it exists
            if signature.is_empty() {
                return Err(TxLogError::InvalidSignature);
            }
        } else if self.signature.is_none() {
            return Err(TxLogError::MissingSignature);
        }
        Ok(())
    }
}

/// Transaction operation types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionOperation {
    /// Graph transformation
    GraphTransformation {
        input_refs: Vec<DefRef>,
        output_ref: DefRef,
        rule_ref: DefRef,
        strategy_ref: Option<DefRef>,
    },
    /// Schema migration
    SchemaMigration {
        from_schema: DefRef,
        to_schema: DefRef,
        migration_rules: Vec<DefRef>,
    },
    /// Definition registration
    DefinitionRegistration {
        def_ref: DefRef,
        definition_type: DefinitionType,
    },
    /// Witness validation
    WitnessValidation {
        witness_refs: Vec<DefRef>,
        validation_result: bool,
    },
}

/// Definition type for registration operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DefinitionType {
    /// Function definition
    Function,
    /// Type definition
    Type,
    /// Rule definition
    Rule,
    /// Strategy definition
    Strategy,
    /// Schema definition
    Schema,
}

/// Hybrid Logical Clock for causal ordering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HLC {
    /// Physical timestamp (milliseconds since epoch)
    pub physical: u64,
    /// Logical counter
    pub logical: u32,
    /// Node ID
    pub node_id: String,
}

impl HLC {
    /// Create a new HLC
    pub fn new(node_id: String) -> Self {
        let physical = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        Self {
            physical,
            logical: 0,
            node_id,
        }
    }

    /// Check if HLC is valid
    pub fn is_valid(&self) -> bool {
        !self.node_id.is_empty() && self.physical > 0
    }

    /// Compare two HLC timestamps
    pub fn compare(&self, other: &HLC) -> std::cmp::Ordering {
        match self.physical.cmp(&other.physical) {
            std::cmp::Ordering::Equal => self.logical.cmp(&other.logical),
            other => other,
        }
    }

    /// Update HLC with knowledge of another timestamp
    pub fn update_with(&mut self, other: &HLC) {
        if other.physical > self.physical {
            self.physical = other.physical;
            self.logical = other.logical + 1;
        } else if other.physical == self.physical {
            self.logical = self.logical.max(other.logical) + 1;
        }
    }
}

/// Provenance chain for tracking causality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceChain {
    /// Starting DefRef
    pub start_ref: DefRef,
    /// Chain of transactions and operations
    pub chain: Vec<ProvenanceLink>,
    /// Final result
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

    /// Check if the chain is valid
    pub fn is_valid(&self) -> bool {
        // Implementation would verify the chain integrity
        true
    }
}

/// Link in the provenance chain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceLink {
    /// Transaction that performed the operation
    pub transaction_ref: TransactionRef,
    /// Operation performed
    pub operation: TransactionOperation,
    /// Input references
    pub inputs: Vec<DefRef>,
    /// Output reference
    pub output: DefRef,
    /// Timestamp
    pub timestamp: HLC,
}

/// Transaction log configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TxLogConfig {
    /// Require signatures on transactions
    pub require_signatures: bool,
    /// Maximum transaction chain length
    pub max_chain_length: Option<usize>,
    /// Enable provenance tracking
    pub enable_provenance: bool,
    /// Enable witness validation
    pub enable_witnesses: bool,
    /// Cache size for recent transactions
    pub cache_size: usize,
}

impl Default for TxLogConfig {
    fn default() -> Self {
        Self {
            require_signatures: true,
            max_chain_length: Some(1000),
            enable_provenance: true,
            enable_witnesses: true,
            cache_size: 10000,
        }
    }
}

/// Transaction log error
#[derive(Debug, Clone)]
pub enum TxLogError {
    /// Transaction not found
    TransactionNotFound(TransactionRef),
    /// Parent transaction not found
    ParentNotFound(TransactionRef),
    /// Invalid HLC timestamp
    InvalidHLC,
    /// Invalid signature
    InvalidSignature,
    /// Missing signature
    MissingSignature,
    /// Provenance chain too long
    ChainTooLong,
    /// Integrity verification failed
    IntegrityFailed(String),
    /// DAG inconsistency
    DAGInconsistency(String),
}

impl std::fmt::Display for TxLogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxLogError::TransactionNotFound(tx_ref) => write!(f, "Transaction not found: {}", tx_ref.tx_id),
            TxLogError::ParentNotFound(tx_ref) => write!(f, "Parent transaction not found: {}", tx_ref.tx_id),
            TxLogError::InvalidHLC => write!(f, "Invalid HLC timestamp"),
            TxLogError::InvalidSignature => write!(f, "Invalid signature"),
            TxLogError::MissingSignature => write!(f, "Missing signature"),
            TxLogError::ChainTooLong => write!(f, "Provenance chain too long"),
            TxLogError::IntegrityFailed(msg) => write!(f, "Integrity verification failed: {}", msg),
            TxLogError::DAGInconsistency(msg) => write!(f, "DAG inconsistency: {}", msg),
        }
    }
}

impl std::error::Error for TxLogError {}

/// Integrity report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    /// DAG integrity status
    pub dag_integrity: bool,
    /// Provenance integrity status
    pub provenance_integrity: bool,
    /// Witness integrity status
    pub witness_integrity: bool,
    /// Overall integrity status
    pub overall_integrity: bool,
}

impl IntegrityReport {
    /// Check if all components are intact
    pub fn is_intact(&self) -> bool {
        self.overall_integrity
    }

    /// Get failed components
    pub fn failed_components(&self) -> Vec<String> {
        let mut failed = Vec::new();

        if !self.dag_integrity {
            failed.push("DAG".to_string());
        }
        if !self.provenance_integrity {
            failed.push("Provenance".to_string());
        }
        if !self.witness_integrity {
            failed.push("Witnesses".to_string());
        }

        failed
    }
}

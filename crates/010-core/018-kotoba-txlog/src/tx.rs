//! # Transaction Definitions and Operations
//!
//! This module provides transaction definitions and core operations
//! for the transaction log system.

use super::*;
use kotoba_types::*;
use kotoba_codebase::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Transaction with full metadata
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
    /// Transaction hash (computed)
    pub hash: Option<Hash>,
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
            hash: None,
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

    /// Compute transaction hash
    pub fn compute_hash(&self) -> Hash {
        let content = serde_json::to_vec(self).expect("Failed to serialize transaction");
        Hash::from_sha256(&content)
    }

    /// Get transaction hash (compute if not cached)
    pub fn get_hash(&mut self) -> Hash {
        if let Some(ref hash) = self.hash {
            hash.clone()
        } else {
            let hash = self.compute_hash();
            self.hash = Some(hash.clone());
            hash
        }
    }

    /// Verify signatures
    pub fn verify_signatures(&self) -> Result<(), TxLogError> {
        if let Some(ref signature) = self.signature {
            // Implementation would verify the signature using the author's public key
            // For now, just check if it exists and is not empty
            if signature.is_empty() {
                return Err(TxLogError::InvalidSignature);
            }
        } else {
            return Err(TxLogError::MissingSignature);
        }
        Ok(())
    }

    /// Verify transaction integrity
    pub fn verify_integrity(&self) -> Result<(), TxLogError> {
        // Verify HLC
        if !self.hlc.is_valid() {
            return Err(TxLogError::InvalidHLC);
        }

        // Verify signature if present
        if self.signature.is_some() {
            self.verify_signatures()?;
        }

        // Verify operation-specific constraints
        self.verify_operation()?;

        Ok(())
    }

    /// Verify operation-specific constraints
    fn verify_operation(&self) -> Result<(), TxLogError> {
        match &self.operation {
            TransactionOperation::GraphTransformation {
                input_refs,
                output_ref,
                rule_ref,
                strategy_ref,
            } => {
                // Verify all references are valid DefRefs
                if input_refs.is_empty() {
                    return Err(TxLogError::IntegrityFailed(
                        "Graph transformation must have at least one input".to_string()
                    ));
                }

                if output_ref.def_type != DefType::Function {
                    return Err(TxLogError::IntegrityFailed(
                        "Graph transformation output must be a function".to_string()
                    ));
                }

                if rule_ref.def_type != DefType::Rule {
                    return Err(TxLogError::IntegrityFailed(
                        "Graph transformation rule must be a rule".to_string()
                    ));
                }

                if let Some(strategy_ref) = strategy_ref {
                    if strategy_ref.def_type != DefType::Strategy {
                        return Err(TxLogError::IntegrityFailed(
                            "Graph transformation strategy must be a strategy".to_string()
                        ));
                    }
                }
            },
            TransactionOperation::SchemaMigration {
                from_schema,
                to_schema,
                migration_rules,
            } => {
                if from_schema.def_type != DefType::Schema {
                    return Err(TxLogError::IntegrityFailed(
                        "Migration source must be a schema".to_string()
                    ));
                }

                if to_schema.def_type != DefType::Schema {
                    return Err(TxLogError::IntegrityFailed(
                        "Migration target must be a schema".to_string()
                    ));
                }

                for rule_ref in migration_rules {
                    if rule_ref.def_type != DefType::Rule {
                        return Err(TxLogError::IntegrityFailed(
                            "Migration rules must be rules".to_string()
                        ));
                    }
                }
            },
            TransactionOperation::DefinitionRegistration {
                def_ref,
                definition_type,
            } => {
                // Verify the DefRef type matches the declared type
                let expected_type = match definition_type {
                    DefinitionType::Function => DefType::Function,
                    DefinitionType::Type => DefType::Type,
                    DefinitionType::Rule => DefType::Rule,
                    DefinitionType::Strategy => DefType::Strategy,
                    DefinitionType::Schema => DefType::Schema,
                };

                if def_ref.def_type != expected_type {
                    return Err(TxLogError::IntegrityFailed(
                        format!("DefRef type {} does not match declared type {:?}",
                               def_ref.def_type, expected_type)
                    ));
                }
            },
            TransactionOperation::WitnessValidation {
                witness_refs,
                validation_result,
            } => {
                if witness_refs.is_empty() {
                    return Err(TxLogError::IntegrityFailed(
                        "Witness validation must have witnesses".to_string()
                    ));
                }

                // Validation result should be stored as metadata
                if !*validation_result {
                    return Err(TxLogError::IntegrityFailed(
                        "Witness validation failed".to_string()
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get transaction dependencies
    pub fn dependencies(&self) -> Vec<DefRef> {
        match &self.operation {
            TransactionOperation::GraphTransformation { input_refs, .. } => {
                input_refs.clone()
            },
            TransactionOperation::SchemaMigration { from_schema, to_schema, migration_rules } => {
                let mut deps = vec![from_schema.clone(), to_schema.clone()];
                deps.extend(migration_rules.iter().cloned());
                deps
            },
            TransactionOperation::DefinitionRegistration { def_ref, .. } => {
                vec![def_ref.clone()]
            },
            TransactionOperation::WitnessValidation { witness_refs, .. } => {
                witness_refs.clone()
            }
        }
    }

    /// Get transaction outputs
    pub fn outputs(&self) -> Vec<DefRef> {
        match &self.operation {
            TransactionOperation::GraphTransformation { output_ref, .. } => {
                vec![output_ref.clone()]
            },
            TransactionOperation::SchemaMigration { to_schema, .. } => {
                vec![to_schema.clone()]
            },
            TransactionOperation::DefinitionRegistration { def_ref, .. } => {
                vec![def_ref.clone()]
            },
            TransactionOperation::WitnessValidation { .. } => {
                Vec::new() // Validation doesn't produce new DefRefs
            }
        }
    }

    /// Check if transaction affects a specific DefRef
    pub fn affects(&self, def_ref: &DefRef) -> bool {
        self.dependencies().contains(def_ref) || self.outputs().contains(def_ref)
    }

    /// Get transaction summary
    pub fn summary(&self) -> TransactionSummary {
        TransactionSummary {
            tx_id: self.tx_id.clone(),
            author: self.author.clone(),
            operation_type: self.operation.operation_type(),
            timestamp: self.hlc.physical,
            parent_count: self.parents.len(),
            signature_present: self.signature.is_some(),
            witness_count: self.witnesses.len(),
        }
    }
}

/// Transaction summary for quick overview
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionSummary {
    /// Transaction ID
    pub tx_id: String,
    /// Author
    pub author: String,
    /// Operation type
    pub operation_type: String,
    /// Timestamp
    pub timestamp: u64,
    /// Number of parents
    pub parent_count: usize,
    /// Whether signature is present
    pub signature_present: bool,
    /// Number of witnesses
    pub witness_count: usize,
}

/// Transaction operation with helper methods
impl TransactionOperation {
    /// Get operation type as string
    pub fn operation_type(&self) -> String {
        match self {
            TransactionOperation::GraphTransformation { .. } => "graph_transformation".to_string(),
            TransactionOperation::SchemaMigration { .. } => "schema_migration".to_string(),
            TransactionOperation::DefinitionRegistration { .. } => "definition_registration".to_string(),
            TransactionOperation::WitnessValidation { .. } => "witness_validation".to_string(),
        }
    }

    /// Check if operation is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, TransactionOperation::WitnessValidation { .. })
    }

    /// Get operation cost estimate
    pub fn estimated_cost(&self) -> f64 {
        match self {
            TransactionOperation::GraphTransformation { .. } => 10.0,
            TransactionOperation::SchemaMigration { migration_rules, .. } => {
                5.0 + migration_rules.len() as f64 * 2.0
            },
            TransactionOperation::DefinitionRegistration { .. } => 1.0,
            TransactionOperation::WitnessValidation { witness_refs, .. } => {
                witness_refs.len() as f64 * 0.5
            }
        }
    }
}

/// Transaction builder for fluent API
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    tx: Transaction,
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new(tx_id: String, author: String) -> Self {
        let hlc = HLC::new("default".to_string());
        let tx = Transaction::new(tx_id, hlc, Vec::new(), author, TransactionOperation::DefinitionRegistration {
            def_ref: DefRef::new(&[], DefType::Type),
            definition_type: DefinitionType::Type,
        });

        Self { tx }
    }

    /// Set HLC timestamp
    pub fn with_hlc(mut self, hlc: HLC) -> Self {
        self.tx.hlc = hlc;
        self
    }

    /// Add parent transactions
    pub fn with_parents(mut self, parents: Vec<TransactionRef>) -> Self {
        self.tx.parents = parents;
        self
    }

    /// Set operation
    pub fn with_operation(mut self, operation: TransactionOperation) -> Self {
        self.tx.operation = operation;
        self
    }

    /// Add witness
    pub fn with_witness(mut self, witness: DefRef) -> Self {
        self.tx.witnesses.push(witness);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.tx.metadata.insert(key, value);
        self
    }

    /// Build the transaction
    pub fn build(self) -> Transaction {
        self.tx
    }
}

/// Transaction pool for managing pending transactions
#[derive(Debug, Clone)]
pub struct TransactionPool {
    /// Pending transactions
    pub pending: HashMap<TransactionRef, Transaction>,
    /// Transaction order
    pub order: Vec<TransactionRef>,
    /// Maximum pool size
    pub max_size: usize,
}

impl TransactionPool {
    /// Create a new transaction pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pending: HashMap::new(),
            order: Vec::new(),
            max_size,
        }
    }

    /// Add a transaction to the pool
    pub fn add_transaction(&mut self, tx: Transaction) -> Result<(), TxLogError> {
        let tx_ref = TransactionRef::from_transaction(&tx);

        if self.pending.len() >= self.max_size {
            // Remove oldest transaction if pool is full
            if let Some(oldest_ref) = self.order.first().cloned() {
                self.pending.remove(&oldest_ref);
                self.order.remove(0);
            }
        }

        self.pending.insert(tx_ref.clone(), tx);
        self.order.push(tx_ref);

        Ok(())
    }

    /// Get a transaction from the pool
    pub fn get_transaction(&self, tx_ref: &TransactionRef) -> Option<&Transaction> {
        self.pending.get(tx_ref)
    }

    /// Remove a transaction from the pool
    pub fn remove_transaction(&mut self, tx_ref: &TransactionRef) -> Option<Transaction> {
        let tx = self.pending.remove(tx_ref);
        if let Some(ref tx) = tx {
            let tx_ref_copy = TransactionRef::from_transaction(tx);
            if let Some(pos) = self.order.iter().position(|r| r == &tx_ref_copy) {
                self.order.remove(pos);
            }
        }
        tx
    }

    /// Get all pending transactions
    pub fn get_all(&self) -> Vec<&Transaction> {
        self.order.iter()
            .filter_map(|tx_ref| self.pending.get(tx_ref))
            .collect()
    }

    /// Clear the pool
    pub fn clear(&mut self) {
        self.pending.clear();
        self.order.clear();
    }

    /// Get pool size
    pub fn size(&self) -> usize {
        self.pending.len()
    }

    /// Check if pool is full
    pub fn is_full(&self) -> bool {
        self.pending.len() >= self.max_size
    }
}

/// Transaction validator
#[derive(Debug, Clone)]
pub struct TransactionValidator {
    /// Validation rules
    pub rules: ValidationRules,
}

impl TransactionValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            rules: ValidationRules::default(),
        }
    }

    /// Validate a transaction
    pub fn validate(&self, tx: &Transaction) -> Result<ValidationResult, TxLogError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check HLC validity
        if !tx.hlc.is_valid() {
            errors.push("Invalid HLC timestamp".to_string());
        }

        // Check parent references
        if tx.parents.is_empty() && !tx.tx_id.starts_with("genesis") {
            warnings.push("Transaction has no parents".to_string());
        }

        // Check operation validity
        if let Err(e) = tx.verify_operation() {
            errors.push(format!("Operation validation failed: {}", e));
        }

        // Check signature
        if tx.signature.is_none() {
            warnings.push("Transaction is not signed".to_string());
        }

        // Check metadata size
        if serde_json::to_string(&tx.metadata)?.len() > 1024 * 1024 {
            warnings.push("Transaction metadata is very large".to_string());
        }

        Ok(ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
}

/// Validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    /// Require signatures
    pub require_signatures: bool,
    /// Maximum metadata size
    pub max_metadata_size: usize,
    /// Allow unsigned transactions
    pub allow_unsigned: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            require_signatures: true,
            max_metadata_size: 1024 * 1024, // 1MB
            allow_unsigned: false,
        }
    }
}

/// Transaction statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TransactionStats {
    /// Total transactions processed
    pub total_count: usize,
    /// Transactions by operation type
    pub by_operation: HashMap<String, usize>,
    /// Average transaction size
    pub avg_size: f64,
    /// Validation failure rate
    pub validation_failure_rate: f64,
    /// Average processing time
    pub avg_processing_time: std::time::Duration,
}

impl TransactionStats {
    /// Update statistics with new transaction
    pub fn update(&mut self, tx: &Transaction, processing_time: std::time::Duration) {
        self.total_count += 1;
        self.by_operation.insert(
            tx.operation.operation_type(),
            self.by_operation.get(&tx.operation.operation_type()).unwrap_or(&0) + 1
        );

        // Update average size
        let size = serde_json::to_vec(tx).map(|v| v.len()).unwrap_or(0);
        self.avg_size = (self.avg_size * (self.total_count - 1) as f64 + size as f64) / self.total_count as f64;

        // Update average processing time
        self.avg_processing_time = (self.avg_processing_time * (self.total_count - 1) as u32 + processing_time) / self.total_count as u32;
    }
}

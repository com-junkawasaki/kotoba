//! # Rule Definitions
//!
//! This module provides rule definitions for graph rewriting.

use super::*;
use serde::{Deserialize, Serialize};

/// Rule definition for graph rewriting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleDef {
    /// Rule name
    pub name: String,
    /// Left-hand side pattern
    pub lhs: GraphPattern,
    /// Right-hand side replacement
    pub rhs: GraphPattern,
    /// Rule metadata
    pub metadata: RuleMetadata,
}

impl RuleDef {
    /// Create a new rule definition
    pub fn new(name: String, lhs: GraphPattern, rhs: GraphPattern) -> Self {
        Self {
            name,
            lhs,
            rhs,
            metadata: RuleMetadata::default(),
        }
    }

    /// Set rule metadata
    pub fn with_metadata(mut self, metadata: RuleMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Graph pattern for matching and replacement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    /// Nodes in the pattern
    pub nodes: Vec<PatternNode>,
    /// Edges in the pattern
    pub edges: Vec<PatternEdge>,
    /// Conditions that must hold
    pub conditions: Vec<PatternCondition>,
}

/// Pattern node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternNode {
    /// Node ID in the pattern
    pub id: String,
    /// Node type constraint
    pub node_type: Option<TypeDef>,
    /// Node attributes
    pub attributes: Vec<PatternAttribute>,
}

/// Pattern edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternEdge {
    /// Source node ID
    pub source: String,
    /// Edge label
    pub label: String,
    /// Target node ID
    pub target: String,
    /// Edge attributes
    pub attributes: Vec<PatternAttribute>,
}

/// Pattern attribute
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternAttribute {
    /// Attribute key
    pub key: String,
    /// Attribute value pattern
    pub value: PatternValue,
}

/// Pattern value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternValue {
    /// Literal value
    Literal(Value),
    /// Variable binding
    Variable(String),
    /// Predicate function
    Predicate(String, Vec<PatternValue>),
}

/// Pattern condition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternCondition {
    /// Node existence
    NodeExists(String),
    /// Edge existence
    EdgeExists(String, String, String),
    /// Attribute equality
    AttributeEquals(String, String, Value),
    /// Path existence
    PathExists(String, Vec<String>, String),
    /// Custom predicate
    Predicate(String, Vec<String>),
}

/// Rule metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleMetadata {
    /// Rule description
    pub description: Option<String>,
    /// Rule priority
    pub priority: i32,
    /// Rule cost
    pub cost: Option<f64>,
    /// Rule properties
    pub properties: RuleProperties,
    /// Dependencies on other rules
    pub dependencies: Vec<DefRef>,
}

/// Rule properties
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuleProperties {
    /// Is the rule idempotent?
    pub idempotent: bool,
    /// Is the rule commutative with other rules?
    pub commutative: bool,
    /// Does the rule have an inverse?
    pub has_inverse: bool,
    /// Is the rule linear (no variable reuse)?
    pub linear: bool,
    /// Maximum number of matches
    pub max_matches: Option<usize>,
    /// Parallel execution safety
    pub parallel_safe: bool,
}

/// Rule execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExecutionReport {
    /// Rule that was executed
    pub rule_def: DefRef,
    /// Number of matches found
    pub match_count: usize,
    /// Number of applications performed
    pub application_count: usize,
    /// Execution time in nanoseconds
    pub execution_time_ns: u64,
    /// Memory usage in bytes
    pub memory_usage: Option<u64>,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

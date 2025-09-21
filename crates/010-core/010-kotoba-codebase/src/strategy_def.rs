//! # Strategy Definitions
//!
//! This module provides strategy definitions for rule application ordering.

use super::*;
use serde::{Deserialize, Serialize};

/// Strategy definition for rule application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDef {
    /// Strategy name
    pub name: String,
    /// Strategy type
    pub strategy_type: StrategyType,
    /// Rule ordering
    pub rule_order: RuleOrdering,
    /// Strategy metadata
    pub metadata: StrategyMetadata,
}

impl StrategyDef {
    /// Create a new strategy definition
    pub fn new(name: String, strategy_type: StrategyType) -> Self {
        Self {
            name,
            strategy_type,
            rule_order: RuleOrdering::default(),
            metadata: StrategyMetadata::default(),
        }
    }
}

/// Strategy type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    /// Sequential application
    Sequential,
    /// Parallel application
    Parallel,
    /// Layered application (phases)
    Layered(Vec<StrategyPhase>),
    /// Conditional application
    Conditional {
        condition: DefRef,
        then_strategy: Box<StrategyDef>,
        else_strategy: Box<StrategyDef>,
    },
    /// Prioritized application
    Prioritized(PriorityQueue),
    /// Custom strategy function
    Custom(DefRef),
}

/// Strategy phase for layered strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPhase {
    /// Phase name
    pub name: String,
    /// Rules in this phase
    pub rules: Vec<DefRef>,
    /// Phase priority
    pub priority: i32,
    /// Dependencies on other phases
    pub dependencies: Vec<String>,
}

/// Rule ordering specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleOrdering {
    /// Rules applied in the order given
    Ordered(Vec<DefRef>),
    /// Rules applied in priority order
    PriorityOrder,
    /// Rules applied in dependency order
    DependencyOrder,
    /// Rules applied in random order
    RandomOrder,
    /// Custom ordering function
    Custom(DefRef),
}

impl Default for RuleOrdering {
    fn default() -> Self {
        RuleOrdering::PriorityOrder
    }
}

/// Priority queue for prioritized strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueue {
    /// Rules with their priorities
    pub rules: Vec<(DefRef, i32)>,
}

/// Strategy metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyMetadata {
    /// Strategy description
    pub description: Option<String>,
    /// Expected performance characteristics
    pub performance: PerformanceCharacteristics,
    /// Strategy properties
    pub properties: StrategyProperties,
}

/// Performance characteristics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerformanceCharacteristics {
    /// Expected time complexity
    pub time_complexity: Option<String>,
    /// Expected space complexity
    pub space_complexity: Option<String>,
    /// Parallelization potential
    pub parallelization_factor: Option<f64>,
    /// Memory usage estimate
    pub memory_estimate: Option<u64>,
}

/// Strategy properties
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StrategyProperties {
    /// Is the strategy deterministic?
    pub deterministic: bool,
    /// Is the strategy complete (guarantees termination)?
    pub complete: bool,
    /// Is the strategy confluent (order-independent)?
    pub confluent: bool,
    /// Maximum number of iterations
    pub max_iterations: Option<usize>,
    /// Timeout in milliseconds
    pub timeout_ms: Option<u64>,
}

/// Strategy execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyExecutionReport {
    /// Strategy that was executed
    pub strategy_def: DefRef,
    /// Rules executed
    pub rules_executed: Vec<RuleExecutionReport>,
    /// Total execution time
    pub total_time_ns: u64,
    /// Strategy convergence status
    pub converged: bool,
    /// Number of iterations performed
    pub iterations: usize,
    /// Final state hash
    pub final_state: Hash,
}

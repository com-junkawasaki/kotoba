//! # Strategy Definitions
//!
//! This module provides strategy definitions for rule application ordering.

use kotoba_codebase::{DefRef, DefType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strategy definition for rule application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyDef {
    /// Strategy name
    pub name: String,
    /// Strategy type
    pub strategy_type: StrategyType,
    /// Metadata
    pub metadata: StrategyMetadata,
}

/// Strategy metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMetadata {
    pub description: String,
    pub version: String,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Strategy execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyExecutionResult {
    /// Strategy that was executed
    pub strategy_ref: DefRef,
    /// Rules applied during execution
    pub rules_applied: Vec<crate::rule_def::ExecutionRecord>,
    /// Final state hash
    pub final_state: Option<kotoba_codebase::Hash>,
    /// Execution statistics
    pub statistics: ExecutionStatistics,
}

/// Rule execution report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleExecutionReport {
    /// Rule reference
    pub rule_ref: DefRef,
    /// Application count
    pub applications: usize,
    /// Execution time (nanoseconds)
    pub execution_time: u64,
    /// Success/failure status
    pub status: ExecutionStatus,
}

/// Execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStatistics {
    /// Total rules processed
    pub total_rules: usize,
    /// Rules successfully applied
    pub successful_applications: usize,
    /// Rules failed to apply
    pub failed_applications: usize,
    /// Total execution time (nanoseconds)
    pub total_time: u64,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Failed(String),
}

/// Strategy types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyType {
    /// Sequential execution
    Sequential,
    /// Parallel execution
    Parallel,
    /// Layered execution with phases
    Layered(Vec<StrategyPhase>),
    /// Conditional execution
    Conditional {
        condition: DefRef,
        then_strategy: Box<StrategyDef>,
        else_strategy: Box<StrategyDef>,
    },
    /// Prioritized execution
    Prioritized(PriorityQueue),
    /// Custom strategy
    Custom(DefRef),
}

/// Strategy phase for layered execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyPhase {
    /// Phase name
    pub name: String,
    /// Rules to execute in this phase
    pub rules: Vec<DefRef>,
    /// Dependencies on other phases
    pub dependencies: Vec<String>,
}

/// Priority queue for prioritized strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityQueue {
    /// Priority-ordered rules
    pub rules: Vec<(DefRef, i32)>, // (rule_ref, priority)
}

/// Rule ordering strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleOrdering {
    /// Execute rules in specified order
    Ordered(Vec<DefRef>),
    /// Execute rules in priority order
    PriorityOrder,
    /// Execute rules in dependency order
    DependencyOrder,
    /// Execute rules in random order
    RandomOrder,
    /// Custom ordering
    Custom(DefRef),
}

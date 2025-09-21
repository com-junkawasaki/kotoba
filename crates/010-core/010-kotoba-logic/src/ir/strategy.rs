//! Strategy-IR（極小戦略表現）

use serde::{Deserialize, Serialize};
use crate::types::*;

/// Strategy operator
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum StrategyOp {
    /// Apply once
    Once {
        rule: String,  // Rule name or hash
    },

    /// Exhaustive application
    Exhaust {
        rule: String,
        #[serde(default)]
        order: Order,
        #[serde(skip_serializing_if = "Option::is_none")]
        measure: Option<String>,
    },

    /// Conditional repetition
    While {
        rule: String,
        pred: String,  // Predicate name
        #[serde(default)]
        order: Order,
    },

    /// Sequential execution
    Seq {
        strategies: Vec<Box<StrategyOp>>,
    },

    /// Choice (first successful)
    Choice {
        strategies: Vec<Box<StrategyOp>>,
    },

    /// Priority-based choice
    Priority {
        strategies: Vec<PrioritizedStrategy>,
    },
}

/// Prioritized strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedStrategy {
    /// Strategy
    pub strategy: Box<StrategyOp>,
    /// Priority
    pub priority: i32,
}

/// Application order
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Order {
    #[default]
    #[serde(rename = "topdown")]
    TopDown,

    #[serde(rename = "bottomup")]
    BottomUp,

    #[serde(rename = "fair")]
    Fair,
}

/// Strategy IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyIR {
    /// Strategy
    pub strategy: StrategyOp,
}

impl StrategyIR {
    /// Create a new strategy
    pub fn new(strategy: StrategyOp) -> Self {
        Self { strategy }
    }
}

/// Strategy execution result
#[derive(Debug, Clone)]
pub struct StrategyResult {
    /// Applied count
    pub applied_count: usize,
    /// Final graph
    pub final_graph: GraphRef_,
    /// Applied patches
    pub patches: Vec<crate::ir::patch::Patch>,
}

/// External predicates/measures trait
pub trait Externs {
    /// Check if degree is greater than or equal to k
    fn deg_ge(&self, v: VertexId, k: u32) -> bool;

    /// Check if edge count is non-increasing (termination measure)
    fn edge_count_nonincreasing(&self, g0: &GraphRef_, g1: &GraphRef_) -> bool;

    /// Custom predicate
    fn custom_pred(&self, name: &str, args: &[crate::ir::Value]) -> bool;
}

/// Graph reference for strategy execution
#[derive(Debug, Clone)]
pub struct GraphRef_ {
    /// Graph instance
    pub graph: GraphInstance,
}

impl GraphRef_ {
    /// Create a new graph reference
    pub fn new(graph: GraphInstance) -> Self {
        Self { graph }
    }

    /// Get vertex count
    pub fn vertex_count(&self) -> usize {
        self.graph.core.nodes.len()
    }

    /// Get edge count
    pub fn edge_count(&self) -> usize {
        self.graph.core.edges.len()
    }
}

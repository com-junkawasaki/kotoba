//! # Function Definitions
//!
//! This module provides function definitions for the Kotoba codebase.

use super::*;
use serde::{Deserialize, Serialize};

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDef {
    /// Function name
    pub name: String,
    /// Input types
    pub inputs: Vec<TypeDef>,
    /// Output type
    pub output: TypeDef,
    /// Function body (implementation)
    pub body: FunctionBody,
    /// Metadata
    pub metadata: FunctionMetadata,
}

impl FunctionDef {
    /// Create a new function definition
    pub fn new(name: String, inputs: Vec<TypeDef>, output: TypeDef, body: FunctionBody) -> Self {
        Self {
            name,
            inputs,
            output,
            body,
            metadata: FunctionMetadata::default(),
        }
    }
}

/// Function body implementation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FunctionBody {
    /// External function (native implementation)
    External(String),
    /// Graph rewrite rules
    GraphRewrite(Vec<RuleDef>),
    /// Composition of other functions
    Composition(Vec<DefRef>),
    /// Conditional execution
    Conditional {
        condition: DefRef,
        then_branch: DefRef,
        else_branch: DefRef,
    },
}

/// Function metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FunctionMetadata {
    /// Function description
    pub description: Option<String>,
    /// Complexity measure
    pub complexity: Option<u64>,
    /// Optimization hints
    pub optimization_hints: Vec<OptimizationHint>,
    /// Cost model parameters
    pub cost_params: HashMap<String, f64>,
}

/// Optimization hints for function compilation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationHint {
    /// This function is pure (no side effects)
    Pure,
    /// This function is idempotent
    Idempotent,
    /// This function is commutative with the given function
    Commutative(DefRef),
    /// This function has an inverse
    HasInverse(DefRef),
    /// This function is associative
    Associative,
    /// Custom hint
    Custom(String),
}

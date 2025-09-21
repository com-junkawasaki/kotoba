//! Rule-IR（DPO型付き属性グラフ書換え）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use kotoba_types::{VertexId, Value as KotobaValue, Properties, PropertyKey, Label};

/// Guard condition (named predicate)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    /// Predicate name (e.g., "deg_ge")
    pub ref_: String,
    /// Arguments
    pub args: HashMap<String, KotobaValue>,
}

/// Graph pattern element
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphElement {
    /// Variable name
    pub id: String,
    /// Type (optional)
    pub type_: Option<Label>,
    /// Properties (optional)
    pub props: Option<Properties>,
}

/// Edge definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    /// Edge ID
    pub id: String,
    /// Source vertex
    pub src: String,
    /// Destination vertex
    pub dst: String,
    /// Type (optional)
    pub type_: Option<Label>,
}

/// Graph pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    /// Nodes
    pub nodes: Vec<GraphElement>,
    /// Edges
    pub edges: Vec<EdgeDef>,
}

/// Negative Application Condition (NAC) for rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleNac {
    /// Nodes
    pub nodes: Vec<GraphElement>,
    /// Edges
    pub edges: Vec<EdgeDef>,
}

/// DPO rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleIR {
    /// Rule name
    pub name: String,
    /// Type definitions
    pub types: HashMap<String, Vec<Label>>,
    /// Left-hand side (L)
    pub lhs: GraphPattern,
    /// Context (K)
    pub context: GraphPattern,
    /// Right-hand side (R)
    pub rhs: GraphPattern,
    /// Negative conditions
    pub nacs: Vec<RuleNac>,
    /// Guard conditions
    pub guards: Vec<Guard>,
}

impl RuleIR {
    /// Create a new rule
    pub fn new(name: String) -> Self {
        Self {
            name,
            types: HashMap::new(),
            lhs: GraphPattern::new(),
            context: GraphPattern::new(),
            rhs: GraphPattern::new(),
            nacs: Vec::new(),
            guards: Vec::new(),
        }
    }

    /// Add type definition
    pub fn with_type(mut self, name: String, labels: Vec<Label>) -> Self {
        self.types.insert(name, labels);
        self
    }

    /// Add guard condition
    pub fn with_guard(mut self, guard: Guard) -> Self {
        self.guards.push(guard);
        self
    }

    /// Add NAC
    pub fn with_nac(mut self, nac: RuleNac) -> Self {
        self.nacs.push(nac);
        self
    }
}

/// Rule match result
#[derive(Debug, Clone)]
pub struct Match {
    /// Variable to vertex ID mapping
    pub mapping: HashMap<String, VertexId>,
    /// Match score
    pub score: f64,
}

/// Multiple match results
pub type Matches = Vec<Match>;

//! Core types for EAF-IPG (ENGI EAF-IPG)
//!
//! Unified IR types for AST, dataflow, control flow, memory, typing, effects, time, and capabilities.

use serde::{Deserialize, Serialize};
use indexmap::IndexMap;
use std::collections::HashMap;

/// Layer types in the EAF-IPG model
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    Syntax,   // Ordered AST representation
    Data,     // SSA/dataflow dependencies
    Control,  // CFG edges and branching
    Memory,   // MemorySSA and alias classes
    Typing,   // Type inference and checking
    Effect,   // Algebraic effects and purity
    Time,     // Happens-before relationships
    Capability, // CHERI-style memory capabilities
}

impl Layer {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "syntax" => Some(Self::Syntax),
            "data" => Some(Self::Data),
            "control" => Some(Self::Control),
            "memory" => Some(Self::Memory),
            "typing" => Some(Self::Typing),
            "effect" => Some(Self::Effect),
            "time" => Some(Self::Time),
            "capability" => Some(Self::Capability),
            _ => None,
        }
    }
}

/// Node in the EAF-IPG graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub properties: IndexMap<String, serde_json::Value>,
}

/// Edge in the EAF-IPG graph
#[derive(Debug, Clone)]
pub struct Edge {
    pub id: String,
    pub layer: Layer,
    pub kind: String,
    pub properties: IndexMap<String, serde_json::Value>,
}

impl<'de> Deserialize<'de> for Edge {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct EdgeHelper {
            id: String,
            layer: String,
            #[serde(rename = "type")]
            kind: String,
            properties: IndexMap<String, serde_json::Value>,
        }

        let helper = EdgeHelper::deserialize(deserializer)?;
        let layer = Layer::from_str(&helper.layer)
            .ok_or_else(|| serde::de::Error::custom(format!("Unknown layer: {}", helper.layer)))?;

        Ok(Edge {
            id: helper.id,
            layer,
            kind: helper.kind,
            properties: helper.properties,
        })
    }
}

impl Serialize for Edge {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;

        let mut state = serializer.serialize_struct("Edge", 4)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("layer", &serde_json::to_string(&self.layer).map_err(serde::ser::Error::custom)?)?;
        state.serialize_field("type", &self.kind)?;
        state.serialize_field("properties", &self.properties)?;
        state.end()
    }
}

/// Incidence relationship between nodes and edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incidence {
    pub node: String,
    pub edge: String,
    #[serde(rename = "type")]
    pub role: String, // "source", "target", "cap_in", "cap_out", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pos: Option<usize>, // Position for ordered arguments
    pub properties: IndexMap<String, serde_json::Value>,
}

/// Complete EAF-IPG graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    pub node: Vec<Node>,
    pub edge: Vec<Edge>,
    pub incidence: Vec<Incidence>,
}

/// Execution DAG node types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpKind {
    // Control flow
    Phi { arity: usize },
    Branch,
    Jump,

    // Data operations
    Load,
    Store,
    Call,

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,

    // Memory operations with capabilities
    CapLoad,
    CapStore,

    // MMIO operations
    MmioRead,
    MmioWrite,

    // Effects
    Effect { effect_type: String },
}

/// Execution DAG node
#[derive(Debug, Clone)]
pub struct ExecNode {
    pub id: String,
    pub op: OpKind,
    pub properties: IndexMap<String, serde_json::Value>,
}

/// Execution DAG edge types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecEdgeKind {
    Data,      // Data dependency
    Control,   // Control dependency
    Memory,    // Memory dependency (MemorySSA)
    Enable,    // Enabling dependency
    Time,      // Happens-before relationship
}

/// Execution DAG edge
#[derive(Debug, Clone)]
pub struct ExecEdge {
    pub from: String,
    pub to: String,
    pub kind: ExecEdgeKind,
}

/// Execution DAG
#[derive(Debug, Clone)]
pub struct ExecDag {
    pub nodes: Vec<ExecNode>,
    pub edges: Vec<ExecEdge>,
}

/// Capability structure (CHERI-style)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub base: u64,
    pub length: u64,
    pub cursor: u64,
    pub perms: Vec<String>, // ["load", "store", "execute"]
    pub tag: bool,
}

/// Runtime value types
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Capability(Capability),
    Address(u64),
}

/// Runtime state
#[derive(Debug)]
pub struct Runtime {
    pub values: HashMap<String, Value>,
    pub memory: HashMap<u64, u8>, // Simple byte-addressable memory
    pub capabilities: HashMap<String, Capability>,
}

impl Runtime {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            memory: HashMap::new(),
            capabilities: HashMap::new(),
        }
    }
}

/// Utility functions for working with graphs
impl Graph {
    /// Get node by ID
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.node.iter().find(|n| n.id == id)
    }

    /// Get edge by ID
    pub fn get_edge(&self, id: &str) -> Option<&Edge> {
        self.edge.iter().find(|e| e.id == id)
    }

    /// Get all incidences for a node
    pub fn node_incidences(&self, node_id: &str) -> Vec<&Incidence> {
        self.incidence.iter()
            .filter(|i| i.node == node_id)
            .collect()
    }

    /// Get all incidences for an edge
    pub fn edge_incidences(&self, edge_id: &str) -> Vec<&Incidence> {
        self.incidence.iter()
            .filter(|i| i.edge == edge_id)
            .collect()
    }
}

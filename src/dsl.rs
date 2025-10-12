//! DSL utilities for EAF-IPG
//!
//! Helper functions and utilities for constructing EAF-IPG graphs
//! from JSON program representations.

use indexmap::IndexMap;
use kotoba_types::*;

/// Helper functions for common DSL patterns
pub struct Dsl;

/// Constructor functions for graph elements (mirroring Jsonnet DSL)
impl Dsl {
    /// Create a graph from nodes, edges, and incidences
    pub fn graph(nodes: Vec<Node>, edges: Vec<Edge>, incidences: Vec<Incidence>) -> Graph {
        Graph { node: nodes, edge: edges, incidence: incidences }
    }

    /// Create a node
    pub fn node(id: &str, kind: &str, properties: IndexMap<String, serde_json::Value>) -> Node {
        Node {
            id: id.to_string(),
            kind: kind.to_string(),
            properties,
        }
    }

    /// Create an edge
    pub fn edge(id: &str, layer: Layer, kind: &str, properties: IndexMap<String, serde_json::Value>) -> Edge {
        Edge {
            id: id.to_string(),
            layer,
            kind: kind.to_string(),
            properties,
        }
    }

    /// Create an incidence
    pub fn incidence(node: &str, edge: &str, role: &str) -> Incidence {
        Incidence {
            node: node.to_string(),
            edge: edge.to_string(),
            role: role.to_string(),
            pos: None,
            properties: IndexMap::new(),
        }
    }

    /// Create an incidence with position
    pub fn incidence_with_pos(node: &str, edge: &str, role: &str, pos: usize) -> Incidence {
        Incidence {
            node: node.to_string(),
            edge: edge.to_string(),
            role: role.to_string(),
            pos: Some(pos),
            properties: IndexMap::new(),
        }
    }

    /// Merge two graphs
    pub fn merge_graphs(a: Graph, b: Graph) -> Graph {
        Graph {
            node: [a.node, b.node].concat(),
            edge: [a.edge, b.edge].concat(),
            incidence: [a.incidence, b.incidence].concat(),
        }
    }
}

/// Layer constants (matching Jsonnet DSL)
pub mod layers {
    use super::*;

    pub const SYNTAX: Layer = Layer::Syntax;
    pub const DATA: Layer = Layer::Data;
    pub const CONTROL: Layer = Layer::Control;
    pub const MEMORY: Layer = Layer::Memory;
    pub const TYPING: Layer = Layer::Typing;
    pub const EFFECT: Layer = Layer::Effect;
    pub const TIME: Layer = Layer::Time;
    pub const CAPABILITY: Layer = Layer::Capability;
}

/// Common node types
pub mod node_types {
    pub const PHI: &str = "Phi";
    pub const LOAD: &str = "Load";
    pub const STORE: &str = "Store";
    pub const CALL: &str = "Call";
    pub const BRANCH: &str = "Branch";
    pub const JUMP: &str = "Jump";
    pub const CAPABILITY: &str = "Capability";
    pub const MMIO: &str = "Mmio";
}

/// Common edge types
pub mod edge_types {
    pub const ARG: &str = "arg";
    pub const RESULT: &str = "result";
    pub const CONTROL: &str = "control";
    pub const DATA: &str = "data";
    pub const USE: &str = "use";
    pub const DEF: &str = "def";
}

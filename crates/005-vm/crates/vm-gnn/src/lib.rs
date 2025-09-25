//! Merkle DAG: vm_gnn
//! This crate defines the Program Interaction Hypergraph (PIH) used as
//! the core Intermediate Representation (IR) for the VM.
//!
//! The PIH model provides:
//! - **Bipartite hypergraph structure**: Events (operations) and Entities (values/states)
//! - **DPO rewriting rules**: Safe graph transformations with NACs
//! - **GNN integration**: Node embeddings for learning-based optimization
//! - **Merkle DAG compatibility**: Content-addressable and immutable structures
//!
//! ## Key Components
//!
//! - [`ProgramInteractionHypergraph`]: The main hypergraph structure
//! - [`Event`]: Operation nodes in the bipartite graph
//! - [`Entity`]: Value/state nodes in the bipartite graph
//! - [`DpoRule`]: Double Pushout rewriting rules for safe transformations
//! - [`NegativeApplicationCondition`]: NACs for prohibiting unsafe rewrites
//!
//! ## Usage
//!
//! ```rust
//! use vm_gnn::*;
//!
//! // Create a simple multiplication PIH
//! let inputs = vec![("x".to_string(), EntityKind::Val, "i32".to_string())];
//! let outputs = vec![("result".to_string(), EntityKind::Val, "i32".to_string())];
//! let constants = vec![("eight".to_string(), serde_json::json!(8))];
//!
//! let pih = convert_computation_to_pih("mul", inputs, outputs, constants);
//!
//! // Apply strength reduction rule
//! let rule = create_strength_reduction_rule();
//! // ... rule application logic would go here
//! ```

#![allow(dead_code)] // TODO: Remove this later on

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Represents a Negative Application Condition (NAC) for DPO rewriting.
/// A NAC specifies a pattern that, if present, prohibits the application of a rule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NegativeApplicationCondition {
    pub name: String,
    pub description: String,
    /// Additional incidence edges that define the forbidden pattern.
    pub forbidden_incidence: Vec<Incidence>,
    /// Additional state edges that are forbidden.
    pub forbidden_state_edges: Vec<StateEdge>,
}

/// Represents a Double Pushout (DPO) rewriting rule.
/// A DPO rule consists of a left-hand side (LHS), right-hand side (RHS), and negative application conditions (NACs).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DpoRule {
    pub name: String,
    pub description: String,
    /// Left-hand side: the pattern to match and remove.
    pub lhs: ProgramInteractionHypergraph,
    /// Right-hand side: the pattern to add after removal.
    pub rhs: ProgramInteractionHypergraph,
    /// Negative application conditions: patterns that must NOT be present for the rule to apply.
    pub nacs: Vec<NegativeApplicationCondition>,
}

// --- Node Types (Bipartite Graph) ---

/// A unique identifier for an Event node in the hypergraph.
pub type EventId = String;

/// A unique identifier for an Entity node in the hypergraph.
pub type EntityId = String;

/// Represents the `Event` part of the bipartite hypergraph.
/// An Event is an operation, such as an arithmetic operation, a function call, or a memory access.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: EventId,
    pub opcode: String,
    pub dtype: String,
    #[serde(default = "default_can_throw")]
    pub can_throw: bool,
    // Additional attributes like lanes, latency, cost, etc., can be added here.
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

fn default_can_throw() -> bool {
    false
}

/// Represents the kind of an `Entity` node.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum EntityKind {
    /// SSA value, constant, argument, or return value.
    Val,
    /// An abstract object or memory region.
    Obj,
    /// A versioned memory state (similar to Memory-SSA).
    State,
    /// A control point for modeling dominance and post-dominance.
    Ctrl,
}

/// Represents the `Entity` part of the bipartite hypergraph.
/// An Entity is a value, an object, a state, or a control point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entity {
    pub id: EntityId,
    pub kind: EntityKind,
    #[serde(rename = "type")]
    pub entity_type: String,
    // Additional attributes like is_const, value, alias-class, etc.
    #[serde(flatten)]
    pub attributes: HashMap<String, serde_json::Value>,
}

// --- Incidence (Ports) ---

/// Defines the role of a port on an `Event` node.
/// This specifies the purpose of the connection to an `Entity`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum PortRole {
    DataIn,
    DataOut,
    CtrlIn,
    CtrlOut,
    StateIn,
    StateOut,
    Obj,
    ExcOut,
    // Can be extended with other custom roles.
    Other(String),
}

/// Represents an incidence edge in the hypergraph, connecting an Event to an Entity via a Port.
/// This is the hyperedge itself.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Incidence {
    pub event: EventId,
    /// The port name, e.g., "data_in[0]", "state_in[0]".
    /// The string format allows flexibility.
    pub port: String,
    pub entity: EntityId,
    // Additional attributes can be added here, e.g. for mutability.
}

// --- State Edges ---

/// Represents a direct edge between two `State` entities, forming a version chain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StateEdge {
    pub from: EntityId,
    pub to: EntityId,
}

// --- The Hypergraph ---

/// Represents node embeddings for GNN processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeEmbedding {
    pub node_id: String,
    pub embedding: Vec<f32>,
}

/// Represents the complete Program Interaction Hypergraph (PIH).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramInteractionHypergraph {
    pub events: HashMap<EventId, Event>,
    pub entities: HashMap<EntityId, Entity>,
    pub incidence: Vec<Incidence>,
    pub state_edges: Vec<StateEdge>,
    /// Node embeddings computed by GNN for learning-based optimization.
    #[serde(default)]
    pub node_embeddings: HashMap<String, Vec<f32>>,
}

impl PartialEq for ProgramInteractionHypergraph {
    fn eq(&self, other: &Self) -> bool {
        self.events == other.events &&
        self.entities == other.entities &&
        self.incidence == other.incidence &&
        self.state_edges == other.state_edges
        // Note: node_embeddings may not be compared for equality in rule matching
    }
}

/// Converts a simple computation pattern into a PIH representation.
/// This is a basic converter that can be extended to handle more complex patterns.
pub fn convert_computation_to_pih(
    opcode: &str,
    inputs: Vec<(String, EntityKind, String)>, // (id, kind, type)
    outputs: Vec<(String, EntityKind, String)>, // (id, kind, type)
    constants: Vec<(String, serde_json::Value)>, // (id, value)
) -> ProgramInteractionHypergraph {
    let mut pih = ProgramInteractionHypergraph::new();

    // Create event
    let event = Event {
        id: format!("event_{}", opcode),
        opcode: opcode.to_string(),
        dtype: "i32".to_string(), // Default to i32, can be parameterized
        can_throw: false,
        attributes: HashMap::new(),
    };
    pih.events.insert(event.id.clone(), event);

    // Create input entities
    let input_count = inputs.len();
    let constant_count = constants.len();
    for (id, kind, entity_type) in inputs {
        let entity = Entity {
            id: id.clone(),
            kind,
            entity_type: entity_type.clone(),
            attributes: HashMap::new(),
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_in[{}]", pih.incidence.len()),
            entity: id,
        });
    }

    // Create constant entities
    for (id, value) in constants {
        let mut attributes = HashMap::new();
        attributes.insert("is_const".to_string(), json!(true));
        attributes.insert("value".to_string(), value);

        let entity = Entity {
            id: id.clone(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes,
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_in[{}]", pih.incidence.len()),
            entity: id,
        });
    }

    // Create output entities
    for (id, kind, entity_type) in outputs {
        let entity = Entity {
            id: id.clone(),
            kind,
            entity_type: entity_type.clone(),
            attributes: HashMap::new(),
        };
        pih.entities.insert(id.clone(), entity);

        // Add incidence edge
        pih.incidence.push(Incidence {
            event: format!("event_{}", opcode),
            port: format!("data_out[{}]", pih.incidence.len() - input_count - constant_count),
            entity: id,
        });
    }

    pih
}

impl ProgramInteractionHypergraph {
    pub fn new() -> Self {
        Self {
            events: HashMap::new(),
            entities: HashMap::new(),
            incidence: Vec::new(),
            state_edges: Vec::new(),
            node_embeddings: HashMap::new(),
        }
    }
}

impl Default for ProgramInteractionHypergraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_pih_serialization_deserialization() {
        let mut pih = ProgramInteractionHypergraph::new();

        // Nodes
        let event1 = Event {
            id: "e1".to_string(),
            opcode: "mul".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let entity_x = Entity {
            id: "v_x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let mut const_attr = HashMap::new();
        const_attr.insert("is_const".to_string(), json!(true));
        const_attr.insert("value".to_string(), json!(8));
        let entity_c = Entity {
            id: "v_c".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: const_attr,
        };
        let entity_out = Entity {
            id: "v_out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let state3 = Entity {
            id: "s3".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: HashMap::new(),
        };
        let state4 = Entity {
            id: "s4".to_string(),
            kind: EntityKind::State,
            entity_type: "heap".to_string(),
            attributes: HashMap::new(),
        };

        pih.events.insert(event1.id.clone(), event1);
        pih.entities.insert(entity_x.id.clone(), entity_x);
        pih.entities.insert(entity_c.id.clone(), entity_c);
        pih.entities.insert(entity_out.id.clone(), entity_out);
        pih.entities.insert(state3.id.clone(), state3);
        pih.entities.insert(state4.id.clone(), state4);

        // Incidence
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_in[0]".to_string(),
            entity: "v_x".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_in[1]".to_string(),
            entity: "v_c".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "data_out[0]".to_string(),
            entity: "v_out".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "state_in[0]".to_string(),
            entity: "s3".to_string(),
        });
        pih.incidence.push(Incidence {
            event: "e1".to_string(),
            port: "state_out[0]".to_string(),
            entity: "s4".to_string(),
        });

        // State Edges
        pih.state_edges.push(StateEdge {
            from: "s3".to_string(),
            to: "s4".to_string(),
        });

        let serialized = serde_json::to_string_pretty(&pih).unwrap();
        
        // This is a simplified check. A more robust test would compare field by field.
        assert!(serialized.contains("\"opcode\": \"mul\""));
        assert!(serialized.contains("\"kind\": \"State\""));
        assert!(serialized.contains("\"port\": \"data_in[1]\""));
        assert!(serialized.contains("\"from\": \"s3\""));

        let deserialized: ProgramInteractionHypergraph = serde_json::from_str(&serialized).unwrap();
        assert_eq!(deserialized.events.len(), 1);
        assert_eq!(deserialized.entities.len(), 5); // v_x, v_c, v_out, s3, s4
        assert_eq!(deserialized.incidence.len(), 5);
        assert_eq!(deserialized.state_edges.len(), 1);
        assert_eq!(deserialized.entities.get("v_c").unwrap().attributes.get("value").unwrap(), &json!(8));
    }

    #[test]
    fn test_strength_reduction_rule() {
        let rule = create_strength_reduction_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, c, out
        assert_eq!(rule.lhs.incidence.len(), 3);
        assert_eq!(rule.lhs.events.get("mul_op").unwrap().opcode, "mul");

        // Check RHS structure
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 3); // x, shift_amt, out
        assert_eq!(rule.rhs.incidence.len(), 3);
        assert_eq!(rule.rhs.events.get("shl_op").unwrap().opcode, "shl");

        // Check NAC
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_floating_point");
    }

    #[test]
    fn test_convert_computation_to_pih() {
        let inputs = vec![
            ("x".to_string(), EntityKind::Val, "i32".to_string()),
        ];
        let outputs = vec![
            ("result".to_string(), EntityKind::Val, "i32".to_string()),
        ];
        let constants = vec![
            ("eight".to_string(), json!(8)),
        ];

        let pih = convert_computation_to_pih("mul", inputs, outputs, constants);

        assert_eq!(pih.events.len(), 1);
        assert_eq!(pih.entities.len(), 3); // x, eight, result
        assert_eq!(pih.incidence.len(), 3); // 1 input + 1 constant + 1 output
        assert_eq!(pih.events.get("event_mul").unwrap().opcode, "mul");

        // Check constant entity
        let const_entity = pih.entities.get("eight").unwrap();
        assert_eq!(const_entity.attributes.get("is_const").unwrap(), &json!(true));
        assert_eq!(const_entity.attributes.get("value").unwrap(), &json!(8));
    }

    /// Creates a strength reduction rule: mul(x, 2^k) → shl(x, k)
    pub fn create_strength_reduction_rule() -> DpoRule {
        // LHS: mul operation with constant power of 2
        let mut lhs = ProgramInteractionHypergraph::new();
        let mul_event = Event {
            id: "mul_op".to_string(),
            opcode: "mul".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };
        let x_entity = Entity {
            id: "x".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let c_entity = Entity {
            id: "c".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(8)), // 2^3
            ].iter().cloned().collect(),
        };
        let out_entity = Entity {
            id: "out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(mul_event.id.clone(), mul_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(c_entity.id.clone(), c_entity);
        lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "c".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "mul_op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // RHS: equivalent shift operation
        let mut rhs = ProgramInteractionHypergraph::new();
        let shift_amount = Entity {
            id: "shift_amt".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(3)), // log2(8)
            ].iter().cloned().collect(),
        };
        let shl_event = Event {
            id: "shl_op".to_string(),
            opcode: "shl".to_string(),
            dtype: "i32".to_string(),
            can_throw: false,
            attributes: HashMap::new(),
        };

        rhs.events.insert(shl_event.id.clone(), shl_event);
        rhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        rhs.entities.insert(shift_amount.id.clone(), shift_amount);
        rhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "shift_amt".to_string(),
        });
        rhs.incidence.push(Incidence {
            event: "shl_op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // NAC: Don't apply if dtype is floating point (due to rounding differences)
        let floating_point_nac = NegativeApplicationCondition {
            name: "no_floating_point".to_string(),
            description: "Don't apply strength reduction to floating point types".to_string(),
            forbidden_incidence: vec![Incidence {
                event: "mul_op".to_string(),
                port: "dtype".to_string(),
                entity: "float_type".to_string(),
            }],
            forbidden_state_edges: vec![],
        };

        DpoRule {
            name: "StrengthReduction".to_string(),
            description: "Convert multiplication by power of 2 to shift operation".to_string(),
            lhs,
            rhs,
            nacs: vec![floating_point_nac],
        }
    }
}

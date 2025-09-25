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
//! The vm-gnn crate provides core data structures and algorithms for Program Interaction Hypergraphs:
//!
//! - [`ProgramInteractionHypergraph`]: Main hypergraph structure
//! - [`Event`]: Operation nodes
//! - [`Entity`]: Value/state nodes
//! - [`DpoRule`]: Double Pushout rewriting rules
//! - [`convert_computation_to_pih()`]: Convert computation patterns to PIH
//!
//! See the unit tests for detailed usage examples.

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

/// Creates a constant folding rule: add(x, 0) → x, mul(x, 1) → x
pub fn create_constant_folding_rule() -> DpoRule {
    // LHS: operation with identity constant
    let mut lhs = ProgramInteractionHypergraph::new();
    let op_event = Event {
        id: "op".to_string(),
        opcode: "add".to_string(), // Could be add, mul, etc.
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
    let identity_entity = Entity {
        id: "identity".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: [
            ("is_const".to_string(), json!(true)),
            ("value".to_string(), json!(0)), // 0 for add, 1 for mul
        ].iter().cloned().collect(),
    };
    let out_entity = Entity {
        id: "out".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(op_event.id.clone(), op_event);
    lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
    lhs.entities.insert(identity_entity.id.clone(), identity_entity);
    lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_in[1]".to_string(),
        entity: "identity".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "op".to_string(),
        port: "data_out[0]".to_string(),
        entity: "out".to_string(),
    });

    // RHS: just pass through x
    let mut rhs = ProgramInteractionHypergraph::new();
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(out_entity.id.clone(), out_entity);

    DpoRule {
        name: "ConstantFolding".to_string(),
        description: "Eliminate operations with identity constants".to_string(),
        lhs,
        rhs,
        nacs: vec![], // No negative conditions for this simple rule
    }
}

/// Creates a dead code elimination rule
pub fn create_dead_code_elimination_rule() -> DpoRule {
    // LHS: computation result that is never used
    let mut lhs = ProgramInteractionHypergraph::new();
    let compute_event = Event {
        id: "compute".to_string(),
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
    let y_entity = Entity {
        id: "y".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let unused_entity = Entity {
        id: "unused".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(compute_event.id.clone(), compute_event);
    lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
    lhs.entities.insert(y_entity.id.clone(), y_entity.clone());
    lhs.entities.insert(unused_entity.id.clone(), unused_entity.clone());

    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_in[0]".to_string(),
        entity: "x".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_in[1]".to_string(),
        entity: "y".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "compute".to_string(),
        port: "data_out[0]".to_string(),
        entity: "unused".to_string(),
    });

    // RHS: remove the unused computation entirely
    let mut rhs = ProgramInteractionHypergraph::new();
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(y_entity.id.clone(), y_entity);

    // NAC: Don't eliminate if result is actually used somewhere
    let used_result_nac = NegativeApplicationCondition {
        name: "result_is_used".to_string(),
        description: "Don't eliminate if the result is used by another operation".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "other_op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "unused".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "DeadCodeElimination".to_string(),
        description: "Remove computations whose results are never used".to_string(),
        lhs,
        rhs,
        nacs: vec![used_result_nac],
    }
}

/// Creates a loop fusion rule: adjacent loops with same iteration space can be fused
pub fn create_loop_fusion_rule() -> DpoRule {
    // LHS: Two adjacent loops with same bounds and no dependencies between them
    let mut lhs = ProgramInteractionHypergraph::new();

    // Loop 1: for(i=0; i<N; i++) { a[i] = b[i] + c[i]; }
    let loop1_event = Event {
        id: "loop1".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let a_entity = Entity {
        id: "a".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let b_entity = Entity {
        id: "b".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let c_entity = Entity {
        id: "c".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(loop1_event.id.clone(), loop1_event);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(a_entity.id.clone(), a_entity.clone());
    lhs.entities.insert(b_entity.id.clone(), b_entity.clone());
    lhs.entities.insert(c_entity.id.clone(), c_entity.clone());

    // Loop 1 body: a[i] = b[i] + c[i]
    lhs.incidence.push(Incidence {
        event: "loop1".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "loop1".to_string(),
        port: "body".to_string(),
        entity: "load_b".to_string(),
    });

    // Loop 2: for(i=0; i<N; i++) { d[i] = e[i] * f[i]; }
    let loop2_event = Event {
        id: "loop2".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let d_entity = Entity {
        id: "d".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let e_entity = Entity {
        id: "e".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let f_entity = Entity {
        id: "f".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(loop2_event.id.clone(), loop2_event);
    lhs.entities.insert(d_entity.id.clone(), d_entity.clone());
    lhs.entities.insert(e_entity.id.clone(), e_entity.clone());
    lhs.entities.insert(f_entity.id.clone(), f_entity.clone());

    // Loop 2 body: d[i] = e[i] * f[i]
    lhs.incidence.push(Incidence {
        event: "loop2".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "loop2".to_string(),
        port: "body".to_string(),
        entity: "load_e".to_string(),
    });

    // RHS: Fused loop with both operations
    let mut rhs = ProgramInteractionHypergraph::new();
    let fused_loop = Event {
        id: "fused_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };

    rhs.events.insert(fused_loop.id.clone(), fused_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(a_entity.id.clone(), a_entity);
    rhs.entities.insert(b_entity.id.clone(), b_entity);
    rhs.entities.insert(c_entity.id.clone(), c_entity);
    rhs.entities.insert(d_entity.id.clone(), d_entity);
    rhs.entities.insert(e_entity.id.clone(), e_entity);
    rhs.entities.insert(f_entity.id.clone(), f_entity);

    // Fused loop body: a[i] = b[i] + c[i]; d[i] = e[i] * f[i];
    rhs.incidence.push(Incidence {
        event: "fused_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "fused_loop".to_string(),
        port: "body".to_string(),
        entity: "fused_body".to_string(),
    });

    // NAC: No dependencies between loops
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_loop_dependencies".to_string(),
        description: "Cannot fuse loops if there are dependencies between them".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "loop2".to_string(),
            port: "dependency".to_string(),
            entity: "loop1_output".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "LoopFusion".to_string(),
        description: "Fuse adjacent loops with same iteration space".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
}

/// Creates a vectorization rule: scalar operations → SIMD operations
pub fn create_vectorization_rule() -> DpoRule {
    // LHS: Scalar addition loop
    let mut lhs = ProgramInteractionHypergraph::new();
    let scalar_loop = Event {
        id: "scalar_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let a_entity = Entity {
        id: "a".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };
    let b_entity = Entity {
        id: "b".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(scalar_loop.id.clone(), scalar_loop);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(a_entity.id.clone(), a_entity.clone());
    lhs.entities.insert(b_entity.id.clone(), b_entity.clone());

    lhs.incidence.push(Incidence {
        event: "scalar_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "scalar_loop".to_string(),
        port: "body".to_string(),
        entity: "scalar_add".to_string(),
    });

    // RHS: Vectorized loop with SIMD operations
    let mut rhs = ProgramInteractionHypergraph::new();
    let vector_loop = Event {
        id: "vector_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(4)), // Process 4 elements per iteration
        ].iter().cloned().collect(),
    };
    let vector_entity = Entity {
        id: "vector".to_string(),
        kind: EntityKind::Val,
        entity_type: "__m128i".to_string(), // SIMD vector type
        attributes: HashMap::new(),
    };

    rhs.events.insert(vector_loop.id.clone(), vector_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(a_entity.id.clone(), a_entity);
    rhs.entities.insert(b_entity.id.clone(), b_entity);
    rhs.entities.insert(vector_entity.id.clone(), vector_entity);

    rhs.incidence.push(Incidence {
        event: "vector_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "vector_loop".to_string(),
        port: "body".to_string(),
        entity: "simd_add".to_string(),
    });

    // NAC: Data must be aligned for SIMD operations
    let alignment_nac = NegativeApplicationCondition {
        name: "aligned_data".to_string(),
        description: "Data must be properly aligned for SIMD operations".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "scalar_loop".to_string(),
            port: "unaligned".to_string(),
            entity: "data".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "Vectorization".to_string(),
        description: "Convert scalar operations to SIMD vector operations".to_string(),
        lhs,
        rhs,
        nacs: vec![alignment_nac],
    }
}

/// Creates a parallelization rule: sequential loop → parallel loop
pub fn create_parallelization_rule() -> DpoRule {
    // LHS: Sequential loop
    let mut lhs = ProgramInteractionHypergraph::new();
    let seq_loop = Event {
        id: "sequential_loop".to_string(),
        opcode: "for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
        ].iter().cloned().collect(),
    };
    let i_entity = Entity {
        id: "i".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };
    let array_entity = Entity {
        id: "array".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32*".to_string(),
        attributes: HashMap::new(),
    };

    lhs.events.insert(seq_loop.id.clone(), seq_loop);
    lhs.entities.insert(i_entity.id.clone(), i_entity.clone());
    lhs.entities.insert(array_entity.id.clone(), array_entity.clone());

    lhs.incidence.push(Incidence {
        event: "sequential_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    lhs.incidence.push(Incidence {
        event: "sequential_loop".to_string(),
        port: "body".to_string(),
        entity: "sequential_compute".to_string(),
    });

    // RHS: Parallel loop using OpenMP or similar
    let mut rhs = ProgramInteractionHypergraph::new();
    let parallel_loop = Event {
        id: "parallel_loop".to_string(),
        opcode: "parallel_for".to_string(),
        dtype: "i32".to_string(),
        can_throw: false,
        attributes: [
            ("start".to_string(), json!(0)),
            ("end".to_string(), json!("N")),
            ("step".to_string(), json!(1)),
            ("num_threads".to_string(), json!(4)),
        ].iter().cloned().collect(),
    };
    let thread_id_entity = Entity {
        id: "thread_id".to_string(),
        kind: EntityKind::Val,
        entity_type: "i32".to_string(),
        attributes: HashMap::new(),
    };

    rhs.events.insert(parallel_loop.id.clone(), parallel_loop);
    rhs.entities.insert(i_entity.id.clone(), i_entity);
    rhs.entities.insert(array_entity.id.clone(), array_entity);
    rhs.entities.insert(thread_id_entity.id.clone(), thread_id_entity);

    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "index".to_string(),
        entity: "i".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "thread_id".to_string(),
        entity: "thread_id".to_string(),
    });
    rhs.incidence.push(Incidence {
        event: "parallel_loop".to_string(),
        port: "body".to_string(),
        entity: "parallel_compute".to_string(),
    });

    // NAC: No loop-carried dependencies
    let no_dependency_nac = NegativeApplicationCondition {
        name: "no_loop_dependencies".to_string(),
        description: "Cannot parallelize if there are loop-carried dependencies".to_string(),
        forbidden_incidence: vec![Incidence {
            event: "sequential_loop".to_string(),
            port: "dependency".to_string(),
            entity: "previous_iteration".to_string(),
        }],
        forbidden_state_edges: vec![],
    };

    DpoRule {
        name: "Parallelization".to_string(),
        description: "Convert sequential loops to parallel execution".to_string(),
        lhs,
        rhs,
        nacs: vec![no_dependency_nac],
    }
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
    rhs.entities.insert(x_entity.id.clone(), x_entity);
    rhs.entities.insert(shift_amount.id.clone(), shift_amount);
    rhs.entities.insert(out_entity.id.clone(), out_entity);

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

    #[test]
    fn test_constant_folding_rule() {
        let rule = create_constant_folding_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, identity, out
        assert_eq!(rule.lhs.incidence.len(), 3);
        assert_eq!(rule.lhs.events.get("op").unwrap().opcode, "add");

        // Check RHS structure (simplified - just entities, no operations)
        assert_eq!(rule.rhs.events.len(), 0);
        assert_eq!(rule.rhs.entities.len(), 2); // x, out
        assert_eq!(rule.rhs.incidence.len(), 0); // No operations

        // Check identity constant
        let identity_entity = rule.lhs.entities.get("identity").unwrap();
        assert_eq!(identity_entity.attributes.get("value").unwrap(), &json!(0));

        // Check NACs
        assert_eq!(rule.nacs.len(), 0); // No negative conditions
    }

    #[test]
    fn test_dead_code_elimination_rule() {
        let rule = create_dead_code_elimination_rule();

        // Check LHS structure
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // x, y, unused
        assert_eq!(rule.lhs.incidence.len(), 3);

        // Check RHS structure (unused entities removed)
        assert_eq!(rule.rhs.events.len(), 0);
        assert_eq!(rule.rhs.entities.len(), 2); // x, y (unused removed)
        assert_eq!(rule.rhs.incidence.len(), 0);

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "result_is_used");
    }

    #[test]
    fn test_loop_fusion_rule() {
        let rule = create_loop_fusion_rule();

        // Check LHS structure (2 loops)
        assert_eq!(rule.lhs.events.len(), 2); // loop1, loop2
        assert_eq!(rule.lhs.entities.len(), 7); // i, a, b, c, d, e, f
        assert_eq!(rule.lhs.incidence.len(), 4); // 2 loops * 2 incidence each

        // Check RHS structure (1 fused loop)
        assert_eq!(rule.rhs.events.len(), 1); // fused_loop
        assert_eq!(rule.rhs.entities.len(), 7); // All entities preserved
        assert_eq!(rule.rhs.incidence.len(), 2); // 1 loop * 2 incidence

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_loop_dependencies");
    }

    #[test]
    fn test_vectorization_rule() {
        let rule = create_vectorization_rule();

        // Check LHS structure (scalar loop)
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 3); // i, a, b
        assert_eq!(rule.lhs.incidence.len(), 2);

        // Check RHS structure (vectorized loop)
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 4); // i, a, b, vector
        assert_eq!(rule.rhs.incidence.len(), 2);

        // Check SIMD vector type
        assert!(rule.rhs.entities.get("vector").unwrap().entity_type == "__m128i");

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "aligned_data");
    }

    #[test]
    fn test_parallelization_rule() {
        let rule = create_parallelization_rule();

        // Check LHS structure (sequential loop)
        assert_eq!(rule.lhs.events.len(), 1);
        assert_eq!(rule.lhs.entities.len(), 2); // i, array
        assert_eq!(rule.lhs.incidence.len(), 2);

        // Check RHS structure (parallel loop)
        assert_eq!(rule.rhs.events.len(), 1);
        assert_eq!(rule.rhs.entities.len(), 3); // i, array, thread_id
        assert_eq!(rule.rhs.incidence.len(), 3); // Added thread_id

        // Check parallel loop attributes
        let parallel_loop = rule.rhs.events.get("parallel_loop").unwrap();
        assert!(parallel_loop.attributes.get("num_threads") == Some(&json!(4)));

        // Check NACs
        assert_eq!(rule.nacs.len(), 1);
        assert_eq!(rule.nacs[0].name, "no_loop_dependencies");
    }

    /// Creates a constant folding rule: add(x, 0) → x, mul(x, 1) → x
    pub fn create_constant_folding_rule() -> DpoRule {
        // LHS: operation with identity constant
        let mut lhs = ProgramInteractionHypergraph::new();
        let op_event = Event {
            id: "op".to_string(),
            opcode: "add".to_string(), // Could be add, mul, etc.
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
        let identity_entity = Entity {
            id: "identity".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: [
                ("is_const".to_string(), json!(true)),
                ("value".to_string(), json!(0)), // 0 for add, 1 for mul
            ].iter().cloned().collect(),
        };
        let out_entity = Entity {
            id: "out".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(op_event.id.clone(), op_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(identity_entity.id.clone(), identity_entity);
        lhs.entities.insert(out_entity.id.clone(), out_entity.clone());

        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_in[1]".to_string(),
            entity: "identity".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "op".to_string(),
            port: "data_out[0]".to_string(),
            entity: "out".to_string(),
        });

        // RHS: just pass through x
        let mut rhs = ProgramInteractionHypergraph::new();
        rhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        rhs.entities.insert(out_entity.id.clone(), out_entity.clone());
        // Direct connection: x → out (no operation needed)

        DpoRule {
            name: "ConstantFolding".to_string(),
            description: "Eliminate operations with identity constants".to_string(),
            lhs,
            rhs,
            nacs: vec![], // No negative conditions for this simple rule
        }
    }

    /// Creates a dead code elimination rule
    pub fn create_dead_code_elimination_rule() -> DpoRule {
        // LHS: computation result that is never used
        let mut lhs = ProgramInteractionHypergraph::new();
        let compute_event = Event {
            id: "compute".to_string(),
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
        let y_entity = Entity {
            id: "y".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };
        let unused_entity = Entity {
            id: "unused".to_string(),
            kind: EntityKind::Val,
            entity_type: "i32".to_string(),
            attributes: HashMap::new(),
        };

        lhs.events.insert(compute_event.id.clone(), compute_event);
        lhs.entities.insert(x_entity.id.clone(), x_entity.clone());
        lhs.entities.insert(y_entity.id.clone(), y_entity.clone());
        lhs.entities.insert(unused_entity.id.clone(), unused_entity.clone());

        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_in[0]".to_string(),
            entity: "x".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_in[1]".to_string(),
            entity: "y".to_string(),
        });
        lhs.incidence.push(Incidence {
            event: "compute".to_string(),
            port: "data_out[0]".to_string(),
            entity: "unused".to_string(),
        });

        // RHS: remove the unused computation entirely
        let mut rhs = ProgramInteractionHypergraph::new();
        rhs.entities.insert(x_entity.id.clone(), x_entity);
        rhs.entities.insert(y_entity.id.clone(), y_entity);
        // No events, no unused entity

        // NAC: Don't eliminate if result is actually used somewhere
        let used_result_nac = NegativeApplicationCondition {
            name: "result_is_used".to_string(),
            description: "Don't eliminate if the result is used by another operation".to_string(),
            forbidden_incidence: vec![Incidence {
                event: "other_op".to_string(),
                port: "data_in[0]".to_string(),
                entity: "unused".to_string(),
            }],
            forbidden_state_edges: vec![],
        };

        DpoRule {
            name: "DeadCodeElimination".to_string(),
            description: "Remove computations whose results are never used".to_string(),
            lhs,
            rhs,
            nacs: vec![used_result_nac],
        }
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

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
//! - [`Edge`]: Operation nodes in the bipartite graph
//! - [`Node`]: Value/state nodes in the bipartite graph
//! - [`DpoRule`]: Double Pushout rewriting rules for safe transformations
//! - [`NegativeApplicationCondition`]: NACs for prohibiting unsafe rewrites
//!
//! ## Usage
//!
//! The vm-gnn crate provides core data structures and algorithms for Program Interaction Hypergraphs:
//!
//! - [`ProgramInteractionHypergraph`]: Main hypergraph structure
//! - [`Edge`]: Operation nodes
//! - [`Node`]: Value/state nodes
//! - [`DpoRule`]: Double Pushout rewriting rules
//! - [`convert_computation_to_pih()`]: Convert computation patterns to PIH
//!
//! See the unit tests for detailed usage examples.

#![allow(dead_code)] // TODO: Remove this later on

use std::collections::HashMap;
use serde_json::json;

// Core data structures module
pub mod core;

// CID computation and Merkle DAG module
pub mod cid;

// DPO (Double Pushout) rewriting rules module
pub mod dpo;

// GNN (Graph Neural Network) features and training module
pub mod gnn;

// Hardware-specific optimization features module
pub mod hardware;

// Production training system module
pub mod training;

// Synthetic data generation module
pub mod synthesis;

// Re-export all public items from submodules for convenient access
pub use crate::core::*;
pub use crate::cid::*;
pub use crate::dpo::*;
pub use crate::gnn::*;
pub use crate::hardware::*;
pub use crate::training::*;
pub use crate::synthesis::*;

/// Convert computation patterns to Program Interaction Hypergraph.
/// This function creates a PIH representation from basic computation elements.
pub fn convert_computation_to_pih(
    opcode: &str,
    inputs: Vec<(String, NodeKind, String)>, // (id, kind, type)
    outputs: Vec<(String, NodeKind, String)>, // (id, kind, type)
    constants: Vec<(String, serde_json::Value)>, // (id, value)
) -> ProgramInteractionHypergraph {
    let mut pih = ProgramInteractionHypergraph::new();

    // Create event edge
    let event_edge = Edge {
        id: format!("event_{}", opcode),
        kind: EdgeKind::Event,
        label: Some(opcode.to_string()),
        opcode: Some(opcode.to_string()),
        dtype: Some("i32".to_string()),
        can_throw: false,
        attributes: [
            ("opcode".to_string(), json!(opcode)),
            ("dtype".to_string(), json!("i32")), // Default to i32, can be parameterized
        ].iter().cloned().collect(),
        cid: None,
    };
    pih.edges.push(event_edge);

    // Create input nodes
    let input_count = inputs.len();
    let constant_count = constants.len();
    for (id, kind, node_type) in inputs {
        let node = Node {
            id: id.clone(),
            kind,
            node_type: node_type.clone(),
            entity_type: Some(node_type.clone()),
            attributes: HashMap::new(),
            cid: None,
        };
        pih.nodes.push(node);

        // Add incidence
        pih.incidences.push(Incidence {
            edge: format!("event_{}", opcode),
            node: id,
            role: RoleKind::DataIn,
            idx: Some(pih.incidences.len() as u32),
            attrs: HashMap::new(),
            cid: None,
        });
    }

    // Create constant nodes
    for (id, value) in constants {
        let mut attributes = HashMap::new();
        attributes.insert("is_const".to_string(), json!(true));
        attributes.insert("value".to_string(), value);

        let node = Node {
            id: id.clone(),
            kind: NodeKind::Val,
            node_type: "i32".to_string(),
            entity_type: Some("i32".to_string()),
            attributes,
            cid: None,
        };
        pih.nodes.push(node);

        // Add incidence
        pih.incidences.push(Incidence {
            edge: format!("event_{}", opcode),
            node: id,
            role: RoleKind::DataIn,
            idx: Some(pih.incidences.len() as u32),
            attrs: HashMap::new(),
            cid: None,
        });
    }

    // Create output nodes
    for (id, kind, node_type) in outputs {
        let node = Node {
            id: id.clone(),
            kind,
            node_type: node_type.clone(),
            entity_type: Some(node_type.clone()),
            attributes: HashMap::new(),
            cid: None,
        };
        pih.nodes.push(node);

        // Add incidence
        pih.incidences.push(Incidence {
            edge: format!("event_{}", opcode),
            node: id,
            role: RoleKind::DataOut,
            idx: Some(pih.incidences.len() as u32),
            attrs: HashMap::new(),
            cid: None,
        });
    }

    pih
}

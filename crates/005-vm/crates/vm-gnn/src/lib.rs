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

//! # Kotoba Workflow Engine (Itonami)
//!
//! Temporal-inspired workflow engine built on top of Kotoba's graph rewriting system.
//!
//! ## Features
//!
//! - **Temporal Patterns**: Sequence, Parallel, Decision, Wait, Saga, Activity, Sub-workflow
//! - **MVCC Persistence**: Workflow state management with Merkle DAG
//! - **Graph-based Execution**: Declarative workflow definition using graph transformations
//! - **Activity System**: Extensible activity execution framework
//! - **Event Sourcing**: Complete audit trail of workflow execution
//!
//! ## Example
//!
//! ```rust
//! use kotoba_workflow::prelude::*;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create activity registry
//!     let registry = ActivityRegistry::new();
//!
//!     // Activity system is ready!
//!     println!("Workflow activity system ready!");
//!     Ok(())
//! }
//! ```

pub mod ir;
pub mod executor;
pub mod store;
pub mod activity;

// Re-export main types
pub use ir::{WorkflowIR, WorkflowExecution, WorkflowExecutionId, ActivityIR, DummyGraphRef, ExecutionStatus};
pub use executor::{ActivityRegistry, Activity};
pub use store::{WorkflowStore, StorageBackend, StorageFactory};
pub use activity::prelude::*;

// TODO: Implement WorkflowEngine
// For now, this crate provides basic IR and activity system

/// Workflow error
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Prelude for convenient imports
pub mod prelude {
    pub use super::{
        WorkflowIR, WorkflowExecution, WorkflowExecutionId,
        ActivityRegistry, Activity, WorkflowStore, ExecutionStatus,
    };
}

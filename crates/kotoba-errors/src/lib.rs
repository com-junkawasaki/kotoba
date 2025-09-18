//! `kotoba-errors`
//!
//! Shared error types for the Kotoba ecosystem to prevent circular dependencies.

use thiserror::Error;

/// The primary error type for the entire Kotoba ecosystem.
#[derive(Debug, Error)]
pub enum KotobaError {
    #[error("Parse error: {0}")]
    Parse(String),
    #[error("Execution error: {0}")]
    Execution(String),
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Validation error: {0}")]
    Validation(String),
    #[error("Rewrite error: {0}")]
    Rewrite(String),
    #[error("Security error: {0}")]
    Security(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Configuration error: {0}")]
    Configuration(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Network error: {0}")]
    Network(String),
    #[error("Workflow error: {0}")]
    Workflow(String), // Variant to hold stringified WorkflowError
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Error type specific to the `kotoba-workflow` crate.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    // #[error("Activity execution failed: {0}")]
    // ActivityFailed(#[from] ActivityError), // ActivityError is not defined here
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),
    #[error("Graph operation failed: {0}")]
    GraphError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    // #[error("Invalid step type for this executor: {0:?}")]
    // InvalidStepType(kotoba_routing::schema::WorkflowStepType),
    #[error("Context variable not found: {0}")]
    ContextVariableNotFound(String),
}

/// Allow `WorkflowError` to be converted into `KotobaError`.
/// This is the key to breaking the circular dependency.
impl From<WorkflowError> for KotobaError {
    fn from(err: WorkflowError) -> Self {
        KotobaError::Workflow(err.to_string())
    }
}

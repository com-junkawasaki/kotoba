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
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    // --- 認証・認可関連エラー ---
    #[error("Authentication error: {0}")]
    Authentication(String),
    #[error("Authorization error: {0}")]
    Authorization(String),
    #[error("Access denied to resource: {0}")]
    AccessDenied(String),
    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),
    #[error("Session expired: {0}")]
    SessionExpired(String),
    #[error("Insufficient permissions: {0}")]
    InsufficientPermissions(String),

    // --- 暗号化関連エラー ---
    #[error("Cryptography error: {0}")]
    Cryptography(String),
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
}

/// Error type specific to the `kotoba-workflow` crate.
#[derive(Debug, Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Activity execution failed: {0}")]
    ActivityFailed(String),
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
    #[error("Invalid step type: {0}")]
    InvalidStepType(String),
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

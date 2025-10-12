//! ENG EAF-IPG Runtime
//!
//! Unified Intermediate Representation for programming languages
//! combining AST, dataflow, control flow, memory, typing, effects, and time.

pub mod validator;
pub mod runtime;
pub mod dsl;

// Re-export types from the new crates
pub use kotoba_types::*;
pub use engidb;

/// Result type for EAF-IPG operations
pub type Result<T> = std::result::Result<T, Error>;

/// Error types for EAF-IPG operations
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("JSON parsing error: {0}")]
    JsonParse(#[from] serde_json::Error),

    #[error("Jsonnet evaluation error: {0}")]
    JsonnetEval(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Db(#[from] engidb::Error),
}

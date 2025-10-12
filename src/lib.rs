//! ENG EAF-IPG Runtime
//!
//! Unified Intermediate Representation for programming languages
//! combining AST, dataflow, control flow, memory, typing, effects, and time.

pub mod types;
pub mod validator;
pub mod runtime;
pub mod dsl;
pub mod storage;

/// Re-export commonly used types
pub use types::*;
pub use validator::*;
pub use runtime::*;

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

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Redb database error: {0}")]
    Redb(String),
}

// Blanket implementation to convert all specific redb error types into our Error::Redb.
impl From<redb::DatabaseError> for Error {
    fn from(err: redb::DatabaseError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::TransactionError> for Error {
    fn from(err: redb::TransactionError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::TableError> for Error {
    fn from(err: redb::TableError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::StorageError> for Error {
    fn from(err: redb::StorageError) -> Self { Error::Redb(err.to_string()) }
}
impl From<redb::CommitError> for Error {
    fn from(err: redb::CommitError) -> Self { Error::Redb(err.to_string()) }
}

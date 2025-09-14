//! Error types for Kotoba2TSX operations

use thiserror::Error;

/// Main error type for Kotoba2TSX operations
#[derive(Error, Debug)]
pub enum Kotoba2TSError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Jsonnet evaluation error: {0}")]
    Jsonnet(String),

    #[error("Invalid component configuration: {0}")]
    InvalidComponent(String),

    #[error("Missing required field: {field} in {component}")]
    MissingField { field: String, component: String },

    #[error("Unsupported component type: {0}")]
    UnsupportedComponentType(String),

    #[error("Code generation error: {0}")]
    CodeGeneration(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Invalid file format: {0}")]
    InvalidFileFormat(String),

    #[error("Component not found: {0}")]
    ComponentNotFound(String),

    #[error("Circular dependency detected: {0}")]
    CircularDependency(String),

    #[error("Invalid prop type: {prop} = {value}")]
    InvalidPropType { prop: String, value: String },

    #[error("Generic error: {0}")]
    Generic(String),
}

pub type Result<T> = std::result::Result<T, Kotoba2TSError>;

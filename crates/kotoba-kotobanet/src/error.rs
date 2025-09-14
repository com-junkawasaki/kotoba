//! Error types for Kotoba Kotobanet

use thiserror::Error;

/// Errors that can occur in Kotoba Kotobanet operations
#[derive(Debug, Error)]
pub enum KotobaNetError {
    #[error("Jsonnet evaluation error: {0}")]
    Jsonnet(#[from] kotoba_jsonnet::JsonnetError),

    #[error("HTTP parsing error: {0}")]
    HttpParse(String),

    #[error("Frontend parsing error: {0}")]
    FrontendParse(String),

    #[error("Deploy configuration error: {0}")]
    DeployConfig(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),


    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, KotobaNetError>;

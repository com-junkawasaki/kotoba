//! Handler error types

use std::fmt;

/// Handler execution errors
#[derive(Debug, thiserror::Error)]
pub enum HandlerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[cfg(feature = "wasm")]
    #[error("WASM error: {0}")]
    Wasm(String),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, HandlerError>;

#[cfg(feature = "wasm")]
impl From<wasm_bindgen::JsValue> for HandlerError {
    fn from(value: wasm_bindgen::JsValue) -> Self {
        HandlerError::Wasm(format!("{:?}", value))
    }
}

#[cfg(feature = "server")]
impl From<hyper::Error> for HandlerError {
    fn from(err: hyper::Error) -> Self {
        HandlerError::Network(err.to_string())
    }
}

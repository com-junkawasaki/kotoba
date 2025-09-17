//! Common types for handler operations

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Handler execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerContext {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub body: Option<String>,
    pub environment: HashMap<String, String>,
}

/// Handler configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerConfig {
    pub timeout_ms: u64,
    pub max_memory_mb: u64,
    pub enable_caching: bool,
    pub enable_logging: bool,
}

/// Handler result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerResult {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub execution_time_ms: u64,
    pub memory_used_mb: f64,
}

/// Handler metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub capabilities: Vec<String>,
}

/// WebSocket message types
#[cfg(feature = "websocket")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSocketMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping,
    Pong,
    Close,
}

/// Execution mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Sync,
    Async,
    Streaming,
}

/// Handler capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerCapabilities {
    pub supports_async: bool,
    pub supports_streaming: bool,
    pub supports_websocket: bool,
    pub supports_file_upload: bool,
    pub max_payload_size: u64,
    pub supported_content_types: Vec<String>,
}

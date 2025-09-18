//! # Kotoba Server Core
//!
//! Core HTTP server library for Kotoba providing basic HTTP/GraphQL server functionality.
//! This crate contains the foundational server components without workflow dependencies.

pub mod server;
pub mod router;
pub mod middleware;
pub mod handlers;

pub use server::{HttpServer, ServerConfig, ServerBuilder};
pub use router::AppRouter;
pub use middleware::{CorsConfig, LoggingConfig};
pub use handlers::{HealthHandler, NotFoundHandler};

use axum::{
    Router,
    response::IntoResponse,
    http::StatusCode,
};
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;
use std::net::SocketAddr;

/// Core server error type
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("HTTP server error: {0}")]
    Http(#[from] hyper::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Handler error: {0}")]
    Handler(String),
}

/// Result type for server operations
pub type Result<T> = std::result::Result<T, ServerError>;

/// Convert KotobaError to Axum response
pub fn kotoba_error_to_response(err: &kotoba_errors::KotobaError) -> axum::response::Response {
    let (status, message) = match err {
        kotoba_errors::KotobaError::NotFound(resource) =>
            (axum::http::StatusCode::NOT_FOUND, format!("Resource not found: {}", resource)),
        kotoba_errors::KotobaError::Validation(details) =>
            (axum::http::StatusCode::BAD_REQUEST, format!("Validation failed: {}", details)),
        kotoba_errors::KotobaError::Security(details) =>
            (axum::http::StatusCode::FORBIDDEN, format!("Forbidden: {}", details)),
        kotoba_errors::KotobaError::InvalidArgument(details) =>
            (axum::http::StatusCode::BAD_REQUEST, format!("Invalid argument: {}", details)),
        _ => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "An internal server error occurred".to_string()),
    };

    // Log the full error for debugging
    tracing::error!("An error occurred: {:?}", err);

    (status, message).into_response()
}

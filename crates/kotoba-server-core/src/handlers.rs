//! Common HTTP handlers

use axum::{
    response::{Json, IntoResponse},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::router::HealthResponse;

/// Health check handler
pub async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse::default())
}

/// Liveness probe handler
pub async fn liveness() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Readiness probe handler
pub async fn readiness() -> impl IntoResponse {
    // TODO: Add actual readiness checks (database connections, etc.)
    (StatusCode::OK, "OK")
}

/// 404 Not Found handler
pub async fn not_found() -> impl IntoResponse {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({
            "error": "Not Found",
            "message": "The requested resource was not found",
            "code": 404
        })),
    )
}

/// Method not allowed handler
pub async fn method_not_allowed() -> impl IntoResponse {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(serde_json::json!({
            "error": "Method Not Allowed",
            "message": "The requested method is not allowed for this resource",
            "code": 405
        })),
    )
}

/// Generic error handler
pub async fn internal_error() -> impl IntoResponse {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "error": "Internal Server Error",
            "message": "An internal server error occurred",
            "code": 500
        })),
    )
}

/// Health check handler struct for more complex health checks
#[derive(Debug)]
pub struct HealthHandler {
    start_time: DateTime<Utc>,
}

impl HealthHandler {
    pub fn new() -> Self {
        Self {
            start_time: Utc::now(),
        }
    }

    pub async fn handle(&self) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            timestamp: Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
}

impl Default for HealthHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Not found handler with customizable response
#[derive(Debug)]
pub struct NotFoundHandler {
    message: String,
}

impl NotFoundHandler {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub async fn handle(&self) -> impl IntoResponse {
        (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({
                "error": "Not Found",
                "message": self.message,
                "code": 404
            })),
        )
    }
}

impl Default for NotFoundHandler {
    fn default() -> Self {
        Self::new("The requested resource was not found")
    }
}

/// Metrics endpoint response
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsResponse {
    pub uptime_seconds: i64,
    pub total_requests: u64,
    pub active_connections: u32,
    pub memory_usage_mb: f64,
}

impl Default for MetricsResponse {
    fn default() -> Self {
        Self {
            uptime_seconds: 0,
            total_requests: 0,
            active_connections: 0,
            memory_usage_mb: 0.0,
        }
    }
}

/// Metrics handler (basic implementation)
pub async fn metrics() -> Json<MetricsResponse> {
    // TODO: Integrate with actual metrics collection
    Json(MetricsResponse::default())
}


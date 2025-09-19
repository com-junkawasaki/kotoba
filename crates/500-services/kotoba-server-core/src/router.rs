//! Router utilities and helpers

use axum::{
    Router,
    routing::{get, post, put, delete, patch},
    response::{Json, IntoResponse},
    extract::{Path, Query, State},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application router with common utilities
#[derive(Debug)]
pub struct AppRouter {
    router: Router,
}

impl AppRouter {
    pub fn new() -> Self {
        Self {
            router: Router::new(),
        }
    }

    pub fn route(mut self, path: &str, method_router: axum::routing::MethodRouter<()>) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    pub fn nest(mut self, path: &str, router: Router) -> Self {
        self.router = self.router.nest(path, router);
        self
    }

    pub fn merge<R>(mut self, other: R) -> Self
    where
        R: Into<Router>,
    {
        self.router = self.router.merge(other.into());
        self
    }

    // Layer functionality is handled at the server level
    // Individual routes should use axum's built-in middleware

    pub fn fallback(mut self, method_router: axum::routing::MethodRouter<()>) -> Self {
        self.router = self.router.fallback(method_router);
        self
    }

    pub fn build(self) -> Router {
        self.router
    }
}

impl Default for AppRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Router> for AppRouter {
    fn from(router: Router) -> Self {
        Self { router }
    }
}

impl From<AppRouter> for Router {
    fn from(app_router: AppRouter) -> Self {
        app_router.router
    }
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

impl Default for HealthResponse {
    fn default() -> Self {
        Self {
            status: "ok".to_string(),
            timestamp: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Query parameters extractor
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<usize>,
    pub limit: Option<usize>,
}

impl Default for PaginationParams {
    fn default() -> Self {
        Self {
            page: Some(1),
            limit: Some(10),
        }
    }
}

impl PaginationParams {
    pub fn offset(&self) -> usize {
        let page = self.page.unwrap_or(1).saturating_sub(1);
        let limit = self.limit.unwrap_or(10);
        page * limit
    }

    pub fn limit(&self) -> usize {
        self.limit.unwrap_or(10)
    }
}

/// Generic API response wrapper
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub meta: Option<HashMap<String, serde_json::Value>>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            meta: None,
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
            meta: None,
        }
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.meta.get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

impl<T: Serialize> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> axum::response::Response {
        let status = if self.success {
            axum::http::StatusCode::OK
        } else {
            axum::http::StatusCode::BAD_REQUEST
        };

        (status, Json(self)).into_response()
    }
}

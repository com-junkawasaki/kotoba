//! Middleware utilities

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    http::{HeaderMap, StatusCode},
};
use tower_http::cors::{CorsLayer, Any};
use std::time::Instant;

/// CORS configuration
#[derive(Debug, Clone)]
pub struct CorsConfig {
    pub allow_credentials: bool,
    pub allow_headers: Vec<String>,
    pub allow_methods: Vec<String>,
    pub allow_origins: Vec<String>,
    pub max_age: Option<usize>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_credentials: false,
            allow_headers: vec!["*".to_string()],
            allow_methods: vec!["*".to_string()],
            allow_origins: vec!["*".to_string()],
            max_age: Some(86400), // 24 hours
        }
    }
}

impl CorsConfig {
    pub fn build_layer(&self) -> CorsLayer {
        let mut layer = CorsLayer::new();

        if self.allow_origins.contains(&"*".to_string()) {
            layer = layer.allow_origin(Any);
        }

        if self.allow_methods.contains(&"*".to_string()) {
            layer = layer.allow_methods(Any);
        }

        if self.allow_headers.contains(&"*".to_string()) {
            layer = layer.allow_headers(Any);
        }

        if self.allow_credentials {
            layer = layer.allow_credentials(true);
        }

        if let Some(max_age) = self.max_age {
            layer = layer.max_age(std::time::Duration::from_secs(max_age as u64));
        }

        layer
    }
}

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_errors: bool,
    pub exclude_paths: Vec<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            log_requests: true,
            log_responses: true,
            log_errors: true,
            exclude_paths: vec!["/health".to_string()],
        }
    }
}

/// Request logging middleware
pub async fn request_logger(
    req: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();

    tracing::info!("→ {} {} {:?}", method, uri, version);

    let res = next.run(req).await;

    let duration = start.elapsed();
    let status = res.status();

    if status.is_success() {
        tracing::info!("← {} {} ({}ms)", status, uri, duration.as_millis());
    } else if status.is_client_error() {
        tracing::warn!("← {} {} ({}ms)", status, uri, duration.as_millis());
    } else if status.is_server_error() {
        tracing::error!("← {} {} ({}ms)", status, uri, duration.as_millis());
    }

    res
}

/// Security headers middleware
pub async fn security_headers(
    req: Request,
    next: Next,
) -> Response {
    let mut res = next.run(req).await;

    let headers = res.headers_mut();
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());
    headers.insert("Referrer-Policy", "strict-origin-when-cross-origin".parse().unwrap());

    res
}

//! Vercel GraphQL API for Kotoba
//!
//! This module provides a GraphQL API endpoint for Vercel Functions
//! with Redis backend for graph database operations.

use std::sync::Arc;
use vercel_runtime::{run, Error, Request, Response};
use tower_http::cors::CorsLayer;
use tower::util::ServiceExt;
use axum::{
    body::{Body, to_bytes},
    routing::{get, post},
    Router, Extension,
};

mod graphql;
use graphql::{VercelContext, graphql_playground, health_check};

// Global context - initialized once per function instance
lazy_static::lazy_static! {
    static ref CONTEXT: Arc<VercelContext> = {
        let redis_url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        Arc::new(VercelContext::new(&redis_url))
    };
}

async fn graphql_handler(
    Extension(context): Extension<Arc<VercelContext>>,
    axum::Json(payload): axum::Json<serde_json::Value>,
) -> impl axum::response::IntoResponse {
    let request: async_graphql::Request = match serde_json::from_value(payload) {
        Ok(req) => req,
        Err(_) => async_graphql::Request::new(""),
    };

    let response = context.schema.execute(request).await;
    let json_response = serde_json::to_value(&response).unwrap_or(serde_json::Value::Null);

    axum::Json(json_response)
}

async fn handler(request: Request) -> Result<Response<vercel_runtime::Body>, Error> {
    // Create router for each request
    let app = Router::new()
        .route("/api/graphql", post(graphql_handler))
        .route("/api/graphql/playground", get(graphql_playground))
        .route("/api/health", get(health_check))
        .layer(Extension(CONTEXT.clone()))
        .layer(CorsLayer::permissive());

    // Convert Vercel request to Axum request
    let (parts, vercel_body) = request.into_parts();

    // Convert Vercel Body to Axum Body
    let axum_body = match vercel_body {
        vercel_runtime::Body::Text(text) => Body::from(text),
        vercel_runtime::Body::Binary(bytes) => Body::from(bytes),
        vercel_runtime::Body::Empty => Body::empty(),
    };

    let axum_request = axum::http::Request::from_parts(parts, axum_body);

    // Process with Axum
    let response = app.oneshot(axum_request).await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    // Convert Axum response back to Vercel response
    let (parts, axum_body) = response.into_parts();
    let body_bytes = to_bytes(axum_body, 1024 * 1024).await
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

    let vercel_body = vercel_runtime::Body::Binary(body_bytes.to_vec());

    Ok(Response::builder()
        .status(parts.status)
        .body(vercel_body)
        .map_err(|e| Error::from(std::io::Error::new(std::io::ErrorKind::Other, e)))?)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    println!("🚀 Starting Kotoba GraphQL API with Redis backend");

    run(handler).await
}

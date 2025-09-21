//! Vercel GraphQL API for Kotoba
//!
//! This module provides a GraphQL API endpoint for Vercel Functions
//! with Redis backend for graph database operations.

use std::sync::Arc;
use axum::{
    routing::{get, post},
    Router, Extension,
};
use vercel_runtime::{run, Error};
use tower_http::cors::CorsLayer;

mod graphql;
use graphql::{VercelContext, graphql_handler, graphql_playground, health_check};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Get Redis URL from environment
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());

    println!("🚀 Starting Kotoba GraphQL API with Redis backend");
    println!("📍 Redis URL: {}", redis_url);

    // Create context with Redis store
    let context = Arc::new(VercelContext::new(&redis_url));

    // Build the router
    let app = Router::new()
        .route("/api/graphql", post(graphql_handler))
        .route("/api/graphql/playground", get(graphql_playground))
        .route("/api/health", get(health_check))
        .layer(Extension(context))
        .layer(CorsLayer::permissive());

    // For Vercel Functions, we need to use the vercel_runtime
    run(app).await
}

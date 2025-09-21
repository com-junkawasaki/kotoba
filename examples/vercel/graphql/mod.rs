//! Vercel GraphQL API module

pub mod schema;
pub mod redis_store;

use std::sync::Arc;
use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use axum::response::{Html, IntoResponse};
use schema::{create_schema, KotobaSchema};
use redis_store::RedisGraphStore;

/// GraphQL context for Vercel
pub struct VercelContext {
    pub schema: KotobaSchema,
    pub store: Arc<RedisGraphStore>,
}

impl VercelContext {
    pub async fn new(redis_url: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let store = Arc::new(
            RedisGraphStore::new(redis_url, "kotoba:storage").await?
        );
        let schema = create_schema(store.clone());

        Ok(Self { schema, store })
    }
}

/// GraphQL playground handler
pub async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/api/graphql")
            .title("Kotoba GraphQL Playground")
    ))
}


/// Health check handler
pub async fn health_check() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "kotoba-graphql",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

//! HTTP server implementation

use axum::{
    Router,
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
// Simplified server implementation using axum
use std::net::SocketAddr;
use tower_http::cors::{CorsLayer, Any};
use tower_http::trace::TraceLayer;

use crate::{ServerError, Result};

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub cors_enabled: bool,
    pub tracing_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8100,
            cors_enabled: true,
            tracing_enabled: true,
        }
    }
}

/// HTTP server builder
#[derive(Debug)]
pub struct ServerBuilder {
    config: ServerConfig,
    router: Router,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            router: Router::new(),
        }
    }

    pub fn config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    pub fn cors_enabled(mut self, enabled: bool) -> Self {
        self.config.cors_enabled = enabled;
        self
    }

    pub fn tracing_enabled(mut self, enabled: bool) -> Self {
        self.config.tracing_enabled = enabled;
        self
    }

    pub fn build(self) -> Result<HttpServer> {
        let mut router = self.router;

        // Add CORS middleware
        if self.config.cors_enabled {
            router = router.layer(
                CorsLayer::new()
                    .allow_origin(Any)
                    .allow_methods(Any)
                    .allow_headers(Any),
            );
        }

        // Add tracing middleware
        if self.config.tracing_enabled {
            router = router.layer(TraceLayer::new_for_http());
        }

        Ok(HttpServer {
            config: self.config,
            router,
        })
    }
}

/// HTTP server instance
#[derive(Debug)]
pub struct HttpServer {
    config: ServerConfig,
    router: Router,
}

impl HttpServer {
    pub fn builder() -> ServerBuilder {
        ServerBuilder::new()
    }

    pub async fn serve(self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port)
            .parse::<SocketAddr>()
            .map_err(|e| ServerError::Config(format!("Invalid address: {}", e)))?;

        tracing::info!("🌐 Kotoba HTTP Server listening on {}", addr);

        axum::serve(
            tokio::net::TcpListener::bind(&addr).await?,
            self.router,
        )
        .await
        .map_err(|e| ServerError::Io(e))?;

        Ok(())
    }

    pub fn router(&self) -> &Router {
        &self.router
    }

    pub fn config(&self) -> &ServerConfig {
        &self.config
    }
}

//! # Server Implementation
//!
//! This module provides the HTTP server implementation for the Kotoba API,
//! including request handling, middleware, and server management.

use super::*;
use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::{IntoResponse, Json as AxumJson},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::{
    compression::CompressionLayer,
    cors::CorsLayer,
    timeout::TimeoutLayer,
    trace::TraceLayer,
};

/// API server for handling HTTP requests
#[derive(Debug)]
pub struct ApiServer {
    /// Server configuration
    pub config: ServerConfig,
    /// Application state
    pub state: Arc<ApiState>,
    /// Server handle for graceful shutdown
    pub server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Metrics
    pub metrics: Arc<ServerMetrics>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        config: ServerConfig,
        engine: ExecutionEngine,
        tx_log: TxLog,
    ) -> Self {
        let state = Arc::new(ApiState::new(engine, tx_log, config.clone()));
        let metrics = Arc::new(ServerMetrics::new());

        Self {
            config,
            state,
            server_handle: None,
            metrics,
        }
    }

    /// Start the server
    pub async fn start(&mut self) -> Result<(), ServerError> {
        let app = self.create_router();

        let addr = self.config.bind_address.parse::<SocketAddr>()
            .map_err(|e| ServerError::InvalidAddress(e.to_string()))?;

        println!("Starting Kotoba API server on {}", addr);

        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(self.shutdown_signal());

        self.server_handle = Some(tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("Server error: {}", e);
            }
        }));

        Ok(())
    }

    /// Stop the server gracefully
    pub async fn stop(&mut self) -> Result<(), ServerError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }

    /// Create the router with all routes and middleware
    fn create_router(&self) -> Router {
        Router::new()
            // Core API routes
            .route("/api/execute", post(api::execute_handler))
            .route("/api/defrefs", get(api::list_defrefs_handler))
            .route("/api/defrefs/:hash", get(api::get_defref_handler))
            .route("/api/defrefs/:hash/execute", post(api::execute_defref_handler))
            .route("/api/patches", post(api::apply_patch_handler))
            .route("/api/transactions/:hash", get(api::transaction_status_handler))
            .route("/api/transactions/:hash/replay", post(api::replay_transaction_handler))
            .route("/api/provenance", post(api::provenance_handler))
            .route("/batch", post(api::batch_handler))

            // Health and system routes
            .route("/health", get(api::health_handler))
            .route("/system/info", get(api::system_info_handler))
            .route("/system/stats", get(api::stats_handler))
            .route("/system/metrics", get(api::metrics_handler))

            // Metrics middleware
            .route_layer(middleware::from_fn_with_state(
                self.metrics.clone(),
                metrics_middleware,
            ))

            // Request logging middleware
            .route_layer(middleware::from_fn(request_logging_middleware))

            // Apply middleware layers
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CompressionLayer::new())
                    .layer(
                        CorsLayer::new()
                            .allow_origin("*")
                            .allow_methods("*")
                            .allow_headers("*")
                    )
                    .layer(TimeoutLayer::new(Duration::from_secs(
                        self.config.request_timeout_seconds
                    )))
                    .timeout(Duration::from_secs(self.config.request_timeout_seconds))
            )

            .with_state(self.state.clone())
    }

    /// Shutdown signal handler
    async fn shutdown_signal(&self) {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen for shutdown signal");
        println!("Shutdown signal received, stopping server...");
    }

    /// Get server metrics
    pub fn get_metrics(&self) -> ServerMetrics {
        (*self.metrics).clone()
    }

    /// Reset server metrics
    pub fn reset_metrics(&self) {
        *self.metrics = ServerMetrics::new();
    }
}

/// Server error
#[derive(Debug)]
pub enum ServerError {
    /// Invalid bind address
    InvalidAddress(String),
    /// Server startup error
    StartupError(String),
    /// Runtime error
    RuntimeError(String),
}

impl std::fmt::Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerError::InvalidAddress(msg) => write!(f, "Invalid address: {}", msg),
            ServerError::StartupError(msg) => write!(f, "Startup error: {}", msg),
            ServerError::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

impl std::error::Error for ServerError {}

/// Server metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetrics {
    /// Total requests processed
    pub total_requests: usize,
    /// Active connections
    pub active_connections: usize,
    /// Requests per second
    pub requests_per_second: f64,
    /// Average response time
    pub average_response_time: Duration,
    /// Error rate
    pub error_rate: f64,
    /// Server uptime
    pub uptime: Duration,
    /// Memory usage (if available)
    pub memory_usage_mb: Option<f64>,
    /// CPU usage (if available)
    pub cpu_usage_percent: Option<f64>,
}

impl ServerMetrics {
    /// Create new metrics
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            active_connections: 0,
            requests_per_second: 0.0,
            average_response_time: Duration::default(),
            error_rate: 0.0,
            uptime: Duration::default(),
            memory_usage_mb: None,
            cpu_usage_percent: None,
        }
    }

    /// Update metrics with request completion
    pub fn update_request(&mut self, success: bool, response_time: Duration) {
        self.total_requests += 1;

        self.average_response_time = (self.average_response_time * (self.total_requests - 1) as u32 + response_time) / self.total_requests as u32;

        // Update error rate
        if !success {
            self.error_rate = (self.error_rate * (self.total_requests - 1) as f64 + 1.0) / self.total_requests as f64;
        }

        // Update RPS (simplified)
        self.requests_per_second = self.total_requests as f64 / self.uptime.as_secs_f64();
    }

    /// Update connection count
    pub fn update_connections(&mut self, active_connections: usize) {
        self.active_connections = active_connections;
    }

    /// Update system metrics
    pub fn update_system_metrics(&mut self, memory_mb: Option<f64>, cpu_percent: Option<f64>) {
        self.memory_usage_mb = memory_mb;
        self.cpu_usage_percent = cpu_percent;
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            1.0
        } else {
            1.0 - self.error_rate
        }
    }
}

impl Default for ServerMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware functions

/// Metrics middleware
async fn metrics_middleware(
    State(metrics): State<Arc<ServerMetrics>>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> impl IntoResponse {
    let start_time = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let duration = start_time.elapsed();
    let status = response.status().as_u16();

    // Update metrics
    let mut metrics_guard = metrics.as_ref();
    metrics_guard.update_request(status < 400, duration);

    // Log the request
    println!(
        "[API] {} {} -> {} ({}ms)",
        method,
        uri,
        status,
        duration.as_millis()
    );

    response
}

/// Request logging middleware
async fn request_logging_middleware(
    request: axum::extract::Request,
    next: middleware::Next,
) -> impl IntoResponse {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let version = request.version();

    println!("[REQUEST] {} {} {:?}", method, uri, version);

    let response = next.run(request).await;

    println!("[RESPONSE] {} -> {}", uri, response.status());

    response
}

/// Metrics handler
async fn metrics_handler(State(metrics): State<Arc<ServerMetrics>>) -> impl IntoResponse {
    let metrics = (*metrics).clone();
    (StatusCode::OK, AxumJson(metrics)).into_response()
}

/// Additional server utilities

/// Server builder for fluent API
#[derive(Debug)]
pub struct ServerBuilder {
    config: ServerConfig,
    engine: Option<ExecutionEngine>,
    tx_log: Option<TxLog>,
}

impl ServerBuilder {
    /// Create a new server builder
    pub fn new() -> Self {
        Self {
            config: ServerConfig::default(),
            engine: None,
            tx_log: None,
        }
    }

    /// Set server configuration
    pub fn with_config(mut self, config: ServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Set execution engine
    pub fn with_engine(mut self, engine: ExecutionEngine) -> Self {
        self.engine = Some(engine);
        self
    }

    /// Set transaction log
    pub fn with_tx_log(mut self, tx_log: TxLog) -> Self {
        self.tx_log = Some(tx_log);
        self
    }

    /// Build the server
    pub fn build(self) -> Result<ApiServer, ServerError> {
        let engine = self.engine.ok_or(ServerError::StartupError("Execution engine not set".to_string()))?;
        let tx_log = self.tx_log.ok_or(ServerError::StartupError("Transaction log not set".to_string()))?;

        Ok(ApiServer::new(self.config, engine, tx_log))
    }
}

/// Server monitor for health checking and metrics collection
#[derive(Debug, Clone)]
pub struct ServerMonitor {
    /// Server metrics
    pub metrics: Arc<ServerMetrics>,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Metrics collection interval
    pub metrics_collection_interval: Duration,
}

impl ServerMonitor {
    /// Create a new server monitor
    pub fn new(metrics: Arc<ServerMetrics>) -> Self {
        Self {
            metrics,
            health_check_interval: Duration::from_secs(30),
            metrics_collection_interval: Duration::from_secs(60),
        }
    }

    /// Start monitoring
    pub async fn start_monitoring(&self) -> Result<(), Box<dyn std::error::Error>> {
        let metrics = self.metrics.clone();

        // Health check task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                // Perform health checks
                self.perform_health_checks().await;
            }
        });

        // Metrics collection task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;

                // Collect system metrics
                self.collect_system_metrics().await;
            }
        });

        Ok(())
    }

    /// Perform health checks
    async fn perform_health_checks(&self) {
        // Implementation would perform actual health checks
        // For now, just log
        println!("[HEALTH] Health check performed");
    }

    /// Collect system metrics
    async fn collect_system_metrics(&self) {
        // Implementation would collect actual system metrics
        // For now, just update uptime
        let mut metrics = (*self.metrics).clone();
        metrics.uptime = std::time::Instant::now().elapsed();
        *self.metrics = metrics;
    }
}

/// Load balancer integration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Load balancer addresses
    pub addresses: Vec<String>,
    /// Health check path
    pub health_check_path: String,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Unhealthy threshold
    pub unhealthy_threshold: usize,
    /// Healthy threshold
    pub healthy_threshold: usize,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            addresses: Vec::new(),
            health_check_path: "/health".to_string(),
            health_check_interval: Duration::from_secs(30),
            unhealthy_threshold: 3,
            healthy_threshold: 2,
        }
    }
}

/// Server configuration with SSL/TLS support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecureServerConfig {
    /// Basic server config
    pub server_config: ServerConfig,
    /// Enable SSL/TLS
    pub enable_ssl: bool,
    /// SSL certificate path
    pub ssl_cert_path: Option<String>,
    /// SSL key path
    pub ssl_key_path: Option<String>,
    /// Enable HTTP/2
    pub enable_http2: bool,
    /// Enable client certificate authentication
    pub enable_client_cert: bool,
}

impl Default for SecureServerConfig {
    fn default() -> Self {
        Self {
            server_config: ServerConfig::default(),
            enable_ssl: false,
            ssl_cert_path: None,
            ssl_key_path: None,
            enable_http2: true,
            enable_client_cert: false,
        }
    }
}

/// Secure API server with SSL/TLS support
#[derive(Debug)]
pub struct SecureApiServer {
    /// Inner server
    pub server: ApiServer,
    /// SSL configuration
    pub ssl_config: SecureServerConfig,
}

impl SecureApiServer {
    /// Create a new secure API server
    pub fn new(
        config: SecureServerConfig,
        engine: ExecutionEngine,
        tx_log: TxLog,
    ) -> Self {
        let server = ApiServer::new(config.server_config, engine, tx_log);

        Self {
            server,
            ssl_config: config,
        }
    }

    /// Start the secure server
    pub async fn start(&mut self) -> Result<(), ServerError> {
        // Implementation would set up SSL/TLS
        // For now, just start the regular server
        self.server.start().await
    }

    /// Stop the secure server
    pub async fn stop(&mut self) -> Result<(), ServerError> {
        self.server.stop().await
    }
}

/// Server diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDiagnostics {
    /// Server status
    pub status: ServerStatus,
    /// Active connections
    pub active_connections: usize,
    /// Memory usage
    pub memory_usage: MemoryUsage,
    /// CPU usage
    pub cpu_usage: CpuUsage,
    /// Disk usage
    pub disk_usage: DiskUsage,
    /// Network usage
    pub network_usage: NetworkUsage,
    /// Recent errors
    pub recent_errors: Vec<String>,
}

impl ServerDiagnostics {
    /// Create new diagnostics
    pub fn new() -> Self {
        Self {
            status: ServerStatus::Starting,
            active_connections: 0,
            memory_usage: MemoryUsage::default(),
            cpu_usage: CpuUsage::default(),
            disk_usage: DiskUsage::default(),
            network_usage: NetworkUsage::default(),
            recent_errors: Vec::new(),
        }
    }

    /// Add error to diagnostics
    pub fn add_error(&mut self, error: String) {
        self.recent_errors.push(error);

        // Keep only last 10 errors
        if self.recent_errors.len() > 10 {
            self.recent_errors.remove(0);
        }
    }

    /// Update status
    pub fn update_status(&mut self, status: ServerStatus) {
        self.status = status;
    }
}

/// Server status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerStatus {
    /// Server is starting
    Starting,
    /// Server is running
    Running,
    /// Server is stopping
    Stopping,
    /// Server is stopped
    Stopped,
    /// Server is in error state
    Error,
}

/// Memory usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryUsage {
    /// Used memory in MB
    pub used_mb: f64,
    /// Available memory in MB
    pub available_mb: f64,
    /// Total memory in MB
    pub total_mb: f64,
}

/// CPU usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CpuUsage {
    /// CPU usage percentage
    pub usage_percent: f64,
    /// Number of cores
    pub cores: usize,
}

/// Disk usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiskUsage {
    /// Used disk space in GB
    pub used_gb: f64,
    /// Available disk space in GB
    pub available_gb: f64,
    /// Total disk space in GB
    pub total_gb: f64,
}

/// Network usage
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkUsage {
    /// Bytes received
    pub bytes_received: u64,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Packets sent
    pub packets_sent: u64,
}

/// Server utilities
pub struct ServerUtils;

impl ServerUtils {
    /// Validate server configuration
    pub fn validate_config(config: &ServerConfig) -> Result<(), String> {
        if config.bind_address.is_empty() {
            return Err("Bind address cannot be empty".to_string());
        }

        if config.max_concurrent_requests == 0 {
            return Err("Max concurrent requests must be greater than 0".to_string());
        }

        if config.request_timeout_seconds == 0 {
            return Err("Request timeout must be greater than 0".to_string());
        }

        Ok(())
    }

    /// Get server info
    pub fn get_server_info() -> ServerInfo {
        ServerInfo {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_time: env!("VERGEN_BUILD_TIMESTAMP").unwrap_or("unknown"),
            git_sha: env!("VERGEN_GIT_SHA").unwrap_or("unknown"),
            rust_version: env!("VERGEN_RUSTC_SEMVER").unwrap_or("unknown"),
            features: vec![
                "api_server".to_string(),
                "ssl_support".to_string(),
                "metrics".to_string(),
                "health_checks".to_string(),
            ],
        }
    }

    /// Format duration for display
    pub fn format_duration(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Server info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    /// Version
    pub version: String,
    /// Build time
    pub build_time: String,
    /// Git SHA
    pub git_sha: String,
    /// Rust version
    pub rust_version: String,
    /// Enabled features
    pub features: Vec<String>,
}

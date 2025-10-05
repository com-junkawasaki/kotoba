//! # Client Interface
//!
//! This module provides client-side interfaces and utilities
//! for interacting with the Kotoba API.

use super::*;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;

/// API client for making requests to Kotoba API
#[derive(Debug, Clone)]
pub struct ApiClient {
    /// Base URL of the API server
    pub base_url: String,
    /// HTTP client
    pub http_client: reqwest::Client,
    /// Configuration
    pub config: ClientConfig,
    /// Authentication token
    pub auth_token: Option<String>,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
            config: ClientConfig::default(),
            auth_token: None,
        }
    }

    /// Create client with authentication
    pub fn with_auth(base_url: String, auth_token: String) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
            config: ClientConfig::default(),
            auth_token: Some(auth_token),
        }
    }

    /// Execute a request
    pub async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError> {
        let url = format!("{}/api/execute", self.base_url);

        let mut req = self.http_client
            .post(&url)
            .json(&request);

        // Add authentication if available
        if let Some(token) = &self.auth_token {
            req = req.bearer_auth(token);
        }

        let response = req.send().await?;

        if response.status().is_success() {
            let api_response = response.json::<ApiResponse>().await?;
            Ok(api_response)
        } else {
            let error_text = response.text().await.unwrap_or_default();
            Err(ClientError::ApiError(response.status().as_u16(), error_text))
        }
    }

    /// Execute DefRef
    pub async fn execute_defref(&self, def_ref: DefRef, options: ExecutionOptions) -> Result<ExecutionResult, ClientError> {
        let request = ApiRequest::new(
            format!("exec_def_{}", def_ref.hash),
            vec![ExecutionTarget::DefRef(def_ref)],
        ).with_options(options);

        let response = self.execute(request).await?;
        response.results.into_iter().next()
            .ok_or(ClientError::InvalidResponse)
    }

    /// Apply patch
    pub async fn apply_patch(&self, patch: Patch, context: ExecutionContext, options: ExecutionOptions) -> Result<ExecutionResult, ClientError> {
        let request = ApiRequest::new(
            format!("patch_{}", patch.patch_id),
            vec![ExecutionTarget::Patch(patch)],
        ).with_context(context)
        .with_options(options);

        let response = self.execute(request).await?;
        response.results.into_iter().next()
            .ok_or(ClientError::InvalidResponse)
    }

    /// Replay transaction
    pub async fn replay_transaction(&self, tx_ref: TransactionRef, options: ExecutionOptions) -> Result<ExecutionResult, ClientError> {
        let request = ApiRequest::new(
            format!("replay_{}", tx_ref.tx_id),
            vec![ExecutionTarget::Transaction(tx_ref)],
        ).with_options(options);

        let response = self.execute(request).await?;
        response.results.into_iter().next()
            .ok_or(ClientError::InvalidResponse)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        let url = format!("{}/health", self.base_url);
        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let health = response.json::<HealthResponse>().await?;
            Ok(health)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Get system information
    pub async fn system_info(&self) -> Result<SystemInfo, ClientError> {
        let url = format!("{}/system/info", self.base_url);
        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let info = response.json::<SystemInfo>().await?;
            Ok(info)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Get execution statistics
    pub async fn execution_stats(&self) -> Result<ExecutionStats, ClientError> {
        let url = format!("{}/system/stats", self.base_url);
        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let stats = response.json::<ExecutionStats>().await?;
            Ok(stats)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// List available DefRefs
    pub async fn list_defrefs(&self, params: ListDefRefsParams) -> Result<Vec<DefRef>, ClientError> {
        let url = format!("{}/api/defrefs", self.base_url);
        let response = self.http_client.get(&url).query(&params).send().await?;

        if response.status().is_success() {
            let defrefs = response.json::<Vec<DefRef>>().await?;
            Ok(defrefs)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Get DefRef details
    pub async fn get_defref(&self, hash: &str) -> Result<DefRef, ClientError> {
        let url = format!("{}/api/defrefs/{}", self.base_url, hash);
        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let defref = response.json::<DefRef>().await?;
            Ok(defref)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Get transaction status
    pub async fn transaction_status(&self, tx_hash: &str) -> Result<TransactionStatus, ClientError> {
        let url = format!("{}/api/transactions/{}", self.base_url, tx_hash);
        let response = self.http_client.get(&url).send().await?;

        if response.status().is_success() {
            let status = response.json::<TransactionStatus>().await?;
            Ok(status)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Query provenance
    pub async fn query_provenance(&self, query: ProvenanceQueryRequest) -> Result<ProvenanceInfo, ClientError> {
        let url = format!("{}/api/provenance", self.base_url);
        let response = self.http_client.post(&url).json(&query).send().await?;

        if response.status().is_success() {
            let provenance = response.json::<ProvenanceInfo>().await?;
            Ok(provenance)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }

    /// Batch execute multiple requests
    pub async fn batch_execute(&self, requests: Vec<ApiRequest>) -> Result<Vec<ApiResponse>, ClientError> {
        let batch_request = BatchRequest {
            request_id: format!("batch_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            requests,
        };

        let url = format!("{}/batch", self.base_url);
        let response = self.http_client.post(&url).json(&batch_request).send().await?;

        if response.status().is_success() {
            let batch_response = response.json::<BatchResponse>().await?;
            Ok(batch_response.responses)
        } else {
            Err(ClientError::HttpError(response.status().as_u16()))
        }
    }
}

/// Client error
#[derive(Debug, Clone)]
pub enum ClientError {
    /// HTTP error
    HttpError(u16),
    /// API error with status code and message
    ApiError(u16, String),
    /// Request timeout
    Timeout,
    /// Network error
    NetworkError(String),
    /// JSON serialization error
    JsonError(String),
    /// Invalid response
    InvalidResponse,
    /// Authentication error
    AuthError(String),
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::HttpError(code) => write!(f, "HTTP error: {}", code),
            ClientError::ApiError(code, msg) => write!(f, "API error {}: {}", code, msg),
            ClientError::Timeout => write!(f, "Request timeout"),
            ClientError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ClientError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            ClientError::InvalidResponse => write!(f, "Invalid response"),
            ClientError::AuthError(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for ClientError {}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
    /// Enable request compression
    pub enable_compression: bool,
    /// Maximum response size in bytes
    pub max_response_size: usize,
    /// User agent string
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_compression: true,
            max_response_size: 50 * 1024 * 1024, // 50MB
            user_agent: format!("kotoba-api-client/{}", env!("CARGO_PKG_VERSION")),
        }
    }
}

/// Client builder for fluent API
#[derive(Debug, Clone)]
pub struct ClientBuilder {
    base_url: String,
    auth_token: Option<String>,
    config: ClientConfig,
}

impl ClientBuilder {
    /// Create a new client builder
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            auth_token: None,
            config: ClientConfig::default(),
        }
    }

    /// Set authentication token
    pub fn with_auth(mut self, auth_token: String) -> Self {
        self.auth_token = Some(auth_token);
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.config.timeout_seconds = timeout_seconds;
        self
    }

    /// Set max retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.config.max_retries = max_retries;
        self
    }

    /// Enable compression
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.config.enable_compression = enable;
        self
    }

    /// Set max response size
    pub fn with_max_response_size(mut self, max_size: usize) -> Self {
        self.config.max_response_size = max_size;
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.config.user_agent = user_agent;
        self
    }

    /// Build the client
    pub fn build(self) -> ApiClient {
        let mut client = ApiClient::new(self.base_url);
        client.auth_token = self.auth_token;
        client.config = self.config;
        client
    }
}

/// Async client trait for testing and dependency injection
#[async_trait::async_trait]
pub trait AsyncApiClient {
    /// Execute request
    async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError>;

    /// Health check
    async fn health_check(&self) -> Result<HealthResponse, ClientError>;
}

#[async_trait::async_trait]
impl AsyncApiClient for ApiClient {
    async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError> {
        self.execute(request).await
    }

    async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.health_check().await
    }
}

/// Mock client for testing
#[derive(Debug, Clone)]
pub struct MockApiClient {
    /// Mock responses
    pub responses: HashMap<String, ApiResponse>,
    /// Request history
    pub request_history: Vec<ApiRequest>,
}

impl MockApiClient {
    /// Create a new mock client
    pub fn new() -> Self {
        Self {
            responses: HashMap::new(),
            request_history: Vec::new(),
        }
    }

    /// Add mock response
    pub fn add_response(&mut self, request_id: String, response: ApiResponse) {
        self.responses.insert(request_id, response);
    }

    /// Get request history
    pub fn get_request_history(&self) -> &[ApiRequest] {
        &self.request_history
    }

    /// Clear history
    pub fn clear_history(&mut self) {
        self.request_history.clear();
    }
}

#[async_trait::async_trait]
impl AsyncApiClient for MockApiClient {
    async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError> {
        self.request_history.push(request.clone());

        self.responses.get(&request.request_id)
            .cloned()
            .ok_or(ClientError::InvalidResponse)
    }

    async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        Ok(HealthResponse {
            status: "healthy".to_string(),
            version: "1.0.0".to_string(),
            uptime_seconds: 0,
            active_connections: 0,
        })
    }
}

/// Client utilities
pub struct ClientUtils;

impl ClientUtils {
    /// Create a DefRef execution request
    pub fn create_defref_request(def_ref: DefRef, options: ExecutionOptions) -> ApiRequest {
        ApiRequest::new(
            format!("exec_def_{}", def_ref.hash),
            vec![ExecutionTarget::DefRef(def_ref)],
        ).with_options(options)
    }

    /// Create a patch application request
    pub fn create_patch_request(patch: Patch, context: ExecutionContext, options: ExecutionOptions) -> ApiRequest {
        ApiRequest::new(
            format!("patch_{}", patch.patch_id),
            vec![ExecutionTarget::Patch(patch)],
        ).with_context(context)
        .with_options(options)
    }

    /// Create a transaction replay request
    pub fn create_replay_request(tx_ref: TransactionRef, options: ExecutionOptions) -> ApiRequest {
        ApiRequest::new(
            format!("replay_{}", tx_ref.tx_id),
            vec![ExecutionTarget::Transaction(tx_ref)],
        ).with_options(options)
    }

    /// Create a batch request
    pub fn create_batch_request(requests: Vec<ApiRequest>) -> BatchRequest {
        BatchRequest {
            request_id: format!("batch_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()),
            requests,
        }
    }

    /// Validate request before sending
    pub fn validate_request(request: &ApiRequest) -> Result<(), String> {
        if request.request_id.is_empty() {
            return Err("Request ID cannot be empty".to_string());
        }

        if request.targets.is_empty() {
            return Err("Request must have at least one target".to_string());
        }

        for target in &request.targets {
            match target {
                ExecutionTarget::DefRef(def_ref) => {
                    if def_ref.hash.to_string().is_empty() {
                        return Err("DefRef hash cannot be empty".to_string());
                    }
                }
                ExecutionTarget::Patch(patch) => {
                    if patch.patch_id.is_empty() {
                        return Err("Patch ID cannot be empty".to_string());
                    }
                    if patch.operations.is_empty() {
                        return Err("Patch must have at least one operation".to_string());
                    }
                }
                ExecutionTarget::Transaction(tx_ref) => {
                    if tx_ref.tx_id.is_empty() {
                        return Err("Transaction ID cannot be empty".to_string());
                    }
                }
            }
        }

        Ok(())
    }

    /// Estimate request cost
    pub fn estimate_request_cost(request: &ApiRequest) -> RequestCost {
        let mut estimated_memory_mb = 0.0;
        let mut estimated_cpu_time_ms = 0;

        for target in &request.targets {
            match target {
                ExecutionTarget::DefRef(_) => {
                    estimated_memory_mb += 100.0;
                    estimated_cpu_time_ms += 500;
                }
                ExecutionTarget::Patch(patch) => {
                    estimated_memory_mb += patch.operations.len() as f64 * 50.0;
                    estimated_cpu_time_ms += patch.operations.len() as u64 * 200;
                }
                ExecutionTarget::Transaction(_) => {
                    estimated_memory_mb += 200.0;
                    estimated_cpu_time_ms += 1000;
                }
            }
        }

        RequestCost {
            estimated_memory_mb,
            estimated_cpu_time_ms,
        }
    }
}

/// Request cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCost {
    /// Estimated memory usage in MB
    pub estimated_memory_mb: f64,
    /// Estimated CPU time in milliseconds
    pub estimated_cpu_time_ms: u64,
}

/// Client metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientMetrics {
    /// Total requests sent
    pub total_requests: usize,
    /// Successful requests
    pub successful_requests: usize,
    /// Failed requests
    pub failed_requests: usize,
    /// Average request time
    pub average_request_time: std::time::Duration,
    /// Total bytes sent
    pub total_bytes_sent: usize,
    /// Total bytes received
    pub total_bytes_received: usize,
    /// Request rate per second
    pub requests_per_second: f64,
}

impl Default for ClientMetrics {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            average_request_time: std::time::Duration::default(),
            total_bytes_sent: 0,
            total_bytes_received: 0,
            requests_per_second: 0.0,
        }
    }
}

impl ClientMetrics {
    /// Update metrics with request result
    pub fn update(&mut self, success: bool, request_time: std::time::Duration, bytes_sent: usize, bytes_received: usize) {
        self.total_requests += 1;

        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        self.average_request_time = (self.average_request_time * (self.total_requests - 1) as u32 + request_time) / self.total_requests as u32;
        self.total_bytes_sent += bytes_sent;
        self.total_bytes_received += bytes_received;

        // Update RPS (simplified)
        self.requests_per_second = self.total_requests as f64 / request_time.as_secs_f64();
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.successful_requests as f64 / self.total_requests as f64
        }
    }

    /// Get average bytes per request
    pub fn average_bytes_per_request(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.total_bytes_sent + self.total_bytes_received) as f64 / self.total_requests as f64
        }
    }
}

/// Client with metrics
#[derive(Debug, Clone)]
pub struct MetricsApiClient {
    /// Inner client
    pub client: ApiClient,
    /// Metrics
    pub metrics: Arc<std::sync::Mutex<ClientMetrics>>,
}

impl MetricsApiClient {
    /// Create a new metrics client
    pub fn new(base_url: String) -> Self {
        Self {
            client: ApiClient::new(base_url),
            metrics: Arc::new(std::sync::Mutex::new(ClientMetrics::default())),
        }
    }

    /// Get metrics
    pub fn get_metrics(&self) -> ClientMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        *self.metrics.lock().unwrap() = ClientMetrics::default();
    }
}

#[async_trait::async_trait]
impl AsyncApiClient for MetricsApiClient {
    async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError> {
        let start_time = std::time::Instant::now();

        let request_size = serde_json::to_vec(&request).map(|v| v.len()).unwrap_or(0);
        let result = self.client.execute(request).await;

        let response_size = match &result {
            Ok(response) => serde_json::to_vec(response).map(|v| v.len()).unwrap_or(0),
            Err(_) => 0,
        };

        let request_time = start_time.elapsed();
        let success = result.is_ok();

        self.metrics.lock().unwrap().update(
            success,
            request_time,
            request_size,
            response_size,
        );

        result
    }

    async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.client.health_check().await
    }
}

/// Retry client with automatic retry logic
#[derive(Debug, Clone)]
pub struct RetryApiClient {
    /// Inner client
    pub client: ApiClient,
    /// Configuration
    pub config: RetryConfig,
}

impl RetryApiClient {
    /// Create a new retry client
    pub fn new(client: ApiClient, config: RetryConfig) -> Self {
        Self { client, config }
    }
}

#[async_trait::async_trait]
impl AsyncApiClient for RetryApiClient {
    async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ClientError> {
        let mut last_error = None;

        for attempt in 0..=self.config.max_attempts {
            match self.client.execute(request.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);

                    // Don't retry on certain errors
                    if matches!(last_error, Some(ClientError::AuthError(_))) {
                        break;
                    }

                    // Don't retry if we've exceeded max attempts
                    if attempt >= self.config.max_attempts {
                        break;
                    }

                    // Wait before retrying
                    tokio::time::sleep(self.config.delay_between_attempts).await;
                }
            }
        }

        Err(last_error.unwrap_or(ClientError::InvalidResponse))
    }

    async fn health_check(&self) -> Result<HealthResponse, ClientError> {
        self.client.health_check().await
    }
}

/// Retry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Delay between attempts
    pub delay_between_attempts: std::time::Duration,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum delay
    pub max_delay: std::time::Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            delay_between_attempts: std::time::Duration::from_millis(1000),
            backoff_multiplier: 2.0,
            max_delay: std::time::Duration::from_secs(30),
        }
    }
}

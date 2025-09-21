//! # Kotoba API
//!
//! DefRef/patch resolution and execution API layer.
//!
//! This crate provides a unified API for resolving DefRef/patch references
//! and executing graph transformations. It serves as the main interface
//! between clients and the Kotoba system.

pub mod api;
pub mod resolver;
pub mod executor;
pub mod client;
pub mod server;

use kotoba_types::*;
use kotoba_codebase::*;
use kotoba_compose::*;
use kotoba_rewrite_kernel::*;
use kotoba_graph_core::*;
use kotoba_txlog::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API request for DefRef/patch resolution and execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRequest {
    /// Request ID
    pub request_id: String,
    /// DefRef/patch references to resolve and execute
    pub targets: Vec<ExecutionTarget>,
    /// Execution context
    pub context: ExecutionContext,
    /// Options
    pub options: ExecutionOptions,
}

impl ApiRequest {
    /// Create a new API request
    pub fn new(request_id: String, targets: Vec<ExecutionTarget>) -> Self {
        Self {
            request_id,
            targets,
            context: ExecutionContext::default(),
            options: ExecutionOptions::default(),
        }
    }

    /// Add execution context
    pub fn with_context(mut self, context: ExecutionContext) -> Self {
        self.context = context;
        self
    }

    /// Add execution options
    pub fn with_options(mut self, options: ExecutionOptions) -> Self {
        self.options = options;
        self
    }
}

/// Execution target (DefRef or patch)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionTarget {
    /// DefRef to resolve and execute
    DefRef(DefRef),
    /// Patch to apply
    Patch(Patch),
    /// Transaction to replay
    Transaction(TransactionRef),
}

impl ExecutionTarget {
    /// Get target type as string
    pub fn target_type(&self) -> &'static str {
        match self {
            ExecutionTarget::DefRef(_) => "def_ref",
            ExecutionTarget::Patch(_) => "patch",
            ExecutionTarget::Transaction(_) => "transaction",
        }
    }
}

/// Patch representation for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    /// Patch ID
    pub patch_id: String,
    /// Description
    pub description: String,
    /// Operations in the patch
    pub operations: Vec<PatchOperation>,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

impl Patch {
    /// Create a new patch
    pub fn new(patch_id: String, description: String) -> Self {
        Self {
            patch_id,
            description,
            operations: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add operation
    pub fn with_operation(mut self, operation: PatchOperation) -> Self {
        self.operations.push(operation);
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Patch operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatchOperation {
    /// Add a definition
    AddDef(DefRef),
    /// Remove a definition
    RemoveDef(DefRef),
    /// Transform graph
    TransformGraph {
        input_ref: DefRef,
        rule_ref: DefRef,
        strategy_ref: Option<DefRef>,
    },
    /// Migrate schema
    MigrateSchema {
        from_ref: DefRef,
        to_ref: DefRef,
        rules: Vec<DefRef>,
    },
}

/// Execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Available DefRefs
    pub available_defs: HashMap<DefRef, Value>,
    /// Current graph state
    pub graph_state: Option<GraphRef>,
    /// Transaction log
    pub tx_log: Option<TransactionRef>,
    /// Environment variables
    pub environment: HashMap<String, String>,
    /// Resource limits
    pub limits: ResourceLimits,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            available_defs: HashMap::new(),
            graph_state: None,
            tx_log: None,
            environment: HashMap::new(),
            limits: ResourceLimits::default(),
        }
    }
}

impl ExecutionContext {
    /// Create a new execution context
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a DefRef
    pub fn with_def(mut self, def_ref: DefRef, value: Value) -> Self {
        self.available_defs.insert(def_ref, value);
        self
    }

    /// Set graph state
    pub fn with_graph_state(mut self, graph_ref: GraphRef) -> Self {
        self.graph_state = Some(graph_ref);
        self
    }

    /// Set transaction log
    pub fn with_tx_log(mut self, tx_ref: TransactionRef) -> Self {
        self.tx_log = Some(tx_ref);
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.environment.insert(key, value);
        self
    }

    /// Set resource limits
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }
}

/// Execution options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionOptions {
    /// Execution mode
    pub mode: ExecutionMode,
    /// Enable parallel execution
    pub parallel: bool,
    /// Enable validation
    pub validate: bool,
    /// Enable provenance tracking
    pub track_provenance: bool,
    /// Enable witness collection
    pub collect_witnesses: bool,
    /// Timeout in seconds
    pub timeout_seconds: Option<u64>,
    /// Maximum memory usage
    pub max_memory_mb: Option<usize>,
}

impl Default for ExecutionOptions {
    fn default() -> Self {
        Self {
            mode: ExecutionMode::Normal,
            parallel: true,
            validate: true,
            track_provenance: true,
            collect_witnesses: true,
            timeout_seconds: Some(300),
            max_memory_mb: Some(1024),
        }
    }
}

impl ExecutionOptions {
    /// Create options for fast execution
    pub fn fast() -> Self {
        Self {
            mode: ExecutionMode::Fast,
            parallel: true,
            validate: false,
            track_provenance: false,
            collect_witnesses: false,
            timeout_seconds: Some(30),
            max_memory_mb: Some(512),
        }
    }

    /// Create options for safe execution
    pub fn safe() -> Self {
        Self {
            mode: ExecutionMode::Safe,
            parallel: false,
            validate: true,
            track_provenance: true,
            collect_witnesses: true,
            timeout_seconds: Some(600),
            max_memory_mb: Some(2048),
        }
    }
}

/// Execution mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Normal execution
    Normal,
    /// Fast execution (less validation, more performance)
    Fast,
    /// Safe execution (more validation, less performance)
    Safe,
    /// Debug execution (detailed logging, single-threaded)
    Debug,
}

/// Resource limits for execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum execution time in seconds
    pub max_time_seconds: Option<u64>,
    /// Maximum memory usage in MB
    pub max_memory_mb: Option<usize>,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: Option<f64>,
    /// Maximum number of concurrent operations
    pub max_concurrent_operations: Option<usize>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_time_seconds: Some(300),
            max_memory_mb: Some(1024),
            max_cpu_percent: Some(80.0),
            max_concurrent_operations: Some(10),
        }
    }
}

/// API response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    /// Request ID
    pub request_id: String,
    /// Success status
    pub success: bool,
    /// Results
    pub results: Vec<ExecutionResult>,
    /// Execution time
    pub execution_time_ms: u64,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Metadata
    pub metadata: HashMap<String, Value>,
}

impl ApiResponse {
    /// Create a successful response
    pub fn success(request_id: String, results: Vec<ExecutionResult>, execution_time_ms: u64) -> Self {
        Self {
            request_id,
            success: true,
            results,
            execution_time_ms,
            error_message: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a failed response
    pub fn failure(request_id: String, error_message: String, execution_time_ms: u64) -> Self {
        Self {
            request_id,
            success: false,
            results: Vec::new(),
            execution_time_ms,
            error_message: Some(error_message),
            metadata: HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: String, value: Value) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Target that was executed
    pub target: ExecutionTarget,
    /// Success status
    pub success: bool,
    /// Output DefRefs
    pub outputs: Vec<DefRef>,
    /// Execution time
    pub execution_time_ms: u64,
    /// Transaction created
    pub transaction_ref: Option<TransactionRef>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Provenance information
    pub provenance: Option<ProvenanceInfo>,
}

impl ExecutionResult {
    /// Create a successful result
    pub fn success(
        target: ExecutionTarget,
        outputs: Vec<DefRef>,
        execution_time_ms: u64,
        transaction_ref: Option<TransactionRef>,
    ) -> Self {
        Self {
            target,
            success: true,
            outputs,
            execution_time_ms,
            transaction_ref,
            error_message: None,
            provenance: None,
        }
    }

    /// Create a failed result
    pub fn failure(
        target: ExecutionTarget,
        error_message: String,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            target,
            success: false,
            outputs: Vec::new(),
            execution_time_ms,
            transaction_ref: None,
            error_message: Some(error_message),
            provenance: None,
        }
    }

    /// Add provenance information
    pub fn with_provenance(mut self, provenance: ProvenanceInfo) -> Self {
        self.provenance = Some(provenance);
        self
    }
}

/// Provenance information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceInfo {
    /// Transaction chain
    pub transaction_chain: Vec<TransactionRef>,
    /// Input DefRefs
    pub input_refs: Vec<DefRef>,
    /// Output DefRefs
    pub output_refs: Vec<DefRef>,
    /// Witnesses collected
    pub witnesses: Vec<DefRef>,
}

impl ProvenanceInfo {
    /// Create new provenance info
    pub fn new() -> Self {
        Self {
            transaction_chain: Vec::new(),
            input_refs: Vec::new(),
            output_refs: Vec::new(),
            witnesses: Vec::new(),
        }
    }

    /// Add transaction to chain
    pub fn with_transaction(mut self, tx_ref: TransactionRef) -> Self {
        self.transaction_chain.push(tx_ref);
        self
    }

    /// Add input DefRef
    pub fn with_input(mut self, input_ref: DefRef) -> Self {
        self.input_refs.push(input_ref);
        self
    }

    /// Add output DefRef
    pub fn with_output(mut self, output_ref: DefRef) -> Self {
        self.output_refs.push(output_ref);
        self
    }

    /// Add witness
    pub fn with_witness(mut self, witness_ref: DefRef) -> Self {
        self.witnesses.push(witness_ref);
        self
    }
}

/// API client for interacting with Kotoba API
#[derive(Debug, Clone)]
pub struct ApiClient {
    /// Base URL
    pub base_url: String,
    /// HTTP client
    pub client: reqwest::Client,
    /// Configuration
    pub config: ClientConfig,
}

impl ApiClient {
    /// Create a new API client
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
            config: ClientConfig::default(),
        }
    }

    /// Execute a request
    pub async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let url = format!("{}/api/execute", self.base_url);

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await?;

        if response.status().is_success() {
            let api_response = response.json::<ApiResponse>().await?;
            Ok(api_response)
        } else {
            Err(ApiError::HttpError(response.status().as_u16()))
        }
    }

    /// Health check
    pub async fn health_check(&self) -> Result<HealthResponse, ApiError> {
        let url = format!("{}/health", self.base_url);

        let response = self.client
            .get(&url)
            .send()
            .await?;

        if response.status().is_success() {
            let health = response.json::<HealthResponse>().await?;
            Ok(health)
        } else {
            Err(ApiError::HttpError(response.status().as_u16()))
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Request timeout in seconds
    pub timeout_seconds: u64,
    /// Maximum number of retries
    pub max_retries: u32,
    /// Retry delay in milliseconds
    pub retry_delay_ms: u64,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
        }
    }
}

/// Health response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    /// Status
    pub status: String,
    /// Version
    pub version: String,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Active connections
    pub active_connections: usize,
}

/// API error
#[derive(Debug, Clone)]
pub enum ApiError {
    /// HTTP error
    HttpError(u16),
    /// Request timeout
    Timeout,
    /// Network error
    NetworkError(String),
    /// JSON serialization error
    JsonError(String),
    /// Invalid response
    InvalidResponse,
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::HttpError(code) => write!(f, "HTTP error: {}", code),
            ApiError::Timeout => write!(f, "Request timeout"),
            ApiError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            ApiError::JsonError(msg) => write!(f, "JSON error: {}", msg),
            ApiError::InvalidResponse => write!(f, "Invalid response"),
        }
    }
}

impl std::error::Error for ApiError {}

/// API server
#[derive(Debug, Clone)]
pub struct ApiServer {
    /// Server configuration
    pub config: ServerConfig,
    /// Execution engine
    pub executor: ExecutionEngine,
    /// Transaction log
    pub tx_log: TxLog,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(config: ServerConfig, executor: ExecutionEngine, tx_log: TxLog) -> Self {
        Self {
            config,
            executor,
            tx_log,
        }
    }

    /// Start the server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        use axum::{routing::post, Router};

        let app = Router::new()
            .route("/api/execute", post(api::execute_handler))
            .route("/health", axum::routing::get(api::health_handler));

        let listener = tokio::net::TcpListener::bind(&self.config.bind_address).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Bind address
    pub bind_address: String,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Request timeout in seconds
    pub request_timeout_seconds: u64,
    /// Enable CORS
    pub enable_cors: bool,
    /// Enable metrics
    pub enable_metrics: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_address: "127.0.0.1:8080".to_string(),
            max_concurrent_requests: 100,
            request_timeout_seconds: 300,
            enable_cors: true,
            enable_metrics: true,
        }
    }
}

/// Execution engine for processing requests
#[derive(Debug, Clone)]
pub struct ExecutionEngine {
    /// Rewrite kernel
    pub rewrite_kernel: RewriteKernel,
    /// Graph processor
    pub graph_processor: GraphProcessor,
    /// Resolver
    pub resolver: DefRefResolver,
    /// Configuration
    pub config: EngineConfig,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(
        rewrite_kernel: RewriteKernel,
        graph_processor: GraphProcessor,
        resolver: DefRefResolver,
    ) -> Self {
        Self {
            rewrite_kernel,
            graph_processor,
            resolver,
            config: EngineConfig::default(),
        }
    }

    /// Execute a request
    pub async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let start_time = std::time::Instant::now();

        // Resolve DefRef/patch references
        let resolved_targets = self.resolver.resolve_targets(&request.targets).await?;

        // Execute each target
        let mut results = Vec::new();
        for (target, resolved) in request.targets.iter().zip(resolved_targets.iter()) {
            let result = self.execute_target(target, resolved, &request.context, &request.options).await?;
            results.push(result);
        }

        let execution_time = start_time.elapsed();

        Ok(ApiResponse::success(
            request.request_id,
            results,
            execution_time.as_millis() as u64,
        ))
    }

    /// Execute a single target
    async fn execute_target(
        &self,
        target: &ExecutionTarget,
        resolved: &ResolvedTarget,
        context: &ExecutionContext,
        options: &ExecutionOptions,
    ) -> Result<ExecutionResult, ApiError> {
        match target {
            ExecutionTarget::DefRef(def_ref) => {
                self.execute_def_ref(def_ref, resolved, context, options).await
            },
            ExecutionTarget::Patch(patch) => {
                self.execute_patch(patch, resolved, context, options).await
            },
            ExecutionTarget::Transaction(tx_ref) => {
                self.execute_transaction(tx_ref, resolved, context, options).await
            }
        }
    }

    /// Execute a DefRef
    async fn execute_def_ref(
        &self,
        def_ref: &DefRef,
        resolved: &ResolvedTarget,
        context: &ExecutionContext,
        options: &ExecutionOptions,
    ) -> Result<ExecutionResult, ApiError> {
        // Implementation would execute the resolved DefRef
        Ok(ExecutionResult::success(
            ExecutionTarget::DefRef(def_ref.clone()),
            vec![def_ref.clone()],
            0,
            None,
        ))
    }

    /// Execute a patch
    async fn execute_patch(
        &self,
        patch: &Patch,
        resolved: &ResolvedTarget,
        context: &ExecutionContext,
        options: &ExecutionOptions,
    ) -> Result<ExecutionResult, ApiError> {
        // Implementation would apply the patch
        Ok(ExecutionResult::success(
            ExecutionTarget::Patch(patch.clone()),
            vec![],
            0,
            None,
        ))
    }

    /// Execute a transaction
    async fn execute_transaction(
        &self,
        tx_ref: &TransactionRef,
        resolved: &ResolvedTarget,
        context: &ExecutionContext,
        options: &ExecutionOptions,
    ) -> Result<ExecutionResult, ApiError> {
        // Implementation would replay the transaction
        Ok(ExecutionResult::success(
            ExecutionTarget::Transaction(tx_ref.clone()),
            vec![],
            0,
            None,
        ))
    }
}

/// Resolved target
#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    /// Resolved DefRefs
    pub def_refs: Vec<DefRef>,
    /// Execution plan
    pub execution_plan: ExecutionPlan,
    /// Dependencies
    pub dependencies: Vec<DefRef>,
}

/// DefRef resolver
#[derive(Debug, Clone)]
pub struct DefRefResolver {
    /// Resolution cache
    pub cache: HashMap<DefRef, ResolvedTarget>,
    /// Configuration
    pub config: ResolverConfig,
}

impl DefRefResolver {
    /// Create a new resolver
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            config: ResolverConfig::default(),
        }
    }

    /// Resolve execution targets
    pub async fn resolve_targets(&self, targets: &[ExecutionTarget]) -> Result<Vec<ResolvedTarget>, ApiError> {
        let mut resolved = Vec::new();

        for target in targets {
            let resolved_target = self.resolve_target(target).await?;
            resolved.push(resolved_target);
        }

        Ok(resolved)
    }

    /// Resolve a single target
    async fn resolve_target(&self, target: &ExecutionTarget) -> Result<ResolvedTarget, ApiError> {
        match target {
            ExecutionTarget::DefRef(def_ref) => self.resolve_def_ref(def_ref),
            ExecutionTarget::Patch(patch) => self.resolve_patch(patch),
            ExecutionTarget::Transaction(tx_ref) => self.resolve_transaction(tx_ref),
        }
    }

    /// Resolve a DefRef
    fn resolve_def_ref(&self, def_ref: &DefRef) -> Result<ResolvedTarget, ApiError> {
        // Check cache first
        if let Some(cached) = self.cache.get(def_ref) {
            return Ok(cached.clone());
        }

        // Implementation would resolve the DefRef
        Ok(ResolvedTarget {
            def_refs: vec![def_ref.clone()],
            execution_plan: ExecutionPlan::default(),
            dependencies: Vec::new(),
        })
    }

    /// Resolve a patch
    fn resolve_patch(&self, patch: &Patch) -> Result<ResolvedTarget, ApiError> {
        // Implementation would resolve patch operations
        Ok(ResolvedTarget {
            def_refs: Vec::new(),
            execution_plan: ExecutionPlan::default(),
            dependencies: Vec::new(),
        })
    }

    /// Resolve a transaction
    fn resolve_transaction(&self, tx_ref: &TransactionRef) -> Result<ResolvedTarget, ApiError> {
        // Implementation would resolve transaction dependencies
        Ok(ResolvedTarget {
            def_refs: Vec::new(),
            execution_plan: ExecutionPlan::default(),
            dependencies: Vec::new(),
        })
    }
}

/// Resolver configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    /// Cache size
    pub cache_size: usize,
    /// Timeout for resolution
    pub timeout_seconds: u64,
    /// Maximum recursion depth
    pub max_depth: usize,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_size: 10000,
            timeout_seconds: 30,
            max_depth: 100,
        }
    }
}

/// Execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Execution steps
    pub steps: Vec<ExecutionStep>,
    /// Estimated execution time
    pub estimated_time_ms: u64,
    /// Parallel execution enabled
    pub parallel: bool,
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self {
            steps: Vec::new(),
            estimated_time_ms: 0,
            parallel: false,
        }
    }
}

/// Execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step ID
    pub step_id: String,
    /// Operation to perform
    pub operation: ExecutionOperation,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Estimated duration
    pub estimated_duration_ms: u64,
}

/// Execution operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionOperation {
    /// Resolve DefRef
    ResolveDef(DefRef),
    /// Apply patch
    ApplyPatch(Patch),
    /// Execute transaction
    ExecuteTransaction(TransactionRef),
    /// Validate result
    ValidateResult,
}

/// Engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineConfig {
    /// Enable caching
    pub enable_caching: bool,
    /// Enable parallel processing
    pub enable_parallel: bool,
    /// Maximum concurrent operations
    pub max_concurrent: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            enable_parallel: true,
            max_concurrent: 10,
            enable_metrics: true,
        }
    }
}

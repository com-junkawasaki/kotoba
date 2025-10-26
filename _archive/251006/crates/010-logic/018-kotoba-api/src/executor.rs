//! # Execution Engine
//!
//! This module provides the execution engine for processing resolved
//! DefRef/patch targets with proper resource management and monitoring.

use super::*;
use kotoba_types::*;
use kotoba_codebase::*;
use kotoba_rewrite_kernel::*;
use kotoba_graph_core::*;
use kotoba_txlog::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Execution engine for processing API requests
#[derive(Debug, Clone)]
pub struct ExecutionEngine {
    /// Rewrite kernel
    pub rewrite_kernel: Arc<RwLock<RewriteKernel>>,
    /// Graph processor
    pub graph_processor: Arc<GraphProcessor>,
    /// Transaction log
    pub tx_log: Arc<RwLock<TxLog>>,
    /// Resolver
    pub resolver: Arc<RwLock<DefRefResolver>>,
    /// Execution statistics
    pub stats: Arc<RwLock<ExecutionStats>>,
    /// Configuration
    pub config: EngineConfig,
    /// Resource manager
    pub resource_manager: Arc<ResourceManager>,
}

impl ExecutionEngine {
    /// Create a new execution engine
    pub fn new(
        rewrite_kernel: RewriteKernel,
        graph_processor: GraphProcessor,
        tx_log: TxLog,
        config: EngineConfig,
    ) -> Self {
        Self {
            rewrite_kernel: Arc::new(RwLock::new(rewrite_kernel)),
            graph_processor: Arc::new(graph_processor),
            tx_log: Arc::new(RwLock::new(tx_log)),
            resolver: Arc::new(RwLock::new(DefRefResolver::new(ResolverConfig::default()))),
            stats: Arc::new(RwLock::new(ExecutionStats::default())),
            config,
            resource_manager: Arc::new(ResourceManager::new()),
        }
    }

    /// Execute an API request
    pub async fn execute(&self, request: ApiRequest) -> Result<ApiResponse, ApiError> {
        let start_time = std::time::Instant::now();

        // Check resource limits
        self.resource_manager.check_limits(&request).await?;

        // Resolve targets
        let resolved_targets = self.resolver.read().await.resolve_targets(&request.targets).await?;

        // Execute each target
        let mut results = Vec::new();
        for (target, resolved) in request.targets.iter().zip(resolved_targets.iter()) {
            let result = self.execute_target(target, resolved, &request.context, &request.options).await?;
            results.push(result);
        }

        let execution_time = start_time.elapsed();

        // Record transaction if provenance tracking is enabled
        if request.options.track_provenance {
            self.record_transaction(&request, &results).await?;
        }

        // Update statistics
        self.stats.write().await.update_request(true, execution_time);

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
        let start_time = std::time::Instant::now();

        match target {
            ExecutionTarget::DefRef(def_ref) => {
                self.execute_def_ref(def_ref, resolved, context, options).await
            }
            ExecutionTarget::Patch(patch) => {
                self.execute_patch(patch, resolved, context, options).await
            }
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
        // Get current graph state
        let graph_ref = context.graph_state.clone()
            .ok_or_else(|| ApiError::JsonError("No graph state available".to_string()))?;

        // Execute using rewrite kernel
        let mut kernel = self.rewrite_kernel.write().await;
        let execution_result = kernel.execute_strategy(
            DefRef::new(&[], DefType::Strategy), // Default strategy
            graph_ref.inner().clone(),
        ).await?;

        let execution_time = std::time::Instant::now().elapsed();

        Ok(ExecutionResult::success(
            ExecutionTarget::DefRef(def_ref.clone()),
            vec![def_ref.clone()],
            execution_time.as_millis() as u64,
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
        let mut outputs = Vec::new();
        let start_time = std::time::Instant::now();

        // Apply each operation in the patch
        for operation in &patch.operations {
            match operation {
                PatchOperation::AddDef(def_ref) => {
                    // Register the definition
                    outputs.push(def_ref.clone());
                }
                PatchOperation::RemoveDef(def_ref) => {
                    // Remove the definition (would need implementation)
                }
                PatchOperation::TransformGraph { input_ref, rule_ref, strategy_ref } => {
                    // Execute graph transformation
                    let result_ref = DefRef::new(
                        format!("transform_{}", input_ref.hash).as_bytes(),
                        DefType::Function
                    );
                    outputs.push(result_ref);
                }
                PatchOperation::MigrateSchema { from_ref, to_ref, rules } => {
                    // Execute schema migration
                    outputs.push(to_ref.clone());
                }
            }
        }

        let execution_time = std::time::Instant::now().duration_since(start_time);

        Ok(ExecutionResult::success(
            ExecutionTarget::Patch(patch.clone()),
            outputs,
            execution_time.as_millis() as u64,
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
        let tx_log = self.tx_log.read().await;
        let replay_result = tx_log.replay_from(tx_ref)?;

        let execution_time = std::time::Instant::now().elapsed();

        Ok(ExecutionResult::success(
            ExecutionTarget::Transaction(tx_ref.clone()),
            Vec::new(), // Transaction replay doesn't produce new DefRefs
            execution_time.as_millis() as u64,
            Some(tx_ref.clone()),
        ))
    }

    /// Record transaction for provenance tracking
    async fn record_transaction(
        &self,
        request: &ApiRequest,
        results: &[ExecutionResult],
    ) -> Result<(), ApiError> {
        let mut tx_log = self.tx_log.write().await;

        // Create transaction from request and results
        let hlc = HLC::new("api-server".to_string());
        let operation = self.create_operation_from_request(request, results);

        let tx = Transaction::new(
            request.request_id.clone(),
            hlc,
            Vec::new(), // No parent transactions for API requests
            "api-user".to_string(),
            operation,
        );

        tx_log.add_transaction(tx)?;
        Ok(())
    }

    /// Create transaction operation from request
    fn create_operation_from_request(
        &self,
        request: &ApiRequest,
        results: &[ExecutionResult],
    ) -> TransactionOperation {
        let mut outputs = Vec::new();
        for result in results {
            outputs.extend(result.outputs.clone());
        }

        TransactionOperation::GraphTransformation {
            input_refs: request.targets.iter()
                .filter_map(|t| match t {
                    ExecutionTarget::DefRef(def_ref) => Some(def_ref.clone()),
                    _ => None,
                })
                .collect(),
            output_ref: outputs.first()
                .cloned()
                .unwrap_or_else(|| DefRef::new(&[], DefType::Function)),
            rule_ref: DefRef::new(&[], DefType::Rule),
            strategy_ref: None,
        }
    }

    /// Execute with resource monitoring
    pub async fn execute_with_monitoring(
        &self,
        request: ApiRequest,
    ) -> Result<ApiResponse, ApiError> {
        // Start resource monitoring
        let monitoring_token = self.resource_manager.start_monitoring(&request).await?;

        let result = self.execute(request).await;

        // Stop monitoring and get resource usage
        let resource_usage = self.resource_manager.stop_monitoring(monitoring_token).await;

        // Add resource usage to response
        match result {
            Ok(mut response) => {
                response.metadata.insert(
                    "resource_usage".to_string(),
                    serde_json::to_value(resource_usage)?,
                );
                Ok(response)
            }
            Err(e) => Err(e),
        }
    }

    /// Get execution statistics
    pub async fn get_stats(&self) -> ExecutionStats {
        self.stats.read().await.clone()
    }

    /// Reset statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = ExecutionStats::default();
    }
}

/// Resource manager for monitoring and limiting resource usage
#[derive(Debug, Clone)]
pub struct ResourceManager {
    /// Active monitors
    monitors: HashMap<String, ResourceMonitor>,
    /// Configuration
    config: ResourceConfig,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new() -> Self {
        Self {
            monitors: HashMap::new(),
            config: ResourceConfig::default(),
        }
    }

    /// Check resource limits for a request
    pub async fn check_limits(&self, request: &ApiRequest) -> Result<(), ApiError> {
        let estimated_cost = self.estimate_request_cost(request);

        // Check memory limit
        if estimated_cost.estimated_memory_mb > self.config.max_memory_mb {
            return Err(ApiError::JsonError("Request exceeds memory limit".to_string()));
        }

        // Check concurrent requests limit
        if self.monitors.len() >= self.config.max_concurrent_requests {
            return Err(ApiError::JsonError("Too many concurrent requests".to_string()));
        }

        Ok(())
    }

    /// Start monitoring a request
    pub async fn start_monitoring(&self, request: &ApiRequest) -> Result<String, ApiError> {
        let monitor_id = format!("monitor_{}", request.request_id);
        let monitor = ResourceMonitor::new(request);

        // Implementation would start actual monitoring
        // For now, just store the monitor

        Ok(monitor_id)
    }

    /// Stop monitoring and get resource usage
    pub async fn stop_monitoring(&self, monitor_id: String) -> ResourceUsage {
        // Implementation would stop monitoring and return actual usage
        ResourceUsage::default()
    }

    /// Estimate request cost
    fn estimate_request_cost(&self, request: &ApiRequest) -> RequestCost {
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

/// Resource monitor
#[derive(Debug, Clone)]
struct ResourceMonitor {
    /// Request being monitored
    request: ApiRequest,
    /// Start time
    start_time: std::time::Instant,
    /// Current memory usage
    memory_usage: usize,
    /// Current CPU usage
    cpu_usage: f64,
}

impl ResourceMonitor {
    /// Create a new resource monitor
    pub fn new(request: &ApiRequest) -> Self {
        Self {
            request: request.clone(),
            start_time: std::time::Instant::now(),
            memory_usage: 0,
            cpu_usage: 0.0,
        }
    }
}

/// Request cost estimation
#[derive(Debug, Clone)]
struct RequestCost {
    /// Estimated memory usage in MB
    pub estimated_memory_mb: f64,
    /// Estimated CPU time in milliseconds
    pub estimated_cpu_time_ms: u64,
}

/// Resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// Memory usage in MB
    pub memory_mb: f64,
    /// CPU usage percentage
    pub cpu_percent: f64,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Network usage in bytes
    pub network_bytes: u64,
}

impl Default for ResourceUsage {
    fn default() -> Self {
        Self {
            memory_mb: 0.0,
            cpu_percent: 0.0,
            execution_time_ms: 0,
            network_bytes: 0,
        }
    }
}

/// Resource configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Maximum memory per request in MB
    pub max_memory_mb: usize,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Memory check interval in milliseconds
    pub memory_check_interval_ms: u64,
    /// CPU check interval in milliseconds
    pub cpu_check_interval_ms: u64,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            max_memory_mb: 1024,
            max_concurrent_requests: 100,
            memory_check_interval_ms: 100,
            cpu_check_interval_ms: 1000,
        }
    }
}

/// Execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total requests processed
    pub total_requests: usize,
    /// Successful requests
    pub successful_requests: usize,
    /// Failed requests
    pub failed_requests: usize,
    /// Average execution time
    pub average_execution_time: std::time::Duration,
    /// Requests per second
    pub requests_per_second: f64,
    /// Total resource usage
    pub total_resource_usage: ResourceUsage,
    /// Peak concurrent requests
    pub peak_concurrent_requests: usize,
}

impl ExecutionStats {
    /// Update with successful request
    pub fn update_request(&mut self, success: bool, execution_time: std::time::Duration) {
        self.total_requests += 1;

        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        self.average_execution_time = (self.average_execution_time * (self.total_requests - 1) as u32 + execution_time) / self.total_requests as u32;

        // Update RPS (simplified)
        self.requests_per_second = self.total_requests as f64 / execution_time.as_secs_f64();
    }

    /// Update resource usage
    pub fn update_resource_usage(&mut self, usage: ResourceUsage) {
        self.total_resource_usage = usage;
    }

    /// Update peak concurrent requests
    pub fn update_peak_concurrent(&mut self, concurrent: usize) {
        self.peak_concurrent_requests = self.peak_concurrent_requests.max(concurrent);
    }
}

/// Execution profiler for performance analysis
#[derive(Debug, Clone)]
pub struct ExecutionProfiler {
    /// Profile data
    pub profile_data: HashMap<String, ProfileEntry>,
    /// Configuration
    pub config: ProfileConfig,
}

impl ExecutionProfiler {
    /// Create a new profiler
    pub fn new() -> Self {
        Self {
            profile_data: HashMap::new(),
            config: ProfileConfig::default(),
        }
    }

    /// Start profiling a request
    pub fn start_profiling(&mut self, request_id: &str) -> String {
        let profile_id = format!("profile_{}", request_id);

        let entry = ProfileEntry {
            request_id: request_id.to_string(),
            start_time: std::time::Instant::now(),
            end_time: None,
            steps: Vec::new(),
            memory_usage: Vec::new(),
            cpu_usage: Vec::new(),
        };

        self.profile_data.insert(profile_id.clone(), entry);
        profile_id
    }

    /// Record profiling step
    pub fn record_step(&mut self, profile_id: &str, step_name: String, duration: std::time::Duration) {
        if let Some(entry) = self.profile_data.get_mut(profile_id) {
            entry.steps.push(ProfileStep {
                name: step_name,
                duration,
                timestamp: std::time::Instant::now(),
            });
        }
    }

    /// Record memory usage
    pub fn record_memory_usage(&mut self, profile_id: &str, memory_mb: f64) {
        if let Some(entry) = self.profile_data.get_mut(profile_id) {
            entry.memory_usage.push((std::time::Instant::now(), memory_mb));
        }
    }

    /// Record CPU usage
    pub fn record_cpu_usage(&mut self, profile_id: &str, cpu_percent: f64) {
        if let Some(entry) = self.profile_data.get_mut(profile_id) {
            entry.cpu_usage.push((std::time::Instant::now(), cpu_percent));
        }
    }

    /// End profiling
    pub fn end_profiling(&mut self, profile_id: &str) {
        if let Some(entry) = self.profile_data.get_mut(profile_id) {
            entry.end_time = Some(std::time::Instant::now());
        }
    }

    /// Get profile report
    pub fn get_profile_report(&self, profile_id: &str) -> Option<ProfileReport> {
        self.profile_data.get(profile_id).map(|entry| {
            let total_time = entry.end_time
                .map(|end| end.duration_since(entry.start_time))
                .unwrap_or_else(|| std::time::Instant::now().duration_since(entry.start_time));

            ProfileReport {
                request_id: entry.request_id.clone(),
                total_time,
                steps: entry.steps.clone(),
                memory_usage: entry.memory_usage.clone(),
                cpu_usage: entry.cpu_usage.clone(),
                summary: self.generate_summary(entry),
            }
        })
    }

    /// Generate profile summary
    fn generate_summary(&self, entry: &ProfileEntry) -> ProfileSummary {
        let total_steps = entry.steps.len();
        let total_step_time: std::time::Duration = entry.steps.iter().map(|s| s.duration).sum();
        let average_step_time = if total_steps > 0 {
            total_step_time / total_steps as u32
        } else {
            std::time::Duration::default()
        };

        ProfileSummary {
            total_steps,
            total_step_time,
            average_step_time,
            max_memory_mb: entry.memory_usage.iter().map(|(_, mb)| mb).fold(0.0, f64::max),
            average_memory_mb: if entry.memory_usage.is_empty() {
                0.0
            } else {
                entry.memory_usage.iter().map(|(_, mb)| mb).sum::<f64>() / entry.memory_usage.len() as f64
            },
            max_cpu_percent: entry.cpu_usage.iter().map(|(_, cpu)| cpu).fold(0.0, f64::max),
            average_cpu_percent: if entry.cpu_usage.is_empty() {
                0.0
            } else {
                entry.cpu_usage.iter().map(|(_, cpu)| cpu).sum::<f64>() / entry.cpu_usage.len() as f64
            },
        }
    }
}

/// Profile entry
#[derive(Debug, Clone)]
struct ProfileEntry {
    /// Request ID
    pub request_id: String,
    /// Start time
    pub start_time: std::time::Instant,
    /// End time
    pub end_time: Option<std::time::Instant>,
    /// Execution steps
    pub steps: Vec<ProfileStep>,
    /// Memory usage over time
    pub memory_usage: Vec<(std::time::Instant, f64)>,
    /// CPU usage over time
    pub cpu_usage: Vec<(std::time::Instant, f64)>,
}

/// Profile step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileStep {
    /// Step name
    pub name: String,
    /// Step duration
    pub duration: std::time::Duration,
    /// Timestamp when step occurred
    pub timestamp: std::time::Instant,
}

/// Profile report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileReport {
    /// Request ID
    pub request_id: String,
    /// Total execution time
    pub total_time: std::time::Duration,
    /// Execution steps
    pub steps: Vec<ProfileStep>,
    /// Memory usage over time
    pub memory_usage: Vec<(std::time::Instant, f64)>,
    /// CPU usage over time
    pub cpu_usage: Vec<(std::time::Instant, f64)>,
    /// Summary
    pub summary: ProfileSummary,
}

/// Profile summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileSummary {
    /// Total number of steps
    pub total_steps: usize,
    /// Total time spent in steps
    pub total_step_time: std::time::Duration,
    /// Average time per step
    pub average_step_time: std::time::Duration,
    /// Maximum memory usage in MB
    pub max_memory_mb: f64,
    /// Average memory usage in MB
    pub average_memory_mb: f64,
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    /// Average CPU usage percentage
    pub average_cpu_percent: f64,
}

/// Profile configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// Enable detailed profiling
    pub enable_detailed: bool,
    /// Profile memory usage
    pub profile_memory: bool,
    /// Profile CPU usage
    pub profile_cpu: bool,
    /// Profile step timing
    pub profile_steps: bool,
    /// Maximum profile entries
    pub max_entries: usize,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            enable_detailed: true,
            profile_memory: true,
            profile_cpu: true,
            profile_steps: true,
            max_entries: 1000,
        }
    }
}

/// Execution validator for pre-execution validation
#[derive(Debug, Clone)]
pub struct ExecutionValidator {
    /// Validation rules
    pub rules: ValidationRules,
}

impl ExecutionValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            rules: ValidationRules::default(),
        }
    }

    /// Validate request before execution
    pub async fn validate_request(&self, request: &ApiRequest) -> Result<ValidationReport, ApiError> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Check request size
        let request_size = serde_json::to_vec(request).map(|v| v.len()).unwrap_or(0);
        if request_size > self.rules.max_request_size {
            errors.push("Request too large".to_string());
        }

        // Check target count
        if request.targets.len() > self.rules.max_targets {
            errors.push(format!("Too many targets: {}", request.targets.len()));
        }

        // Check for duplicate targets
        let mut seen = HashSet::new();
        for target in &request.targets {
            let key = match target {
                ExecutionTarget::DefRef(def_ref) => format!("def_{}", def_ref.hash),
                ExecutionTarget::Patch(patch) => format!("patch_{}", patch.patch_id),
                ExecutionTarget::Transaction(tx_ref) => format!("tx_{}", tx_ref.tx_id),
            };

            if !seen.insert(key.clone()) {
                warnings.push(format!("Duplicate target: {}", key));
            }
        }

        // Check context validity
        if let Some(graph_ref) = &request.context.graph_state {
            // Validate graph reference
            if graph_ref.inner().vertex_count() == 0 && graph_ref.inner().edge_count() == 0 {
                warnings.push("Graph state is empty".to_string());
            }
        }

        Ok(ValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
            total_validated: 1,
        })
    }
}

/// Validation rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRules {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum number of targets
    pub max_targets: usize,
    /// Require valid context
    pub require_context: bool,
    /// Allow empty results
    pub allow_empty_results: bool,
}

impl Default for ValidationRules {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10MB
            max_targets: 100,
            require_context: false,
            allow_empty_results: true,
        }
    }
}

/// Validation report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Whether validation passed
    pub is_valid: bool,
    /// Validation errors
    pub errors: Vec<String>,
    /// Validation warnings
    pub warnings: Vec<String>,
    /// Number of items validated
    pub total_validated: usize,
}

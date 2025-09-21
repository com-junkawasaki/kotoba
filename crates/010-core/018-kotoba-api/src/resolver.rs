//! # DefRef/Patch Resolution
//!
//! This module provides resolution functionality for DefRef and patch
//! references, handling dependency resolution and execution planning.

use super::*;
use kotoba_types::*;
use kotoba_codebase::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;

/// DefRef resolver for resolving references and dependencies
#[derive(Debug, Clone)]
pub struct DefRefResolver {
    /// Resolution cache
    pub cache: HashMap<String, ResolvedDefRef>,
    /// Dependency graph
    pub dependency_graph: HashMap<String, HashSet<String>>,
    /// Configuration
    pub config: ResolverConfig,
    /// Resolution statistics
    pub stats: ResolutionStats,
}

impl DefRefResolver {
    /// Create a new DefRef resolver
    pub fn new(config: ResolverConfig) -> Self {
        Self {
            cache: HashMap::new(),
            dependency_graph: HashMap::new(),
            config,
            stats: ResolutionStats::default(),
        }
    }

    /// Resolve execution targets
    pub async fn resolve_targets(&mut self, targets: &[ExecutionTarget]) -> Result<Vec<ResolvedTarget>, ApiError> {
        let mut resolved = Vec::new();

        for target in targets {
            let start_time = std::time::Instant::now();

            match self.resolve_target(target).await {
                Ok(resolved_target) => {
                    resolved.push(resolved_target);
                    self.stats.update_success(start_time.elapsed());
                }
                Err(e) => {
                    self.stats.update_failure(start_time.elapsed());
                    return Err(e);
                }
            }
        }

        Ok(resolved)
    }

    /// Resolve a single target
    pub async fn resolve_target(&mut self, target: &ExecutionTarget) -> Result<ResolvedTarget, ApiError> {
        match target {
            ExecutionTarget::DefRef(def_ref) => self.resolve_def_ref(def_ref).await,
            ExecutionTarget::Patch(patch) => self.resolve_patch(patch).await,
            ExecutionTarget::Transaction(tx_ref) => self.resolve_transaction(tx_ref).await,
        }
    }

    /// Resolve a DefRef
    pub async fn resolve_def_ref(&mut self, def_ref: &DefRef) -> Result<ResolvedTarget, ApiError> {
        let cache_key = self.make_cache_key(def_ref);

        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(ResolvedTarget {
                def_refs: vec![def_ref.clone()],
                execution_plan: ExecutionPlan::default(),
                dependencies: cached.dependencies.clone(),
            });
        }

        // Resolve dependencies
        let dependencies = self.resolve_def_ref_dependencies(def_ref).await?;

        // Create execution plan
        let execution_plan = self.create_execution_plan(def_ref, &dependencies)?;

        // Cache the result
        let resolved = ResolvedDefRef {
            def_ref: def_ref.clone(),
            dependencies: dependencies.clone(),
            resolved_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };

        self.cache.insert(cache_key, resolved);

        // Update dependency graph
        self.dependency_graph.insert(
            def_ref.hash.to_string(),
            dependencies.iter().map(|d| d.hash.to_string()).collect()
        );

        Ok(ResolvedTarget {
            def_refs: vec![def_ref.clone()],
            execution_plan,
            dependencies,
        })
    }

    /// Resolve patch
    pub async fn resolve_patch(&mut self, patch: &Patch) -> Result<ResolvedTarget, ApiError> {
        let mut all_dependencies = HashSet::new();
        let mut execution_steps = Vec::new();

        for (i, operation) in patch.operations.iter().enumerate() {
            let step = self.resolve_patch_operation(operation, i).await?;
            all_dependencies.extend(step.dependencies);
            execution_steps.push(step);
        }

        let execution_plan = ExecutionPlan {
            steps: execution_steps.into_iter().map(|s| s.step).collect(),
            estimated_time_ms: self.estimate_patch_time(&patch.operations),
            parallel: false,
        };

        Ok(ResolvedTarget {
            def_refs: Vec::new(), // Patches don't produce DefRefs directly
            execution_plan,
            dependencies: all_dependencies.into_iter().collect(),
        })
    }

    /// Resolve transaction
    pub async fn resolve_transaction(&mut self, tx_ref: &TransactionRef) -> Result<ResolvedTarget, ApiError> {
        // Implementation would look up the transaction and resolve its dependencies
        // For now, return a basic resolved target
        Ok(ResolvedTarget {
            def_refs: Vec::new(),
            execution_plan: ExecutionPlan::default(),
            dependencies: Vec::new(),
        })
    }

    /// Resolve DefRef dependencies
    async fn resolve_def_ref_dependencies(&mut self, def_ref: &DefRef) -> Result<Vec<DefRef>, ApiError> {
        let mut dependencies = HashSet::new();
        let mut to_visit = vec![def_ref.clone()];
        let mut visited = HashSet::new();

        while let Some(current) = to_visit.pop() {
            if visited.contains(&current) {
                continue;
            }

            visited.insert(current.clone());

            // Get definition content
            let def_content = self.get_definition_content(&current).await?;

            // Extract dependencies from content
            let def_dependencies = self.extract_dependencies_from_content(&def_content)?;

            for dep in def_dependencies {
                dependencies.insert(dep.clone());
                to_visit.push(dep);
            }

            // Prevent infinite recursion
            if dependencies.len() > self.config.max_dependencies {
                return Err(ApiError::JsonError("Too many dependencies".to_string()));
            }
        }

        Ok(dependencies.into_iter().collect())
    }

    /// Resolve patch operation
    async fn resolve_patch_operation(&mut self, operation: &PatchOperation, step_id: usize) -> Result<ResolvedOperation, ApiError> {
        match operation {
            PatchOperation::AddDef(def_ref) => {
                let dependencies = self.resolve_def_ref_dependencies(def_ref).await?;

                Ok(ResolvedOperation {
                    step: ExecutionStep {
                        step_id: format!("add_def_{}", step_id),
                        operation: ExecutionOperation::ResolveDef(def_ref.clone()),
                        dependencies: dependencies.iter().map(|d| d.hash.to_string()).collect(),
                        estimated_duration_ms: 100,
                    },
                    dependencies,
                })
            }
            PatchOperation::RemoveDef(def_ref) => {
                Ok(ResolvedOperation {
                    step: ExecutionStep {
                        step_id: format!("remove_def_{}", step_id),
                        operation: ExecutionOperation::ResolveDef(def_ref.clone()),
                        dependencies: vec![def_ref.hash.to_string()],
                        estimated_duration_ms: 50,
                    },
                    dependencies: vec![def_ref.clone()],
                })
            }
            PatchOperation::TransformGraph { input_ref, rule_ref, strategy_ref } => {
                let mut dependencies = vec![input_ref.clone(), rule_ref.clone()];
                if let Some(strategy_ref) = strategy_ref {
                    dependencies.push(strategy_ref.clone());
                }

                Ok(ResolvedOperation {
                    step: ExecutionStep {
                        step_id: format!("transform_{}", step_id),
                        operation: ExecutionOperation::ResolveDef(rule_ref.clone()),
                        dependencies: dependencies.iter().map(|d| d.hash.to_string()).collect(),
                        estimated_duration_ms: 500,
                    },
                    dependencies,
                })
            }
            PatchOperation::MigrateSchema { from_ref, to_ref, rules } => {
                let mut dependencies = vec![from_ref.clone(), to_ref.clone()];
                dependencies.extend(rules.iter().cloned());

                Ok(ResolvedOperation {
                    step: ExecutionStep {
                        step_id: format!("migrate_{}", step_id),
                        operation: ExecutionOperation::ResolveDef(from_ref.clone()),
                        dependencies: dependencies.iter().map(|d| d.hash.to_string()).collect(),
                        estimated_duration_ms: 1000,
                    },
                    dependencies,
                })
            }
        }
    }

    /// Get definition content (placeholder implementation)
    async fn get_definition_content(&self, _def_ref: &DefRef) -> Result<Vec<u8>, ApiError> {
        // Implementation would fetch definition content from storage
        Ok(Vec::new())
    }

    /// Extract dependencies from definition content (placeholder implementation)
    fn extract_dependencies_from_content(&self, _content: &[u8]) -> Result<Vec<DefRef>, ApiError> {
        // Implementation would parse content and extract DefRef dependencies
        Ok(Vec::new())
    }

    /// Create execution plan for DefRef
    fn create_execution_plan(&self, def_ref: &DefRef, dependencies: &[DefRef]) -> Result<ExecutionPlan, ApiError> {
        let mut steps = Vec::new();

        // Add dependency resolution steps
        for (i, dep) in dependencies.iter().enumerate() {
            steps.push(ExecutionStep {
                step_id: format!("resolve_dep_{}_{}", def_ref.hash, i),
                operation: ExecutionOperation::ResolveDef(dep.clone()),
                dependencies: vec![dep.hash.to_string()],
                estimated_duration_ms: 50,
            });
        }

        // Add main execution step
        steps.push(ExecutionStep {
            step_id: format!("execute_{}", def_ref.hash),
            operation: ExecutionOperation::ResolveDef(def_ref.clone()),
            dependencies: dependencies.iter().map(|d| d.hash.to_string()).collect(),
            estimated_duration_ms: 200,
        });

        Ok(ExecutionPlan {
            steps,
            estimated_time_ms: self.estimate_def_ref_time(def_ref, dependencies),
            parallel: dependencies.len() <= 3, // Allow parallel resolution of small dependency sets
        })
    }

    /// Estimate execution time for DefRef
    fn estimate_def_ref_time(&self, def_ref: &DefRef, dependencies: &[DefRef]) -> u64 {
        let base_time = match def_ref.def_type {
            DefType::Function => 200,
            DefType::Type => 50,
            DefType::Rule => 300,
            DefType::Strategy => 100,
            DefType::Schema => 500,
        };

        let dependency_time = dependencies.len() as u64 * 50;
        base_time + dependency_time
    }

    /// Estimate execution time for patch
    fn estimate_patch_time(&self, operations: &[PatchOperation]) -> u64 {
        operations.iter().map(|op| match op {
            PatchOperation::AddDef(_) | PatchOperation::RemoveDef(_) => 100,
            PatchOperation::TransformGraph { .. } => 500,
            PatchOperation::MigrateSchema { .. } => 1000,
        }).sum()
    }

    /// Make cache key for DefRef
    fn make_cache_key(&self, def_ref: &DefRef) -> String {
        format!("{}_{}", def_ref.def_type, def_ref.hash)
    }

    /// Clear resolution cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.dependency_graph.clear();
    }

    /// Get resolution statistics
    pub fn get_stats(&self) -> &ResolutionStats {
        &self.stats
    }
}

/// Resolved DefRef with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedDefRef {
    /// DefRef that was resolved
    pub def_ref: DefRef,
    /// Dependencies
    pub dependencies: Vec<DefRef>,
    /// Resolution timestamp
    pub resolved_at: u64,
}

/// Resolved operation
#[derive(Debug, Clone)]
pub struct ResolvedOperation {
    /// Execution step
    pub step: ExecutionStep,
    /// Dependencies
    pub dependencies: Vec<DefRef>,
}

/// Resolution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    /// Cache size limit
    pub cache_size_limit: usize,
    /// Maximum dependency depth
    pub max_dependency_depth: usize,
    /// Maximum dependencies per DefRef
    pub max_dependencies: usize,
    /// Resolution timeout in seconds
    pub timeout_seconds: u64,
    /// Enable parallel resolution
    pub enable_parallel: bool,
}

impl Default for ResolverConfig {
    fn default() -> Self {
        Self {
            cache_size_limit: 10000,
            max_dependency_depth: 50,
            max_dependencies: 1000,
            timeout_seconds: 30,
            enable_parallel: true,
        }
    }
}

/// Resolution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionStats {
    /// Total resolutions performed
    pub total_resolutions: usize,
    /// Successful resolutions
    pub successful_resolutions: usize,
    /// Failed resolutions
    pub failed_resolutions: usize,
    /// Average resolution time
    pub average_resolution_time: std::time::Duration,
    /// Cache hit rate
    pub cache_hit_rate: f64,
    /// Total dependencies resolved
    pub total_dependencies: usize,
}

impl ResolutionStats {
    /// Update with successful resolution
    pub fn update_success(&mut self, duration: std::time::Duration) {
        self.total_resolutions += 1;
        self.successful_resolutions += 1;

        self.average_resolution_time = (self.average_resolution_time * (self.total_resolutions - 1) as u32 + duration) / self.total_resolutions as u32;
        self.update_cache_rate();
    }

    /// Update with failed resolution
    pub fn update_failure(&mut self, duration: std::time::Duration) {
        self.total_resolutions += 1;
        self.failed_resolutions += 1;

        self.average_resolution_time = (self.average_resolution_time * (self.total_resolutions - 1) as u32 + duration) / self.total_resolutions as u32;
        self.update_cache_rate();
    }

    /// Add dependencies
    pub fn add_dependencies(&mut self, count: usize) {
        self.total_dependencies += count;
    }

    /// Update cache hit rate
    fn update_cache_rate(&mut self) {
        if self.total_resolutions > 0 {
            self.cache_hit_rate = self.successful_resolutions as f64 / self.total_resolutions as f64;
        }
    }
}

/// Dependency resolver trait
#[async_trait::async_trait]
pub trait DependencyResolver {
    /// Resolve dependencies for a DefRef
    async fn resolve_dependencies(&self, def_ref: &DefRef) -> Result<Vec<DefRef>, ApiError>;

    /// Check if dependencies are satisfied
    async fn check_dependencies(&self, def_ref: &DefRef, available: &HashSet<DefRef>) -> Result<bool, ApiError>;

    /// Get dependency graph
    fn get_dependency_graph(&self, def_ref: &DefRef) -> Result<HashSet<DefRef>, ApiError>;
}

#[async_trait::async_trait]
impl DependencyResolver for DefRefResolver {
    async fn resolve_dependencies(&self, def_ref: &DefRef) -> Result<Vec<DefRef>, ApiError> {
        // Implementation would resolve actual dependencies
        Ok(Vec::new())
    }

    async fn check_dependencies(&self, _def_ref: &DefRef, _available: &HashSet<DefRef>) -> Result<bool, ApiError> {
        // Implementation would check if all dependencies are available
        Ok(true)
    }

    fn get_dependency_graph(&self, def_ref: &DefRef) -> Result<HashSet<DefRef>, ApiError> {
        Ok(self.dependency_graph
            .get(&def_ref.hash.to_string())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|hash_str| {
                // Convert hash string back to DefRef (simplified)
                Some(DefRef {
                    hash: Hash::from_sha256(hash_str.as_bytes()),
                    def_type: DefType::Function,
                    name: None,
                })
            })
            .collect())
    }
}

/// Circular dependency detector
#[derive(Debug, Clone)]
pub struct CircularDependencyDetector {
    /// Visited nodes during traversal
    visited: HashSet<String>,
    /// Nodes in current path
    current_path: HashSet<String>,
}

impl CircularDependencyDetector {
    /// Create a new detector
    pub fn new() -> Self {
        Self {
            visited: HashSet::new(),
            current_path: HashSet::new(),
        }
    }

    /// Detect circular dependencies starting from a DefRef
    pub fn detect_circular(&mut self, def_ref: &DefRef, dependency_graph: &HashMap<String, HashSet<String>>) -> Result<bool, ApiError> {
        let def_ref_key = def_ref.hash.to_string();

        if self.current_path.contains(&def_ref_key) {
            return Ok(true); // Circular dependency detected
        }

        if self.visited.contains(&def_ref_key) {
            return Ok(false); // Already processed
        }

        self.visited.insert(def_ref_key.clone());
        self.current_path.insert(def_ref_key.clone());

        if let Some(dependencies) = dependency_graph.get(&def_ref_key) {
            for dep_key in dependencies {
                let dep_def_ref = DefRef {
                    hash: Hash::from_sha256(dep_key.as_bytes()),
                    def_type: DefType::Function,
                    name: None,
                };

                if self.detect_circular(&dep_def_ref, dependency_graph)? {
                    return Ok(true);
                }
            }
        }

        self.current_path.remove(&def_ref_key);
        Ok(false)
    }

    /// Reset detector state
    pub fn reset(&mut self) {
        self.visited.clear();
        self.current_path.clear();
    }
}

/// Dependency graph analyzer
#[derive(Debug, Clone)]
pub struct DependencyGraphAnalyzer {
    /// Dependency graph
    pub graph: HashMap<String, HashSet<String>>,
    /// Analyzer configuration
    pub config: AnalyzerConfig,
}

impl DependencyGraphAnalyzer {
    /// Create a new analyzer
    pub fn new(graph: HashMap<String, HashSet<String>>, config: AnalyzerConfig) -> Self {
        Self { graph, config }
    }

    /// Analyze dependency graph
    pub fn analyze(&self) -> AnalysisResult {
        let mut result = AnalysisResult::default();

        // Analyze each node
        for (node, dependencies) in &self.graph {
            result.total_nodes += 1;

            if dependencies.is_empty() {
                result.leaf_nodes += 1;
            }

            result.max_dependencies = result.max_dependencies.max(dependencies.len());
        }

        // Find cycles
        let mut detector = CircularDependencyDetector::new();
        for (node, _) in &self.graph {
            detector.reset();

            let def_ref = DefRef {
                hash: Hash::from_sha256(node.as_bytes()),
                def_type: DefType::Function,
                name: None,
            };

            if detector.detect_circular(&def_ref, &self.graph).unwrap_or(false) {
                result.cycles_detected = true;
                break;
            }
        }

        // Compute strongly connected components
        result.strongly_connected_components = self.compute_scc();

        result
    }

    /// Compute strongly connected components
    fn compute_scc(&self) -> usize {
        // Tarjan's algorithm implementation would go here
        // For now, return a placeholder
        1
    }

    /// Get dependency levels (topological sort)
    pub fn get_dependency_levels(&self) -> Vec<Vec<String>> {
        let mut levels = Vec::new();
        let mut visited = HashSet::new();
        let mut current_level = HashSet::new();

        // Find nodes with no dependencies (level 0)
        for (node, dependencies) in &self.graph {
            if dependencies.is_empty() {
                current_level.insert(node.clone());
                visited.insert(node.clone());
            }
        }

        if !current_level.is_empty() {
            levels.push(current_level.into_iter().collect());
        }

        // Build subsequent levels
        while !visited.len() == self.graph.len() {
            let mut next_level = HashSet::new();

            for (node, dependencies) in &self.graph {
                if !visited.contains(node) {
                    let mut all_deps_visited = true;

                    for dep in dependencies {
                        if !visited.contains(dep) {
                            all_deps_visited = false;
                            break;
                        }
                    }

                    if all_deps_visited {
                        next_level.insert(node.clone());
                        visited.insert(node.clone());
                    }
                }
            }

            if !next_level.is_empty() {
                levels.push(next_level.into_iter().collect());
            } else {
                break; // No progress, cycle detected
            }
        }

        levels
    }
}

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Total number of nodes
    pub total_nodes: usize,
    /// Number of leaf nodes (no dependencies)
    pub leaf_nodes: usize,
    /// Maximum dependencies for any node
    pub max_dependencies: usize,
    /// Whether cycles were detected
    pub cycles_detected: bool,
    /// Number of strongly connected components
    pub strongly_connected_components: usize,
}

impl Default for AnalysisResult {
    fn default() -> Self {
        Self {
            total_nodes: 0,
            leaf_nodes: 0,
            max_dependencies: 0,
            cycles_detected: false,
            strongly_connected_components: 0,
        }
    }
}

/// Analyzer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzerConfig {
    /// Enable cycle detection
    pub enable_cycle_detection: bool,
    /// Enable SCC computation
    pub enable_scc_computation: bool,
    /// Maximum graph size for analysis
    pub max_graph_size: usize,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            enable_cycle_detection: true,
            enable_scc_computation: true,
            max_graph_size: 10000,
        }
    }
}

/// Resolution planner for optimizing execution
#[derive(Debug, Clone)]
pub struct ResolutionPlanner {
    /// Resolver
    pub resolver: DefRefResolver,
    /// Analyzer
    pub analyzer: DependencyGraphAnalyzer,
    /// Configuration
    pub config: PlannerConfig,
}

impl ResolutionPlanner {
    /// Create a new planner
    pub fn new(resolver: DefRefResolver, config: PlannerConfig) -> Self {
        Self {
            resolver,
            analyzer: DependencyGraphAnalyzer::new(HashMap::new(), AnalyzerConfig::default()),
            config,
        }
    }

    /// Plan resolution for multiple targets
    pub async fn plan_resolution(&mut self, targets: &[ExecutionTarget]) -> Result<ResolutionPlan, ApiError> {
        // Resolve all targets
        let resolved_targets = self.resolver.resolve_targets(targets).await?;

        // Build dependency graph
        self.build_dependency_graph(&resolved_targets);

        // Analyze dependencies
        let analysis = self.analyzer.analyze();

        // Create execution plan
        let execution_plan = self.create_execution_plan(&resolved_targets, &analysis)?;

        Ok(ResolutionPlan {
            targets: resolved_targets,
            analysis,
            execution_plan,
            estimated_total_time: self.estimate_total_time(&execution_plan),
        })
    }

    /// Build dependency graph from resolved targets
    fn build_dependency_graph(&mut self, resolved_targets: &[ResolvedTarget]) {
        let mut graph = HashMap::new();

        for resolved in resolved_targets {
            for def_ref in &resolved.def_refs {
                let key = def_ref.hash.to_string();
                let dependencies: HashSet<String> = resolved.dependencies.iter()
                    .map(|d| d.hash.to_string())
                    .collect();
                graph.insert(key, dependencies);
            }
        }

        self.analyzer = DependencyGraphAnalyzer::new(graph, AnalyzerConfig::default());
    }

    /// Create execution plan
    fn create_execution_plan(
        &self,
        resolved_targets: &[ResolvedTarget],
        analysis: &AnalysisResult,
    ) -> Result<ExecutionPlan, ApiError> {
        if analysis.cycles_detected {
            return Err(ApiError::JsonError("Circular dependencies detected".to_string()));
        }

        let dependency_levels = self.analyzer.get_dependency_levels();
        let mut steps = Vec::new();

        for (level_idx, level) in dependency_levels.iter().enumerate() {
            for node in level {
                // Find the DefRef for this node
                for resolved in resolved_targets {
                    for def_ref in &resolved.def_refs {
                        if def_ref.hash.to_string() == *node {
                            steps.push(ExecutionStep {
                                step_id: format!("level_{}_node_{}", level_idx, node),
                                operation: ExecutionOperation::ResolveDef(def_ref.clone()),
                                dependencies: vec![node.clone()],
                                estimated_duration_ms: 100,
                            });
                            break;
                        }
                    }
                }
            }
        }

        Ok(ExecutionPlan {
            steps,
            estimated_time_ms: self.estimate_total_time_from_steps(&steps),
            parallel: steps.len() <= 5,
        })
    }

    /// Estimate total time from steps
    fn estimate_total_time_from_steps(&self, steps: &[ExecutionStep]) -> u64 {
        steps.iter().map(|step| step.estimated_duration_ms).sum()
    }

    /// Estimate total time
    fn estimate_total_time(&self, execution_plan: &ExecutionPlan) -> u64 {
        execution_plan.estimated_time_ms
    }
}

/// Planner configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// Enable parallel planning
    pub enable_parallel: bool,
    /// Maximum planning time
    pub max_planning_time_seconds: u64,
    /// Enable optimization
    pub enable_optimization: bool,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            enable_parallel: true,
            max_planning_time_seconds: 10,
            enable_optimization: true,
        }
    }
}

/// Resolution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolutionPlan {
    /// Resolved targets
    pub targets: Vec<ResolvedTarget>,
    /// Dependency analysis
    pub analysis: AnalysisResult,
    /// Execution plan
    pub execution_plan: ExecutionPlan,
    /// Estimated total execution time
    pub estimated_total_time: u64,
}

impl ResolutionPlan {
    /// Get plan summary
    pub fn summary(&self) -> String {
        format!(
            "Resolution Plan: {} targets, {} steps, {}ms estimated",
            self.targets.len(),
            self.execution_plan.steps.len(),
            self.estimated_total_time
        )
    }

    /// Check if plan is valid
    pub fn is_valid(&self) -> bool {
        !self.analysis.cycles_detected && !self.execution_plan.steps.is_empty()
    }
}

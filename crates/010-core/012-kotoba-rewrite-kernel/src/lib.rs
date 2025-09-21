//! # Kotoba Rewrite Kernel
//!
//! Rule/Strategy/Scheduler + independence analysis for graph rewriting.
//!
//! This crate provides the core kernel for graph rewriting operations,
//! including rule application, strategy execution, scheduling, and
//! independence analysis for parallel execution.

pub mod rule;
pub mod strategy;
pub mod scheduler;
pub mod independence;
pub mod kernel;

use kotoba_codebase::*;
use kotoba_compose::*;
use kotoba_types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Rewrite kernel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewriteKernelConfig {
    /// Maximum number of rule applications per step
    pub max_applications: Option<usize>,
    /// Maximum execution time per step (milliseconds)
    pub max_time_ms: Option<u64>,
    /// Parallel execution settings
    pub parallel_execution: ParallelConfig,
    /// Independence analysis settings
    pub independence_analysis: IndependenceConfig,
}

/// Parallel execution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelConfig {
    /// Enable parallel execution
    pub enabled: bool,
    /// Maximum number of parallel workers
    pub max_workers: Option<usize>,
    /// Minimum batch size for parallel processing
    pub min_batch_size: usize,
}

/// Independence analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndependenceConfig {
    /// Enable independence analysis
    pub enabled: bool,
    /// Analysis depth
    pub depth: usize,
    /// Cache independence results
    pub cache_results: bool,
}

/// Rewrite kernel for executing graph transformations
#[derive(Debug, Clone)]
pub struct RewriteKernel {
    /// Configuration
    pub config: RewriteKernelConfig,
    /// Rule registry
    pub rule_registry: HashMap<DefRef, RuleDef>,
    /// Strategy registry
    pub strategy_registry: HashMap<DefRef, StrategyDef>,
    /// Independence analyzer
    pub independence_analyzer: IndependenceAnalyzer,
    /// Scheduler
    pub scheduler: Scheduler,
    /// Execution statistics
    pub stats: ExecutionStats,
}

impl RewriteKernel {
    /// Create a new rewrite kernel
    pub fn new(config: RewriteKernelConfig) -> Self {
        Self {
            config,
            rule_registry: HashMap::new(),
            strategy_registry: HashMap::new(),
            independence_analyzer: IndependenceAnalyzer::new(config.independence_analysis.clone()),
            scheduler: Scheduler::new(),
            stats: ExecutionStats::default(),
        }
    }

    /// Register a rule
    pub fn register_rule(&mut self, rule_def: RuleDef) -> DefRef {
        let def_ref = DefRef::new(
            serde_json::to_vec(&rule_def).expect("Failed to serialize rule"),
            DefType::Rule,
        );
        self.rule_registry.insert(def_ref.clone(), rule_def);
        def_ref
    }

    /// Register a strategy
    pub fn register_strategy(&mut self, strategy_def: StrategyDef) -> DefRef {
        let def_ref = DefRef::new(
            serde_json::to_vec(&strategy_def).expect("Failed to serialize strategy"),
            DefType::Strategy,
        );
        self.strategy_registry.insert(def_ref.clone(), strategy_def);
        def_ref
    }

    /// Execute a strategy on a graph
    pub fn execute_strategy(
        &mut self,
        strategy_ref: DefRef,
        input_graph: Graph,
    ) -> Result<ExecutionResult, KernelError> {
        let strategy = self.strategy_registry.get(&strategy_ref)
            .ok_or(KernelError::StrategyNotFound(strategy_ref))?;

        // Analyze independence for parallel execution
        if self.config.independence_analysis.enabled {
            self.independence_analyzer.analyze(&self.rule_registry)?;
        }

        // Execute the strategy
        let result = self.scheduler.execute(strategy, input_graph, &self.config)?;

        // Update statistics
        self.stats.update(&result);

        Ok(result)
    }
}

/// Execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Output graph
    pub output_graph: Graph,
    /// Rules applied
    pub rules_applied: Vec<RuleExecutionReport>,
    /// Execution time
    pub execution_time: std::time::Duration,
    /// Success status
    pub success: bool,
    /// Error message if failed
    pub error_message: Option<String>,
}

/// Execution statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Total rules applied
    pub total_rules_applied: usize,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
    /// Average rules per second
    pub avg_rules_per_second: f64,
    /// Success rate
    pub success_rate: f64,
}

impl ExecutionStats {
    /// Update statistics with new result
    pub fn update(&mut self, result: &ExecutionResult) {
        self.total_rules_applied += result.rules_applied.len();
        self.total_execution_time += result.execution_time;

        if self.total_execution_time.as_secs() > 0 {
            self.avg_rules_per_second = self.total_rules_applied as f64 / self.total_execution_time.as_secs_f64();
        }
    }
}

/// Kernel error
#[derive(Debug, Clone)]
pub enum KernelError {
    /// Strategy not found
    StrategyNotFound(DefRef),
    /// Rule not found
    RuleNotFound(DefRef),
    /// Independence analysis failed
    IndependenceAnalysisFailed(String),
    /// Execution failed
    ExecutionFailed(String),
    /// Timeout
    Timeout,
}

impl std::fmt::Display for KernelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KernelError::StrategyNotFound(def_ref) => write!(f, "Strategy not found: {}", def_ref),
            KernelError::RuleNotFound(def_ref) => write!(f, "Rule not found: {}", def_ref),
            KernelError::IndependenceAnalysisFailed(msg) => write!(f, "Independence analysis failed: {}", msg),
            KernelError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            KernelError::Timeout => write!(f, "Execution timeout"),
        }
    }
}

impl std::error::Error for KernelError {}

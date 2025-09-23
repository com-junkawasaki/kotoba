//! # Kernel Core Implementation
//!
//! This module provides the core kernel implementation for graph rewriting operations.

use super::*;
use crate::strategy_def::StrategyDef;
use crate::rule_def::{RuleMatcher, RuleApplicator, RuleOptimizer, RuleExecutionResult, RuleAnalysis};
use crate::strategy::StrategyExecutor;
use kotoba_codebase::{DefRef, DefType};
use std::collections::HashMap;

/// Kernel implementation for rule execution
#[derive(Debug, Clone)]
pub struct Kernel {
    /// Rule registry
    pub rule_registry: HashMap<DefRef, kotoba_types::RuleDPO>,
    /// Strategy registry
    pub strategy_registry: HashMap<DefRef, StrategyDef>,
    /// Execution configuration
    pub config: KernelConfig,
    /// Performance statistics
    pub stats: KernelStats,
}

impl Kernel {
    /// Create a new kernel
    pub fn new(config: KernelConfig) -> Self {
        Self {
            rule_registry: HashMap::new(),
            strategy_registry: HashMap::new(),
            config,
            stats: KernelStats::default(),
        }
    }

    /// Register a rule
    pub fn register_rule(&mut self, rule_def: kotoba_types::RuleDPO) -> DefRef {
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

    /// Execute a rule on a graph
    pub fn execute_rule(
        &mut self,
        rule_ref: DefRef,
        graph: &mut crate::rule::GraphKind,
    ) -> Result<RuleExecutionResult, KernelError> {
        let rule = self.rule_registry.get(&rule_ref)
            .ok_or(KernelError::RuleNotFound(rule_ref.clone()))?;

        let start_time = std::time::Instant::now();

        // Create rule matcher
        let matcher = RuleMatcher::new(rule.clone());

        // Find matches
        let matches = matcher.find_matches(graph)
            .map_err(|e| KernelError::ExecutionFailed(e.to_string()))?;

        // Apply rule to each match
        let mut applications = Vec::new();
        let mut success = true;
        let mut error_message = None;

        for match_result in matches {
            if !matcher.validate_match(&match_result, graph) {
                continue;
            }

            // Create rule applicator
            let applicator = RuleApplicator::new(rule.clone());

            // Apply rule
            match applicator.apply(graph) {
                Ok(application) => {
                    applications.push(application);
                },
                Err(e) => {
                    success = false;
                    error_message = Some(e.to_string());
                    break;
                }
            }
        }

        let execution_time = start_time.elapsed();

        // Update statistics
        self.stats.update_rule_execution(
            rule_ref,
            applications.len(),
            execution_time,
            success,
        );

        Ok(RuleExecutionResult {
            rule_ref,
            success,
            execution_time: execution_time.as_nanos() as u64,
            error_message,
        })
    }

    /// Execute a strategy on a graph
    pub fn execute_strategy(
        &mut self,
        strategy_ref: DefRef,
        graph: &mut crate::rule::GraphKind,
    ) -> Result<crate::strategy_def::StrategyExecutionResult, KernelError> {
        let strategy = self.strategy_registry.get(&strategy_ref)
            .ok_or(KernelError::StrategyNotFound(strategy_ref.clone()))?;

        let start_time = std::time::Instant::now();

        // Create strategy executor
        let executor = StrategyExecutor::new(
            strategy.clone(),
            self.rule_registry.clone(),
        );

        // Execute strategy
        let result = executor.execute(graph, self)?;

        let execution_time = start_time.elapsed();

        // Update statistics
        self.stats.update_strategy_execution(
            strategy_ref,
            result.rules_applied.len(),
            execution_time,
            result.success,
        );

        Ok(crate::strategy_def::StrategyExecutionResult {
            strategy_ref,
            rules_applied: result.rules_applied,
            total_applications: result.total_applications,
            execution_time,
            success: result.success,
            error_message: result.error_message,
            final_state: None, // TODO: compute final state hash
        })
    }

    /// Optimize a rule for better performance
    pub fn optimize_rule(&self, rule_ref: DefRef) -> Result<kotoba_types::RuleDPO, KernelError> {
        let rule = self.rule_registry.get(&rule_ref)
            .ok_or(KernelError::RuleNotFound(rule_ref))?;

        let optimizer = RuleOptimizer::new();
        let mut optimized_rule = rule.clone();
        optimizer.optimize_rule(&mut optimized_rule);

        Ok(optimized_rule)
    }

    /// Analyze rule properties
    pub fn analyze_rule(&self, rule_ref: DefRef) -> Result<crate::rule_def::RuleAnalysis, KernelError> {
        let rule = self.rule_registry.get(&rule_ref)
            .ok_or(KernelError::RuleNotFound(rule_ref))?;

        let optimizer = RuleOptimizer::new();
        Ok(optimizer.analyze_rule(rule))
    }

    /// Get performance statistics
    pub fn get_stats(&self) -> &KernelStats {
        &self.stats
    }

    /// Reset performance statistics
    pub fn reset_stats(&mut self) {
        self.stats = KernelStats::default();
    }
}

/// Kernel configuration
#[derive(Debug, Clone)]
pub struct KernelConfig {
    /// Enable optimizations
    pub enable_optimizations: bool,
    /// Enable parallel execution
    pub enable_parallel_execution: bool,
    /// Maximum rule applications per execution
    pub max_applications: Option<usize>,
    /// Execution timeout
    pub timeout: Option<std::time::Duration>,
}

impl Default for KernelConfig {
    fn default() -> Self {
        Self {
            enable_optimizations: true,
            enable_parallel_execution: true,
            max_applications: None,
            timeout: Some(std::time::Duration::from_secs(300)), // 5 minutes
        }
    }
}

/// Kernel statistics
#[derive(Debug, Clone, Default)]
pub struct KernelStats {
    /// Rule execution statistics
    pub rule_stats: HashMap<DefRef, RuleStats>,
    /// Strategy execution statistics
    pub strategy_stats: HashMap<DefRef, StrategyStats>,
    /// Total rules executed
    pub total_rules_executed: usize,
    /// Total strategies executed
    pub total_strategies_executed: usize,
    /// Total execution time
    pub total_execution_time: std::time::Duration,
}

impl KernelStats {
    /// Update rule execution statistics
    pub fn update_rule_execution(
        &mut self,
        rule_ref: DefRef,
        applications: usize,
        execution_time: std::time::Duration,
        success: bool,
    ) {
        let stats = self.rule_stats.entry(rule_ref).or_insert_with(RuleStats::default);
        stats.applications += applications;
        stats.total_time += execution_time;
        stats.success_count += if success { 1 } else { 0 };
        stats.failure_count += if success { 0 } else { 1 };
        stats.call_count += 1;

        self.total_rules_executed += 1;
        self.total_execution_time += execution_time;
    }

    /// Update strategy execution statistics
    pub fn update_strategy_execution(
        &mut self,
        strategy_ref: DefRef,
        rules_applied: usize,
        execution_time: std::time::Duration,
        success: bool,
    ) {
        let stats = self.strategy_stats.entry(strategy_ref).or_insert_with(StrategyStats::default);
        stats.rules_applied += rules_applied;
        stats.total_time += execution_time;
        stats.success_count += if success { 1 } else { 0 };
        stats.failure_count += if success { 0 } else { 1 };
        stats.call_count += 1;

        self.total_strategies_executed += 1;
        self.total_execution_time += execution_time;
    }

    /// Get average execution time per rule
    pub fn average_rule_time(&self) -> std::time::Duration {
        if self.total_rules_executed == 0 {
            std::time::Duration::default()
        } else {
            self.total_execution_time / self.total_rules_executed as u32
        }
    }

    /// Get average execution time per strategy
    pub fn average_strategy_time(&self) -> std::time::Duration {
        if self.total_strategies_executed == 0 {
            std::time::Duration::default()
        } else {
            self.total_execution_time / self.total_strategies_executed as u32
        }
    }
}

/// Rule execution statistics
#[derive(Debug, Clone, Default)]
pub struct RuleStats {
    /// Number of times the rule was called
    pub call_count: usize,
    /// Number of successful applications
    pub applications: usize,
    /// Total execution time
    pub total_time: std::time::Duration,
    /// Number of successful executions
    pub success_count: usize,
    /// Number of failed executions
    pub failure_count: usize,
}

impl RuleStats {
    /// Success rate for this rule
    pub fn success_rate(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.call_count as f64
        }
    }

    /// Average execution time per call
    pub fn average_time(&self) -> std::time::Duration {
        if self.call_count == 0 {
            std::time::Duration::default()
        } else {
            self.total_time / self.call_count as u32
        }
    }
}

/// Strategy execution statistics
#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    /// Number of times the strategy was called
    pub call_count: usize,
    /// Total rules applied by this strategy
    pub rules_applied: usize,
    /// Total execution time
    pub total_time: std::time::Duration,
    /// Number of successful executions
    pub success_count: usize,
    /// Number of failed executions
    pub failure_count: usize,
}

impl StrategyStats {
    /// Success rate for this strategy
    pub fn success_rate(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.success_count as f64 / self.call_count as f64
        }
    }

    /// Average execution time per call
    pub fn average_time(&self) -> std::time::Duration {
        if self.call_count == 0 {
            std::time::Duration::default()
        } else {
            self.total_time / self.call_count as u32
        }
    }

    /// Average rules applied per execution
    pub fn average_rules_applied(&self) -> f64 {
        if self.call_count == 0 {
            0.0
        } else {
            self.rules_applied as f64 / self.call_count as f64
        }
    }
}

/// Kernel execution errors
#[derive(Debug, Clone)]
pub enum KernelError {
    /// Rule not found in registry
    RuleNotFound(DefRef),
    /// Strategy not found in registry
    StrategyNotFound(DefRef),
    /// Execution error from strategy executor
    ExecutionError(crate::strategy::ExecutionError),
    /// Independence analysis error
    IndependenceError(String),
    /// Timeout during execution
    Timeout,
    /// Resource limit exceeded
    ResourceLimitExceeded(String),
}

impl From<crate::strategy::ExecutionError> for KernelError {
    fn from(err: crate::strategy::ExecutionError) -> Self {
        KernelError::ExecutionError(err)
    }
}

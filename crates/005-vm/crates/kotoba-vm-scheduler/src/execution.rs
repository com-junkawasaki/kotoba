use std::collections::HashMap;
use vm_types::{Dag, TaskId, CachedResult, Task, HardwareTile, HardwareTileType, TaskCharacteristics, HardwareCharacteristics, ComputationType};
use crate::runtime::DataflowRuntime;
use crate::scheduler::Scheduler;
use crate::memoization::MemoizationEngine;

/// Merkle DAG: vm.ExecutionEngine
/// High-level execution engine that orchestrates scheduling and execution
pub struct ExecutionEngine {
    runtime: Box<dyn DataflowRuntime>,
    scheduler: Box<dyn Scheduler>,
    memo_engine: Box<dyn MemoizationEngine>,
    dag_registry: HashMap<String, Dag>,
    execution_history: HashMap<String, ExecutionResult>,
}

#[derive(Debug, Clone)]
pub struct ExecutionResult {
    pub dag_id: String,
    pub total_execution_time: f64,
    pub tasks_completed: usize,
    pub tasks_failed: usize,
    pub total_power_consumption: f64,
    pub average_resource_utilization: f64,
    pub cache_hit_rate: f64,
    pub success: bool,
}

pub struct ExecutionMetrics {
    pub throughput: f64,
    pub latency: f64,
    pub efficiency: f64,
    pub scalability: f64,
    pub reliability: f64,
}

impl ExecutionEngine {
    pub fn new(runtime: Box<dyn DataflowRuntime>, scheduler: Box<dyn Scheduler>, memo_engine: Box<dyn MemoizationEngine>) -> Self {
        ExecutionEngine {
            runtime,
            scheduler,
            memo_engine,
            dag_registry: HashMap::new(),
            execution_history: HashMap::new(),
        }
    }

    /// Register a DAG for execution
    pub fn register_dag(&mut self, dag_id: String, dag: Dag) {
        self.dag_registry.insert(dag_id, dag);
    }

    /// Execute a registered DAG
    pub fn execute_dag(&mut self, dag_id: String) -> Result<ExecutionResult, String> {
        // Get the DAG
        let dag = self.dag_registry.get(&dag_id)
            .ok_or_else(|| format!("DAG {} not found", dag_id))?;

        // Schedule the DAG
        let schedule = self.scheduler.schedule_dag(dag)?;

        // Execute scheduled tasks
        let mut tasks_completed = 0;
        let mut tasks_failed = 0;
        let mut total_execution_time = 0.0;
        let mut cache_hits = 0;
        let mut cache_misses = 0;

        for task_id in &schedule.execution_order {
            // Check cache first
            if self.memo_engine.check_cache(*task_id).is_some() {
                cache_hits += 1;
                tasks_completed += 1;
                continue;
            }

            // Submit and execute task
            if let Some(task) = dag.get_task(*task_id) {
                match self.runtime.submit_task(task.clone()) {
                    submitted_id => {
                        match self.runtime.execute_task(submitted_id) {
                            Ok(result) => {
                                self.memo_engine.store_result(submitted_id, result);
                                tasks_completed += 1;
                            }
                            Err(e) => {
                                tasks_failed += 1;
                                eprintln!("Task {} failed: {}", task_id, e);
                            }
                        }
                    }
                }
            }

            cache_misses += 1;
        }

        // Calculate metrics
        let cache_hit_rate = if (cache_hits + cache_misses) > 0 {
            cache_hits as f64 / (cache_hits + cache_misses) as f64
        } else {
            0.0
        };

        // Get performance metrics from scheduler
        let metrics = self.scheduler.get_performance_metrics();

        let result = ExecutionResult {
            dag_id: dag_id.clone(),
            total_execution_time: schedule.estimated_completion_time,
            tasks_completed,
            tasks_failed,
            total_power_consumption: schedule.total_power_consumption,
            average_resource_utilization: schedule.resource_utilization,
            cache_hit_rate,
            success: tasks_failed == 0,
        };

        self.execution_history.insert(dag_id, result.clone());
        Ok(result)
    }

    /// Execute multiple DAGs in parallel
    pub fn execute_dags_parallel(&mut self, dag_ids: Vec<String>) -> Vec<Result<ExecutionResult, String>> {
        // For now, execute sequentially to avoid Send/Sync issues
        // TODO: Implement proper parallel execution with thread-safe components
        dag_ids.into_iter()
            .map(|dag_id| self.execute_dag(dag_id))
            .collect()
    }

    /// Get execution metrics for a DAG
    pub fn get_execution_metrics(&self, dag_id: &str) -> Option<&ExecutionResult> {
        self.execution_history.get(dag_id)
    }

    /// Optimize execution plan based on historical data
    pub fn optimize_execution_plan(&mut self) -> Result<(), String> {
        // Analyze execution history
        let successful_executions: Vec<_> = self.execution_history.values()
            .filter(|result| result.success)
            .collect();

        if successful_executions.is_empty() {
            return Ok(()); // No history to optimize
        }

        // Calculate average metrics
        let avg_throughput = successful_executions.iter()
            .map(|r| r.tasks_completed as f64 / r.total_execution_time)
            .sum::<f64>() / successful_executions.len() as f64;

        let avg_latency = successful_executions.iter()
            .map(|r| r.total_execution_time)
            .sum::<f64>() / successful_executions.len() as f64;

        let avg_efficiency = successful_executions.iter()
            .map(|r| r.average_resource_utilization)
            .sum::<f64>() / successful_executions.len() as f64;

        let avg_cache_hit_rate = successful_executions.iter()
            .map(|r| r.cache_hit_rate)
            .sum::<f64>() / successful_executions.len() as f64;

        // Use metrics to optimize future executions
        // This could involve adjusting scheduling policies, cache sizes, etc.
        println!("Execution optimization based on {} successful runs:", successful_executions.len());
        println!("  Average throughput: {:.2} tasks/sec", avg_throughput);
        println!("  Average latency: {:.2} sec", avg_latency);
        println!("  Average efficiency: {:.2}%", avg_efficiency * 100.0);
        println!("  Average cache hit rate: {:.2}%", avg_cache_hit_rate * 100.0);

        Ok(())
    }

    /// Get comprehensive execution metrics
    pub fn get_comprehensive_metrics(&self) -> ExecutionMetrics {
        let successful_executions: Vec<_> = self.execution_history.values()
            .filter(|result| result.success)
            .collect();

        if successful_executions.is_empty() {
            return ExecutionMetrics {
                throughput: 0.0,
                latency: 0.0,
                efficiency: 0.0,
                scalability: 0.0,
                reliability: 0.0,
            };
        }

        let total_tasks: usize = successful_executions.iter().map(|r| r.tasks_completed).sum();
        let total_time: f64 = successful_executions.iter().map(|r| r.total_execution_time).sum();
        let total_failures: usize = self.execution_history.values()
            .filter(|r| !r.success)
            .map(|r| r.tasks_failed)
            .sum();

        let throughput = total_tasks as f64 / total_time;
        let latency = total_time / successful_executions.len() as f64;
        let efficiency = successful_executions.iter()
            .map(|r| r.average_resource_utilization)
            .sum::<f64>() / successful_executions.len() as f64;
        let scalability = if total_tasks > 0 { 1.0 } else { 0.0 }; // Placeholder
        let reliability = 1.0 - (total_failures as f64 / (total_tasks + total_failures) as f64);

        ExecutionMetrics {
            throughput,
            latency,
            efficiency,
            scalability,
            reliability,
        }
    }

    /// Clear execution history
    pub fn clear_history(&mut self) {
        self.execution_history.clear();
    }

    /// Get registered DAGs
    pub fn get_registered_dags(&self) -> Vec<&String> {
        self.dag_registry.keys().collect()
    }

    /// Remove a DAG from registry
    pub fn remove_dag(&mut self, dag_id: &str) {
        self.dag_registry.remove(dag_id);
        self.execution_history.remove(dag_id);
    }
}

impl Default for ExecutionEngine {
    fn default() -> Self {
        let runtime = Box::new(crate::runtime::DataflowRuntimeImpl::default());
        let scheduler = Box::new(crate::scheduler::HardwareAwareScheduler::default());
        let memo_engine = Box::new(crate::memoization::MemoizationEngineImpl::default());

        Self::new(runtime, scheduler, memo_engine)
    }
}

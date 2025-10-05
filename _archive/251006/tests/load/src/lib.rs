//! KotobaDB Load Testing Framework
//!
//! Comprehensive load testing framework for KotobaDB including:
//! - Workload generators (YCSB, custom patterns)
//! - Performance metrics collection
//! - Concurrency testing
//! - Scalability testing
//! - Stress testing under high load

pub mod workload;
pub mod metrics;
pub mod runner;
pub mod scenarios;
pub mod config;
pub mod reporter;

use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

/// Load test result summary
#[derive(Debug, Clone)]
pub struct LoadTestResult {
    pub total_operations: u64,
    pub duration: Duration,
    pub operations_per_second: f64,
    pub latency_percentiles: LatencyPercentiles,
    pub error_count: u64,
    pub error_rate: f64,
}

/// Latency percentiles in microseconds
#[derive(Debug, Clone)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
}

/// Load test configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub duration: Duration,
    pub concurrency: usize,
    pub warmup_duration: Duration,
    pub operations_per_second: Option<u64>, // Rate limiting
    pub enable_metrics: bool,
    pub collect_detailed_latency: bool,
}

/// Workload operation types
#[derive(Debug, Clone)]
pub enum Operation {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Update { key: Vec<u8>, value: Vec<u8> },
    Read { key: Vec<u8> },
    Delete { key: Vec<u8> },
    Scan { start_key: Vec<u8>, limit: usize },
}

/// Operation result
#[derive(Debug, Clone)]
pub struct OperationResult {
    pub operation: Operation,
    pub latency_us: u64,
    pub success: bool,
    pub error_message: Option<String>,
}

/// Load test runner trait
#[async_trait::async_trait]
pub trait LoadTestRunner {
    async fn setup(&mut self) -> Result<(), Box<dyn std::error::Error>>;
    async fn run_operation(&self, op: Operation) -> Result<OperationResult, Box<dyn std::error::Error>>;
    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

/// Generic load test executor
pub struct LoadTestExecutor<R: LoadTestRunner> {
    runner: Arc<Mutex<R>>,
    config: LoadTestConfig,
}

impl<R: LoadTestRunner> LoadTestExecutor<R> {
    pub fn new(runner: R, config: LoadTestConfig) -> Self {
        Self {
            runner: Arc::new(Mutex::new(runner)),
            config,
        }
    }

    pub async fn execute(&self, workload: Box<dyn workload::WorkloadGenerator>) -> Result<LoadTestResult, Box<dyn std::error::Error>> {
        // Setup
        {
            let mut runner = self.runner.lock().await;
            runner.setup().await?;
        }

        // Warmup phase
        if self.config.warmup_duration > Duration::ZERO {
            println!("Starting warmup phase ({}s)...", self.config.warmup_duration.as_secs());
            self.run_workload(workload.as_ref(), self.config.warmup_duration, true).await?;
            println!("Warmup completed");
        }

        // Main test phase
        println!("Starting main test phase ({}s) with {} concurrency...",
                self.config.duration.as_secs(), self.config.concurrency);
        let start_time = Instant::now();

        let results = self.run_workload(workload.as_ref(), self.config.duration, false).await?;
        let actual_duration = start_time.elapsed();

        // Teardown
        {
            let mut runner = self.runner.lock().await;
            runner.teardown().await?;
        }

        // Calculate final metrics
        let total_operations = results.len() as u64;
        let operations_per_second = total_operations as f64 / actual_duration.as_secs_f64();

        let mut latencies: Vec<u64> = results.iter().map(|r| r.latency_us).collect();
        latencies.sort_unstable();

        let error_count = results.iter().filter(|r| !r.success).count() as u64;
        let error_rate = error_count as f64 / total_operations as f64;

        let latency_percentiles = Self::calculate_percentiles(&latencies);

        Ok(LoadTestResult {
            total_operations,
            duration: actual_duration,
            operations_per_second,
            latency_percentiles,
            error_count,
            error_rate,
        })
    }

    async fn run_workload(&self, workload: &dyn workload::WorkloadGenerator, duration: Duration, is_warmup: bool)
        -> Result<Vec<OperationResult>, Box<dyn std::error::Error>> {

        let mut results = Vec::new();
        let start_time = Instant::now();

        // Create worker tasks
        let mut handles = Vec::new();
        for worker_id in 0..self.config.concurrency {
            let runner = Arc::clone(&self.runner);
            let workload_clone = workload.clone_box();

            let handle = tokio::spawn(async move {
                let mut worker_results = Vec::new();
                let mut op_count = 0u64;

                loop {
                    // Check if test duration exceeded
                    if start_time.elapsed() >= duration {
                        break;
                    }

                    // Rate limiting
                    if let Some(ops_per_sec) = self.config.operations_per_second {
                        let target_ops = (start_time.elapsed().as_secs_f64() * ops_per_sec as f64 / self.config.concurrency as f64) as u64;
                        if op_count >= target_ops {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            continue;
                        }
                    }

                    // Generate and execute operation
                    let operation = workload_clone.generate_operation(worker_id, op_count);
                    let op_start = Instant::now();

                    let result = {
                        let runner_guard = runner.lock().await;
                        runner_guard.run_operation(operation.clone()).await
                    };

                    let latency_us = op_start.elapsed().as_micros() as u64;

                    let op_result = match result {
                        Ok(_) => OperationResult {
                            operation,
                            latency_us,
                            success: true,
                            error_message: None,
                        },
                        Err(e) => OperationResult {
                            operation,
                            latency_us,
                            success: false,
                            error_message: Some(e.to_string()),
                        },
                    };

                    if !is_warmup {
                        worker_results.push(op_result);
                    }

                    op_count += 1;
                }

                worker_results
            });

            handles.push(handle);
        }

        // Collect results from all workers
        for handle in handles {
            let worker_results = handle.await?;
            results.extend(worker_results);
        }

        Ok(results)
    }

    fn calculate_percentiles(latencies: &[u64]) -> LatencyPercentiles {
        if latencies.is_empty() {
            return LatencyPercentiles {
                p50: 0, p95: 0, p99: 0, p999: 0, max: 0,
            };
        }

        let len = latencies.len();
        LatencyPercentiles {
            p50: latencies[len / 2],
            p95: latencies[(len as f64 * 0.95) as usize],
            p99: latencies[(len as f64 * 0.99) as usize],
            p999: latencies[(len as f64 * 0.999) as usize],
            max: *latencies.last().unwrap(),
        }
    }
}

/// Convenience function to run a load test
pub async fn run_load_test<R: LoadTestRunner>(
    runner: R,
    workload: Box<dyn workload::WorkloadGenerator>,
    config: LoadTestConfig,
) -> Result<LoadTestResult, Box<dyn std::error::Error>> {
    let executor = LoadTestExecutor::new(runner, config);
    executor.execute(workload).await
}

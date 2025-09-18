//! KotobaDB Benchmarking Suite
//!
//! Comprehensive performance benchmarking framework for KotobaDB including:
//! - CRUD operations benchmarking
//! - Query performance analysis
//! - Transaction throughput testing
//! - Memory usage profiling
//! - Storage operations benchmarking

pub mod runner;
pub mod analyzer;
pub mod generator;
pub mod reporter;
pub mod metrics;
pub mod workloads;

use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// Benchmark configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    /// Duration of each benchmark run
    pub duration: Duration,

    /// Number of concurrent operations
    pub concurrency: usize,

    /// Warmup duration before actual measurement
    pub warmup_duration: Duration,

    /// Rate limiting (operations per second)
    pub operations_per_second: Option<u64>,

    /// Enable detailed latency measurement
    pub measure_latency: bool,

    /// Enable memory profiling
    pub profile_memory: bool,

    /// Enable storage profiling
    pub profile_storage: bool,

    /// Custom benchmark parameters
    pub parameters: HashMap<String, String>,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            duration: Duration::from_secs(60),
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            measure_latency: true,
            profile_memory: true,
            profile_storage: true,
            parameters: HashMap::new(),
        }
    }
}

/// Benchmark result summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,

    /// Start timestamp
    pub start_time: chrono::DateTime<chrono::Utc>,

    /// End timestamp
    pub end_time: chrono::DateTime<chrono::Utc>,

    /// Total operations performed
    pub total_operations: u64,

    /// Operations per second
    pub operations_per_second: f64,

    /// Latency percentiles (microseconds)
    pub latency_percentiles: LatencyPercentiles,

    /// Error count and rate
    pub error_count: u64,
    pub error_rate: f64,

    /// Memory usage statistics
    pub memory_stats: Option<MemoryStats>,

    /// Storage statistics
    pub storage_stats: Option<StorageStats>,

    /// Custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

/// Latency percentiles in microseconds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatencyPercentiles {
    pub p50: u64,
    pub p95: u64,
    pub p99: u64,
    pub p999: u64,
    pub max: u64,
}

impl Default for LatencyPercentiles {
    fn default() -> Self {
        Self {
            p50: 0,
            p95: 0,
            p99: 0,
            p999: 0,
            max: 0,
        }
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub peak_memory_mb: f64,
    pub average_memory_mb: f64,
    pub memory_efficiency: f64, // operations per MB
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub total_bytes_written: u64,
    pub total_bytes_read: u64,
    pub storage_efficiency: f64, // operations per byte
    pub iops: f64,
}

/// Benchmark suite that runs multiple benchmarks
pub struct BenchmarkSuite {
    benchmarks: Vec<Box<dyn Benchmark>>,
    config: BenchmarkConfig,
}

impl BenchmarkSuite {
    pub fn new(config: BenchmarkConfig) -> Self {
        Self {
            benchmarks: Vec::new(),
            config,
        }
    }

    pub fn add_benchmark<B: Benchmark + 'static>(mut self, benchmark: B) -> Self {
        self.benchmarks.push(Box::new(benchmark));
        self
    }

    pub async fn run_all(&self) -> Result<Vec<BenchmarkResult>, Box<dyn std::error::Error>> {
        let mut results = Vec::new();

        for benchmark in &self.benchmarks {
            println!("Running benchmark: {}", benchmark.name());
            let result = benchmark.run(&self.config).await?;
            results.push(result);
        }

        Ok(results)
    }
}

/// Individual benchmark trait
#[async_trait::async_trait]
pub trait Benchmark {
    /// Get benchmark name
    fn name(&self) -> &str;

    /// Setup benchmark (database initialization, data population, etc.)
    async fn setup(&mut self, config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>>;

    /// Run the benchmark
    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>>;

    /// Cleanup after benchmark
    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

/// Performance baseline for regression detection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub benchmark_name: String,
    pub baseline_result: BenchmarkResult,
    pub recorded_at: chrono::DateTime<chrono::Utc>,
    pub environment_info: HashMap<String, String>,
}

/// Compare benchmark result against baseline
pub fn compare_with_baseline(result: &BenchmarkResult, baseline: &PerformanceBaseline) -> RegressionAnalysis {
    let throughput_change = (result.operations_per_second - baseline.baseline_result.operations_per_second)
        / baseline.baseline_result.operations_per_second * 100.0;

    let latency_change_p95 = (result.latency_percentiles.p95 as f64 - baseline.baseline_result.latency_percentiles.p95 as f64)
        / baseline.baseline_result.latency_percentiles.p95 as f64 * 100.0;

    let error_rate_change = (result.error_rate - baseline.baseline_result.error_rate)
        / baseline.baseline_result.error_rate * 100.0;

    RegressionAnalysis {
        benchmark_name: result.name.clone(),
        throughput_change_percent: throughput_change,
        latency_change_p95_percent: latency_change_p95,
        error_rate_change_percent: error_rate_change,
        has_regression: throughput_change < -5.0 || latency_change_p95 > 10.0 || error_rate_change > 50.0,
        significance: if throughput_change.abs() > 10.0 || latency_change_p95.abs() > 20.0 {
            RegressionSignificance::High
        } else if throughput_change.abs() > 5.0 || latency_change_p95.abs() > 10.0 {
            RegressionSignificance::Medium
        } else {
            RegressionSignificance::Low
        },
    }
}

/// Regression analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysis {
    pub benchmark_name: String,
    pub throughput_change_percent: f64,
    pub latency_change_p95_percent: f64,
    pub error_rate_change_percent: f64,
    pub has_regression: bool,
    pub significance: RegressionSignificance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionSignificance {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for RegressionSignificance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegressionSignificance::Low => write!(f, "Low"),
            RegressionSignificance::Medium => write!(f, "Medium"),
            RegressionSignificance::High => write!(f, "High"),
        }
    }
}

impl RegressionAnalysis {
    pub fn report(&self) -> String {
        format!(
            r#"Regression Analysis: {}
===============================
Throughput Change: {:.2}%
Latency p95 Change: {:.2}%
Error Rate Change: {:.2}%
Has Regression: {}
Significance: {}"#,
            self.benchmark_name,
            self.throughput_change_percent,
            self.latency_change_p95_percent,
            self.error_rate_change_percent,
            if self.has_regression { "YES ⚠️" } else { "NO ✅" },
            self.significance
        )
    }
}

/// Utility functions for benchmark execution
pub mod utils {
    use super::*;

    /// Generate test data for benchmarking
    pub fn generate_test_data(record_count: usize) -> Vec<(Vec<u8>, Vec<u8>)> {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let mut data = Vec::with_capacity(record_count);

        for i in 0..record_count {
            let key = format!("key_{:010}", i).into_bytes();
            let value = format!("value_{}_{:020}", i, rng.gen::<u64>()).into_bytes();
            data.push((key, value));
        }

        data
    }

    /// Calculate operations per second
    pub fn calculate_ops_per_second(total_operations: u64, duration: Duration) -> f64 {
        total_operations as f64 / duration.as_secs_f64()
    }

    /// Calculate latency percentiles from samples
    pub fn calculate_latency_percentiles(mut latencies: Vec<u64>) -> LatencyPercentiles {
        if latencies.is_empty() {
            return LatencyPercentiles::default();
        }

        latencies.sort_unstable();
        let len = latencies.len();

        LatencyPercentiles {
            p50: latencies[len / 2],
            p95: latencies[(len as f64 * 0.95) as usize],
            p99: latencies[(len as f64 * 0.99) as usize],
            p999: latencies[(len as f64 * 0.999) as usize],
            max: latencies[len - 1],
        }
    }
}

//! Benchmark Runner Implementation
//!
//! Provides the core benchmarking execution engine with:
//! - Configurable workload execution
//! - Performance metrics collection
//! - Resource monitoring
//! - Statistical analysis

use crate::{
    Benchmark, BenchmarkConfig, BenchmarkResult, LatencyPercentiles,
    MemoryStats, StorageStats,
};
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{Process, System};
use tokio::sync::mpsc;

/// Benchmark runner for executing performance tests
pub struct BenchmarkRunner {
    config: BenchmarkConfig,
    system: Arc<Mutex<System>>,
    metrics_collector: Option<MetricsCollector>,
}

impl BenchmarkRunner {
    pub fn new(config: BenchmarkConfig) -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            config,
            system: Arc::new(Mutex::new(system)),
            metrics_collector: None,
        }
    }

    /// Enable metrics collection
    pub fn with_metrics_collection(mut self) -> Self {
        self.metrics_collector = Some(MetricsCollector::new());
        self
    }

    /// Run a single benchmark
    pub async fn run_benchmark<B: Benchmark>(
        &self,
        mut benchmark: B,
    ) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        let benchmark_name = benchmark.name().to_string();

        // Setup phase
        println!("Setting up benchmark: {}", benchmark_name);
        benchmark.setup(&self.config).await?;

        // Warmup phase
        if self.config.warmup_duration > Duration::ZERO {
            println!("Running warmup for {}s...", self.config.warmup_duration.as_secs());
            let warmup_start = Instant::now();

            while warmup_start.elapsed() < self.config.warmup_duration {
                benchmark.run_warmup_operation().await?;
            }
        }

        // Main benchmark execution
        println!("Running benchmark for {}s with {} concurrency...",
                self.config.duration.as_secs(), self.config.concurrency);

        let result = self.execute_benchmark(&benchmark, &benchmark_name).await?;

        // Teardown
        benchmark.teardown().await?;

        Ok(result)
    }

    /// Execute the actual benchmark measurement
    async fn execute_benchmark<B: Benchmark>(
        &self,
        benchmark: &B,
        benchmark_name: &str,
    ) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        let start_time = chrono::Utc::now();
        let mut total_operations = 0u64;
        let mut error_count = 0u64;
        let mut latencies = Vec::new();

        // Start resource monitoring if enabled
        let memory_monitor = if self.config.profile_memory {
            Some(MemoryMonitor::new(Arc::clone(&self.system)))
        } else {
            None
        };

        let storage_monitor = if self.config.profile_storage {
            Some(StorageMonitor::new())
        } else {
            None
        };

        let benchmark_start = Instant::now();

        // Create worker tasks
        let mut handles = Vec::new();
        for worker_id in 0..self.config.concurrency {
            let config = self.config.clone();
            let (tx, mut rx) = mpsc::unbounded_channel();

            // Worker task
            let handle = tokio::spawn(async move {
                let mut worker_operations = 0u64;
                let mut worker_errors = 0u64;
                let mut worker_latencies = Vec::new();

                let mut op_count = 0u64;
                let worker_start = Instant::now();

                loop {
                    // Check if benchmark duration exceeded
                    if worker_start.elapsed() >= config.duration {
                        break;
                    }

                    // Rate limiting
                    if let Some(ops_per_sec) = config.operations_per_second {
                        let target_ops = (worker_start.elapsed().as_secs_f64() * ops_per_sec as f64 / config.concurrency as f64) as u64;
                        if op_count >= target_ops {
                            tokio::time::sleep(Duration::from_millis(1)).await;
                            continue;
                        }
                    }

                    let op_start = Instant::now();
                    let result = benchmark.run_operation(worker_id, op_count).await;
                    let latency_us = op_start.elapsed().as_micros() as u64;

                    match result {
                        Ok(_) => {
                            if config.measure_latency {
                                worker_latencies.push(latency_us);
                            }
                        }
                        Err(_) => {
                            worker_errors += 1;
                        }
                    }

                    worker_operations += 1;
                    op_count += 1;

                    // Send progress update
                    let _ = tx.send((worker_operations, worker_errors, latency_us));
                }

                (worker_operations, worker_errors, worker_latencies)
            });

            handles.push(handle);

            // Progress monitoring task
            if let Some(ref collector) = self.metrics_collector {
                let mut rx = rx;
                let collector = collector.clone();
                tokio::spawn(async move {
                    while let Some((ops, errors, latency)) = rx.recv().await {
                        collector.record_operation(ops, errors, latency).await;
                    }
                });
            }
        }

        // Wait for all workers to complete
        for handle in handles {
            let (worker_ops, worker_errors, worker_latencies) = handle.await?;
            total_operations += worker_ops;
            error_count += worker_errors;

            if self.config.measure_latency {
                latencies.extend(worker_latencies);
            }
        }

        let end_time = chrono::Utc::now();
        let actual_duration = benchmark_start.elapsed();

        // Calculate metrics
        let operations_per_second = total_operations as f64 / actual_duration.as_secs_f64();
        let error_rate = if total_operations > 0 {
            error_count as f64 / total_operations as f64
        } else {
            0.0
        };

        let latency_percentiles = if self.config.measure_latency && !latencies.is_empty() {
            calculate_latency_percentiles(latencies)
        } else {
            LatencyPercentiles::default()
        };

        // Collect resource statistics
        let memory_stats = if let Some(ref monitor) = memory_monitor {
            Some(monitor.get_stats())
        } else {
            None
        };

        let storage_stats = if let Some(ref monitor) = storage_monitor {
            Some(monitor.get_stats())
        } else {
            None
        };

        let result = BenchmarkResult {
            name: benchmark_name.to_string(),
            start_time,
            end_time,
            total_operations,
            operations_per_second,
            latency_percentiles,
            error_count,
            error_rate,
            memory_stats,
            storage_stats,
            custom_metrics: HashMap::new(),
        };

        Ok(result)
    }
}

/// Metrics collector for real-time monitoring
#[derive(Clone)]
pub struct MetricsCollector {
    operations: Arc<Mutex<u64>>,
    errors: Arc<Mutex<u64>>,
    latencies: Arc<Mutex<Vec<u64>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(0)),
            errors: Arc::new(Mutex::new(0)),
            latencies: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn record_operation(&self, operations: u64, errors: u64, latency: u64) {
        *self.operations.lock().unwrap() += operations;
        *self.errors.lock().unwrap() += errors;
        self.latencies.lock().unwrap().push(latency);
    }

    pub fn get_snapshot(&self) -> MetricsSnapshot {
        let operations = *self.operations.lock().unwrap();
        let errors = *self.errors.lock().unwrap();
        let latencies = self.latencies.lock().unwrap().clone();

        MetricsSnapshot {
            operations,
            errors,
            error_rate: if operations > 0 { errors as f64 / operations as f64 } else { 0.0 },
            latency_percentiles: if !latencies.is_empty() {
                calculate_latency_percentiles(latencies)
            } else {
                LatencyPercentiles::default()
            },
        }
    }
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub operations: u64,
    pub errors: u64,
    pub error_rate: f64,
    pub latency_percentiles: LatencyPercentiles,
}

/// Memory usage monitor
pub struct MemoryMonitor {
    system: Arc<Mutex<System>>,
    samples: Arc<Mutex<Vec<f64>>>,
}

impl MemoryMonitor {
    pub fn new(system: Arc<Mutex<System>>) -> Self {
        let samples = Arc::new(Mutex::new(Vec::new()));

        // Start background monitoring
        let samples_clone = Arc::clone(&samples);
        let system_clone = Arc::clone(&system);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(100));
            loop {
                interval.tick().await;
                let mut sys = system_clone.lock().unwrap();
                sys.refresh_memory();

                let memory_mb = sys.used_memory() as f64 / 1024.0 / 1024.0;
                samples_clone.lock().unwrap().push(memory_mb);
            }
        });

        Self { system, samples }
    }

    pub fn get_stats(&self) -> MemoryStats {
        let samples = self.samples.lock().unwrap();
        if samples.is_empty() {
            return MemoryStats {
                peak_memory_mb: 0.0,
                average_memory_mb: 0.0,
                memory_efficiency: 0.0,
            };
        }

        let peak_memory = samples.iter().cloned().fold(0.0, f64::max);
        let average_memory = samples.iter().sum::<f64>() / samples.len() as f64;

        MemoryStats {
            peak_memory_mb: peak_memory,
            average_memory_mb: average_memory,
            memory_efficiency: 0.0, // Will be set by benchmark result
        }
    }
}

/// Storage I/O monitor
pub struct StorageMonitor {
    bytes_written: Arc<Mutex<u64>>,
    bytes_read: Arc<Mutex<u64>>,
    operations: Arc<Mutex<u64>>,
}

impl StorageMonitor {
    pub fn new() -> Self {
        Self {
            bytes_written: Arc::new(Mutex::new(0)),
            bytes_read: Arc::new(Mutex::new(0)),
            operations: Arc::new(Mutex::new(0)),
        }
    }

    pub fn record_write(&self, bytes: u64) {
        *self.bytes_written.lock().unwrap() += bytes;
        *self.operations.lock().unwrap() += 1;
    }

    pub fn record_read(&self, bytes: u64) {
        *self.bytes_read.lock().unwrap() += bytes;
        *self.operations.lock().unwrap() += 1;
    }

    pub fn get_stats(&self) -> StorageStats {
        let bytes_written = *self.bytes_written.lock().unwrap();
        let bytes_read = *self.bytes_read.lock().unwrap();
        let operations = *self.operations.lock().unwrap();

        StorageStats {
            total_bytes_written: bytes_written,
            total_bytes_read: bytes_read,
            storage_efficiency: if bytes_written > 0 { operations as f64 / bytes_written as f64 } else { 0.0 },
            iops: operations as f64, // Simplified - should be calculated over time
        }
    }
}

/// Calculate latency percentiles from samples
fn calculate_latency_percentiles(mut latencies: Vec<u64>) -> LatencyPercentiles {
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

/// Extension trait for benchmarks to add warmup functionality
#[async_trait::async_trait]
pub trait BenchmarkExt: Benchmark {
    /// Run a single warmup operation
    async fn run_warmup_operation(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.run_operation(0, 0).await?;
        Ok(())
    }
}

impl<B: Benchmark> BenchmarkExt for B {}

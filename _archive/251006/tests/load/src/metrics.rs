//! Performance Metrics Collection for Load Testing
//!
//! Comprehensive metrics collection including:
//! - Latency histograms and percentiles
//! - Throughput measurements
//! - Error rate tracking
//! - Resource usage monitoring
//! - Custom metrics support

use crate::{OperationResult, LatencyPercentiles, LoadTestResult};
use hdrhistogram::Histogram;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// Metrics collector for load testing
#[derive(Debug)]
pub struct MetricsCollector {
    histogram: Arc<Mutex<Histogram<u64>>>,
    operation_counts: Arc<Mutex<HashMap<String, u64>>>,
    error_counts: Arc<Mutex<HashMap<String, u64>>>,
    start_time: Instant,
    custom_metrics: Arc<Mutex<HashMap<String, Vec<f64>>>>,
    sender: Option<mpsc::UnboundedSender<OperationResult>>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Self {
        let mut histogram = Histogram::<u64>::new_with_bounds(1, 10_000_000, 3).unwrap();

        Self {
            histogram: Arc::new(Mutex::new(histogram)),
            operation_counts: Arc::new(Mutex::new(HashMap::new())),
            error_counts: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
            custom_metrics: Arc::new(Mutex::new(HashMap::new())),
            sender: None,
        }
    }

    /// Create a collector with background processing
    pub fn with_background_processing() -> (Self, MetricsProcessor) {
        let (tx, rx) = mpsc::unbounded_channel();

        let collector = Self {
            histogram: Arc::new(Mutex::new(Histogram::<u64>::new_with_bounds(1, 10_000_000, 3).unwrap())),
            operation_counts: Arc::new(Mutex::new(HashMap::new())),
            error_counts: Arc::new(Mutex::new(HashMap::new())),
            start_time: Instant::now(),
            custom_metrics: Arc::new(Mutex::new(HashMap::new())),
            sender: Some(tx),
        };

        let processor = MetricsProcessor::new(rx, collector.histogram.clone(), collector.operation_counts.clone(), collector.error_counts.clone());

        (collector, processor)
    }

    /// Record an operation result
    pub fn record_operation(&self, result: OperationResult) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(result);
        } else {
            self.record_operation_sync(result);
        }
    }

    /// Synchronously record an operation result
    pub fn record_operation_sync(&self, result: OperationResult) {
        // Record latency
        if let Ok(mut hist) = self.histogram.lock() {
            let _ = hist.record(result.latency_us);
        }

        // Record operation type
        let op_type = match result.operation {
            crate::Operation::Insert { .. } => "insert",
            crate::Operation::Update { .. } => "update",
            crate::Operation::Read { .. } => "read",
            crate::Operation::Delete { .. } => "delete",
            crate::Operation::Scan { .. } => "scan",
        };

        if let Ok(mut counts) = self.operation_counts.lock() {
            *counts.entry(op_type.to_string()).or_insert(0) += 1;
        }

        // Record errors
        if !result.success {
            if let Ok(mut errors) = self.error_counts.lock() {
                let error_type = result.error_message.as_deref().unwrap_or("unknown");
                *errors.entry(error_type.to_string()).or_insert(0) += 1;
            }
        }
    }

    /// Record a custom metric
    pub fn record_custom_metric(&self, name: &str, value: f64) {
        if let Ok(mut metrics) = self.custom_metrics.lock() {
            metrics.entry(name.to_string()).or_insert_with(Vec::new).push(value);
        }
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let histogram = self.histogram.lock().unwrap();
        let operation_counts = self.operation_counts.lock().unwrap().clone();
        let error_counts = self.error_counts.lock().unwrap().clone();
        let custom_metrics = self.custom_metrics.lock().unwrap().clone();

        let total_operations: u64 = operation_counts.values().sum();
        let total_errors: u64 = error_counts.values().sum();

        MetricsSnapshot {
            elapsed: self.start_time.elapsed(),
            total_operations,
            total_errors,
            error_rate: if total_operations > 0 { total_errors as f64 / total_operations as f64 } else { 0.0 },
            latency_percentiles: LatencyPercentiles {
                p50: histogram.value_at_percentile(50.0),
                p95: histogram.value_at_percentile(95.0),
                p99: histogram.value_at_percentile(99.0),
                p999: histogram.value_at_percentile(99.9),
                max: histogram.max(),
            },
            operations_per_second: total_operations as f64 / self.start_time.elapsed().as_secs_f64(),
            operation_counts,
            error_counts,
            custom_metrics,
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        if let Ok(mut hist) = self.histogram.lock() {
            hist.reset();
        }
        if let Ok(mut counts) = self.operation_counts.lock() {
            counts.clear();
        }
        if let Ok(mut errors) = self.error_counts.lock() {
            errors.clear();
        }
        if let Ok(mut custom) = self.custom_metrics.lock() {
            custom.clear();
        }
        self.start_time = Instant::now();
    }
}

/// Metrics snapshot for reporting
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub elapsed: Duration,
    pub total_operations: u64,
    pub total_errors: u64,
    pub error_rate: f64,
    pub latency_percentiles: LatencyPercentiles,
    pub operations_per_second: f64,
    pub operation_counts: HashMap<String, u64>,
    pub error_counts: HashMap<String, u64>,
    pub custom_metrics: HashMap<String, Vec<f64>>,
}

impl MetricsSnapshot {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            r#"Load Test Results Summary
===============================
Elapsed Time: {:.2}s
Total Operations: {}
Operations/sec: {:.0}
Error Rate: {:.2}%

Latency Percentiles (μs):
  50th: {} μs
  95th: {} μs
  99th: {} μs
  99.9th: {} μs
  Max: {} μs

Operation Breakdown:
{}

Error Breakdown:
{}
"#,
            self.elapsed.as_secs_f64(),
            self.total_operations,
            self.operations_per_second,
            self.error_rate * 100.0,
            self.latency_percentiles.p50,
            self.latency_percentiles.p95,
            self.latency_percentiles.p99,
            self.latency_percentiles.p999,
            self.latency_percentiles.max,
            self.operation_counts.iter()
                .map(|(op, count)| format!("  {}: {}", op, count))
                .collect::<Vec<_>>()
                .join("\n"),
            self.error_counts.iter()
                .map(|(error, count)| format!("  {}: {}", error, count))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }

    /// Export to CSV format
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("metric,value\n");
        csv.push_str(&format!("elapsed_seconds,{:.2}\n", self.elapsed.as_secs_f64()));
        csv.push_str(&format!("total_operations,{}\n", self.total_operations));
        csv.push_str(&format!("operations_per_second,{:.2}\n", self.operations_per_second));
        csv.push_str(&format!("error_rate,{:.4}\n", self.error_rate));
        csv.push_str(&format!("latency_p50,{}\n", self.latency_percentiles.p50));
        csv.push_str(&format!("latency_p95,{}\n", self.latency_percentiles.p95));
        csv.push_str(&format!("latency_p99,{}\n", self.latency_percentiles.p99));
        csv.push_str(&format!("latency_p999,{}\n", self.latency_percentiles.p999));
        csv.push_str(&format!("latency_max,{}\n", self.latency_percentiles.max));

        for (op, count) in &self.operation_counts {
            csv.push_str(&format!("op_{},{}\n", op, count));
        }

        for (error, count) in &self.error_counts {
            csv.push_str(&format!("error_{},{}\n", error, count));
        }

        csv
    }
}

/// Background metrics processor
pub struct MetricsProcessor {
    receiver: mpsc::UnboundedReceiver<OperationResult>,
    histogram: Arc<Mutex<Histogram<u64>>>,
    operation_counts: Arc<Mutex<HashMap<String, u64>>>,
    error_counts: Arc<Mutex<HashMap<String, u64>>>,
}

impl MetricsProcessor {
    pub fn new(
        receiver: mpsc::UnboundedReceiver<OperationResult>,
        histogram: Arc<Mutex<Histogram<u64>>>,
        operation_counts: Arc<Mutex<HashMap<String, u64>>>,
        error_counts: Arc<Mutex<HashMap<String, u64>>>,
    ) -> Self {
        Self {
            receiver,
            histogram,
            operation_counts,
            error_counts,
        }
    }

    /// Run the background processing loop
    pub async fn run(mut self) {
        while let Some(result) = self.receiver.recv().await {
            // Record latency
            if let Ok(mut hist) = self.histogram.lock() {
                let _ = hist.record(result.latency_us);
            }

            // Record operation type
            let op_type = match result.operation {
                crate::Operation::Insert { .. } => "insert",
                crate::Operation::Update { .. } => "update",
                crate::Operation::Read { .. } => "read",
                crate::Operation::Delete { .. } => "delete",
                crate::Operation::Scan { .. } => "scan",
            };

            if let Ok(mut counts) = self.operation_counts.lock() {
                *counts.entry(op_type.to_string()).or_insert(0) += 1;
            }

            // Record errors
            if !result.success {
                if let Ok(mut errors) = self.error_counts.lock() {
                    let error_type = result.error_message.as_deref().unwrap_or("unknown");
                    *errors.entry(error_type.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}

/// System resource monitor
#[derive(Debug)]
pub struct ResourceMonitor {
    start_time: Instant,
    cpu_samples: Vec<f64>,
    memory_samples: Vec<u64>,
}

impl ResourceMonitor {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            cpu_samples: Vec::new(),
            memory_samples: Vec::new(),
        }
    }

    /// Sample current resource usage
    pub fn sample(&mut self) {
        // Note: In a real implementation, you would use system monitoring libraries
        // like sysinfo or heim to collect actual CPU and memory usage
        // For now, we'll collect placeholder data

        // Placeholder CPU usage (0-100%)
        let cpu_usage = (self.start_time.elapsed().as_millis() % 100) as f64;
        self.cpu_samples.push(cpu_usage);

        // Placeholder memory usage in bytes
        let memory_usage = 100_000_000 + (self.start_time.elapsed().as_millis() * 1000) as u64;
        self.memory_samples.push(memory_usage);
    }

    /// Get resource usage summary
    pub fn summary(&self) -> ResourceSummary {
        if self.cpu_samples.is_empty() {
            return ResourceSummary::default();
        }

        let cpu_avg = self.cpu_samples.iter().sum::<f64>() / self.cpu_samples.len() as f64;
        let cpu_max = self.cpu_samples.iter().cloned().fold(0.0, f64::max);

        let memory_avg = self.memory_samples.iter().sum::<u64>() as f64 / self.memory_samples.len() as f64;
        let memory_max = *self.memory_samples.iter().max().unwrap_or(&0);

        ResourceSummary {
            cpu_avg,
            cpu_max,
            memory_avg_mb: memory_avg / (1024.0 * 1024.0),
            memory_max_mb: memory_max as f64 / (1024.0 * 1024.0),
            sample_count: self.cpu_samples.len(),
        }
    }
}

#[derive(Debug, Default)]
pub struct ResourceSummary {
    pub cpu_avg: f64,
    pub cpu_max: f64,
    pub memory_avg_mb: f64,
    pub memory_max_mb: f64,
    pub sample_count: usize,
}

impl ResourceSummary {
    pub fn report(&self) -> String {
        format!(
            "Resource Usage Summary:\n  CPU: {:.1}% avg, {:.1}% max\n  Memory: {:.1} MB avg, {:.1} MB max\n  Samples: {}",
            self.cpu_avg, self.cpu_max, self.memory_avg_mb, self.memory_max_mb, self.sample_count
        )
    }
}

/// Time-series metrics for continuous monitoring
#[derive(Debug)]
pub struct TimeSeriesMetrics {
    timestamps: Vec<f64>,
    latencies: Vec<u64>,
    operations_per_second: Vec<f64>,
}

impl TimeSeriesMetrics {
    pub fn new() -> Self {
        Self {
            timestamps: Vec::new(),
            latencies: Vec::new(),
            operations_per_second: Vec::new(),
        }
    }

    pub fn record(&mut self, timestamp: f64, latency_us: u64, ops_per_sec: f64) {
        self.timestamps.push(timestamp);
        self.latencies.push(latency_us);
        self.operations_per_second.push(ops_per_sec);
    }

    pub fn export_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        let data = serde_json::json!({
            "timestamps": self.timestamps,
            "latencies_us": self.latencies,
            "operations_per_second": self.operations_per_second,
        });
        Ok(serde_json::to_string_pretty(&data)?)
    }
}

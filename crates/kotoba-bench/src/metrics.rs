//! Benchmark Metrics Collection
//!
//! Real-time metrics collection during benchmark execution

use crate::{BenchmarkResult, MemoryStats, StorageStats};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use sysinfo::{ProcessExt, System, SystemExt};

/// Real-time metrics collector
pub struct RealTimeMetrics {
    start_time: Instant,
    operation_count: Arc<Mutex<u64>>,
    error_count: Arc<Mutex<u64>>,
    latencies: Arc<Mutex<Vec<u64>>>,
    memory_samples: Arc<Mutex<Vec<f64>>>,
    system: Arc<Mutex<System>>,
}

impl RealTimeMetrics {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            start_time: Instant::now(),
            operation_count: Arc::new(Mutex::new(0)),
            error_count: Arc::new(Mutex::new(0)),
            latencies: Arc::new(Mutex::new(Vec::new())),
            memory_samples: Arc::new(Mutex::new(Vec::new())),
            system: Arc::new(Mutex::new(system)),
        }
    }

    /// Record a successful operation
    pub fn record_success(&self, latency_us: u64) {
        *self.operation_count.lock().unwrap() += 1;
        self.latencies.lock().unwrap().push(latency_us);
    }

    /// Record a failed operation
    pub fn record_error(&self, latency_us: u64) {
        *self.operation_count.lock().unwrap() += 1;
        *self.error_count.lock().unwrap() += 1;
        self.latencies.lock().unwrap().push(latency_us);
    }

    /// Sample memory usage
    pub fn sample_memory(&self) {
        let mut system = self.system.lock().unwrap();
        system.refresh_memory();

        let memory_mb = system.used_memory() as f64 / 1024.0 / 1024.0;
        self.memory_samples.lock().unwrap().push(memory_mb);
    }

    /// Get current metrics snapshot
    pub fn snapshot(&self) -> MetricsSnapshot {
        let operation_count = *self.operation_count.lock().unwrap();
        let error_count = *self.error_count.lock().unwrap();
        let latencies = self.latencies.lock().unwrap().clone();
        let memory_samples = self.memory_samples.lock().unwrap().clone();

        let elapsed = self.start_time.elapsed();

        MetricsSnapshot {
            elapsed,
            operation_count,
            error_count,
            operations_per_second: operation_count as f64 / elapsed.as_secs_f64(),
            error_rate: if operation_count > 0 { error_count as f64 / operation_count as f64 } else { 0.0 },
            latency_percentiles: if !latencies.is_empty() {
                calculate_percentiles(&latencies)
            } else {
                crate::LatencyPercentiles::default()
            },
            memory_stats: if !memory_samples.is_empty() {
                Some(MemoryStats {
                    peak_memory_mb: memory_samples.iter().cloned().fold(0.0, f64::max),
                    average_memory_mb: memory_samples.iter().sum::<f64>() / memory_samples.len() as f64,
                    memory_efficiency: 0.0, // Calculated later
                })
            } else {
                None
            },
            storage_stats: None, // Not implemented in real-time collection
        }
    }

    /// Reset all metrics
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        *self.operation_count.lock().unwrap() = 0;
        *self.error_count.lock().unwrap() = 0;
        self.latencies.lock().unwrap().clear();
        self.memory_samples.lock().unwrap().clear();
    }
}

/// Metrics snapshot
#[derive(Debug, Clone)]
pub struct MetricsSnapshot {
    pub elapsed: Duration,
    pub operation_count: u64,
    pub error_count: u64,
    pub operations_per_second: f64,
    pub error_rate: f64,
    pub latency_percentiles: crate::LatencyPercentiles,
    pub memory_stats: Option<MemoryStats>,
    pub storage_stats: Option<StorageStats>,
}

/// Calculate latency percentiles from samples
fn calculate_percentiles(latencies: &[u64]) -> crate::LatencyPercentiles {
    if latencies.is_empty() {
        return crate::LatencyPercentiles::default();
    }

    let mut sorted = latencies.to_vec();
    sorted.sort_unstable();

    let len = sorted.len();
    crate::LatencyPercentiles {
        p50: sorted[len / 2],
        p95: sorted[(len as f64 * 0.95) as usize],
        p99: sorted[(len as f64 * 0.99) as usize],
        p999: sorted[(len as f64 * 0.999) as usize],
        max: sorted[len - 1],
    }
}

/// Performance trend analyzer
pub struct TrendAnalyzer {
    historical_data: Vec<MetricsSnapshot>,
    window_size: usize,
}

impl TrendAnalyzer {
    pub fn new(window_size: usize) -> Self {
        Self {
            historical_data: Vec::new(),
            window_size,
        }
    }

    /// Add a new metrics snapshot
    pub fn add_snapshot(&mut self, snapshot: MetricsSnapshot) {
        self.historical_data.push(snapshot);

        // Keep only recent data
        if self.historical_data.len() > self.window_size {
            self.historical_data.remove(0);
        }
    }

    /// Analyze performance trends
    pub fn analyze_trends(&self) -> TrendAnalysis {
        if self.historical_data.len() < 2 {
            return TrendAnalysis {
                throughput_trend: 0.0,
                latency_trend: 0.0,
                memory_trend: 0.0,
                stability_score: 0.0,
                trend_description: "Insufficient data for trend analysis".to_string(),
            };
        }

        let recent = &self.historical_data[self.historical_data.len().saturating_sub(5)..];

        // Calculate throughput trend (linear regression slope)
        let throughput_values: Vec<f64> = recent.iter().map(|s| s.operations_per_second).collect();
        let throughput_trend = calculate_trend(&throughput_values);

        // Calculate latency trend
        let latency_values: Vec<f64> = recent.iter().map(|s| s.latency_percentiles.p95 as f64).collect();
        let latency_trend = calculate_trend(&latency_values);

        // Calculate memory trend
        let memory_values: Vec<f64> = recent.iter()
            .filter_map(|s| s.memory_stats.as_ref().map(|m| m.average_memory_mb))
            .collect();
        let memory_trend = if !memory_values.is_empty() {
            calculate_trend(&memory_values)
        } else {
            0.0
        };

        // Calculate stability score (coefficient of variation)
        let throughput_cv = calculate_coefficient_of_variation(&throughput_values);
        let latency_cv = calculate_coefficient_of_variation(&latency_values);
        let stability_score = 1.0 - (throughput_cv + latency_cv) / 2.0; // Normalize to 0-1

        let trend_description = format!(
            "Throughput: {:.1}% change, Latency: {:.1}% change, Stability: {:.1}",
            throughput_trend, latency_trend, stability_score
        );

        TrendAnalysis {
            throughput_trend,
            latency_trend,
            memory_trend,
            stability_score: stability_score.max(0.0).min(1.0),
            trend_description,
        }
    }
}

/// Trend analysis result
#[derive(Debug, Clone)]
pub struct TrendAnalysis {
    pub throughput_trend: f64, // Percentage change per unit time
    pub latency_trend: f64,    // Percentage change per unit time
    pub memory_trend: f64,     // Percentage change per unit time
    pub stability_score: f64,  // 0.0 (unstable) to 1.0 (stable)
    pub trend_description: String,
}

/// Calculate linear trend (slope) from data points
fn calculate_trend(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let n = values.len() as f64;
    let x_sum: f64 = (0..values.len()).map(|i| i as f64).sum();
    let y_sum: f64 = values.iter().sum();
    let xy_sum: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
    let x_squared_sum: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

    let slope = (n * xy_sum - x_sum * y_sum) / (n * x_squared_sum - x_sum.powi(2));

    // Return as percentage change
    if values.first().unwrap_or(&1.0) != &0.0 {
        (slope / values[0]) * 100.0
    } else {
        0.0
    }
}

/// Calculate coefficient of variation (normalized standard deviation)
fn calculate_coefficient_of_variation(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }

    let mean = values.iter().sum::<f64>() / values.len() as f64;
    if mean == 0.0 {
        return 0.0;
    }

    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
    let std_dev = variance.sqrt();

    std_dev / mean
}

/// Performance profiler for detailed analysis
pub struct PerformanceProfiler {
    metrics: RealTimeMetrics,
    trend_analyzer: TrendAnalyzer,
    profile_data: Arc<Mutex<HashMap<String, Vec<f64>>>>,
}

impl PerformanceProfiler {
    pub fn new() -> Self {
        Self {
            metrics: RealTimeMetrics::new(),
            trend_analyzer: TrendAnalyzer::new(100),
            profile_data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start profiling
    pub fn start_profiling(&mut self) {
        self.metrics.reset();
    }

    /// Record a profiling event
    pub fn record_event(&self, event_type: &str, value: f64) {
        self.profile_data.lock().unwrap()
            .entry(event_type.to_string())
            .or_insert_with(Vec::new)
            .push(value);
    }

    /// Sample current performance metrics
    pub fn sample(&mut self) {
        self.metrics.sample_memory();
        let snapshot = self.metrics.snapshot();
        self.trend_analyzer.add_snapshot(snapshot);
    }

    /// Generate profiling report
    pub fn generate_report(&self) -> ProfilingReport {
        let trends = self.trend_analyzer.analyze_trends();
        let profile_data = self.profile_data.lock().unwrap().clone();

        let mut event_summaries = HashMap::new();
        for (event_type, values) in &profile_data {
            if !values.is_empty() {
                let mean = values.iter().sum::<f64>() / values.len() as f64;
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);

                event_summaries.insert(event_type.clone(), EventSummary {
                    count: values.len(),
                    mean,
                    min,
                    max,
                    standard_deviation: calculate_std_dev(values, mean),
                });
            }
        }

        ProfilingReport {
            trends,
            event_summaries,
            recommendations: self.generate_recommendations(&trends, &event_summaries),
        }
    }

    fn generate_recommendations(&self, trends: &TrendAnalysis, events: &HashMap<String, EventSummary>) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Throughput recommendations
        if trends.throughput_trend < -5.0 {
            recommendations.push("Consider optimizing CPU-bound operations".to_string());
        }

        // Latency recommendations
        if trends.latency_trend > 10.0 {
            recommendations.push("Investigate I/O bottlenecks and caching strategies".to_string());
        }

        // Memory recommendations
        if trends.memory_trend > 20.0 {
            recommendations.push("Monitor memory leaks and consider memory pooling".to_string());
        }

        // Stability recommendations
        if trends.stability_score < 0.7 {
            recommendations.push("Address performance variability issues".to_string());
        }

        // Event-based recommendations
        if let Some(gc_event) = events.get("gc_time") {
            if gc_event.mean > 10.0 {
                recommendations.push("Optimize garbage collection or reduce object allocation".to_string());
            }
        }

        if recommendations.is_empty() {
            recommendations.push("Performance is stable and within acceptable ranges".to_string());
        }

        recommendations
    }
}

/// Profiling report
#[derive(Debug, Clone)]
pub struct ProfilingReport {
    pub trends: TrendAnalysis,
    pub event_summaries: HashMap<String, EventSummary>,
    pub recommendations: Vec<String>,
}

/// Event summary statistics
#[derive(Debug, Clone)]
pub struct EventSummary {
    pub count: usize,
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub standard_deviation: f64,
}

/// Calculate standard deviation
fn calculate_std_dev(values: &[f64], mean: f64) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }

    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / (values.len() - 1) as f64;
    variance.sqrt()
}

/// Benchmark comparison utilities
pub struct BenchmarkComparator {
    baseline_results: HashMap<String, BenchmarkResult>,
}

impl BenchmarkComparator {
    pub fn new() -> Self {
        Self {
            baseline_results: HashMap::new(),
        }
    }

    /// Set baseline result for a benchmark
    pub fn set_baseline(&mut self, name: &str, result: BenchmarkResult) {
        self.baseline_results.insert(name.to_string(), result);
    }

    /// Compare current result with baseline
    pub fn compare(&self, name: &str, current: &BenchmarkResult) -> Option<ComparisonResult> {
        self.baseline_results.get(name).map(|baseline| {
            let throughput_change = (current.operations_per_second - baseline.operations_per_second)
                / baseline.operations_per_second * 100.0;

            let latency_change = (current.latency_percentiles.p95 as f64 - baseline.latency_percentiles.p95 as f64)
                / baseline.latency_percentiles.p95 as f64 * 100.0;

            let error_change = (current.error_rate - baseline.error_rate)
                / baseline.error_rate * 100.0;

            ComparisonResult {
                benchmark_name: name.to_string(),
                throughput_change_percent: throughput_change,
                latency_change_percent: latency_change,
                error_change_percent: error_change,
                has_regression: throughput_change < -5.0 || latency_change > 10.0,
                significance: Self::calculate_significance(throughput_change, latency_change),
            }
        })
    }

    fn calculate_significance(throughput_change: f64, latency_change: f64) -> ComparisonSignificance {
        let total_change = throughput_change.abs() + latency_change.abs();

        if total_change > 50.0 {
            ComparisonSignificance::High
        } else if total_change > 20.0 {
            ComparisonSignificance::Medium
        } else {
            ComparisonSignificance::Low
        }
    }
}

/// Comparison result
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub benchmark_name: String,
    pub throughput_change_percent: f64,
    pub latency_change_percent: f64,
    pub error_change_percent: f64,
    pub has_regression: bool,
    pub significance: ComparisonSignificance,
}

#[derive(Debug, Clone)]
pub enum ComparisonSignificance {
    Low,
    Medium,
    High,
}

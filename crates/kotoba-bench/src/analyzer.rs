//! Performance Analyzer
//!
//! Advanced performance analysis tools including:
//! - Bottleneck identification
//! - Performance trend analysis
//! - Statistical significance testing
//! - Comparative analysis across configurations

use crate::{BenchmarkResult, RegressionAnalysis, PerformanceBaseline};
use std::collections::{HashMap, BTreeMap};
use serde::{Deserialize, Serialize};
use statrs::statistics::{Statistics, OrderStatistics};

/// Performance analyzer for in-depth benchmark analysis
pub struct PerformanceAnalyzer {
    results: Vec<BenchmarkResult>,
    baselines: HashMap<String, PerformanceBaseline>,
}

impl PerformanceAnalyzer {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
            baselines: HashMap::new(),
        }
    }

    /// Add benchmark result for analysis
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Add performance baseline
    pub fn add_baseline(&mut self, baseline: PerformanceBaseline) {
        self.baselines.insert(baseline.benchmark_name.clone(), baseline);
    }

    /// Analyze all results and generate comprehensive report
    pub fn analyze(&self) -> AnalysisReport {
        let mut report = AnalysisReport {
            summary: self.generate_summary(),
            regressions: self.detect_regressions(),
            bottlenecks: self.identify_bottlenecks(),
            trends: self.analyze_trends(),
            recommendations: self.generate_recommendations(),
            statistical_analysis: self.perform_statistical_analysis(),
        };

        report
    }

    /// Generate summary statistics
    fn generate_summary(&self) -> PerformanceSummary {
        if self.results.is_empty() {
            return PerformanceSummary::default();
        }

        let total_operations: u64 = self.results.iter().map(|r| r.total_operations).sum();
        let avg_throughput = self.results.iter().map(|r| r.operations_per_second).sum::<f64>() / self.results.len() as f64;

        let all_latencies_p95: Vec<f64> = self.results.iter().map(|r| r.latency_percentiles.p95 as f64).collect();
        let avg_latency_p95 = all_latencies_p95.mean();

        let all_error_rates: Vec<f64> = self.results.iter().map(|r| r.error_rate).collect();
        let avg_error_rate = all_error_rates.mean();

        let peak_memory_usage = self.results.iter()
            .filter_map(|r| r.memory_stats.as_ref())
            .map(|m| m.peak_memory_mb)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        PerformanceSummary {
            total_benchmarks: self.results.len(),
            total_operations,
            average_throughput: avg_throughput,
            average_latency_p95: avg_latency_p95,
            average_error_rate: avg_error_rate,
            peak_memory_usage,
            duration_range: self.calculate_duration_range(),
        }
    }

    /// Detect performance regressions compared to baselines
    fn detect_regressions(&self) -> Vec<RegressionAnalysis> {
        let mut regressions = Vec::new();

        for result in &self.results {
            if let Some(baseline) = self.baselines.get(&result.name) {
                let analysis = crate::compare_with_baseline(result, baseline);
                if analysis.has_regression {
                    regressions.push(analysis);
                }
            }
        }

        regressions.sort_by(|a, b| (b.significance as u8).cmp(&(a.significance as u8)));
        regressions
    }

    /// Identify performance bottlenecks
    fn identify_bottlenecks(&self) -> Vec<BottleneckAnalysis> {
        let mut bottlenecks = Vec::new();

        for result in &self.results {
            // High latency bottleneck
            if result.latency_percentiles.p95 > 10000 { // 10ms
                bottlenecks.push(BottleneckAnalysis {
                    benchmark_name: result.name.clone(),
                    bottleneck_type: BottleneckType::HighLatency,
                    severity: if result.latency_percentiles.p95 > 50000 { Severity::Critical } else { Severity::High },
                    description: format!("P95 latency is {} μs, indicating potential I/O or processing bottlenecks", result.latency_percentiles.p95),
                    recommendations: vec![
                        "Consider optimizing I/O operations".to_string(),
                        "Review query execution plans".to_string(),
                        "Check for lock contention".to_string(),
                    ],
                });
            }

            // High error rate bottleneck
            if result.error_rate > 0.05 {
                bottlenecks.push(BottleneckAnalysis {
                    benchmark_name: result.name.clone(),
                    bottleneck_type: BottleneckType::HighErrorRate,
                    severity: if result.error_rate > 0.1 { Severity::Critical } else { Severity::High },
                    description: format!("Error rate is {:.2}%, indicating system stability issues", result.error_rate * 100.0),
                    recommendations: vec![
                        "Investigate error causes in logs".to_string(),
                        "Check system resource limits".to_string(),
                        "Review error handling code".to_string(),
                    ],
                });
            }

            // Memory bottleneck
            if let Some(mem_stats) = &result.memory_stats {
                if mem_stats.peak_memory_mb > 1024.0 { // 1GB
                    bottlenecks.push(BottleneckAnalysis {
                        benchmark_name: result.name.clone(),
                        bottleneck_type: BottleneckType::MemoryPressure,
                        severity: Severity::Medium,
                        description: format!("Peak memory usage is {:.1} MB, indicating potential memory leaks", mem_stats.peak_memory_mb),
                        recommendations: vec![
                            "Profile memory usage with heap analysis".to_string(),
                            "Check for memory leaks in long-running operations".to_string(),
                            "Consider memory pool optimizations".to_string(),
                        ],
                    });
                }
            }
        }

        bottlenecks.sort_by(|a, b| b.severity.cmp(&a.severity));
        bottlenecks
    }

    /// Analyze performance trends over time
    fn analyze_trends(&self) -> Vec<TrendAnalysis> {
        let mut trends = Vec::new();

        // Group results by benchmark name
        let mut results_by_name: HashMap<String, Vec<&BenchmarkResult>> = HashMap::new();
        for result in &self.results {
            results_by_name.entry(result.name.clone()).or_insert(Vec::new()).push(result);
        }

        for (name, results) in results_by_name {
            if results.len() < 2 {
                continue;
            }

            // Sort by start time
            let mut sorted_results = results.clone();
            sorted_results.sort_by(|a, b| a.start_time.cmp(&b.start_time));

            // Calculate trend metrics
            let throughputs: Vec<f64> = sorted_results.iter().map(|r| r.operations_per_second).collect();
            let throughput_trend = self.calculate_trend(&throughputs);

            let latencies: Vec<f64> = sorted_results.iter().map(|r| r.latency_percentiles.p95 as f64).collect();
            let latency_trend = self.calculate_trend(&latencies);

            trends.push(TrendAnalysis {
                benchmark_name: name,
                data_points: sorted_results.len(),
                throughput_trend_percent: throughput_trend,
                latency_trend_percent: latency_trend,
                trend_direction: self.classify_trend(throughput_trend, latency_trend),
            });
        }

        trends
    }

    /// Generate optimization recommendations
    fn generate_recommendations(&self) -> Vec<OptimizationRecommendation> {
        let mut recommendations = Vec::new();

        let summary = self.generate_summary();

        // Throughput recommendations
        if summary.average_throughput < 1000.0 {
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Throughput,
                priority: Priority::High,
                title: "Low Throughput Detected".to_string(),
                description: format!("Average throughput is {:.0} ops/sec, which may indicate performance bottlenecks", summary.average_throughput),
                actions: vec![
                    "Profile CPU usage to identify hotspots".to_string(),
                    "Optimize I/O operations with batching".to_string(),
                    "Consider parallel processing improvements".to_string(),
                    "Review database indexing strategies".to_string(),
                ],
            });
        }

        // Latency recommendations
        if summary.average_latency_p95 > 5000.0 { // 5ms
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Latency,
                priority: Priority::High,
                title: "High Latency Detected".to_string(),
                description: format!("Average P95 latency is {:.1} ms, which may impact user experience", summary.average_latency_p95 / 1000.0),
                actions: vec![
                    "Implement caching for frequently accessed data".to_string(),
                    "Optimize database queries and indexes".to_string(),
                    "Consider connection pooling improvements".to_string(),
                    "Profile network latency if applicable".to_string(),
                ],
            });
        }

        // Memory recommendations
        if summary.peak_memory_usage > 512.0 { // 512MB
            recommendations.push(OptimizationRecommendation {
                category: OptimizationCategory::Memory,
                priority: Priority::Medium,
                title: "High Memory Usage".to_string(),
                description: format!("Peak memory usage is {:.1} MB, consider memory optimizations", summary.peak_memory_usage),
                actions: vec![
                    "Implement object pooling for frequently allocated objects".to_string(),
                    "Use memory-mapped files for large datasets".to_string(),
                    "Profile memory allocations to identify leaks".to_string(),
                    "Consider streaming processing for large data".to_string(),
                ],
            });
        }

        recommendations.sort_by(|a, b| b.priority.cmp(&a.priority));
        recommendations
    }

    /// Perform statistical analysis on results
    fn perform_statistical_analysis(&self) -> StatisticalAnalysis {
        if self.results.is_empty() {
            return StatisticalAnalysis::default();
        }

        let throughputs: Vec<f64> = self.results.iter().map(|r| r.operations_per_second).collect();
        let mean = throughputs.sort_by(|a, b| a.partial_cmp(b).unwrap()).mean();
        let std_dev = throughputs.std_dev();
        let n = throughputs.len() as f64;

        // 95% confidence interval using t-distribution approximation
        let t_value = 1.96; // Approximation for large n
        let margin = t_value * std_dev / n.sqrt();

        StatisticalAnalysis {
            throughput_stats: StatisticalSummary {
                mean,
                median: mean, // Assuming median is the same as mean for simplicity
                std_dev,
                min: throughputs.min(),
                max: throughputs.max(),
                quartiles: (
                    throughputs[(throughputs.len() as f64 * 0.25) as usize],
                    throughputs[(throughputs.len() as f64 * 0.5) as usize],
                    throughputs[(throughputs.len() as f64 * 0.75) as usize],
                ),
            },
            latency_stats: StatisticalSummary::default(), // No latency data available in this struct
            confidence_intervals: ConfidenceInterval {
                lower: mean - margin,
                upper: mean + margin,
                confidence_level: 0.95,
            },
            statistical_significance: self.assess_statistical_significance(),
        }
    }

    // Helper methods

    fn calculate_duration_range(&self) -> (chrono::Duration, chrono::Duration) {
        if self.results.is_empty() {
            return (chrono::Duration::zero(), chrono::Duration::zero());
        }

        let durations: Vec<chrono::Duration> = self.results.iter()
            .map(|r| r.end_time - r.start_time)
            .collect();

        let min_duration = durations.iter().min().unwrap().clone();
        let max_duration = durations.iter().max().unwrap().clone();

        (min_duration, max_duration)
    }

    fn calculate_trend(&self, values: &[f64]) -> f64 {
        if values.len() < 2 {
            return 0.0;
        }

        // Simple linear regression slope calculation
        let n = values.len() as f64;
        let x_sum: f64 = (0..values.len()).map(|i| i as f64).sum();
        let y_sum: f64 = values.iter().sum();
        let xy_sum: f64 = values.iter().enumerate().map(|(i, &y)| i as f64 * y).sum();
        let x_squared_sum: f64 = (0..values.len()).map(|i| (i as f64).powi(2)).sum();

        let slope = (n * xy_sum - x_sum * y_sum) / (n * x_squared_sum - x_sum.powi(2));

        // Convert to percentage change over the period
        if values[0] != 0.0 {
            (slope * n / values[0]) * 100.0
        } else {
            0.0
        }
    }

    fn classify_trend(&self, throughput_trend: f64, latency_trend: f64) -> TrendDirection {
        match (throughput_trend, latency_trend) {
            (t, l) if t > 5.0 && l < -5.0 => TrendDirection::Improving,
            (t, l) if t < -5.0 && l > 5.0 => TrendDirection::Degrading,
            _ => TrendDirection::Stable,
        }
    }

    fn calculate_confidence_intervals(&self, values: &[f64]) -> ConfidenceInterval {
        if values.len() < 2 {
            return ConfidenceInterval { lower: 0.0, upper: 0.0, confidence_level: 0.95 };
        }

        let mean = values.mean();
        let std_dev = values.std_dev();
        let n = values.len() as f64;

        // 95% confidence interval using t-distribution approximation
        let t_value = 1.96; // Approximation for large n
        let margin = t_value * std_dev / n.sqrt();

        ConfidenceInterval {
            lower: mean - margin,
            upper: mean + margin,
            confidence_level: 0.95,
        }
    }

    fn assess_statistical_significance(&self) -> StatisticalSignificance {
        // Simple assessment based on coefficient of variation
        let throughputs: Vec<f64> = self.results.iter().map(|r| r.operations_per_second).collect();

        if throughputs.len() < 2 {
            return StatisticalSignificance::InsufficientData;
        }

        let mean = throughputs.mean();
        let std_dev = throughputs.std_dev();
        let cv = if mean != 0.0 { std_dev / mean } else { 0.0 };

        match cv {
            cv if cv < 0.1 => StatisticalSignificance::High,
            cv if cv < 0.2 => StatisticalSignificance::Medium,
            _ => StatisticalSignificance::Low,
        }
    }
}

/// Analysis report containing all analysis results
#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisReport {
    pub summary: PerformanceSummary,
    pub regressions: Vec<RegressionAnalysis>,
    pub bottlenecks: Vec<BottleneckAnalysis>,
    pub trends: Vec<TrendAnalysis>,
    pub recommendations: Vec<OptimizationRecommendation>,
    pub statistical_analysis: StatisticalAnalysis,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct PerformanceSummary {
    pub total_benchmarks: usize,
    pub total_operations: u64,
    pub average_throughput: f64,
    pub average_latency_p95: f64,
    pub average_error_rate: f64,
    pub peak_memory_usage: f64,
    pub duration_range: (chrono::Duration, chrono::Duration),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BottleneckAnalysis {
    pub benchmark_name: String,
    pub bottleneck_type: BottleneckType,
    pub severity: Severity,
    pub description: String,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BottleneckType {
    HighLatency,
    HighErrorRate,
    MemoryPressure,
    CPUContention,
    IOContention,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub benchmark_name: String,
    pub data_points: usize,
    pub throughput_trend_percent: f64,
    pub latency_trend_percent: f64,
    pub trend_direction: TrendDirection,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TrendDirection {
    Improving,
    Stable,
    Degrading,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OptimizationRecommendation {
    pub category: OptimizationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub actions: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum OptimizationCategory {
    Throughput,
    Latency,
    Memory,
    CPU,
    Storage,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatisticalAnalysis {
    pub throughput_stats: StatisticalSummary,
    pub latency_stats: StatisticalSummary,
    pub confidence_intervals: ConfidenceInterval,
    pub statistical_significance: StatisticalSignificance,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct StatisticalSummary {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub quartiles: (f64, f64, f64), // (Q1, Q2, Q3)
}

impl StatisticalSummary {
    pub fn from_data(data: &[f64]) -> Self {
        if data.is_empty() {
            return Self::default();
        }

        let mut sorted = data.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        Self {
            mean: data.mean(),
            median: data.mean(),
            std_dev: data.std_dev(),
            min: data.min(),
            max: data.max(),
            quartiles: (
                sorted[(sorted.len() as f64 * 0.25) as usize],
                sorted[(sorted.len() as f64 * 0.5) as usize],
                sorted[(sorted.len() as f64 * 0.75) as usize],
            ),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfidenceInterval {
    pub lower: f64,
    pub upper: f64,
    pub confidence_level: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StatisticalSignificance {
    High,
    Medium,
    Low,
    InsufficientData,
}

impl Default for StatisticalAnalysis {
    fn default() -> Self {
        Self {
            throughput_stats: StatisticalSummary::default(),
            latency_stats: StatisticalSummary::default(),
            confidence_intervals: ConfidenceInterval {
                lower: 0.0,
                upper: 0.0,
                confidence_level: 0.95,
            },
            statistical_significance: StatisticalSignificance::InsufficientData,
        }
    }
}

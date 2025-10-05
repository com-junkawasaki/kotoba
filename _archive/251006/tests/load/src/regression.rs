//! Performance Regression Testing
//!
//! This module provides performance regression testing capabilities,
//! allowing comparison of current performance against baseline measurements
//! and detection of performance degradation over time.

use crate::runner::KotobaDBRunner;
use crate::workload::{WorkloadGenerator, YCSBWorkload};
use crate::{LoadTestConfig, LoadTestResult, run_load_test};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Performance baseline data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceBaseline {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub version: String,
    pub hardware_info: HardwareInfo,
    pub test_results: HashMap<String, LoadTestResult>,
    pub system_metrics: SystemMetrics,
}

/// Hardware information for baseline comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub cpu_model: String,
    pub cpu_cores: usize,
    pub memory_gb: usize,
    pub storage_type: String,
    pub os_info: String,
}

/// System metrics captured during baseline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub disk_usage_percent: f64,
    pub load_average: f64,
}

/// Regression analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysis {
    pub baseline_timestamp: chrono::DateTime<chrono::Utc>,
    pub current_timestamp: chrono::DateTime<chrono::Utc>,
    pub comparisons: HashMap<String, TestComparison>,
    pub overall_assessment: RegressionAssessment,
    pub recommendations: Vec<String>,
}

/// Comparison between baseline and current test results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestComparison {
    pub test_name: String,
    pub baseline_result: LoadTestResult,
    pub current_result: LoadTestResult,
    pub throughput_change_percent: f64,
    pub latency_change_p95_percent: f64,
    pub error_rate_change: f64,
    pub significance: RegressionSignificance,
}

/// Overall regression assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionAssessment {
    Improved,
    Stable,
    MinorRegression,
    MajorRegression,
    CriticalRegression,
}

/// Regression significance level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RegressionSignificance {
    None,
    Minor,
    Moderate,
    Major,
    Critical,
}

/// Capture performance baseline
pub async fn baseline(runner: KotobaDBRunner) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Capturing Performance Baseline");

    let mut test_results = HashMap::new();

    // Run standard benchmark suite for baseline
    let benchmarks = vec![
        ("ycsb_a", YCSBWorkload::workload_a()),
        ("ycsb_b", YCSBWorkload::workload_b()),
        ("ycsb_c", YCSBWorkload::workload_c()),
    ];

    for (name, workload) in benchmarks {
        println!("Running {} for baseline...", name);

        let config = LoadTestConfig {
            duration: Duration::from_secs(60), // 1 minute per test
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let result = run_load_test(runner.clone(), Box::new(workload), config).await?;
        test_results.insert(name.to_string(), result);
    }

    // Capture system information
    let hardware_info = capture_hardware_info();
    let system_metrics = capture_system_metrics();

    let baseline = PerformanceBaseline {
        timestamp: chrono::Utc::now(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        hardware_info,
        test_results,
        system_metrics,
    };

    // Save baseline to file
    let baseline_path = Path::new("performance_baseline.json");
    let baseline_json = serde_json::to_string_pretty(&baseline)?;
    fs::write(baseline_path, baseline_json)?;

    println!("✅ Baseline captured and saved to: {}", baseline_path.display());
    println!("   Timestamp: {}", baseline.timestamp);
    println!("   Version: {}", baseline.version);

    Ok(())
}

/// Compare current performance against baseline
pub async fn compare_baseline(runner: KotobaDBRunner) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Comparing Performance Against Baseline");

    // Load baseline
    let baseline_path = Path::new("performance_baseline.json");
    if !baseline_path.exists() {
        eprintln!("❌ Baseline file not found: {}", baseline_path.display());
        eprintln!("   Run 'baseline' command first to establish performance baseline");
        std::process::exit(1);
    }

    let baseline_data = fs::read_to_string(baseline_path)?;
    let baseline: PerformanceBaseline = serde_json::from_str(&baseline_data)?;

    println!("Loaded baseline from: {}", baseline.timestamp);
    println!("Baseline version: {}", baseline.version);

    // Run current performance tests
    let mut current_results = HashMap::new();

    let benchmarks = vec![
        ("ycsb_a", YCSBWorkload::workload_a()),
        ("ycsb_b", YCSBWorkload::workload_b()),
        ("ycsb_c", YCSBWorkload::workload_c()),
    ];

    for (name, workload) in benchmarks {
        println!("Running {} for comparison...", name);

        let config = LoadTestConfig {
            duration: Duration::from_secs(60),
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let result = run_load_test(runner.clone(), Box::new(workload), config).await?;
        current_results.insert(name.to_string(), result);
    }

    // Analyze regression
    let analysis = analyze_regression(&baseline, &current_results);

    // Display results
    display_regression_analysis(&analysis);

    // Save detailed analysis
    let analysis_path = Path::new("regression_analysis.json");
    let analysis_json = serde_json::to_string_pretty(&analysis)?;
    fs::write(analysis_path, analysis_json)?;

    println!("📄 Detailed analysis saved to: {}", analysis_path.display());

    Ok(())
}

/// Analyze regression between baseline and current results
fn analyze_regression(baseline: &PerformanceBaseline, current_results: &HashMap<String, LoadTestResult>) -> RegressionAnalysis {
    let mut comparisons = HashMap::new();
    let mut total_throughput_change = 0.0;
    let mut regression_count = 0;

    for (test_name, baseline_result) in &baseline.test_results {
        if let Some(current_result) = current_results.get(test_name) {
            let throughput_change = ((current_result.operations_per_second - baseline_result.operations_per_second)
                                   / baseline_result.operations_per_second) * 100.0;

            let latency_change = if baseline_result.latency_percentiles.p95 > 0 {
                ((current_result.latency_percentiles.p95 as f64 - baseline_result.latency_percentiles.p95 as f64)
                / baseline_result.latency_percentiles.p95 as f64) * 100.0
            } else {
                0.0
            };

            let error_rate_change = current_result.error_rate - baseline_result.error_rate;

            // Determine significance
            let significance = determine_significance(throughput_change, latency_change, error_rate_change);

            if matches!(significance, RegressionSignificance::Moderate | RegressionSignificance::Major | RegressionSignificance::Critical) {
                regression_count += 1;
            }

            total_throughput_change += throughput_change;

            comparisons.insert(test_name.clone(), TestComparison {
                test_name: test_name.clone(),
                baseline_result: baseline_result.clone(),
                current_result: current_result.clone(),
                throughput_change_percent: throughput_change,
                latency_change_p95_percent: latency_change,
                error_rate_change,
                significance,
            });
        }
    }

    let avg_throughput_change = total_throughput_change / comparisons.len() as f64;

    // Overall assessment
    let overall_assessment = if avg_throughput_change > 5.0 {
        RegressionAssessment::Improved
    } else if avg_throughput_change > -2.0 && regression_count == 0 {
        RegressionAssessment::Stable
    } else if avg_throughput_change > -10.0 && regression_count <= 1 {
        RegressionAssessment::MinorRegression
    } else if avg_throughput_change > -25.0 || regression_count <= 2 {
        RegressionAssessment::MajorRegression
    } else {
        RegressionAssessment::CriticalRegression
    };

    // Generate recommendations
    let recommendations = generate_regression_recommendations(&overall_assessment, &comparisons);

    RegressionAnalysis {
        baseline_timestamp: baseline.timestamp,
        current_timestamp: chrono::Utc::now(),
        comparisons,
        overall_assessment,
        recommendations,
    }
}

/// Determine the significance of a regression
fn determine_significance(throughput_change: f64, latency_change: f64, error_rate_change: f64) -> RegressionSignificance {
    let throughput_severity = if throughput_change < -25.0 { 3 } else if throughput_change < -10.0 { 2 } else if throughput_change < -5.0 { 1 } else { 0 };
    let latency_severity = if latency_change > 50.0 { 3 } else if latency_change > 25.0 { 2 } else if latency_change > 10.0 { 1 } else { 0 };
    let error_severity = if error_rate_change > 0.1 { 3 } else if error_rate_change > 0.05 { 2 } else if error_rate_change > 0.01 { 1 } else { 0 };

    let max_severity = throughput_severity.max(latency_severity).max(error_severity);

    match max_severity {
        0 => RegressionSignificance::None,
        1 => RegressionSignificance::Minor,
        2 => RegressionSignificance::Moderate,
        3 => RegressionSignificance::Major,
        _ => RegressionSignificance::Critical,
    }
}

/// Generate recommendations based on regression analysis
fn generate_regression_recommendations(
    assessment: &RegressionAssessment,
    comparisons: &HashMap<String, TestComparison>
) -> Vec<String> {
    let mut recommendations = Vec::new();

    match assessment {
        RegressionAssessment::Improved => {
            recommendations.push("🎉 Performance has improved! Consider documenting the optimizations that led to this improvement.".to_string());
        }
        RegressionAssessment::Stable => {
            recommendations.push("✅ Performance is stable. No action required.".to_string());
        }
        RegressionAssessment::MinorRegression => {
            recommendations.push("⚠️ Minor performance regression detected. Monitor closely and investigate if it worsens.".to_string());
        }
        RegressionAssessment::MajorRegression => {
            recommendations.push("🚨 Major performance regression detected. Immediate investigation required.".to_string());
            recommendations.push("   - Review recent code changes for performance impacts".to_string());
            recommendations.push("   - Check system resource utilization".to_string());
            recommendations.push("   - Profile application to identify bottlenecks".to_string());
        }
        RegressionAssessment::CriticalRegression => {
            recommendations.push("🚨🚨 CRITICAL performance regression detected! Emergency response required.".to_string());
            recommendations.push("   - Halt deployment until regression is resolved".to_string());
            recommendations.push("   - Roll back recent changes if possible".to_string());
            recommendations.push("   - Engage performance engineering team immediately".to_string());
        }
    }

    // Add specific recommendations for significant regressions
    for comparison in comparisons.values() {
        if matches!(comparison.significance, RegressionSignificance::Major | RegressionSignificance::Critical) {
            recommendations.push(format!("   - Investigate {}: {:.1}% throughput change, {:.1}% latency change",
                comparison.test_name, comparison.throughput_change_percent, comparison.latency_change_p95_percent));
        }
    }

    recommendations
}

/// Display regression analysis results
fn display_regression_analysis(analysis: &RegressionAnalysis) {
    println!("\n📊 Performance Regression Analysis");
    println!("==================================");
    println!("Baseline: {}", analysis.baseline_timestamp);
    println!("Current:  {}", analysis.current_timestamp);
    println!("Overall Assessment: {:?}", analysis.overall_assessment);
    println!();

    for comparison in analysis.comparisons.values() {
        println!("Test: {}", comparison.test_name);
        println!("  Throughput: {:.1} ops/sec → {:.1} ops/sec ({:+.1}%)",
                 comparison.baseline_result.operations_per_second,
                 comparison.current_result.operations_per_second,
                 comparison.throughput_change_percent);
        println!("  P95 Latency: {}μs → {}μs ({:+.1}%)",
                 comparison.baseline_result.latency_percentiles.p95,
                 comparison.current_result.latency_percentiles.p95,
                 comparison.latency_change_p95_percent);
        println!("  Error Rate: {:.3}% → {:.3}% ({:+.3}%)",
                 comparison.baseline_result.error_rate * 100.0,
                 comparison.current_result.error_rate * 100.0,
                 comparison.error_rate_change * 100.0);
        println!("  Significance: {:?}", comparison.significance);
        println!();
    }

    println!("Recommendations:");
    for rec in &analysis.recommendations {
        println!("  {}", rec);
    }
}

/// Capture hardware information
fn capture_hardware_info() -> HardwareInfo {
    // In a real implementation, you would use system APIs to get this information
    // For now, we'll return placeholder data
    HardwareInfo {
        cpu_model: "Unknown CPU".to_string(),
        cpu_cores: num_cpus::get(),
        memory_gb: 8, // Placeholder
        storage_type: "SSD".to_string(),
        os_info: std::env::consts::OS.to_string(),
    }
}

/// Capture system metrics
fn capture_system_metrics() -> SystemMetrics {
    // In a real implementation, you would capture actual system metrics
    // For now, we'll return placeholder data
    SystemMetrics {
        cpu_usage_percent: 0.0,
        memory_usage_percent: 0.0,
        disk_usage_percent: 0.0,
        load_average: 0.0,
    }
}

/// Trend analysis for performance over time
pub async fn analyze_trends(baseline_path: &Path) -> Result<TrendAnalysis, Box<dyn std::error::Error>> {
    println!("📈 Analyzing Performance Trends");

    if !baseline_path.exists() {
        return Err("Baseline file not found".into());
    }

    // Load historical baselines (in a real implementation, you might store multiple baselines)
    let baseline_data = fs::read_to_string(baseline_path)?;
    let baseline: PerformanceBaseline = serde_json::from_str(&baseline_data)?;

    // For trend analysis, we'd need multiple data points
    // This is a simplified implementation
    let trend = TrendAnalysis {
        time_range: TimeRange {
            start: baseline.timestamp - chrono::Duration::days(30),
            end: chrono::Utc::now(),
        },
        throughput_trend: PerformanceTrend::Stable,
        latency_trend: PerformanceTrend::Stable,
        error_rate_trend: PerformanceTrend::Stable,
        significant_changes: vec![],
    };

    println!("Trend analysis completed (simplified)");
    Ok(trend)
}

/// Performance trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    pub time_range: TimeRange,
    pub throughput_trend: PerformanceTrend,
    pub latency_trend: PerformanceTrend,
    pub error_rate_trend: PerformanceTrend,
    pub significant_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PerformanceTrend {
    Improving,
    Stable,
    Degrading,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_regression_significance() {
        // Test minor regression
        let sig = determine_significance(-3.0, 5.0, 0.005);
        assert!(matches!(sig, RegressionSignificance::Minor));

        // Test major regression
        let sig = determine_significance(-15.0, 30.0, 0.02);
        assert!(matches!(sig, RegressionSignificance::Major));

        // Test no regression
        let sig = determine_significance(1.0, -2.0, 0.0);
        assert!(matches!(sig, RegressionSignificance::None));
    }

    #[test]
    fn test_hardware_info_capture() {
        let info = capture_hardware_info();
        assert!(info.cpu_cores > 0);
        assert!(!info.os_info.is_empty());
    }

    #[tokio::test]
    async fn test_trend_analysis() {
        // Create a temporary baseline file for testing
        let temp_dir = tempfile::tempdir().unwrap();
        let baseline_path = temp_dir.path().join("baseline.json");

        let baseline = PerformanceBaseline {
            timestamp: chrono::Utc::now(),
            version: "1.0.0".to_string(),
            hardware_info: capture_hardware_info(),
            test_results: HashMap::new(),
            system_metrics: capture_system_metrics(),
        };

        let baseline_json = serde_json::to_string(&baseline).unwrap();
        fs::write(&baseline_path, baseline_json).unwrap();

        // Test trend analysis
        let trend = analyze_trends(&baseline_path).await.unwrap();
        assert!(matches!(trend.throughput_trend, PerformanceTrend::Stable));
    }
}

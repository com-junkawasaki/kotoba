//! Metrics Reporter
//!
//! Comprehensive reporting capabilities including:
//! - Console output with colored formatting
//! - JSON/CSV export for analysis
//! - HTML reports with charts
//! - Custom report templates

use crate::BenchmarkResult;
use crate::analyzer::{AnalysisReport, PerformanceSummary, Severity, Priority};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Metrics reporter for generating comprehensive benchmark reports
pub struct MetricsReporter {
    output_dir: String,
    include_charts: bool,
}

impl MetricsReporter {
    pub fn new(output_dir: &str) -> Self {
        fs::create_dir_all(output_dir).unwrap_or_else(|e| {
            eprintln!("Warning: Could not create output directory {}: {}", output_dir, e);
        });

        Self {
            output_dir: output_dir.to_string(),
            include_charts: true,
        }
    }

    pub fn with_charts(mut self, include: bool) -> Self {
        self.include_charts = include;
        self
    }

    /// Generate all reports for benchmark results
    pub fn generate_reports(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        self.generate_console_report(results)?;
        self.generate_json_report(results)?;
        self.generate_csv_report(results)?;
        self.generate_html_report(results)?;
        self.generate_summary_report(results)?;
        Ok(())
    }

    /// Generate analysis reports
    pub fn generate_analysis_reports(&self, analysis: &AnalysisReport) -> Result<(), Box<dyn std::error::Error>> {
        self.generate_analysis_console_report(analysis)?;
        self.generate_analysis_json_report(analysis)?;
        self.generate_analysis_html_report(analysis)?;
        Ok(())
    }

    /// Generate colored console report
    pub fn generate_console_report(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "=".repeat(80));
        println!("{}", "🚀 KotobaDB Benchmark Results".bold().cyan());
        println!("{}", "=".repeat(80));

        for (i, result) in results.iter().enumerate() {
            if results.len() > 1 {
                println!("\n{} Benchmark {}/{}", "📊".bold().yellow(), i + 1, results.len());
                println!("{}", "─".repeat(40));
            }

            // Performance metrics
            println!("\n{}", "Performance Metrics:".bold());
            println!("  🏷️  Name: {}", result.name);
            println!("  ⏱️  Duration: {:.2}s", (result.end_time - result.start_time).num_seconds() as f64 +
                     (result.end_time - result.start_time).num_nanoseconds().unwrap_or(0) as f64 / 1_000_000_000.0);
            println!("  📈 Operations: {}", result.total_operations);
            println!("  🚀 Throughput: {:.0} ops/sec", result.operations_per_second);

            // Latency percentiles
            println!("\n{}", "Latency Percentiles (μs):".bold());
            println!("  50th: {}", result.latency_percentiles.p50);
            println!("  95th: {}", result.latency_percentiles.p95);
            println!("  99th: {}", result.latency_percentiles.p99);
            println!("  99.9th: {}", result.latency_percentiles.p999);
            println!("  Max: {}", result.latency_percentiles.max);

            // Error analysis
            let error_rate_percent = result.error_rate * 100.0;
            let error_color = if error_rate_percent > 5.0 { "red" } else if error_rate_percent > 1.0 { "yellow" } else { "green" };
            println!("\n{}", "Error Analysis:".bold());
            println!("  ❌ Error Rate: {:.3}%", error_rate_percent);
            println!("  📊 Error Count: {}", result.error_count);

            // Resource usage
            if let Some(mem_stats) = &result.memory_stats {
                println!("\n{}", "Memory Usage:".bold());
                println!("  🧠 Peak: {:.1} MB", mem_stats.peak_memory_mb);
                println!("  📊 Average: {:.1} MB", mem_stats.average_memory_mb);
                println!("  ⚡ Efficiency: {:.1} ops/MB", mem_stats.memory_efficiency);
            }

            if let Some(storage_stats) = &result.storage_stats {
                println!("\n{}", "Storage I/O:".bold());
                println!("  💾 Read: {:.1} MB", storage_stats.total_bytes_read as f64 / (1024.0 * 1024.0));
                println!("  ✍️  Written: {:.1} MB", storage_stats.total_bytes_written as f64 / (1024.0 * 1024.0));
                println!("  📈 Efficiency: {:.1} ops/byte", storage_stats.storage_efficiency);
                println!("  ⚡ IOPS: {:.0}", storage_stats.iops);
            }

            // Performance assessment
            self.print_performance_assessment(result);
        }

        println!("\n{}", "=".repeat(80));
        Ok(())
    }

    /// Generate analysis console report
    pub fn generate_analysis_console_report(&self, analysis: &AnalysisReport) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "=".repeat(80));
        println!("{}", "🔍 KotobaDB Performance Analysis".bold().cyan());
        println!("{}", "=".repeat(80));

        // Summary
        let summary = &analysis.summary;
        println!("\n{}", "📈 Performance Summary:".bold());
        println!("  📊 Benchmarks: {}", summary.total_benchmarks);
        println!("  📈 Total Operations: {}", summary.total_operations);
        println!("  🚀 Avg Throughput: {:.0} ops/sec", summary.average_throughput);
        println!("  ⏱️  Avg Latency p95: {:.1} ms", summary.average_latency_p95 / 1000.0);
        println!("  ❌ Avg Error Rate: {:.2}%", summary.average_error_rate * 100.0);
        println!("  🧠 Peak Memory: {:.1} MB", summary.peak_memory_usage);

        // Regressions
        if !analysis.regressions.is_empty() {
            println!("\n{}", "⚠️  Performance Regressions:".bold().red());
            for regression in &analysis.regressions {
                println!("  📉 {}: Throughput {:.1}%, Latency {:.1}% ({})",
                    regression.benchmark_name,
                    regression.throughput_change_percent,
                    regression.latency_change_p95_percent,
                    match regression.significance {
                        crate::RegressionSignificance::High => "High",
                        crate::RegressionSignificance::Medium => "Medium",
                        crate::RegressionSignificance::Low => "Low",
                    });
            }
        }

        // Bottlenecks
        if !analysis.bottlenecks.is_empty() {
            println!("\n{}", "🚧 Performance Bottlenecks:".bold().yellow());
            for bottleneck in &analysis.bottlenecks {
                let severity = match bottleneck.severity {
                    Severity::Critical => "Critical",
                    Severity::High => "High",
                    Severity::Medium => "Medium",
                    Severity::Low => "Low",
                };
                println!("  🔍 {} ({}): {}", bottleneck.benchmark_name, severity, bottleneck.description);
            }
        }

        // Recommendations
        if !analysis.recommendations.is_empty() {
            println!("\n{}", "💡 Optimization Recommendations:".bold().green());
            for rec in &analysis.recommendations {
                let priority = match rec.priority {
                    Priority::Critical => "Critical",
                    Priority::High => "High",
                    Priority::Medium => "Medium",
                    Priority::Low => "Low",
                };
                println!("  🎯 {} ({}): {}", rec.title, priority, rec.description);
            }
        }

        println!("\n{}", "=".repeat(80));
        Ok(())
    }

    /// Generate JSON report
    pub fn generate_json_report(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = Path::new(&self.output_dir).join("benchmark_results.json");

        let json_data = serde_json::json!({
            "report_generated": chrono::Utc::now().to_rfc3339(),
            "results": results.iter().map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "start_time": r.start_time.to_rfc3339(),
                    "end_time": r.end_time.to_rfc3339(),
                    "total_operations": r.total_operations,
                    "operations_per_second": r.operations_per_second,
                    "latency_percentiles_us": {
                        "p50": r.latency_percentiles.p50,
                        "p95": r.latency_percentiles.p95,
                        "p99": r.latency_percentiles.p99,
                        "p999": r.latency_percentiles.p999,
                        "max": r.latency_percentiles.max,
                    },
                    "error_count": r.error_count,
                    "error_rate": r.error_rate,
                    "memory_stats": r.memory_stats.as_ref().map(|m| {
                        serde_json::json!({
                            "peak_memory_mb": m.peak_memory_mb,
                            "average_memory_mb": m.average_memory_mb,
                            "memory_efficiency": m.memory_efficiency,
                        })
                    }),
                    "storage_stats": r.storage_stats.as_ref().map(|s| {
                        serde_json::json!({
                            "total_bytes_written": s.total_bytes_written,
                            "total_bytes_read": s.total_bytes_read,
                            "storage_efficiency": s.storage_efficiency,
                            "iops": s.iops,
                        })
                    }),
                    "custom_metrics": r.custom_metrics,
                })
            }).collect::<Vec<_>>()
        });

        fs::write(&json_path, serde_json::to_string_pretty(&json_data)?)?;
        println!("📄 JSON report saved to: {}", json_path.display());
        Ok(())
    }

    /// Generate analysis JSON report
    pub fn generate_analysis_json_report(&self, analysis: &AnalysisReport) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = Path::new(&self.output_dir).join("performance_analysis.json");

        let json_data = serde_json::json!({
            "analysis_generated": chrono::Utc::now().to_rfc3339(),
            "summary": {
                "total_benchmarks": analysis.summary.total_benchmarks,
                "total_operations": analysis.summary.total_operations,
                "average_throughput": analysis.summary.average_throughput,
                "average_latency_p95": analysis.summary.average_latency_p95,
                "average_error_rate": analysis.summary.average_error_rate,
                "peak_memory_usage": analysis.summary.peak_memory_usage,
            },
            "regressions": analysis.regressions.iter().map(|r| {
                serde_json::json!({
                    "benchmark_name": r.benchmark_name,
                    "throughput_change_percent": r.throughput_change_percent,
                    "latency_change_p95_percent": r.latency_change_p95_percent,
                    "error_rate_change_percent": r.error_rate_change_percent,
                    "has_regression": r.has_regression,
                    "significance": format!("{:?}", r.significance),
                })
            }).collect::<Vec<_>>(),
            "bottlenecks": analysis.bottlenecks.iter().map(|b| {
                serde_json::json!({
                    "benchmark_name": b.benchmark_name,
                    "bottleneck_type": format!("{:?}", b.bottleneck_type),
                    "severity": format!("{:?}", b.severity),
                    "description": b.description,
                    "recommendations": b.recommendations,
                })
            }).collect::<Vec<_>>(),
            "recommendations": analysis.recommendations.iter().map(|r| {
                serde_json::json!({
                    "category": format!("{:?}", r.category),
                    "priority": format!("{:?}", r.priority),
                    "title": r.title,
                    "description": r.description,
                    "actions": r.actions,
                })
            }).collect::<Vec<_>>(),
        });

        fs::write(&json_path, serde_json::to_string_pretty(&json_data)?)?;
        println!("📄 Analysis JSON report saved to: {}", json_path.display());
        Ok(())
    }

    /// Generate CSV report
    pub fn generate_csv_report(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = Path::new(&self.output_dir).join("benchmark_results.csv");

        let mut csv_writer = csv::Writer::from_path(&csv_path)?;

        // Write header
        csv_writer.write_record([
            "benchmark_name", "start_time", "end_time", "duration_seconds",
            "total_operations", "operations_per_second", "latency_p50_us",
            "latency_p95_us", "latency_p99_us", "latency_max_us",
            "error_count", "error_rate", "peak_memory_mb", "avg_memory_mb",
            "memory_efficiency", "bytes_written", "bytes_read", "storage_efficiency", "iops"
        ])?;

        // Write data
        for result in results {
            let duration = (result.end_time - result.start_time).num_seconds() as f64 +
                          (result.end_time - result.start_time).num_nanoseconds().unwrap_or(0) as f64 / 1_000_000_000.0;

            csv_writer.write_record([
                &result.name,
                &result.start_time.to_rfc3339(),
                &result.end_time.to_rfc3339(),
                &duration.to_string(),
                &result.total_operations.to_string(),
                &result.operations_per_second.to_string(),
                &result.latency_percentiles.p50.to_string(),
                &result.latency_percentiles.p95.to_string(),
                &result.latency_percentiles.p99.to_string(),
                &result.latency_percentiles.max.to_string(),
                &result.error_count.to_string(),
                &result.error_rate.to_string(),
                &result.memory_stats.as_ref().map(|m| m.peak_memory_mb.to_string()).unwrap_or_default(),
                &result.memory_stats.as_ref().map(|m| m.average_memory_mb.to_string()).unwrap_or_default(),
                &result.memory_stats.as_ref().map(|m| m.memory_efficiency.to_string()).unwrap_or_default(),
                &result.storage_stats.as_ref().map(|s| s.total_bytes_written.to_string()).unwrap_or_default(),
                &result.storage_stats.as_ref().map(|s| s.total_bytes_read.to_string()).unwrap_or_default(),
                &result.storage_stats.as_ref().map(|s| s.storage_efficiency.to_string()).unwrap_or_default(),
                &result.storage_stats.as_ref().map(|s| s.iops.to_string()).unwrap_or_default(),
            ])?;
        }

        csv_writer.flush()?;
        println!("📊 CSV report saved to: {}", csv_path.display());
        Ok(())
    }

    /// Generate HTML report
    pub fn generate_html_report(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        let html_path = Path::new(&self.output_dir).join("benchmark_report.html");

        let html_content = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>KotobaDB Benchmark Report</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            padding: 30px;
        }}
        h1 {{
            color: #2c3e50;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }}
        .benchmark-card {{
            border: 1px solid #ddd;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            background: #fafafa;
        }}
        .metric-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
            gap: 15px;
            margin: 15px 0;
        }}
        .metric {{
            background: white;
            padding: 15px;
            border-radius: 6px;
            border-left: 4px solid #3498db;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }}
        .metric-label {{
            font-size: 0.9em;
            color: #7f8c8d;
            margin-bottom: 5px;
        }}
        .metric-value {{
            font-size: 1.5em;
            font-weight: bold;
            color: #2c3e50;
        }}
        .chart-container {{
            margin: 20px 0;
            height: 300px;
            position: relative;
        }}
        .status-good {{ border-left-color: #27ae60; }}
        .status-warning {{ border-left-color: #f39c12; }}
        .status-danger {{ border-left-color: #e74c3c; }}
        .timestamp {{
            color: #7f8c8d;
            font-size: 0.9em;
        }}
        .summary-stats {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }}
        .summary-stat {{
            text-align: center;
            padding: 20px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            border-radius: 8px;
        }}
        .summary-stat .value {{
            font-size: 2em;
            font-weight: bold;
            margin-bottom: 5px;
        }}
        .summary-stat .label {{
            font-size: 0.9em;
            opacity: 0.9;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 KotobaDB Benchmark Report</h1>
        <p class="timestamp">Generated on: {}</p>

        {}

        <div class="summary-stats">
            <div class="summary-stat">
                <div class="value">{:.0}</div>
                <div class="label">Avg Throughput (ops/sec)</div>
            </div>
            <div class="summary-stat">
                <div class="value">{:.1}</div>
                <div class="label">Avg Latency p95 (ms)</div>
            </div>
            <div class="summary-stat">
                <div class="summary-stat">
                <div class="value">{:.2}%</div>
                <div class="label">Avg Error Rate</div>
            </div>
            <div class="summary-stat">
                <div class="value">{}</div>
                <div class="label">Total Benchmarks</div>
            </div>
        </div>

        <div class="charts">
            <h2>📊 Performance Charts</h2>
            <div class="chart-container">
                <canvas id="throughputChart"></canvas>
            </div>
            <div class="chart-container">
                <canvas id="latencyChart"></canvas>
            </div>
        </div>
    </div>

    <script>
        // Chart data preparation
        const benchmarkNames = {};
        const throughputData = {};
        const latencyData = {};
        const benchmarkLabels = {};
        const latencyLabels = {};

        // Initialize charts when page loads
        document.addEventListener('DOMContentLoaded', function() {{
            const throughputCtx = document.getElementById('throughputChart').getContext('2d');
            const latencyCtx = document.getElementById('latencyChart').getContext('2d');

            new Chart(throughputCtx, {{
                type: 'bar',
                data: {{
                    labels: {},
                    datasets: [{{
                        label: 'Throughput (ops/sec)',
                        data: {},
                        backgroundColor: 'rgba(52, 152, 219, 0.6)',
                        borderColor: 'rgba(52, 152, 219, 1)',
                        borderWidth: 1
                    }}]
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false,
                    scales: {{
                        y: {{
                            beginAtZero: true
                        }}
                    }}
                }}
            }});

            new Chart(latencyCtx, {{
                type: 'line',
                data: {{
                    labels: {},
                    datasets: [{{
                        label: 'Latency p95 (μs)',
                        data: {},
                        backgroundColor: 'rgba(155, 89, 182, 0.1)',
                        borderColor: 'rgba(155, 89, 182, 1)',
                        borderWidth: 2,
                        fill: true
                    }}]
                }},
                options: {{
                    responsive: true,
                    maintainAspectRatio: false
                }}
            }});
        }});
    </script>
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.generate_html_benchmark_cards(results),
            results.iter().map(|r| r.operations_per_second).sum::<f64>() / results.len() as f64,
            results.iter().map(|r| r.latency_percentiles.p95 as f64 / 1000.0).sum::<f64>() / results.len() as f64,
            results.iter().map(|r| r.error_rate * 100.0).sum::<f64>() / results.len() as f64,
            results.len(),
            serde_json::to_string(&results.iter().map(|r| r.name.clone()).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.operations_per_second).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.name.clone()).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.latency_percentiles.p95).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.name.clone()).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.operations_per_second).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.name.clone()).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.latency_percentiles.p95).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
            serde_json::to_string(&results.iter().map(|r| r.operations_per_second).collect::<Vec<_>>()).unwrap_or_else(|_| "[]".to_string()),
        );

        fs::write(&html_path, html_content)?;
        println!("🌐 HTML report saved to: {}", html_path.display());
        Ok(())
    }

    /// Generate analysis HTML report
    pub fn generate_analysis_html_report(&self, analysis: &AnalysisReport) -> Result<(), Box<dyn std::error::Error>> {
        let html_path = Path::new(&self.output_dir).join("performance_analysis.html");

        let html_content = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>KotobaDB Performance Analysis</title>
    <style>
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            border-radius: 8px;
            box-shadow: 0 2px 10px rgba(0,0,0,0.1);
            padding: 30px;
        }}
        h1 {{
            color: #2c3e50;
            border-bottom: 3px solid #3498db;
            padding-bottom: 10px;
        }}
        .summary-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }}
        .summary-card {{
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 20px;
            border-radius: 8px;
            text-align: center;
        }}
        .summary-card .value {{
            font-size: 2em;
            font-weight: bold;
            margin-bottom: 5px;
        }}
        .summary-card .label {{
            font-size: 0.9em;
            opacity: 0.9;
        }}
        .section {{
            margin: 30px 0;
        }}
        .section h2 {{
            color: #2c3e50;
            border-bottom: 2px solid #bdc3c7;
            padding-bottom: 5px;
        }}
        .alert {{
            padding: 15px;
            border-radius: 6px;
            margin: 10px 0;
        }}
        .alert-regression {{
            background-color: #fee;
            border-left: 4px solid #e74c3c;
        }}
        .alert-bottleneck {{
            background-color: #fff3cd;
            border-left: 4px solid #f39c12;
        }}
        .alert-recommendation {{
            background-color: #d4edda;
            border-left: 4px solid #27ae60;
        }}
        .metric {{
            display: inline-block;
            background: #f8f9fa;
            padding: 10px 15px;
            margin: 5px;
            border-radius: 4px;
            font-family: monospace;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🔍 KotobaDB Performance Analysis</h1>
        <p>Generated on: {}</p>

        <div class="summary-grid">
            <div class="summary-card">
                <div class="value">{}</div>
                <div class="label">Benchmarks</div>
            </div>
            <div class="summary-card">
                <div class="value">{:.0}</div>
                <div class="label">Avg Throughput (ops/sec)</div>
            </div>
            <div class="summary-card">
                <div class="value">{:.1}</div>
                <div class="label">Avg Latency p95 (ms)</div>
            </div>
            <div class="summary-card">
                <div class="value">{:.2}%</div>
                <div class="label">Avg Error Rate</div>
            </div>
        </div>

        {}

        {}
    </div>
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            analysis.summary.total_benchmarks,
            analysis.summary.average_throughput,
            analysis.summary.average_latency_p95 / 1000.0,
            analysis.summary.average_error_rate * 100.0,
            self.generate_analysis_html_sections(analysis),
            self.generate_recommendations_html(analysis),
        );

        fs::write(&html_path, html_content)?;
        println!("🌐 Analysis HTML report saved to: {}", html_path.display());
        Ok(())
    }

    /// Generate summary report
    pub fn generate_summary_report(&self, results: &[BenchmarkResult]) -> Result<(), Box<dyn std::error::Error>> {
        let summary_path = Path::new(&self.output_dir).join("BENCHMARK_SUMMARY.md");

        let mut content = format!(
            "# KotobaDB Benchmark Summary Report\n\n"
        );

        content.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Overall statistics
        let total_ops: u64 = results.iter().map(|r| r.total_operations).sum();
        let avg_throughput = results.iter().map(|r| r.operations_per_second).sum::<f64>() / results.len() as f64;
        let avg_error_rate = results.iter().map(|r| r.error_rate * 100.0).sum::<f64>() / results.len() as f64;

        content.push_str("## Overall Statistics\n\n");
        content.push_str(&format!("- **Total Benchmarks:** {}\n", results.len()));
        content.push_str(&format!("- **Total Operations:** {}\n", total_ops));
        content.push_str(&format!("- **Average Throughput:** {:.0} ops/sec\n", avg_throughput));
        content.push_str(&format!("- **Average Error Rate:** {:.2}%\n\n", avg_error_rate));

        // Individual results
        content.push_str("## Benchmark Results\n\n");
        for result in results {
            content.push_str(&format!("### {}\n\n", result.name));
            content.push_str(&format!("- **Throughput:** {:.0} ops/sec\n", result.operations_per_second));
            content.push_str(&format!("- **Latency p95:** {} μs\n", result.latency_percentiles.p95));
            content.push_str(&format!("- **Error Rate:** {:.2}%\n", result.error_rate * 100.0));
            content.push_str(&format!("- **Total Operations:** {}\n\n", result.total_operations));
        }

        content.push_str("## Performance Assessment\n\n");

        let high_error_benchmarks: Vec<_> = results.iter()
            .filter(|r| r.error_rate > 0.05)
            .map(|r| r.name.as_str())
            .collect();

        if !high_error_benchmarks.is_empty() {
            content.push_str("### ⚠️ High Error Rate Benchmarks\n\n");
            for bench in high_error_benchmarks {
                content.push_str(&format!("- {}\n", bench));
            }
            content.push_str("\n");
        }

        let high_latency_benchmarks: Vec<_> = results.iter()
            .filter(|r| r.latency_percentiles.p95 > 10000)
            .map(|r| r.name.as_str())
            .collect();

        if !high_latency_benchmarks.is_empty() {
            content.push_str("### 🐌 High Latency Benchmarks\n\n");
            for bench in high_latency_benchmarks {
                content.push_str(&format!("- {}\n", bench));
            }
            content.push_str("\n");
        }

        fs::write(&summary_path, content)?;
        println!("📋 Summary report saved to: {}", summary_path.display());
        Ok(())
    }

    // Helper methods

    fn generate_html_benchmark_cards(&self, results: &[BenchmarkResult]) -> String {
        results.iter().enumerate().map(|(i, result)| {
            let throughput_class = if result.operations_per_second > 10000.0 { "status-good" }
                                 else if result.operations_per_second > 5000.0 { "status-warning" }
                                 else { "status-danger" };
            let error_class = if result.error_rate < 0.01 { "status-good" }
                            else if result.error_rate < 0.05 { "status-warning" }
                            else { "status-danger" };

            format!(
                r#"<div class="benchmark-card">
                    <h3>{} - {}</h3>
                    <div class="timestamp">🕒 {} to {}</div>
                    <div class="metric-grid">
                        <div class="metric {}">
                            <div class="metric-label">Throughput</div>
                            <div class="metric-value">{:.0} ops/sec</div>
                        </div>
                        <div class="metric">
                            <div class="metric-label">Latency p95</div>
                            <div class="metric-value">{} μs</div>
                        </div>
                        <div class="metric {}">
                            <div class="metric-label">Error Rate</div>
                            <div class="metric-value">{:.2}%</div>
                        </div>
                        <div class="metric">
                            <div class="metric-label">Total Operations</div>
                            <div class="metric-value">{}</div>
                        </div>
                    </div>
                </div>"#,
                i + 1, result.name,
                result.start_time.format("%H:%M:%S"),
                result.end_time.format("%H:%M:%S"),
                throughput_class, result.operations_per_second,
                result.latency_percentiles.p95,
                error_class, result.error_rate * 100.0,
                result.total_operations
            )
        }).collect::<String>()
    }

    fn generate_analysis_html_sections(&self, analysis: &AnalysisReport) -> String {
        let mut html = String::new();

        // Regressions
        if !analysis.regressions.is_empty() {
            html.push_str(r#"<div class="section">
                <h2>⚠️ Performance Regressions</h2>"#);

            for regression in &analysis.regressions {
                html.push_str(&format!(
                    r#"<div class="alert alert-regression">
                        <strong>{}</strong>: Throughput change: {:.1}%, Latency change: {:.1}%
                        <br><small>Significance: {}</small>
                    </div>"#,
                    regression.benchmark_name,
                    regression.throughput_change_percent,
                    regression.latency_change_p95_percent,
                    format!("{:?}", regression.significance)
                ));
            }
            html.push_str("</div>");
        }

        // Bottlenecks
        if !analysis.bottlenecks.is_empty() {
            html.push_str(r#"<div class="section">
                <h2>🚧 Performance Bottlenecks</h2>"#);

            for bottleneck in &analysis.bottlenecks {
                html.push_str(&format!(
                    r#"<div class="alert alert-bottleneck">
                        <strong>{} ({:?})</strong>: {}
                    </div>"#,
                    bottleneck.benchmark_name,
                    bottleneck.severity,
                    bottleneck.description
                ));
            }
            html.push_str("</div>");
        }

        html
    }

    fn generate_recommendations_html(&self, analysis: &AnalysisReport) -> String {
        if analysis.recommendations.is_empty() {
            return String::new();
        }

        let mut html = r#"<div class="section">
            <h2>💡 Optimization Recommendations</h2>"#.to_string();

        for rec in &analysis.recommendations {
            html.push_str(&format!(
                r#"<div class="alert alert-recommendation">
                    <strong>{} ({:?} Priority)</strong>: {}
                    <ul>"#,
                rec.title,
                rec.priority,
                rec.description
            ));

            for action in &rec.actions {
                html.push_str(&format!("<li>{}</li>", action));
            }

            html.push_str("</ul></div>");
        }

        html.push_str("</div>");
        html
    }

    fn print_performance_assessment(&self, result: &BenchmarkResult) {
        println!("\n{}", "Performance Assessment:".bold());

        // Throughput assessment
        if result.operations_per_second > 10000.0 {
            println!("  ✅ Excellent throughput: {:.0} ops/sec", result.operations_per_second);
        } else if result.operations_per_second > 5000.0 {
            println!("  ⚠️  Good throughput: {:.0} ops/sec", result.operations_per_second);
        } else {
            println!("  ❌ Low throughput: {:.0} ops/sec - consider optimization", result.operations_per_second);
        }

        // Latency assessment
        if result.latency_percentiles.p95 < 1000 {
            println!("  ✅ Excellent latency: {} μs p95", result.latency_percentiles.p95);
        } else if result.latency_percentiles.p95 < 5000 {
            println!("  ⚠️  Acceptable latency: {} μs p95", result.latency_percentiles.p95);
        } else {
            println!("  ❌ High latency: {} μs p95 - investigate bottlenecks", result.latency_percentiles.p95);
        }

        // Error rate assessment
        let error_rate_percent = result.error_rate * 100.0;
        if error_rate_percent < 0.1 {
            println!("  ✅ Excellent reliability: {:.2}% error rate", error_rate_percent);
        } else if error_rate_percent < 1.0 {
            println!("  ⚠️  Acceptable reliability: {:.2}% error rate", error_rate_percent);
        } else {
            println!("  ❌ Poor reliability: {:.2}% error rate - investigate errors", error_rate_percent);
        }
    }
}

// Simple trait for colored output (placeholder)
trait ColoredOutput {
    fn bold(&self) -> String;
    fn cyan(&self) -> String;
    fn yellow(&self) -> String;
    fn green(&self) -> String;
    fn red(&self) -> String;
}

impl ColoredOutput for str {
    fn bold(&self) -> String { format!("\x1b[1m{}\x1b[0m", self) }
    fn cyan(&self) -> String { format!("\x1b[36m{}\x1b[0m", self) }
    fn yellow(&self) -> String { format!("\x1b[33m{}\x1b[0m", self) }
    fn green(&self) -> String { format!("\x1b[32m{}\x1b[0m", self) }
    fn red(&self) -> String { format!("\x1b[31m{}\x1b[0m", self) }
}

impl ColoredOutput for String {
    fn bold(&self) -> String { self.as_str().bold() }
    fn cyan(&self) -> String { self.as_str().cyan() }
    fn yellow(&self) -> String { self.as_str().yellow() }
    fn green(&self) -> String { self.as_str().green() }
    fn red(&self) -> String { self.as_str().red() }
}

//! Test Results Reporter
//!
//! Generates comprehensive reports including:
//! - Console output with colored formatting
//! - JSON/CSV export for analysis
//! - HTML reports for web viewing
//! - Performance trend analysis

use crate::{LoadTestResult, MetricsSnapshot, ResourceSummary};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Test report generator
pub struct TestReporter {
    output_dir: String,
    reports: Vec<TestReport>,
}

#[derive(Debug, Clone)]
pub struct TestReport {
    pub test_name: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub result: LoadTestResult,
    pub resource_usage: Option<ResourceSummary>,
    pub metadata: HashMap<String, String>,
}

impl TestReporter {
    pub fn new(output_dir: &str) -> Self {
        fs::create_dir_all(output_dir).unwrap_or_else(|e| {
            eprintln!("Warning: Could not create output directory {}: {}", output_dir, e);
        });

        Self {
            output_dir: output_dir.to_string(),
            reports: Vec::new(),
        }
    }

    pub fn add_report(&mut self, test_name: &str, result: LoadTestResult) {
        let report = TestReport {
            test_name: test_name.to_string(),
            timestamp: chrono::Utc::now(),
            result,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        self.reports.push(report);
    }

    pub fn add_detailed_report(&mut self, test_name: &str, result: LoadTestResult, resource_usage: ResourceSummary, metadata: HashMap<String, String>) {
        let report = TestReport {
            test_name: test_name.to_string(),
            timestamp: chrono::Utc::now(),
            result,
            resource_usage: Some(resource_usage),
            metadata,
        };

        self.reports.push(report);
    }

    /// Generate all reports
    pub fn generate_reports(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.generate_console_report()?;
        self.generate_json_report()?;
        self.generate_csv_report()?;
        self.generate_html_report()?;
        self.generate_summary_report()?;
        Ok(())
    }

    /// Print colored console report
    pub fn generate_console_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("{}", "=".repeat(80));
        println!("{}", "🚀 KotobaDB Load Test Results".bold().cyan());
        println!("{}", "=".repeat(80));

        for report in &self.reports {
            println!("\n{}", format!("📊 Test: {}", report.test_name).bold().yellow());
            println!("{}", format!("🕒 Timestamp: {}", report.timestamp.format("%Y-%m-%d %H:%M:%S UTC")));

            // Performance metrics
            let r = &report.result;
            println!("\n{}", "Performance Metrics:".bold());
            println!("  ⏱️  Duration: {:.2}s", r.duration.as_secs_f64());
            println!("  📈 Operations: {}", r.total_operations);
            println!("  🚀 Throughput: {:.0} ops/sec", r.operations_per_second);

            // Latency percentiles
            println!("\n{}", "Latency Percentiles (μs):".bold());
            println!("  50th: {}", r.latency_percentiles.p50);
            println!("  95th: {}", r.latency_percentiles.p95);
            println!("  99th: {}", r.latency_percentiles.p99);
            println!("  99.9th: {}", r.latency_percentiles.p999);
            println!("  Max: {}", r.latency_percentiles.max);

            // Error analysis
            let error_rate_percent = r.error_rate * 100.0;
            let error_color = if error_rate_percent > 5.0 { "red" } else if error_rate_percent > 1.0 { "yellow" } else { "green" };
            println!("\n{}", "Error Analysis:".bold());
            println!("  ❌ Error Rate: {:.2}%", error_rate_percent);
            println!("  📊 Error Count: {}", r.error_count);

            // Resource usage
            if let Some(resources) = &report.resource_usage {
                println!("\n{}", "Resource Usage:".bold());
                println!("  🖥️  CPU: {:.1}% avg, {:.1}% max", resources.cpu_avg, resources.cpu_max);
                println!("  💾 Memory: {:.1} MB avg, {:.1} MB max", resources.memory_avg_mb, resources.memory_max_mb);
            }

            // Operation breakdown
            println!("\n{}", "Operation Breakdown:".bold());
            for (op_type, count) in &r.operation_counts {
                let percentage = (*count as f64 / r.total_operations as f64) * 100.0;
                println!("  {}: {} ({:.1}%)", op_type, count, percentage);
            }

            // Performance assessment
            self.print_performance_assessment(r);
        }

        println!("\n{}", "=".repeat(80));
        Ok(())
    }

    /// Generate JSON report
    pub fn generate_json_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let json_path = Path::new(&self.output_dir).join("load_test_results.json");

        let json_data = serde_json::json!({
            "test_run": {
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "total_tests": self.reports.len(),
            },
            "reports": self.reports.iter().map(|r| {
                serde_json::json!({
                    "test_name": r.test_name,
                    "timestamp": r.timestamp.to_rfc3339(),
                    "result": {
                        "total_operations": r.result.total_operations,
                        "duration_seconds": r.result.duration.as_secs_f64(),
                        "operations_per_second": r.result.operations_per_second,
                        "latency_percentiles_us": {
                            "p50": r.result.latency_percentiles.p50,
                            "p95": r.result.latency_percentiles.p95,
                            "p99": r.result.latency_percentiles.p99,
                            "p999": r.result.latency_percentiles.p999,
                            "max": r.result.latency_percentiles.max,
                        },
                        "error_count": r.result.error_count,
                        "error_rate": r.result.error_rate,
                        "operation_counts": r.result.operation_counts,
                        "error_counts": r.result.error_counts,
                    },
                    "resource_usage": r.resource_usage.as_ref().map(|ru| {
                        serde_json::json!({
                            "cpu_avg": ru.cpu_avg,
                            "cpu_max": ru.cpu_max,
                            "memory_avg_mb": ru.memory_avg_mb,
                            "memory_max_mb": ru.memory_max_mb,
                            "sample_count": ru.sample_count,
                        })
                    }),
                    "metadata": r.metadata,
                })
            }).collect::<Vec<_>>()
        });

        fs::write(&json_path, serde_json::to_string_pretty(&json_data)?)?;
        println!("📄 JSON report saved to: {}", json_path.display());
        Ok(())
    }

    /// Generate CSV report
    pub fn generate_csv_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let csv_path = Path::new(&self.output_dir).join("load_test_results.csv");

        let mut csv_writer = csv::Writer::from_path(&csv_path)?;

        // Write header
        csv_writer.write_record([
            "test_name", "timestamp", "duration_seconds", "total_operations",
            "operations_per_second", "latency_p50_us", "latency_p95_us",
            "latency_p99_us", "latency_max_us", "error_count", "error_rate",
            "cpu_avg_percent", "cpu_max_percent", "memory_avg_mb", "memory_max_mb"
        ])?;

        // Write data
        for report in &self.reports {
            let r = &report.result;
            let ru = report.resource_usage.as_ref();

            csv_writer.write_record([
                &report.test_name,
                &report.timestamp.to_rfc3339(),
                &r.duration.as_secs_f64().to_string(),
                &r.total_operations.to_string(),
                &r.operations_per_second.to_string(),
                &r.latency_percentiles.p50.to_string(),
                &r.latency_percentiles.p95.to_string(),
                &r.latency_percentiles.p99.to_string(),
                &r.latency_percentiles.max.to_string(),
                &r.error_count.to_string(),
                &r.error_rate.to_string(),
                &ru.map(|r| r.cpu_avg.to_string()).unwrap_or_default(),
                &ru.map(|r| r.cpu_max.to_string()).unwrap_or_default(),
                &ru.map(|r| r.memory_avg_mb.to_string()).unwrap_or_default(),
                &ru.map(|r| r.memory_max_mb.to_string()).unwrap_or_default(),
            ])?;
        }

        csv_writer.flush()?;
        println!("📊 CSV report saved to: {}", csv_path.display());
        Ok(())
    }

    /// Generate HTML report
    pub fn generate_html_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let html_path = Path::new(&self.output_dir).join("load_test_report.html");

        let html_content = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>KotobaDB Load Test Report</title>
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
        .test-card {{
            border: 1px solid #ddd;
            border-radius: 8px;
            padding: 20px;
            margin: 20px 0;
            background: #fafafa;
        }}
        .metric-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
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
        .good {{ border-left-color: #27ae60; }}
        .warning {{ border-left-color: #f39c12; }}
        .danger {{ border-left-color: #e74c3c; }}
        .chart-container {{
            margin: 20px 0;
            height: 300px;
        }}
        .timestamp {{
            color: #7f8c8d;
            font-size: 0.9em;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>🚀 KotobaDB Load Test Report</h1>
        <p class="timestamp">Generated on: {}</p>

        {}

        <div class="summary">
            <h2>📈 Summary</h2>
            <div class="metric-grid">
                <div class="metric">
                    <div class="metric-label">Total Tests</div>
                    <div class="metric-value">{}</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Average Throughput</div>
                    <div class="metric-value">{:.0} ops/sec</div>
                </div>
                <div class="metric">
                    <div class="metric-label">Average Error Rate</div>
                    <div class="metric-value">{:.2}%</div>
                </div>
            </div>
        </div>
    </div>
</body>
</html>"#,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.generate_html_test_cards(),
            self.reports.len(),
            self.reports.iter().map(|r| r.result.operations_per_second).sum::<f64>() / self.reports.len() as f64,
            self.reports.iter().map(|r| r.result.error_rate * 100.0).sum::<f64>() / self.reports.len() as f64,
        );

        fs::write(&html_path, html_content)?;
        println!("🌐 HTML report saved to: {}", html_path.display());
        Ok(())
    }

    /// Generate summary report
    pub fn generate_summary_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        let summary_path = Path::new(&self.output_dir).join("SUMMARY.md");

        let mut content = format!(
            "# KotobaDB Load Test Summary Report\n\n"
        );

        content.push_str(&format!("**Generated:** {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        content.push_str(&format!("**Total Tests:** {}\n\n", self.reports.len()));

        // Overall statistics
        let total_ops: u64 = self.reports.iter().map(|r| r.result.total_operations).sum();
        let avg_throughput = self.reports.iter().map(|r| r.result.operations_per_second).sum::<f64>() / self.reports.len() as f64;
        let avg_error_rate = self.reports.iter().map(|r| r.result.error_rate * 100.0).sum::<f64>() / self.reports.len() as f64;

        content.push_str("## Overall Statistics\n\n");
        content.push_str(&format!("- **Total Operations:** {}\n", total_ops));
        content.push_str(&format!("- **Average Throughput:** {:.0} ops/sec\n", avg_throughput));
        content.push_str(&format!("- **Average Error Rate:** {:.2}%\n\n", avg_error_rate));

        // Individual test results
        content.push_str("## Test Results\n\n");
        for report in &self.reports {
            let r = &report.result;
            content.push_str(&format!("### {}\n\n", report.test_name));
            content.push_str(&format!("- **Throughput:** {:.0} ops/sec\n", r.operations_per_second));
            content.push_str(&format!("- **Latency p95:** {} μs\n", r.latency_percentiles.p95));
            content.push_str(&format!("- **Error Rate:** {:.2}%\n", r.error_rate * 100.0));

            if let Some(ru) = &report.resource_usage {
                content.push_str(&format!("- **CPU Usage:** {:.1}% avg\n", ru.cpu_avg));
                content.push_str(&format!("- **Memory Usage:** {:.1} MB avg\n", ru.memory_avg_mb));
            }

            content.push_str("\n");
        }

        // Performance assessment
        content.push_str("## Performance Assessment\n\n");

        let high_error_tests: Vec<_> = self.reports.iter()
            .filter(|r| r.result.error_rate > 0.05)
            .map(|r| r.test_name.as_str())
            .collect();

        if !high_error_tests.is_empty() {
            content.push_str("### ⚠️  High Error Rate Tests\n\n");
            for test in high_error_tests {
                content.push_str(&format!("- {}\n", test));
            }
            content.push_str("\n");
        }

        let high_latency_tests: Vec<_> = self.reports.iter()
            .filter(|r| r.result.latency_percentiles.p95 > 10000) // 10ms
            .map(|r| r.test_name.as_str())
            .collect();

        if !high_latency_tests.is_empty() {
            content.push_str("### 🐌 High Latency Tests\n\n");
            for test in high_latency_tests {
                content.push_str(&format!("- {}\n", test));
            }
            content.push_str("\n");
        }

        content.push_str("## Recommendations\n\n");

        if avg_throughput < 1000.0 {
            content.push_str("- Consider optimizing for higher throughput\n");
        }

        if avg_error_rate > 1.0 {
            content.push_str("- Investigate and fix high error rates\n");
        }

        if self.reports.iter().any(|r| r.result.latency_percentiles.p95 > 5000) {
            content.push_str("- Review latency optimization opportunities\n");
        }

        fs::write(&summary_path, content)?;
        println!("📋 Summary report saved to: {}", summary_path.display());
        Ok(())
    }

    fn generate_html_test_cards(&self) -> String {
        self.reports.iter().map(|report| {
            let r = &report.result;
            let throughput_class = if r.operations_per_second > 10000.0 { "good" } else if r.operations_per_second > 5000.0 { "warning" } else { "danger" };
            let error_class = if r.error_rate < 0.01 { "good" } else if r.error_rate < 0.05 { "warning" } else { "danger" };

            format!(
                r#"<div class="test-card">
                    <h3>{}</h3>
                    <div class="timestamp">🕒 {}</div>
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
                report.test_name,
                report.timestamp.format("%Y-%m-%d %H:%M:%S UTC"),
                throughput_class,
                r.operations_per_second,
                r.latency_percentiles.p95,
                error_class,
                r.error_rate * 100.0,
                r.total_operations
            )
        }).collect::<String>()
    }

    fn print_performance_assessment(&self, result: &LoadTestResult) {
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

// Simple trait for colored output (placeholder - in real implementation would use a color library)
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

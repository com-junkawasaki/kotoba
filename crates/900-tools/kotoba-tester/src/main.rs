//! Kotoba Test Runner CLI
//!
//! Command line interface for running Kotoba tests.

use clap::{Arg, Command};
use std::path::PathBuf;
use colored::*;
use kotoba_tester::{TestConfig, TestRunner, TestResult};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("kotoba-test")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kotoba Team")
        .about("Kotoba Test Runner - Run tests for .kotoba files")
        .arg(
            Arg::new("patterns")
                .help("Test file patterns or directories")
                .value_name("PATTERNS")
                .num_args(1..)
                .default_value(".")
        )
        .arg(
            Arg::new("filter")
                .long("filter")
                .short('f')
                .help("Filter tests by name")
                .value_name("FILTER")
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Verbose output")
                .action(clap::ArgAction::SetTrue)
        )
        .arg(
            Arg::new("timeout")
                .long("timeout")
                .short('t')
                .help("Test timeout in seconds")
                .value_name("SECONDS")
                .default_value("30")
        )
        .get_matches();

    let patterns: Vec<String> = matches
        .get_many::<String>("patterns")
        .unwrap_or_default()
        .cloned()
        .collect();

    let filter = matches.get_one::<String>("filter").map(|s| s.as_str());
    let verbose = matches.get_flag("verbose");
    let timeout: u64 = matches
        .get_one::<String>("timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(30);

    // Create test config
    let mut config = TestConfig {
        timeout,
        concurrency: num_cpus::get(),
        filter: filter.map(|s| s.to_string()),
        verbose,
        coverage: false,
        exclude_patterns: vec![
            "node_modules".to_string(),
            ".git".to_string(),
            "target".to_string(),
            "build".to_string(),
        ],
    };

    if let Some(f) = filter {
        config.filter = Some(f.to_string());
    }

    let runner = TestRunner::new(config);
    let result = runner.run_tests(patterns).await?;

    // Report results
    println!("\n{}", "Test Results:".bold());
    println!("{}", "=============".bold());

    for suite in &result.test_suites {
        println!("\n{}", format!("Suite: {}", suite.name).cyan());
        for test_case in &suite.test_cases {
            let status = match test_case.result {
                TestResult::Passed => "✅ PASS".green(),
                TestResult::Failed => "❌ FAIL".red(),
                TestResult::Skipped => "⏭️ SKIP".yellow(),
                TestResult::Pending => "PENDING".cyan(),
                TestResult::Timeout => "TIMEOUT".magenta(),
            };
            println!(
                "  [{}] {} ({}ms)",
                status,
                test_case.name,
                test_case.duration.as_millis()
            );
            if let Some(err) = &test_case.error_message {
                if verbose {
                    println!("    {}", err.red());
                }
            }
        }
    }

    println!("\n{}", "Summary:".bold());
    println!("  Total tests: {}", result.total_tests());
    println!("  Passed: {}", result.passed_tests().to_string().green());
    println!("  Failed: {}", result.failed_tests().to_string().red());
    println!("  Duration: {}ms", result.total_duration.as_millis());

    if result.failed_tests() > 0 {
        std::process::exit(1);
    }

    Ok(())
}

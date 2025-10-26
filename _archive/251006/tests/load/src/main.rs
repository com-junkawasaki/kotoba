//! KotobaDB Load Test Runner
//!
//! Command-line interface for running comprehensive load tests on KotobaDB.
//!
//! Usage examples:
//!   # Run YCSB Workload A
//!   cargo run --bin load_test_runner -- --workload ycsb-a --duration 60
//!
//!   # Run comprehensive test suite
//!   cargo run --bin load_test_runner -- --scenario comprehensive
//!
//!   # Run social network scenario
//!   cargo run --bin load_test_runner -- --scenario social-network --duration 120

use clap::{Parser, Subcommand};
use kotoba_db::DB;
use std::path::PathBuf;
use std::sync::Arc;

mod workload;
mod metrics;
mod runner;
mod scenarios;
mod reporter;
mod config;

use crate::runner::*;
use crate::scenarios::*;
use crate::reporter::TestReporter;

#[derive(Parser)]
#[command(name = "kotoba-load-test")]
#[command(about = "Comprehensive load testing framework for KotobaDB")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a specific workload
    Workload {
        /// Workload type (ycsb-a, ycsb-b, ycsb-c, social-network, ecommerce)
        #[arg(short, long)]
        workload: String,

        /// Test duration in seconds
        #[arg(short, long, default_value = "60")]
        duration: u64,

        /// Number of concurrent workers
        #[arg(short, long, default_value = "32")]
        concurrency: usize,

        /// Database path
        #[arg(short, long, default_value = "/tmp/kotoba_load_test.db")]
        db_path: PathBuf,

        /// Output directory for reports
        #[arg(short, long, default_value = "load_test_reports")]
        output_dir: String,
    },

    /// Run a predefined scenario
    Scenario {
        /// Scenario name (comprehensive, benchmarks, applications, stress, scalability)
        #[arg(short, long)]
        scenario: String,

        /// Database path
        #[arg(short, long, default_value = "/tmp/kotoba_scenario_test.db")]
        db_path: PathBuf,

        /// Output directory for reports
        #[arg(short, long, default_value = "scenario_reports")]
        output_dir: String,
    },

    /// Run performance regression tests
    Regression {
        /// Regression test type (baseline, compare)
        #[arg(short, long)]
        test_type: String,

        /// Database path
        #[arg(short, long, default_value = "/tmp/kotoba_regression_test.db")]
        db_path: PathBuf,

        /// Output directory for reports
        #[arg(short, long, default_value = "regression_reports")]
        output_dir: String,
    },

    /// Run custom load test with detailed configuration
    Custom {
        /// Configuration file path
        #[arg(short, long)]
        config: PathBuf,

        /// Database path
        #[arg(short, long, default_value = "/tmp/kotoba_custom_test.db")]
        db_path: PathBuf,

        /// Output directory for reports
        #[arg(short, long, default_value = "custom_reports")]
        output_dir: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Workload { workload, duration, concurrency, db_path, output_dir } => {
            run_workload_test(workload, duration, concurrency, db_path, output_dir).await
        }
        Commands::Scenario { scenario, db_path, output_dir } => {
            run_scenario_test(scenario, db_path, output_dir).await
        }
        Commands::Regression { test_type, db_path, output_dir } => {
            run_regression_test(test_type, db_path, output_dir).await
        }
        Commands::Custom { config, db_path, output_dir } => {
            run_custom_test(config, db_path, output_dir).await
        }
    }
}

async fn run_workload_test(
    workload: String,
    duration: u64,
    concurrency: usize,
    db_path: PathBuf,
    output_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting KotobaDB Load Test");
    println!("=============================");
    println!("Workload: {}", workload);
    println!("Duration: {}s", duration);
    println!("Concurrency: {}", concurrency);
    println!("Database: {}", db_path.display());
    println!();

    // Setup database
    let db = DB::open_lsm(&db_path).await?;
    let runner = KotobaDBRunner::with_db(db);

    // Create reporter
    let mut reporter = TestReporter::new(&output_dir);

    match workload.as_str() {
        "ycsb-a" => {
            benchmarks::ycsb_a(runner, duration).await?;
        }
        "ycsb-b" => {
            benchmarks::ycsb_b(runner, duration).await?;
        }
        "ycsb-c" => {
            benchmarks::ycsb_c(runner, duration).await?;
        }
        "social-network" => {
            applications::social_network(runner, duration).await?;
        }
        "ecommerce" => {
            applications::ecommerce(runner, duration).await?;
        }
        _ => {
            eprintln!("Unknown workload: {}", workload);
            eprintln!("Available workloads: ycsb-a, ycsb-b, ycsb-c, social-network, ecommerce");
            std::process::exit(1);
        }
    }

    println!("\n📊 Test completed! Generating reports...");
    reporter.generate_reports()?;

    Ok(())
}

async fn run_scenario_test(
    scenario: String,
    db_path: PathBuf,
    output_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting KotobaDB Scenario Test");
    println!("=================================");
    println!("Scenario: {}", scenario);
    println!("Database: {}", db_path.display());
    println!();

    // Setup database
    let db = DB::open_lsm(&db_path).await?;
    let runner = KotobaDBRunner::with_db(db);

    match scenario.as_str() {
        "comprehensive" => {
            run_comprehensive_suite(runner).await?;
        }
        "benchmarks" => {
            println!("Running YCSB benchmarks...");
            benchmarks::ycsb_a(runner.clone(), 30).await?;
            benchmarks::ycsb_b(runner.clone(), 30).await?;
            benchmarks::ycsb_c(runner, 30).await?;
        }
        "applications" => {
            println!("Running application scenarios...");
            applications::social_network(runner.clone(), 30).await?;
            applications::ecommerce(runner, 30).await?;
        }
        "stress" => {
            println!("Running stress tests...");
            stress::hotspot(runner.clone(), 30).await?;
            stress::large_values(runner.clone(), 30).await?;
            stress::max_throughput(runner, 30).await?;
        }
        "scalability" => {
            println!("Running scalability tests...");
            // Note: These require Clone trait, simplified for now
            println!("Scalability tests require runner cloning - use individual tests");
        }
        _ => {
            eprintln!("Unknown scenario: {}", scenario);
            eprintln!("Available scenarios: comprehensive, benchmarks, applications, stress, scalability");
            std::process::exit(1);
        }
    }

    println!("\n✅ Scenario test completed!");
    Ok(())
}

async fn run_regression_test(
    test_type: String,
    db_path: PathBuf,
    output_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Regression Test");
    println!("=========================");
    println!("Type: {}", test_type);
    println!("Database: {}", db_path.display());
    println!();

    // Setup database
    let db = DB::open_lsm(&db_path).await?;
    let runner = KotobaDBRunner::with_db(db);

    match test_type.as_str() {
        "baseline" => {
            regression::baseline(runner).await?;
            println!("✅ Baseline metrics captured!");
        }
        "compare" => {
            regression::compare_baseline(runner).await?;
            println!("✅ Regression analysis completed!");
        }
        _ => {
            eprintln!("Unknown regression test type: {}", test_type);
            eprintln!("Available types: baseline, compare");
            std::process::exit(1);
        }
    }

    Ok(())
}

async fn run_custom_test(
    config_path: PathBuf,
    db_path: PathBuf,
    output_dir: String,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Custom Load Test");
    println!("===========================");
    println!("Config: {}", config_path.display());
    println!("Database: {}", db_path.display());
    println!();

    if !config_path.exists() {
        eprintln!("Configuration file not found: {}", config_path.display());
        std::process::exit(1);
    }

    // Load configuration (simplified - would parse TOML/JSON/YAML)
    println!("Custom configuration loading not yet implemented.");
    println!("Please use predefined workloads and scenarios for now.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_basic_workload_execution() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        // This would be a simplified unit test
        // In practice, you'd have more comprehensive integration tests
        let db = DB::open_lsm(&db_path).await.unwrap();
        let runner = KotobaDBRunner::with_db(db);

        // Test basic runner functionality
        runner.setup().await.unwrap();
        runner.teardown().await.unwrap();
    }
}

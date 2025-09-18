//! Standalone Benchmark Runner
//!
//! This binary provides a simple way to run the standalone benchmarks
//! that were integrated from the benches/ directory.

use std::process::Command;
use std::env;

fn main() {
    println!("🚀 Kotoba Benchmark Runner");
    println!("==========================");

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    let benchmark_name = &args[1];

    match benchmark_name.as_str() {
        "simple" | "simple_benchmark" => {
            run_simple_benchmark();
        }
        "parallel" | "parallel_benchmark" => {
            run_parallel_benchmark();
        }
        "help" | "--help" | "-h" => {
            print_usage();
        }
        _ => {
            println!("❌ Unknown benchmark: {}", benchmark_name);
            print_usage();
        }
    }
}

fn run_simple_benchmark() {
    println!("📊 Running Simple Benchmark...");
    println!("================================");

    let status = Command::new("cargo")
        .args(&["run", "--bin", "simple_benchmark"])
        .status();

    match status {
        Ok(exit_status) if exit_status.success() => {
            println!("✅ Simple benchmark completed successfully");
        }
        Ok(exit_status) => {
            println!("❌ Simple benchmark failed with exit code: {}", exit_status);
        }
        Err(e) => {
            println!("❌ Failed to run simple benchmark: {}", e);
        }
    }
}

fn run_parallel_benchmark() {
    println!("📊 Running Parallel Benchmark...");
    println!("================================");

    let status = Command::new("cargo")
        .args(&["run", "--bin", "parallel_benchmark"])
        .status();

    match status {
        Ok(exit_status) if exit_status.success() => {
            println!("✅ Parallel benchmark completed successfully");
        }
        Ok(exit_status) => {
            println!("❌ Parallel benchmark failed with exit code: {}", exit_status);
        }
        Err(e) => {
            println!("❌ Failed to run parallel benchmark: {}", e);
        }
    }
}

fn print_usage() {
    println!("Usage: cargo run --bin benchmark_runner <benchmark_name>");
    println!();
    println!("Available benchmarks:");
    println!("  simple     - Run the simple benchmark (graph operations)");
    println!("  parallel   - Run the parallel benchmark (concurrent operations)");
    println!("  help       - Show this help message");
    println!();
    println!("Examples:");
    println!("  cargo run --bin benchmark_runner simple");
    println!("  cargo run --bin benchmark_runner parallel");
}

//! Predefined Load Test Scenarios
//!
//! Ready-to-use test scenarios covering:
//! - Standard benchmarks (YCSB, TPC-like)
//! - Application-specific workloads
//! - Stress testing scenarios
//! - Performance regression testing

use crate::workload::*;
use crate::{LoadTestConfig, run_load_test, LoadTestRunner};
use std::time::Duration;

/// Standard benchmark scenarios
pub mod benchmarks {
    use super::*;

    /// Run YCSB Workload A (50% reads, 50% updates)
    pub async fn ycsb_a<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadA::new(100000, 1024));
        let result = run_load_test(runner, workload, config).await?;

        println!("YCSB-A Results:");
        println!("{}", result.summary());
        Ok(())
    }

    /// Run YCSB Workload B (95% reads, 5% updates)
    pub async fn ycsb_b<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadB::new(100000, 1024));
        let result = run_load_test(runner, workload, config).await?;

        println!("YCSB-B Results:");
        println!("{}", result.summary());
        Ok(())
    }

    /// Run YCSB Workload C (100% reads)
    pub async fn ycsb_c<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 64,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadC::new(100000));
        let result = run_load_test(runner, workload, config).await?;

        println!("YCSB-C Results:");
        println!("{}", result.summary());
        Ok(())
    }
}

/// Application-specific scenarios
pub mod applications {
    use super::*;

    /// Social network application scenario
    pub async fn social_network<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 50,
            warmup_duration: Duration::from_secs(15),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(custom::SocialNetworkWorkload::new(10000, 50000));
        let result = run_load_test(runner, workload, config).await?;

        println!("Social Network Scenario Results:");
        println!("{}", result.summary());
        Ok(())
    }

    /// E-commerce application scenario
    pub async fn ecommerce<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 40,
            warmup_duration: Duration::from_secs(15),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(custom::EcommerceWorkload::new(10000, 5000));
        let result = run_load_test(runner, workload, config).await?;

        println!("E-commerce Scenario Results:");
        println!("{}", result.summary());
        Ok(())
    }
}

/// Stress testing scenarios
pub mod stress {
    use super::*;

    /// High contention hotspot scenario
    pub async fn hotspot<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 100,
            warmup_duration: Duration::from_secs(5),
            operations_per_second: Some(10000), // Rate limited
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(stress::HotspotWorkload::new(10000, 10, 0.8));
        let result = run_load_test(runner, workload, config).await?;

        println!("Hotspot Stress Test Results:");
        println!("{}", result.summary());
        Ok(())
    }

    /// Large value stress test
    pub async fn large_values<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 10,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(stress::LargeValueWorkload::new(1000, 1024)); // 1MB values
        let result = run_load_test(runner, workload, config).await?;

        println!("Large Values Stress Test Results:");
        println!("{}", result.summary());
        Ok(())
    }

    /// Maximum throughput test
    pub async fn max_throughput<R: LoadTestRunner>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(duration_secs),
            concurrency: 200,
            warmup_duration: Duration::from_secs(5),
            operations_per_second: None, // Unlimited
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadA::new(1000000, 1024));
        let result = run_load_test(runner, workload, config).await?;

        println!("Max Throughput Test Results:");
        println!("{}", result.summary());
        Ok(())
    }
}

/// Scalability testing scenarios
pub mod scalability {
    use super::*;

    /// Test with increasing concurrency levels
    pub async fn concurrency_scaling<R: LoadTestRunner + Clone>(runner: R, base_duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let concurrency_levels = vec![1, 4, 16, 32, 64, 128];

        println!("Concurrency Scaling Test");
        println!("========================");

        for concurrency in concurrency_levels {
            println!("\nTesting with {} concurrent workers...", concurrency);

            let config = LoadTestConfig {
                duration: Duration::from_secs(base_duration_secs),
                concurrency,
                warmup_duration: Duration::from_secs(5),
                operations_per_second: None,
                enable_metrics: true,
                collect_detailed_latency: true,
            };

            let workload = Box::new(ycsb::WorkloadA::new(100000, 1024));
            let result = run_load_test(runner.clone(), workload, config).await?;

            println!("Concurrency {}: {:.0} ops/sec, p95: {} μs",
                    concurrency, result.operations_per_second, result.latency_percentiles.p95);
        }

        Ok(())
    }

    /// Test with increasing data sizes
    pub async fn data_scaling<R: LoadTestRunner + Clone>(runner: R, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
        let data_sizes_kb = vec![1, 10, 100, 1000];

        println!("Data Size Scaling Test");
        println!("======================");

        for size_kb in data_sizes_kb {
            println!("\nTesting with {} KB values...", size_kb);

            let config = LoadTestConfig {
                duration: Duration::from_secs(duration_secs),
                concurrency: 16,
                warmup_duration: Duration::from_secs(5),
                operations_per_second: None,
                enable_metrics: true,
                collect_detailed_latency: true,
            };

            let workload = Box::new(stress::LargeValueWorkload::new(1000, size_kb));
            let result = run_load_test(runner.clone(), workload, config).await?;

            println!("Size {} KB: {:.0} ops/sec, p95: {} μs",
                    size_kb, result.operations_per_second, result.latency_percentiles.p95);
        }

        Ok(())
    }
}

/// Performance regression testing
pub mod regression {
    use super::*;

    /// Baseline performance test for regression detection
    pub async fn baseline<R: LoadTestRunner>(runner: R) -> Result<(), Box<dyn std::error::Error>> {
        let config = LoadTestConfig {
            duration: Duration::from_secs(60), // 1 minute baseline
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadA::new(100000, 1024));
        let result = run_load_test(runner, workload, config).await?;

        println!("Performance Baseline:");
        println!("{}", result.summary());

        // Save baseline metrics to file for future comparison
        std::fs::write("baseline_metrics.json", serde_json::to_string_pretty(&result)?)?;

        Ok(())
    }

    /// Compare current performance against baseline
    pub async fn compare_baseline<R: LoadTestRunner>(runner: R) -> Result<(), Box<dyn std::error::Error>> {
        // Load baseline metrics
        let baseline_content = std::fs::read_to_string("baseline_metrics.json")?;
        let baseline: crate::LoadTestResult = serde_json::from_str(&baseline_content)?;

        // Run current test
        let config = LoadTestConfig {
            duration: Duration::from_secs(60),
            concurrency: 32,
            warmup_duration: Duration::from_secs(10),
            operations_per_second: None,
            enable_metrics: true,
            collect_detailed_latency: true,
        };

        let workload = Box::new(ycsb::WorkloadA::new(100000, 1024));
        let current = run_load_test(runner, workload, config).await?;

        // Compare results
        let throughput_regression = (baseline.operations_per_second - current.operations_per_second) / baseline.operations_per_second * 100.0;
        let latency_regression = (current.latency_percentiles.p95 as f64 - baseline.latency_percentiles.p95 as f64) / baseline.latency_percentiles.p95 as f64 * 100.0;

        println!("Performance Regression Analysis");
        println!("==============================");
        println!("Throughput: {:.1}% change ({:.0} -> {:.0} ops/sec)",
                throughput_regression, baseline.operations_per_second, current.operations_per_second);
        println!("Latency p95: {:.1}% change ({} -> {} μs)",
                latency_regression, baseline.latency_percentiles.p95, current.latency_percentiles.p95);

        // Alert on significant regressions
        if throughput_regression < -10.0 {
            println!("⚠️  WARNING: Significant throughput regression detected!");
        }
        if latency_regression > 20.0 {
            println!("⚠️  WARNING: Significant latency regression detected!");
        }

        Ok(())
    }
}

/// Comprehensive test suite
pub async fn run_comprehensive_suite<R: LoadTestRunner + Clone>(runner: R) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Comprehensive Load Test Suite");
    println!("=====================================");

    // YCSB benchmarks
    println!("\n--- YCSB Benchmarks ---");
    benchmarks::ycsb_a(runner.clone(), 30).await?;
    benchmarks::ycsb_b(runner.clone(), 30).await?;
    benchmarks::ycsb_c(runner.clone(), 30).await?;

    // Application scenarios
    println!("\n--- Application Scenarios ---");
    applications::social_network(runner.clone(), 30).await?;
    applications::ecommerce(runner.clone(), 30).await?;

    // Stress tests
    println!("\n--- Stress Tests ---");
    stress::hotspot(runner.clone(), 30).await?;
    stress::large_values(runner.clone(), 30).await?;

    // Scalability tests
    println!("\n--- Scalability Tests ---");
    scalability::concurrency_scaling(runner.clone(), 15).await?;
    scalability::data_scaling(runner.clone(), 15).await?;

    println!("\n✅ Comprehensive test suite completed!");
    Ok(())
}

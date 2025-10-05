//! YCSB and Standard Benchmark Implementations
//!
//! This module provides implementations of standard database benchmarks,
//! including YCSB (Yahoo! Cloud Serving Benchmark) workloads and other
//! industry-standard performance tests.

use crate::runner::KotobaDBRunner;
use crate::workload::{WorkloadGenerator, YCSBWorkload};
use crate::{LoadTestConfig, LoadTestResult, run_load_test};
use std::time::Duration;

/// Run YCSB Workload A (50% reads, 50% updates)
pub async fn ycsb_a(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload A (50% reads, 50% updates)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 32,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_a();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload A", &result);
    Ok(())
}

/// Run YCSB Workload B (95% reads, 5% updates)
pub async fn ycsb_b(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload B (95% reads, 5% updates)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 32,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_b();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload B", &result);
    Ok(())
}

/// Run YCSB Workload C (100% reads)
pub async fn ycsb_c(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload C (100% reads)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 32,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_c();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload C", &result);
    Ok(())
}

/// Run YCSB Workload D (95% reads, 5% inserts)
pub async fn ycsb_d(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload D (95% reads, 5% inserts)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 32,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_d();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload D", &result);
    Ok(())
}

/// Run YCSB Workload E (95% scans, 5% inserts)
pub async fn ycsb_e(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload E (95% scans, 5% inserts)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 16, // Lower concurrency for scan workloads
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_e();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload E", &result);
    Ok(())
}

/// Run YCSB Workload F (50% reads, 50% read-modify-writes)
pub async fn ycsb_f(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running YCSB Workload F (50% reads, 50% read-modify-writes)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 24,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = YCSBWorkload::workload_f();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_ycsb_results("Workload F", &result);
    Ok(())
}

/// Run TPC-C benchmark (simplified)
pub async fn tpc_c(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🏪 Running TPC-C Benchmark (OLTP workload simulation)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 20,
        warmup_duration: Duration::from_secs(15),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    // TPC-C simulates warehouse operations with multiple transaction types
    let workload = TPCWorkload::new("TPC-C", vec![
        ("new_order".to_string(), 45.0),     // New order transactions
        ("payment".to_string(), 43.0),       // Payment transactions
        ("order_status".to_string(), 4.0),   // Order status inquiries
        ("delivery".to_string(), 4.0),       // Delivery transactions
        ("stock_level".to_string(), 4.0),    // Stock level inquiries
    ]);

    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_benchmark_results("TPC-C", &result);
    Ok(())
}

/// Run custom benchmark with specified parameters
pub async fn custom_benchmark(
    runner: KotobaDBRunner,
    name: &str,
    read_ratio: f64,
    write_ratio: f64,
    update_ratio: f64,
    scan_ratio: f64,
    duration_secs: u64,
    concurrency: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Running Custom Benchmark: {}", name);
    println!("   Read: {:.1}%, Write: {:.1}%, Update: {:.1}%, Scan: {:.1}%",
             read_ratio * 100.0, write_ratio * 100.0, update_ratio * 100.0, scan_ratio * 100.0);

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = CustomBenchmarkWorkload::new(
        read_ratio, write_ratio, update_ratio, scan_ratio
    );

    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_benchmark_results(name, &result);
    Ok(())
}

/// Print YCSB benchmark results
fn print_ycsb_results(workload_name: &str, result: &LoadTestResult) {
    println!("📊 {} Results:", workload_name);
    println!("   Throughput: {:.1} ops/sec", result.operations_per_second);
    println!("   Total Operations: {}", result.total_operations);
    println!("   Duration: {:.2}s", result.duration.as_secs_f64());
    println!("   Latency (μs): P50={}, P95={}, P99={}, Max={}",
             result.latency_percentiles.p50,
             result.latency_percentiles.p95,
             result.latency_percentiles.p99,
             result.latency_percentiles.max);
    println!("   Error Rate: {:.3}%", result.error_rate * 100.0);
    println!();
}

/// Print general benchmark results
fn print_benchmark_results(benchmark_name: &str, result: &LoadTestResult) {
    println!("📊 {} Results:", benchmark_name);
    println!("   Throughput: {:.1} ops/sec", result.operations_per_second);
    println!("   Total Operations: {}", result.total_operations);
    println!("   Duration: {:.2}s", result.duration.as_secs_f64());
    println!("   Latency (μs): P50={}, P95={}, P99={}, Max={}",
             result.latency_percentiles.p50,
             result.latency_percentiles.p95,
             result.latency_percentiles.p99,
             result.latency_percentiles.max);
    println!("   Error Rate: {:.3}%", result.error_rate * 100.0);
    println!();
}

// Custom workload implementations

/// Simplified TPC workload
pub struct TPCWorkload {
    name: String,
    transaction_types: Vec<(String, f64)>, // (name, probability)
}

impl TPCWorkload {
    pub fn new(name: String, transaction_types: Vec<(String, f64)>) -> Self {
        Self {
            name,
            transaction_types,
        }
    }
}

impl WorkloadGenerator for TPCWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> crate::Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Select transaction type based on probabilities
        let mut cumulative_prob = 0.0;
        let rand_val: f64 = rng.gen();
        let transaction_type = self.transaction_types.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0; // Convert percentage to fraction
                rand_val <= cumulative_prob
            })
            .map(|(name, _)| name)
            .unwrap_or(&self.transaction_types[0].0);

        // Generate operation based on transaction type
        match transaction_type.as_str() {
            "new_order" => {
                let key = format!("order_{}_{}", worker_id, operation_count % 10000);
                let value = format!("new_order_data_{}_{}", worker_id, operation_count);
                crate::Operation::Insert {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                }
            }
            "payment" => {
                let key = format!("payment_{}_{}", worker_id, operation_count % 10000);
                let value = format!("payment_data_{}_{}", worker_id, operation_count);
                crate::Operation::Update {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                }
            }
            "order_status" => {
                let key = format!("order_{}_{}", worker_id, operation_count % 10000);
                crate::Operation::Read {
                    key: key.into_bytes(),
                }
            }
            "delivery" => {
                let key = format!("delivery_{}_{}", worker_id, operation_count % 10000);
                let value = format!("delivery_data_{}_{}", worker_id, operation_count);
                crate::Operation::Update {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                }
            }
            "stock_level" => {
                let key = format!("stock_{}_{}", worker_id, operation_count % 1000);
                crate::Operation::Read {
                    key: key.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                crate::Operation::Read {
                    key: key.into_bytes(),
                }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(TPCWorkload {
            name: self.name.clone(),
            transaction_types: self.transaction_types.clone(),
        })
    }
}

/// Custom benchmark workload with configurable ratios
pub struct CustomBenchmarkWorkload {
    read_ratio: f64,
    write_ratio: f64,
    update_ratio: f64,
    scan_ratio: f64,
}

impl CustomBenchmarkWorkload {
    pub fn new(read_ratio: f64, write_ratio: f64, update_ratio: f64, scan_ratio: f64) -> Self {
        // Normalize ratios
        let total = read_ratio + write_ratio + update_ratio + scan_ratio;
        Self {
            read_ratio: read_ratio / total,
            write_ratio: write_ratio / total,
            update_ratio: update_ratio / total,
            scan_ratio: scan_ratio / total,
        }
    }
}

impl WorkloadGenerator for CustomBenchmarkWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> crate::Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let cumulative_read = self.read_ratio;
        let cumulative_write = cumulative_read + self.write_ratio;
        let cumulative_update = cumulative_write + self.update_ratio;

        let key = format!("key_{}_{}", worker_id, operation_count % 100000);
        let value = format!("value_{}_{}_{}", worker_id, operation_count, chrono::Utc::now().timestamp());

        if rand_val < cumulative_read {
            // Read operation
            crate::Operation::Read {
                key: key.into_bytes(),
            }
        } else if rand_val < cumulative_write {
            // Write operation
            crate::Operation::Insert {
                key: key.into_bytes(),
                value: value.into_bytes(),
            }
        } else if rand_val < cumulative_update {
            // Update operation
            crate::Operation::Update {
                key: key.into_bytes(),
                value: value.into_bytes(),
            }
        } else {
            // Scan operation
            crate::Operation::Scan {
                start_key: key.into_bytes(),
                limit: rng.gen_range(10..100),
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(CustomBenchmarkWorkload {
            read_ratio: self.read_ratio,
            write_ratio: self.write_ratio,
            update_ratio: self.update_ratio,
            scan_ratio: self.scan_ratio,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::KotobaDBRunner;
    use kotoba_db::DB;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_custom_benchmark_workload() {
        let workload = CustomBenchmarkWorkload::new(0.5, 0.3, 0.1, 0.1);

        // Generate a few operations to test
        for i in 0..10 {
            let op = workload.generate_operation(1, i);
            match op {
                crate::Operation::Read { .. } => {} // Valid
                crate::Operation::Insert { .. } => {} // Valid
                crate::Operation::Update { .. } => {} // Valid
                crate::Operation::Scan { .. } => {} // Valid
                _ => panic!("Unexpected operation type"),
            }
        }
    }

    #[tokio::test]
    async fn test_tpc_workload() {
        let workload = TPCWorkload::new("test".to_string(), vec![
            ("read".to_string(), 50.0),
            ("write".to_string(), 50.0),
        ]);

        let op = workload.generate_operation(1, 1);
        // Should generate a valid operation
        match op {
            crate::Operation::Read { .. } | crate::Operation::Insert { .. } => {}
            _ => panic!("Unexpected operation type"),
        }
    }
}

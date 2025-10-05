//! Stress Testing Scenarios
//!
//! This module provides stress testing scenarios designed to push the database
//! to its limits and identify breaking points, performance degradation, and
//! failure modes under extreme conditions.

use crate::runner::KotobaDBRunner;
use crate::workload::WorkloadGenerator;
use crate::{LoadTestConfig, LoadTestResult, run_load_test, Operation};
use std::time::Duration;

/// Hotspot stress test - create access patterns that concentrate on specific data
pub async fn hotspot(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔥 Running Hotspot Stress Test");
    println!("   Testing concentrated access patterns on specific data ranges");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 20,
        warmup_duration: Duration::from_secs(5),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = HotspotWorkload::new(100, 10); // 100 hot keys, 10x access concentration
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Hotspot Stress", &result);
    Ok(())
}

/// Large value stress test - operations with very large data values
pub async fn large_values(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📦 Running Large Values Stress Test");
    println!("   Testing operations with very large data values (1MB+)");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 5, // Lower concurrency due to large data sizes
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = LargeValueWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Large Values Stress", &result);
    Ok(())
}

/// Maximum throughput stress test - push to absolute performance limits
pub async fn max_throughput(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Running Maximum Throughput Stress Test");
    println!("   Pushing database to absolute performance limits");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 200, // Very high concurrency
        warmup_duration: Duration::from_secs(15),
        operations_per_second: None, // No rate limiting - go as fast as possible
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = MaxThroughputWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Max Throughput Stress", &result);

    // Additional analysis for max throughput test
    if result.error_rate > 0.1 {
        println!("⚠️  High error rate ({:.1}%) detected under max throughput", result.error_rate * 100.0);
    }

    if result.latency_percentiles.p99 > 100_000 { // 100ms
        println!("⚠️  Very high latency (P99: {}μs) under max throughput", result.latency_percentiles.p99);
    }

    Ok(())
}

/// Memory pressure stress test - operations under memory constraints
pub async fn memory_pressure(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("💾 Running Memory Pressure Stress Test");
    println!("   Testing database behavior under memory pressure");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 50,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: Some(5000), // Rate limited to build memory pressure
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = MemoryPressureWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Memory Pressure Stress", &result);
    Ok(())
}

/// Disk I/O stress test - operations that maximize disk access
pub async fn disk_io_stress(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("💿 Running Disk I/O Stress Test");
    println!("   Testing database under maximum disk I/O load");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 30,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = DiskIOWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Disk I/O Stress", &result);
    Ok(())
}

/// Connection storm stress test - rapid connection creation/destruction
pub async fn connection_storm(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🌪️  Running Connection Storm Stress Test");
    println!("   Testing rapid connection creation and destruction");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 100,
        warmup_duration: Duration::from_secs(5),
        operations_per_second: Some(2000),
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = ConnectionStormWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Connection Storm Stress", &result);
    Ok(())
}

/// Complex transaction stress test - nested transactions with conflicts
pub async fn transaction_conflicts(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Running Transaction Conflict Stress Test");
    println!("   Testing database under high transaction conflict scenarios");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 25,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = TransactionConflictWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Transaction Conflict Stress", &result);

    // Additional analysis for transaction conflicts
    if result.error_rate > 0.05 {
        println!("⚠️  High conflict rate ({:.1}%) detected", result.error_rate * 100.0);
    }

    Ok(())
}

/// Long-running operation stress test - operations that take extended time
pub async fn long_running_operations(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("⏱️  Running Long-Running Operations Stress Test");
    println!("   Testing database with extended-duration operations");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 10, // Lower concurrency for long-running ops
        warmup_duration: Duration::from_secs(15),
        operations_per_second: Some(100), // Low rate for long operations
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = LongRunningWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_stress_results("Long-Running Operations Stress", &result);

    // Additional analysis for long-running operations
    if result.latency_percentiles.p95 > 1_000_000 { // 1 second
        println!("⚠️  Very long operations detected (P95: {:.1}s)", result.latency_percentiles.p95 as f64 / 1_000_000.0);
    }

    Ok(())
}

/// Print stress test results with additional analysis
fn print_stress_results(test_name: &str, result: &LoadTestResult) {
    println!("📊 {} Results:", test_name);
    println!("   Throughput: {:.1} ops/sec", result.operations_per_second);
    println!("   Total Operations: {}", result.total_operations);
    println!("   Duration: {:.2}s", result.duration.as_secs_f64());
    println!("   Latency (μs): P50={}, P95={}, P99={}, Max={}",
             result.latency_percentiles.p50,
             result.latency_percentiles.p95,
             result.latency_percentiles.p99,
             result.latency_percentiles.max);
    println!("   Error Rate: {:.3}%", result.error_rate * 100.0);

    // Stress test specific analysis
    if result.error_rate > 0.05 {
        println!("   ⚠️  High error rate indicates stress test effectiveness");
    }

    if result.latency_percentiles.p99 > 100_000 { // 100ms
        println!("   ⚠️  High latency indicates performance limits reached");
    }

    if result.operations_per_second > 10000.0 {
        println!("   ✅ Excellent throughput under stress conditions");
    }

    println!();
}

// Stress test workload implementations

/// Hotspot workload - concentrates access on specific keys
pub struct HotspotWorkload {
    hot_key_count: usize,
    concentration_factor: usize,
    operation_count: std::sync::atomic::AtomicU64,
}

impl HotspotWorkload {
    pub fn new(hot_key_count: usize, concentration_factor: usize) -> Self {
        Self {
            hot_key_count,
            concentration_factor,
            operation_count: std::sync::atomic::AtomicU64::new(0),
        }
    }
}

impl WorkloadGenerator for HotspotWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let current_count = self.operation_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        // Create hotspot access pattern
        let is_hot_access = (current_count % (self.concentration_factor as u64)) == 0;
        let key_id = if is_hot_access {
            // Access one of the hot keys
            rng.gen_range(0..self.hot_key_count)
        } else {
            // Access a random key
            rng.gen_range(0..10000)
        };

        let key = format!("hotspot_key_{}", key_id);
        let value = format!("value_{}_{}_{}", key_id, worker_id, current_count);

        match rng.gen_range(0..10) {
            0..=6 => Operation::Read { key: key.into_bytes() },
            7..=8 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            _ => Operation::Insert { key: format!("hotspot_key_{}_{}", key_id, current_count).into_bytes(), value: value.into_bytes() },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(HotspotWorkload {
            hot_key_count: self.hot_key_count,
            concentration_factor: self.concentration_factor,
            operation_count: std::sync::atomic::AtomicU64::new(0),
        })
    }
}

/// Large value workload - operations with very large data
pub struct LargeValueWorkload;

impl LargeValueWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for LargeValueWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Generate large values (1KB to 1MB)
        let value_size = rng.gen_range(1024..1_048_576);
        let value: Vec<u8> = (0..value_size).map(|i| (i % 256) as u8).collect();

        let key = format!("large_value_key_{}_{}", worker_id, operation_count);

        match rng.gen_range(0..5) {
            0..=2 => Operation::Read { key: key.into_bytes() },
            3 => Operation::Update { key: key.into_bytes(), value },
            _ => Operation::Insert { key: key.into_bytes(), value },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(LargeValueWorkload)
    }
}

/// Maximum throughput workload - optimized for raw speed
pub struct MaxThroughputWorkload;

impl MaxThroughputWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for MaxThroughputWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let key = format!("throughput_key_{}_{}", worker_id, operation_count % 100000);
        let value = format!("value_{}_{}", worker_id, operation_count);

        // Bias towards reads for maximum throughput
        match rng.gen_range(0..100) {
            0..=70 => Operation::Read { key: key.into_bytes() },
            71..=85 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            _ => Operation::Insert { key: format!("throughput_key_{}_{}", worker_id, operation_count).into_bytes(), value: value.into_bytes() },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(MaxThroughputWorkload)
    }
}

/// Memory pressure workload - operations designed to build memory pressure
pub struct MemoryPressureWorkload;

impl MemoryPressureWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for MemoryPressureWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        let key = format!("memory_key_{}_{}", worker_id, operation_count % 50000);
        let value = format!("memory_value_{}_{}_{}", worker_id, operation_count, "x".repeat(1000)); // 1KB values

        match rng.gen_range(0..10) {
            0..=3 => Operation::Read { key: key.into_bytes() },
            4..=7 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            _ => Operation::Insert { key: format!("memory_key_{}_{}", worker_id, operation_count).into_bytes(), value: value.into_bytes() },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(MemoryPressureWorkload)
    }
}

/// Disk I/O workload - operations designed to maximize disk access
pub struct DiskIOWorkload;

impl DiskIOWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for DiskIOWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Use scattered keys to maximize disk seeks
        let key_id = rng.gen_range(0..500000);
        let key = format!("disk_key_{:06}", key_id); // Fixed-width for consistent key distribution
        let value = format!("disk_value_{}_{}_{}", key_id, worker_id, operation_count);

        match rng.gen_range(0..20) {
            0..=10 => Operation::Read { key: key.into_bytes() },
            11..=15 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            16..=18 => Operation::Insert { key: format!("disk_key_{:06}_{}", key_id, operation_count).into_bytes(), value: value.into_bytes() },
            _ => Operation::Scan { start_key: key.into_bytes(), limit: rng.gen_range(50..200) },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(DiskIOWorkload)
    }
}

/// Connection storm workload - simulates rapid connection turnover
pub struct ConnectionStormWorkload;

impl ConnectionStormWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for ConnectionStormWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Use session-based keys to simulate connection sessions
        let session_id = operation_count % 1000; // Reuse session IDs to simulate connection reuse
        let key = format!("session_{}_key_{}", session_id, rng.gen_range(0..100));
        let value = format!("session_data_{}_{}_{}", session_id, worker_id, operation_count);

        match rng.gen_range(0..8) {
            0..=5 => Operation::Read { key: key.into_bytes() },
            6 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            _ => Operation::Insert { key: format!("session_{}_key_{}", session_id, operation_count).into_bytes(), value: value.into_bytes() },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(ConnectionStormWorkload)
    }
}

/// Transaction conflict workload - operations designed to create transaction conflicts
pub struct TransactionConflictWorkload;

impl TransactionConflictWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for TransactionConflictWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Use a small set of keys that will conflict frequently
        let shared_key_id = rng.gen_range(0..50); // Only 50 keys for high conflict probability
        let key = format!("conflict_key_{}", shared_key_id);
        let value = format!("conflict_value_{}_{}_{}", shared_key_id, worker_id, operation_count);

        match rng.gen_range(0..5) {
            0..=2 => Operation::Read { key: key.into_bytes() },
            3 => Operation::Update { key: key.into_bytes(), value: value.into_bytes() },
            _ => Operation::Insert { key: format!("conflict_key_{}_{}", shared_key_id, operation_count).into_bytes(), value: value.into_bytes() },
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(TransactionConflictWorkload)
    }
}

/// Long-running operations workload - operations that take extended time
pub struct LongRunningWorkload;

impl LongRunningWorkload {
    pub fn new() -> Self {
        Self
    }
}

impl WorkloadGenerator for LongRunningWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();

        // Generate operations that will result in long-running queries
        match rng.gen_range(0..10) {
            0..=3 => {
                // Range scan with large result set
                let start_key = format!("range_start_{}", rng.gen_range(0..1000));
                Operation::Scan { start_key: start_key.into_bytes(), limit: rng.gen_range(1000..10000) }
            }
            4..=6 => {
                // Read with complex key pattern
                let key = format!("complex_key_{}_{}_{}", worker_id, operation_count, "x".repeat(100));
                Operation::Read { key: key.into_bytes() }
            }
            7..=8 => {
                // Large value update
                let key = format!("large_update_{}_{}", worker_id, operation_count);
                let value = format!("large_value_{}_{}_{}", worker_id, operation_count, "x".repeat(10000));
                Operation::Update { key: key.into_bytes(), value: value.into_bytes() }
            }
            _ => {
                // Complex insert with large data
                let key = format!("complex_insert_{}_{}", worker_id, operation_count);
                let value = format!("complex_data_{}_{}_{}", worker_id, operation_count, "x".repeat(5000));
                Operation::Insert { key: key.into_bytes(), value: value.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(LongRunningWorkload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotspot_workload() {
        let workload = HotspotWorkload::new(10, 5);

        for i in 0..10 {
            let op = workload.generate_operation(1, i);
            match op {
                Operation::Read { .. } | Operation::Update { .. } | Operation::Insert { .. } => {}
                _ => panic!("Unexpected operation type"),
            }
        }
    }

    #[test]
    fn test_large_value_workload() {
        let workload = LargeValueWorkload::new();

        let op = workload.generate_operation(1, 1);
        match op {
            Operation::Read { .. } | Operation::Update { .. } | Operation::Insert { .. } => {}
            _ => panic!("Unexpected operation type"),
        }
    }

    #[test]
    fn test_max_throughput_workload() {
        let workload = MaxThroughputWorkload::new();

        let op = workload.generate_operation(1, 1);
        match op {
            Operation::Read { .. } | Operation::Update { .. } | Operation::Insert { .. } => {}
            _ => panic!("Unexpected operation type"),
        }
    }
}

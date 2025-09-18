//! I/O Profiler
//!
//! Storage and network I/O profiling for performance analysis.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

/// I/O profiler for tracking storage and network operations
pub struct IoProfiler {
    operations: Arc<Mutex<Vec<IoOperation>>>,
    is_running: Arc<Mutex<bool>>,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoOperation {
    pub id: u64,
    pub operation_type: IoOperationType,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub bytes_transferred: u64,
    pub latency_us: Option<u64>,
    pub success: bool,
    pub error_message: Option<String>,
    pub file_path: Option<String>,
    pub thread_id: u64,
    pub stack_trace: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IoOperationType {
    Read,
    Write,
    Flush,
    Sync,
    Open,
    Close,
    Seek,
    Fsync,
    NetworkRead,
    NetworkWrite,
    DatabaseQuery,
    DatabaseWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoSnapshot {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub active_operations: usize,
    pub bytes_read_per_second: f64,
    pub bytes_written_per_second: f64,
    pub average_latency_us: f64,
    pub p95_latency_us: f64,
    pub queue_depth: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IoAnalysis {
    pub total_operations: usize,
    pub total_bytes_read: u64,
    pub total_bytes_written: u64,
    pub average_throughput_mbps: f64,
    pub average_latency_us: f64,
    pub p95_latency_us: u64,
    pub p99_latency_us: u64,
    pub operation_breakdown: HashMap<String, OperationStats>,
    pub bottleneck_analysis: IoBottleneckAnalysis,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationStats {
    pub count: usize,
    pub total_bytes: u64,
    pub average_latency_us: f64,
    pub success_rate: f64,
    pub throughput_mbps: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoBottleneckAnalysis {
    pub is_disk_bound: bool,
    pub is_network_bound: bool,
    pub is_cpu_bound: bool,
    pub sequential_vs_random_ratio: f64,
    pub average_queue_depth: f64,
    pub iops: f64,
    pub bandwidth_saturation: f64,
}

impl IoProfiler {
    pub fn new() -> Self {
        Self {
            operations: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
            _handle: None,
        }
    }

    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("I/O profiler is already running".into());
        }
        *is_running = true;

        // In a real implementation, you would instrument I/O operations
        // For this example, we'll simulate I/O profiling
        let operations = Arc::clone(&self.operations);
        let is_running_clone = Arc::clone(&self.is_running);

        self._handle = Some(tokio::spawn(async move {
            while *is_running_clone.lock().unwrap() {
                tokio::time::sleep(Duration::from_millis(10)).await;

                // Simulate I/O operations for demonstration
                if rand::random::<f32>() < 0.1 { // 10% chance per tick
                    let operation = Self::simulate_io_operation();
                    operations.lock().unwrap().push(operation);
                }
            }
        }));

        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if !*is_running {
            return Err("I/O profiler is not running".into());
        }
        *is_running = false;

        if let Some(handle) = self._handle.take() {
            let _ = tokio::spawn(async move { let _ = handle.await; });
        }

        Ok(())
    }

    pub async fn record_operation(&self, operation: IoOperation) {
        self.operations.lock().unwrap().push(operation);
    }

    pub fn snapshot(&self) -> IoSnapshot {
        let operations = self.operations.lock().unwrap();

        let recent_operations: Vec<&IoOperation> = operations.iter()
            .filter(|op| {
                let age = chrono::Utc::now() - op.start_time;
                age.num_seconds() < 60 // Last 60 seconds
            })
            .collect();

        let active_operations = recent_operations.iter()
            .filter(|op| op.end_time.is_none())
            .count();

        let completed_operations: Vec<&IoOperation> = recent_operations.iter()
            .filter(|op| op.end_time.is_some())
            .cloned()
            .collect();

        let bytes_read_per_second = self.calculate_bytes_per_second(&completed_operations, IoOperationType::Read);
        let bytes_written_per_second = self.calculate_bytes_per_second(&completed_operations, IoOperationType::Write);

        let latencies: Vec<u64> = completed_operations.iter()
            .filter_map(|op| op.latency_us)
            .collect();

        let (average_latency_us, p95_latency_us) = if !latencies.is_empty() {
            let avg = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
            let mut sorted_latencies = latencies.clone();
            sorted_latencies.sort_unstable();
            let p95_idx = (sorted_latencies.len() as f64 * 0.95) as usize;
            let p95 = sorted_latencies.get(p95_idx).copied().unwrap_or(0);
            (avg, p95)
        } else {
            (0.0, 0)
        };

        IoSnapshot {
            timestamp: chrono::Utc::now(),
            active_operations,
            bytes_read_per_second,
            bytes_written_per_second,
            average_latency_us,
            p95_latency_us,
            queue_depth: active_operations,
        }
    }

    pub fn analyze(&self) -> Result<IoAnalysis, Box<dyn std::error::Error>> {
        let operations = self.operations.lock().unwrap();

        let total_operations = operations.len();

        let mut total_bytes_read = 0u64;
        let mut total_bytes_written = 0u64;
        let mut latencies = Vec::new();

        for op in operations.iter() {
            match op.operation_type {
                IoOperationType::Read | IoOperationType::NetworkRead | IoOperationType::DatabaseQuery => {
                    total_bytes_read += op.bytes_transferred;
                }
                IoOperationType::Write | IoOperationType::NetworkWrite | IoOperationType::DatabaseWrite => {
                    total_bytes_written += op.bytes_transferred;
                }
                _ => {}
            }

            if let Some(latency) = op.latency_us {
                latencies.push(latency);
            }
        }

        // Calculate timing metrics
        let duration = if !operations.is_empty() {
            let start = operations.first().unwrap().start_time;
            let end = operations.last().unwrap().end_time.unwrap_or(chrono::Utc::now());
            (end - start).to_std().unwrap_or_default()
        } else {
            Duration::from_secs(1) // Avoid division by zero
        };

        let total_bytes = total_bytes_read + total_bytes_written;
        let average_throughput_mbps = (total_bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0); // Mbps

        let (average_latency_us, p95_latency_us, p99_latency_us) = if !latencies.is_empty() {
            let mut sorted_latencies = latencies.clone();
            sorted_latencies.sort_unstable();

            let avg = latencies.iter().sum::<u64>() as f64 / latencies.len() as f64;
            let p95_idx = (sorted_latencies.len() as f64 * 0.95) as usize;
            let p99_idx = (sorted_latencies.len() as f64 * 0.99) as usize;

            let p95 = sorted_latencies.get(p95_idx).copied().unwrap_or(0);
            let p99 = sorted_latencies.get(p99_idx).copied().unwrap_or(0);

            (avg, p95, p99)
        } else {
            (0.0, 0, 0)
        };

        let operation_breakdown = self.analyze_operation_breakdown(&operations, duration);
        let bottleneck_analysis = self.analyze_bottlenecks(&operations, average_throughput_mbps);
        let recommendations = self.generate_recommendations(&bottleneck_analysis, average_latency_us);

        Ok(IoAnalysis {
            total_operations,
            total_bytes_read,
            total_bytes_written,
            average_throughput_mbps,
            average_latency_us,
            p95_latency_us,
            p99_latency_us,
            operation_breakdown,
            bottleneck_analysis,
            recommendations,
        })
    }

    fn simulate_io_operation() -> IoOperation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let operation_types = [
            IoOperationType::Read,
            IoOperationType::Write,
            IoOperationType::DatabaseQuery,
            IoOperationType::DatabaseWrite,
        ];

        let op_type = operation_types[rng.gen_range(0..operation_types.len())];
        let bytes = rng.gen_range(100..10000);
        let latency_us = rng.gen_range(100..50000); // 0.1ms to 50ms

        IoOperation {
            id: rng.gen(),
            operation_type: op_type,
            start_time: chrono::Utc::now(),
            end_time: Some(chrono::Utc::now()),
            bytes_transferred: bytes,
            latency_us: Some(latency_us),
            success: rng.gen_bool(0.98), // 98% success rate
            error_message: None,
            file_path: Some(format!("/tmp/file_{}.db", rng.gen::<u32>())),
            thread_id: rng.gen(),
            stack_trace: vec![
                "kotoba_db::storage::read_block".to_string(),
                "rocksdb::db::get".to_string(),
                "std::io::read".to_string(),
            ],
        }
    }

    fn calculate_bytes_per_second(&self, operations: &[&IoOperation], op_type: IoOperationType) -> f64 {
        let relevant_ops: Vec<_> = operations.iter()
            .filter(|op| std::mem::discriminant(&op.operation_type) == std::mem::discriminant(&op_type))
            .collect();

        if relevant_ops.is_empty() {
            return 0.0;
        }

        let total_bytes: u64 = relevant_ops.iter().map(|op| op.bytes_transferred).sum();

        // Assume operations are from the last 1 second for simplicity
        total_bytes as f64
    }

    fn analyze_operation_breakdown(&self, operations: &[IoOperation], duration: Duration) -> HashMap<String, OperationStats> {
        let mut breakdown: HashMap<String, Vec<&IoOperation>> = HashMap::new();

        for op in operations {
            let key = format!("{:?}", op.operation_type);
            breakdown.entry(key).or_insert(Vec::new()).push(op);
        }

        let mut result = HashMap::new();

        for (op_type, ops) in breakdown {
            let count = ops.len();
            let total_bytes: u64 = ops.iter().map(|op| op.bytes_transferred).sum();
            let successful_ops = ops.iter().filter(|op| op.success).count();
            let success_rate = successful_ops as f64 / count as f64;

            let latencies: Vec<u64> = ops.iter().filter_map(|op| op.latency_us).collect();
            let average_latency_us = if !latencies.is_empty() {
                latencies.iter().sum::<u64>() as f64 / latencies.len() as f64
            } else {
                0.0
            };

            let throughput_mbps = (total_bytes as f64 * 8.0) / (duration.as_secs_f64() * 1_000_000.0);

            result.insert(op_type, OperationStats {
                count,
                total_bytes,
                average_latency_us,
                success_rate,
                throughput_mbps,
            });
        }

        result
    }

    fn analyze_bottlenecks(&self, operations: &[IoOperation], throughput_mbps: f64) -> IoBottleneckAnalysis {
        // Simplified bottleneck analysis
        // In a real implementation, you would analyze I/O patterns more thoroughly

        let read_ops: Vec<_> = operations.iter()
            .filter(|op| matches!(op.operation_type, IoOperationType::Read | IoOperationType::DatabaseQuery))
            .collect();

        let write_ops: Vec<_> = operations.iter()
            .filter(|op| matches!(op.operation_type, IoOperationType::Write | IoOperationType::DatabaseWrite))
            .collect();

        // Estimate if disk-bound (high latency, low throughput)
        let avg_read_latency = read_ops.iter()
            .filter_map(|op| op.latency_us)
            .sum::<u64>() as f64 / read_ops.len().max(1) as f64;

        let is_disk_bound = avg_read_latency > 10000.0 && throughput_mbps < 100.0; // 10ms avg, <100Mbps

        // Estimate if network-bound (this is simplified)
        let is_network_bound = operations.iter()
            .any(|op| matches!(op.operation_type, IoOperationType::NetworkRead | IoOperationType::NetworkWrite));

        // CPU bound if very low latency but high operation count
        let total_ops = operations.len();
        let avg_latency = operations.iter()
            .filter_map(|op| op.latency_us)
            .sum::<u64>() as f64 / total_ops.max(1) as f64;

        let is_cpu_bound = avg_latency < 100.0 && total_ops > 10000; // <100μs avg, >10k ops

        // Sequential vs random (simplified heuristic)
        let sequential_vs_random_ratio = 0.7; // Placeholder

        let average_queue_depth = operations.iter()
            .filter(|op| op.end_time.is_none())
            .count() as f64;

        let iops = total_ops as f64 / 60.0; // Assume 60 second window

        // Estimate bandwidth saturation (simplified)
        let bandwidth_saturation = if throughput_mbps > 800.0 { 0.9 } else { throughput_mbps / 800.0 };

        IoBottleneckAnalysis {
            is_disk_bound,
            is_network_bound,
            is_cpu_bound,
            sequential_vs_random_ratio,
            average_queue_depth,
            iops,
            bandwidth_saturation,
        }
    }

    fn generate_recommendations(&self, bottlenecks: &IoBottleneckAnalysis, avg_latency: f64) -> Vec<String> {
        let mut recommendations = Vec::new();

        if bottlenecks.is_disk_bound {
            recommendations.push("Disk I/O appears to be a bottleneck. Consider SSD upgrade or RAID configuration.".to_string());
        }

        if bottlenecks.is_network_bound {
            recommendations.push("Network I/O is significant. Consider network optimization or CDN usage.".to_string());
        }

        if bottlenecks.is_cpu_bound {
            recommendations.push("CPU appears to be the bottleneck. Consider CPU upgrade or query optimization.".to_string());
        }

        if avg_latency > 10000.0 { // 10ms
            recommendations.push("High I/O latency detected. Consider optimizing storage access patterns.".to_string());
        }

        if bottlenecks.sequential_vs_random_ratio < 0.3 {
            recommendations.push("High random I/O detected. Consider sequential access patterns or caching.".to_string());
        }

        if bottlenecks.average_queue_depth > 10.0 {
            recommendations.push("High I/O queue depth. Consider increasing I/O parallelism or optimizing concurrent operations.".to_string());
        }

        if bottlenecks.bandwidth_saturation > 0.8 {
            recommendations.push("Storage bandwidth nearly saturated. Consider storage upgrade or load distribution.".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("I/O performance appears normal. No specific recommendations.".to_string());
        }

        recommendations
    }
}

impl IoAnalysis {
    /// Calculate I/O efficiency score (0.0-1.0, higher is better)
    pub fn io_efficiency_score(&self) -> f64 {
        let latency_score = if self.average_latency_us < 1000.0 { 1.0 } // <1ms excellent
                          else if self.average_latency_us < 10000.0 { 0.7 } // <10ms good
                          else { 0.4 }; // >10ms poor

        let throughput_score = if self.average_throughput_mbps > 500.0 { 1.0 } // >500Mbps excellent
                             else if self.average_throughput_mbps > 100.0 { 0.7 } // >100Mbps good
                             else { 0.4 }; // <100Mbps poor

        (latency_score + throughput_score) / 2.0
    }

    /// Get operations with highest latency
    pub fn highest_latency_operations(&self) -> Vec<(String, u64)> {
        // This would require storing operation details
        // For now, return placeholder data
        vec![
            ("DatabaseQuery".to_string(), self.p95_latency_us),
            ("Write".to_string(), self.p99_latency_us),
        ]
    }

    /// Check if I/O performance meets requirements
    pub fn meets_performance_requirements(&self, max_p95_latency_us: u64, min_throughput_mbps: f64) -> bool {
        self.p95_latency_us <= max_p95_latency_us && self.average_throughput_mbps >= min_throughput_mbps
    }
}

//! System Monitor
//!
//! System resource monitoring for CPU, memory, disk, and network usage.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use sysinfo::{CpuExt, DiskExt, NetworkExt, ProcessExt, System, SystemExt};

/// System resource monitor
pub struct SystemMonitor {
    system: Arc<Mutex<System>>,
    snapshots: Arc<Mutex<Vec<SystemMetrics>>>,
    is_running: Arc<Mutex<bool>>,
    sampling_interval: Duration,
    _handle: Option<tokio::task::JoinHandle<()>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub cpu_usage_percent: f64,
    pub memory_usage_percent: f64,
    pub memory_used_mb: f64,
    pub memory_total_mb: f64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub disk_read_ops: u64,
    pub disk_write_ops: u64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
    pub network_rx_packets: u64,
    pub network_tx_packets: u64,
    pub load_average: f64,
    pub process_count: usize,
    pub thread_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemAnalysis {
    pub monitoring_duration: Duration,
    pub average_cpu_usage: f64,
    pub peak_cpu_usage: f64,
    pub average_memory_usage: f64,
    pub peak_memory_usage: f64,
    pub total_disk_read_mb: f64,
    pub total_disk_write_mb: f64,
    pub total_network_rx_mb: f64,
    pub total_network_tx_mb: f64,
    pub resource_trends: ResourceTrends,
    pub bottlenecks: Vec<SystemBottleneck>,
    pub utilization_patterns: UtilizationPatterns,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTrends {
    pub cpu_trend: Trend,
    pub memory_trend: Trend,
    pub disk_trend: Trend,
    pub network_trend: Trend,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Trend {
    Increasing,
    Decreasing,
    Stable,
    Fluctuating,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemBottleneck {
    pub resource_type: ResourceType,
    pub severity: Severity,
    pub description: String,
    pub utilization_percent: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Cpu,
    Memory,
    Disk,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UtilizationPatterns {
    pub peak_hours: Vec<u8>, // Hours of day with highest usage
    pub cpu_spike_frequency: f64,
    pub memory_growth_rate: f64,
    pub io_burst_pattern: bool,
    pub network_burst_pattern: bool,
}

impl SystemMonitor {
    pub fn new() -> Self {
        let mut system = System::new_all();
        system.refresh_all();

        Self {
            system: Arc::new(Mutex::new(system)),
            snapshots: Arc::new(Mutex::new(Vec::new())),
            is_running: Arc::new(Mutex::new(false)),
            sampling_interval: Duration::from_millis(1000), // 1 second sampling
            _handle: None,
        }
    }

    pub fn with_sampling_interval(mut self, interval: Duration) -> Self {
        self.sampling_interval = interval;
        self
    }

    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if *is_running {
            return Err("System monitor is already running".into());
        }
        *is_running = true;

        let system = Arc::clone(&self.system);
        let snapshots = Arc::clone(&self.snapshots);
        let sampling_interval = self.sampling_interval;
        let is_running_clone = Arc::clone(&self.is_running);

        self._handle = Some(tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(sampling_interval);

            while *is_running_clone.lock().unwrap() {
                interval_timer.tick().await;

                let snapshot = Self::capture_system_snapshot(&system).await;
                snapshots.lock().unwrap().push(snapshot);
            }
        }));

        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut is_running = self.is_running.lock().unwrap();
        if !*is_running {
            return Err("System monitor is not running".into());
        }
        *is_running = false;

        if let Some(handle) = self._handle.take() {
            let _ = handle.await;
        }

        Ok(())
    }

    pub async fn snapshot(&self) -> SystemMetrics {
        Self::capture_system_snapshot(&self.system).await
    }

    async fn capture_system_snapshot(system: &Arc<Mutex<System>>) -> SystemMetrics {
        let mut sys = system.lock().unwrap();

        // Refresh system information
        sys.refresh_all();

        // CPU usage
        let cpu_usage_percent: f64 = sys.cpus().iter().map(|cpu| cpu.cpu_usage() as f64).sum::<f64>() / sys.cpus().len() as f64;

        // Memory usage
        let memory_used_mb = sys.used_memory() as f64 / 1024.0 / 1024.0;
        let memory_total_mb = sys.total_memory() as f64 / 1024.0 / 1024.0;
        let memory_usage_percent = (memory_used_mb / memory_total_mb) * 100.0;

        // Disk I/O (simplified - real implementation would track deltas)
        let mut disk_read_bytes = 0u64;
        let mut disk_write_bytes = 0u64;
        let mut disk_read_ops = 0u64;
        let mut disk_write_ops = 0u64;

        for disk in sys.disks() {
            // In a real implementation, you would calculate deltas from previous measurements
            // For this example, we'll use placeholder values
            disk_read_bytes += 1024 * 1024; // 1MB placeholder
            disk_write_bytes += 512 * 1024; // 512KB placeholder
            disk_read_ops += 100;
            disk_write_ops += 50;
        }

        // Network I/O
        let mut network_rx_bytes = 0u64;
        let mut network_tx_bytes = 0u64;
        let mut network_rx_packets = 0u64;
        let mut network_tx_packets = 0u64;

        for (_interface_name, network) in sys.networks() {
            network_rx_bytes += network.received();
            network_tx_bytes += network.transmitted();
            // Packet counts would need to be tracked separately
            network_rx_packets += network_rx_bytes / 1500; // Rough packet estimation
            network_tx_packets += network_tx_bytes / 1500;
        }

        // Load average
        let load_average = sys.load_average().one as f64;

        // Process and thread counts
        let process_count = sys.processes().len();
        let thread_count: usize = sys.processes().values()
            .map(|process| process.tasks.len())
            .sum();

        SystemMetrics {
            timestamp: chrono::Utc::now(),
            cpu_usage_percent,
            memory_usage_percent,
            memory_used_mb,
            memory_total_mb,
            disk_read_bytes,
            disk_write_bytes,
            disk_read_ops,
            disk_write_ops,
            network_rx_bytes,
            network_tx_bytes,
            network_rx_packets,
            network_tx_packets,
            load_average,
            process_count,
            thread_count,
        }
    }

    pub async fn analyze(&self) -> Result<SystemAnalysis, Box<dyn std::error::Error>> {
        let snapshots = self.snapshots.lock().unwrap();

        if snapshots.is_empty() {
            return Ok(SystemAnalysis {
                monitoring_duration: Duration::from_secs(0),
                average_cpu_usage: 0.0,
                peak_cpu_usage: 0.0,
                average_memory_usage: 0.0,
                peak_memory_usage: 0.0,
                total_disk_read_mb: 0.0,
                total_disk_write_mb: 0.0,
                total_network_rx_mb: 0.0,
                total_network_tx_mb: 0.0,
                resource_trends: ResourceTrends {
                    cpu_trend: Trend::Stable,
                    memory_trend: Trend::Stable,
                    disk_trend: Trend::Stable,
                    network_trend: Trend::Stable,
                },
                bottlenecks: Vec::new(),
                utilization_patterns: UtilizationPatterns {
                    peak_hours: Vec::new(),
                    cpu_spike_frequency: 0.0,
                    memory_growth_rate: 0.0,
                    io_burst_pattern: false,
                    network_burst_pattern: false,
                },
                recommendations: vec!["No system monitoring data available".to_string()],
            });
        }

        let monitoring_duration = if snapshots.len() >= 2 {
            (snapshots.last().unwrap().timestamp - snapshots.first().unwrap().timestamp).to_std().unwrap_or_default()
        } else {
            Duration::from_secs(1)
        };

        // Calculate averages and peaks
        let average_cpu_usage = snapshots.iter().map(|s| s.cpu_usage_percent).sum::<f64>() / snapshots.len() as f64;
        let peak_cpu_usage = snapshots.iter().map(|s| s.cpu_usage_percent).fold(0.0, f64::max);

        let average_memory_usage = snapshots.iter().map(|s| s.memory_usage_percent).sum::<f64>() / snapshots.len() as f64;
        let peak_memory_usage = snapshots.iter().map(|s| s.memory_usage_percent).fold(0.0, f64::max);

        let total_disk_read_mb = snapshots.iter().map(|s| s.disk_read_bytes as f64 / 1024.0 / 1024.0).sum::<f64>();
        let total_disk_write_mb = snapshots.iter().map(|s| s.disk_write_bytes as f64 / 1024.0 / 1024.0).sum::<f64>();

        let total_network_rx_mb = snapshots.iter().map(|s| s.network_rx_bytes as f64 / 1024.0 / 1024.0).sum::<f64>();
        let total_network_tx_mb = snapshots.iter().map(|s| s.network_tx_bytes as f64 / 1024.0 / 1024.0).sum::<f64>();

        // Analyze resource trends
        let resource_trends = self.analyze_resource_trends(&snapshots);

        // Identify bottlenecks
        let bottlenecks = self.identify_bottlenecks(&snapshots);

        // Analyze utilization patterns
        let utilization_patterns = self.analyze_utilization_patterns(&snapshots);

        // Generate recommendations
        let recommendations = self.generate_recommendations(
            average_cpu_usage,
            peak_cpu_usage,
            average_memory_usage,
            peak_memory_usage,
            &bottlenecks,
        );

        Ok(SystemAnalysis {
            monitoring_duration,
            average_cpu_usage,
            peak_cpu_usage,
            average_memory_usage,
            peak_memory_usage,
            total_disk_read_mb,
            total_disk_write_mb,
            total_network_rx_mb,
            total_network_tx_mb,
            resource_trends,
            bottlenecks,
            utilization_patterns,
            recommendations,
        })
    }

    fn analyze_resource_trends(&self, snapshots: &[SystemMetrics]) -> ResourceTrends {
        if snapshots.len() < 3 {
            return ResourceTrends {
                cpu_trend: Trend::Stable,
                memory_trend: Trend::Stable,
                disk_trend: Trend::Stable,
                network_trend: Trend::Stable,
            };
        }

        let mid_point = snapshots.len() / 2;
        let first_half_avg = &snapshots[0..mid_point];
        let second_half_avg = &snapshots[mid_point..];

        ResourceTrends {
            cpu_trend: self.calculate_trend(
                first_half_avg.iter().map(|s| s.cpu_usage_percent).collect(),
                second_half_avg.iter().map(|s| s.cpu_usage_percent).collect(),
            ),
            memory_trend: self.calculate_trend(
                first_half_avg.iter().map(|s| s.memory_usage_percent).collect(),
                second_half_avg.iter().map(|s| s.memory_usage_percent).collect(),
            ),
            disk_trend: self.calculate_trend(
                first_half_avg.iter().map(|s| s.disk_read_bytes as f64).collect(),
                second_half_avg.iter().map(|s| s.disk_read_bytes as f64).collect(),
            ),
            network_trend: self.calculate_trend(
                first_half_avg.iter().map(|s| s.network_rx_bytes as f64).collect(),
                second_half_avg.iter().map(|s| s.network_rx_bytes as f64).collect(),
            ),
        }
    }

    fn calculate_trend(&self, first_half: Vec<f64>, second_half: Vec<f64>) -> Trend {
        if first_half.is_empty() || second_half.is_empty() {
            return Trend::Stable;
        }

        let first_avg = first_half.iter().sum::<f64>() / first_half.len() as f64;
        let second_avg = second_half.iter().sum::<f64>() / second_half.len() as f64;

        let change_percent = ((second_avg - first_avg) / first_avg.abs().max(0.1)) * 100.0;

        match change_percent.abs() {
            x if x < 5.0 => Trend::Stable,
            x if x < 20.0 => {
                if change_percent > 0.0 {
                    Trend::Increasing
                } else {
                    Trend::Decreasing
                }
            }
            _ => Trend::Fluctuating,
        }
    }

    fn identify_bottlenecks(&self, snapshots: &[SystemMetrics]) -> Vec<SystemBottleneck> {
        let mut bottlenecks = Vec::new();

        // CPU bottlenecks
        let high_cpu_snapshots: Vec<_> = snapshots.iter()
            .filter(|s| s.cpu_usage_percent > 80.0)
            .collect();

        if !high_cpu_snapshots.is_empty() {
            let duration_seconds = high_cpu_snapshots.len() as f64; // Assuming 1 second per snapshot
            let avg_cpu = high_cpu_snapshots.iter().map(|s| s.cpu_usage_percent).sum::<f64>() / high_cpu_snapshots.len() as f64;

            bottlenecks.push(SystemBottleneck {
                resource_type: ResourceType::Cpu,
                severity: if avg_cpu > 95.0 { Severity::Critical } else { Severity::High },
                description: format!("CPU usage exceeded 80% for {:.1} seconds", duration_seconds),
                utilization_percent: avg_cpu,
                duration_seconds,
            });
        }

        // Memory bottlenecks
        let high_memory_snapshots: Vec<_> = snapshots.iter()
            .filter(|s| s.memory_usage_percent > 85.0)
            .collect();

        if !high_memory_snapshots.is_empty() {
            let duration_seconds = high_memory_snapshots.len() as f64;
            let avg_memory = high_memory_snapshots.iter().map(|s| s.memory_usage_percent).sum::<f64>() / high_memory_snapshots.len() as f64;

            bottlenecks.push(SystemBottleneck {
                resource_type: ResourceType::Memory,
                severity: if avg_memory > 95.0 { Severity::Critical } else { Severity::High },
                description: format!("Memory usage exceeded 85% for {:.1} seconds", duration_seconds),
                utilization_percent: avg_memory,
                duration_seconds,
            });
        }

        // Sort by severity
        bottlenecks.sort_by(|a, b| std::cmp::Reverse(b.severity.clone()).cmp(&std::cmp::Reverse(a.severity.clone())));

        bottlenecks
    }

    fn analyze_utilization_patterns(&self, snapshots: &[SystemMetrics]) -> UtilizationPatterns {
        let cpu_spikes: Vec<_> = snapshots.iter()
            .filter(|s| s.cpu_usage_percent > 70.0)
            .collect();

        let cpu_spike_frequency = cpu_spikes.len() as f64 / snapshots.len() as f64;

        // Simple memory growth rate calculation
        let memory_growth_rate = if snapshots.len() >= 2 {
            let first_memory = snapshots.first().unwrap().memory_used_mb;
            let last_memory = snapshots.last().unwrap().memory_used_mb;
            let time_diff_hours = snapshots.len() as f64 / 3600.0; // Assuming 1 snapshot per second
            (last_memory - first_memory) / time_diff_hours
        } else {
            0.0
        };

        // Detect burst patterns (simplified)
        let io_burst_pattern = snapshots.windows(3).any(|window| {
            let avg_io = window.iter().map(|s| s.disk_read_bytes + s.disk_write_bytes).sum::<u64>() / 3;
            avg_io > 1024 * 1024 // 1MB average I/O
        });

        let network_burst_pattern = snapshots.windows(3).any(|window| {
            let avg_network = window.iter().map(|s| s.network_rx_bytes + s.network_tx_bytes).sum::<u64>() / 3;
            avg_network > 1024 * 1024 // 1MB average network
        });

        UtilizationPatterns {
            peak_hours: vec![9, 10, 11, 14, 15, 16], // Placeholder peak hours
            cpu_spike_frequency,
            memory_growth_rate,
            io_burst_pattern,
            network_burst_pattern,
        }
    }

    fn generate_recommendations(
        &self,
        avg_cpu: f64,
        peak_cpu: f64,
        avg_memory: f64,
        peak_memory: f64,
        bottlenecks: &[SystemBottleneck],
    ) -> Vec<String> {
        let mut recommendations = Vec::new();

        // CPU recommendations
        if peak_cpu > 90.0 {
            recommendations.push("Critical CPU usage detected. Consider CPU upgrade or workload distribution.".to_string());
        } else if avg_cpu > 70.0 {
            recommendations.push("High average CPU usage. Consider optimizing CPU-intensive operations.".to_string());
        }

        // Memory recommendations
        if peak_memory > 95.0 {
            recommendations.push("Critical memory usage detected. Risk of out-of-memory errors.".to_string());
        } else if avg_memory > 80.0 {
            recommendations.push("High memory usage. Consider memory optimization or increased RAM.".to_string());
        }

        // Bottleneck-specific recommendations
        for bottleneck in bottlenecks {
            match bottleneck.resource_type {
                ResourceType::Cpu => {
                    recommendations.push("CPU bottleneck detected. Profile and optimize compute-intensive code paths.".to_string());
                }
                ResourceType::Memory => {
                    recommendations.push("Memory bottleneck detected. Check for memory leaks and optimize memory usage.".to_string());
                }
                ResourceType::Disk => {
                    recommendations.push("Disk I/O bottleneck detected. Consider SSD upgrade or I/O optimization.".to_string());
                }
                ResourceType::Network => {
                    recommendations.push("Network bottleneck detected. Check network configuration and bandwidth.".to_string());
                }
            }
        }

        if recommendations.is_empty() {
            recommendations.push("System resource usage appears normal. No specific recommendations.".to_string());
        }

        recommendations
    }
}

impl SystemAnalysis {
    /// Calculate system health score (0.0-1.0, higher is better)
    pub fn system_health_score(&self) -> f64 {
        let cpu_score = 1.0 - (self.average_cpu_usage / 100.0).min(1.0);
        let memory_score = 1.0 - (self.average_memory_usage / 100.0).min(1.0);
        let bottleneck_penalty = (self.bottlenecks.len() as f64 * 0.1).min(0.3);

        (cpu_score + memory_score) / 2.0 - bottleneck_penalty
    }

    /// Check if system resources are within acceptable limits
    pub fn resources_within_limits(&self, cpu_limit: f64, memory_limit: f64) -> bool {
        self.average_cpu_usage <= cpu_limit && self.average_memory_usage <= memory_limit
    }

    /// Get most critical bottlenecks
    pub fn critical_bottlenecks(&self) -> Vec<&SystemBottleneck> {
        self.bottlenecks.iter()
            .filter(|b| matches!(b.severity, Severity::Critical))
            .collect()
    }
}

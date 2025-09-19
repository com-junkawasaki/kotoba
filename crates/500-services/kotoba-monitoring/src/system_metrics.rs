//! # System Metrics
//!
//! System-level metrics collection for KotobaDB monitoring.

use crate::*;
use std::collections::HashMap;
use tokio::time::{self, Duration};

/// System metrics collector (when `system` feature is enabled)
#[cfg(feature = "system")]
pub struct SystemMetricsCollector {
    /// Collection interval
    interval: Duration,
    /// Last collected metrics
    last_metrics: Arc<RwLock<Option<SystemMetrics>>>,
}

#[cfg(feature = "system")]
impl SystemMetricsCollector {
    pub fn new(interval: Duration) -> Self {
        Self {
            interval,
            last_metrics: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn start_collection(&self) -> Result<(), MonitoringError> {
        let last_metrics = Arc::clone(&self.last_metrics);
        let interval = self.interval;

        tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                match Self::collect_system_metrics().await {
                    Ok(metrics) => {
                        let mut last = last_metrics.write().await;
                        *last = Some(metrics);
                    }
                    Err(e) => {
                        eprintln!("Failed to collect system metrics: {:?}", e);
                    }
                }
            }
        });

        Ok(())
    }

    pub async fn get_system_metrics(&self) -> Option<SystemMetrics> {
        self.last_metrics.read().await.clone()
    }

    async fn collect_system_metrics() -> Result<SystemMetrics, MonitoringError> {
        let system = sysinfo::System::new_all();
        system.refresh_all();

        let cpu_usage = system.global_cpu_info().cpu_usage();

        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_percent = (used_memory as f64 / total_memory as f64) * 100.0;

        let mut total_disk = 0u64;
        let mut used_disk = 0u64;
        for disk in system.disks() {
            total_disk += disk.total_space();
            used_disk += total_disk - disk.available_space();
        }
        let disk_usage_percent = if total_disk > 0 {
            (used_disk as f64 / total_disk as f64) * 100.0
        } else {
            0.0
        };

        // Network stats (simplified)
        let network_rx = system.networks().values()
            .map(|net| net.received())
            .sum::<u64>();
        let network_tx = system.networks().values()
            .map(|net| net.transmitted())
            .sum::<u64>();

        Ok(SystemMetrics {
            cpu_usage_percent: cpu_usage as f64,
            memory_usage_bytes: used_memory,
            memory_usage_percent,
            disk_usage_bytes: used_disk,
            disk_usage_percent,
            network_rx_bytes: network_rx,
            network_tx_bytes: network_tx,
        })
    }
}

/// System metrics collector stub (when `system` feature is disabled)
#[cfg(not(feature = "system"))]
pub struct SystemMetricsCollector;

#[cfg(not(feature = "system"))]
impl SystemMetricsCollector {
    pub fn new(_interval: Duration) -> Self {
        Self
    }

    pub async fn start_collection(&self) -> Result<(), MonitoringError> {
        Ok(())
    }

    pub async fn get_system_metrics(&self) -> Option<SystemMetrics> {
        None
    }
}

/// System metrics recording helpers
pub struct SystemMetricsRecorder {
    collector: Arc<MetricsCollector>,
}

impl SystemMetricsRecorder {
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self { collector }
    }

    pub async fn record_system_metrics(&self, metrics: &SystemMetrics) -> Result<(), MonitoringError> {
        let timestamp = Utc::now();

        // Record CPU usage
        self.collector.record_metric(
            "system_cpu_usage_percent",
            metrics.cpu_usage_percent,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        // Record memory metrics
        self.collector.record_metric(
            "system_memory_usage_bytes",
            metrics.memory_usage_bytes as f64,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        self.collector.record_metric(
            "system_memory_usage_percent",
            metrics.memory_usage_percent,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        // Record disk metrics
        self.collector.record_metric(
            "system_disk_usage_bytes",
            metrics.disk_usage_bytes as f64,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        self.collector.record_metric(
            "system_disk_usage_percent",
            metrics.disk_usage_percent,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        // Record network metrics
        self.collector.record_metric(
            "system_network_rx_bytes",
            metrics.network_rx_bytes as f64,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        self.collector.record_metric(
            "system_network_tx_bytes",
            metrics.network_tx_bytes as f64,
            hashmap! { "component".to_string() => "system".to_string() }
        ).await?;

        Ok(())
    }
}

/// System health checks
pub struct SystemHealthChecker;

impl SystemHealthChecker {
    pub async fn check_system_health(&self) -> Vec<HealthCheckResult> {
        let mut results = Vec::new();

        #[cfg(feature = "system")]
        {
            if let Ok(metrics) = SystemMetricsCollector::collect_system_metrics().await {
                // CPU health check
                let cpu_status = if metrics.cpu_usage_percent > 90.0 {
                    HealthStatus::Unhealthy
                } else if metrics.cpu_usage_percent > 80.0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                results.push(HealthCheckResult {
                    name: "system_cpu".to_string(),
                    status: cpu_status,
                    message: format!("CPU usage: {:.1}%", metrics.cpu_usage_percent),
                    duration: Duration::from_millis(1),
                    details: hashmap! {
                        "cpu_usage_percent".to_string() => metrics.cpu_usage_percent.to_string()
                    },
                });

                // Memory health check
                let memory_status = if metrics.memory_usage_percent > 95.0 {
                    HealthStatus::Unhealthy
                } else if metrics.memory_usage_percent > 85.0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                results.push(HealthCheckResult {
                    name: "system_memory".to_string(),
                    status: memory_status,
                    message: format!("Memory usage: {:.1}%", metrics.memory_usage_percent),
                    duration: Duration::from_millis(1),
                    details: hashmap! {
                        "memory_usage_bytes".to_string() => metrics.memory_usage_bytes.to_string(),
                        "memory_usage_percent".to_string() => metrics.memory_usage_percent.to_string()
                    },
                });

                // Disk health check
                let disk_status = if metrics.disk_usage_percent > 95.0 {
                    HealthStatus::Unhealthy
                } else if metrics.disk_usage_percent > 85.0 {
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Healthy
                };

                results.push(HealthCheckResult {
                    name: "system_disk".to_string(),
                    status: disk_status,
                    message: format!("Disk usage: {:.1}%", metrics.disk_usage_percent),
                    duration: Duration::from_millis(1),
                    details: hashmap! {
                        "disk_usage_bytes".to_string() => metrics.disk_usage_bytes.to_string(),
                        "disk_usage_percent".to_string() => metrics.disk_usage_percent.to_string()
                    },
                });
            } else {
                results.push(HealthCheckResult {
                    name: "system_metrics".to_string(),
                    status: HealthStatus::Unknown,
                    message: "Failed to collect system metrics".to_string(),
                    duration: Duration::from_millis(1),
                    details: HashMap::new(),
                });
            }
        }

        #[cfg(not(feature = "system"))]
        {
            results.push(HealthCheckResult {
                name: "system_metrics".to_string(),
                status: HealthStatus::Unknown,
                message: "System metrics not available (system feature not enabled)".to_string(),
                duration: Duration::from_millis(1),
                details: HashMap::new(),
            });
        }

        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_system_metrics_collector_stub() {
        let collector = SystemMetricsCollector::new(Duration::from_secs(1));
        collector.start_collection().await.unwrap();
        let metrics = collector.get_system_metrics().await;

        #[cfg(feature = "system")]
        assert!(metrics.is_some());
        #[cfg(not(feature = "system"))]
        assert!(metrics.is_none());
    }

    #[test]
    fn test_system_health_checker() {
        let checker = SystemHealthChecker;
        // Note: This test doesn't actually run the async health check
        // In a real test, we'd need to make the check method synchronous or mock it
    }
}

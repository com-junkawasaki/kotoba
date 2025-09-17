//! # KotobaDB Monitoring & Metrics
//!
//! Comprehensive monitoring and metrics collection system for KotobaDB.
//! Provides health checks, performance monitoring, and Prometheus integration.

pub mod metrics_collector;
pub mod health_checker;

pub use metrics_collector::*;
pub use health_checker::*;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use chrono::{DateTime, Utc};

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Enable health checks
    pub enable_health_checks: bool,
    /// Metrics collection interval
    pub collection_interval: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
    /// Metrics retention period
    pub retention_period: Duration,
    /// Maximum stored metrics points
    pub max_metrics_points: usize,
    /// Prometheus export configuration
    pub prometheus_config: PrometheusConfig,
    /// Alerting configuration
    pub alerting_config: AlertingConfig,
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            enable_metrics: true,
            enable_health_checks: true,
            collection_interval: Duration::from_secs(15),
            health_check_interval: Duration::from_secs(30),
            retention_period: Duration::from_secs(3600), // 1 hour
            max_metrics_points: 10000,
            prometheus_config: PrometheusConfig::default(),
            alerting_config: AlertingConfig::default(),
        }
    }
}

/// Prometheus export configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrometheusConfig {
    /// Enable Prometheus export
    pub enabled: bool,
    /// HTTP server address
    pub address: String,
    /// HTTP server port
    pub port: u16,
    /// Metrics path
    pub path: String,
    /// Global labels
    pub global_labels: HashMap<String, String>,
}

impl Default for PrometheusConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            address: "127.0.0.1".to_string(),
            port: 9090,
            path: "/metrics".to_string(),
            global_labels: HashMap::new(),
        }
    }
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Alert rules
    pub rules: Vec<AlertRule>,
    /// Notification endpoints
    pub notifications: Vec<NotificationConfig>,
}

impl Default for AlertingConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            rules: Vec::new(),
            notifications: Vec::new(),
        }
    }
}

/// Alert rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// Metric query expression
    pub query: String,
    /// Alert threshold
    pub threshold: AlertThreshold,
    /// Evaluation interval
    pub evaluation_interval: Duration,
    /// Alert severity
    pub severity: AlertSeverity,
    /// Labels for the alert
    pub labels: HashMap<String, String>,
}

/// Alert threshold
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertThreshold {
    /// Greater than threshold
    GreaterThan(f64),
    /// Less than threshold
    LessThan(f64),
    /// Equal to threshold
    Equal(f64),
    /// Not equal to threshold
    NotEqual(f64),
}

/// Alert severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Notification type
    pub notification_type: NotificationType,
    /// Configuration specific to the notification type
    pub config: HashMap<String, String>,
}

/// Notification types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationType {
    Email,
    Slack,
    Webhook,
    PagerDuty,
}

/// Metric data point
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// Metric name
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Labels
    pub labels: HashMap<String, String>,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Check message
    pub message: String,
    /// Check duration
    pub duration: Duration,
    /// Additional details
    pub details: HashMap<String, String>,
}

/// Health status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
    Unknown,
}

impl Default for HealthStatus {
    fn default() -> Self {
        HealthStatus::Unknown
    }
}

/// Overall system health
#[derive(Debug, Clone, Default)]
pub struct SystemHealth {
    /// Overall status
    pub overall_status: HealthStatus,
    /// Individual check results
    pub checks: Vec<HealthCheckResult>,
    /// System uptime
    pub uptime: Duration,
    /// Last health check time
    pub last_check: DateTime<Utc>,
}

/// Performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    /// Query performance
    pub query_metrics: QueryMetrics,
    /// Storage performance
    pub storage_metrics: StorageMetrics,
    /// System performance
    pub system_metrics: SystemMetrics,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// Query performance metrics
#[derive(Debug, Clone)]
pub struct QueryMetrics {
    pub total_queries: u64,
    pub queries_per_second: f64,
    pub avg_query_latency_ms: f64,
    pub p95_query_latency_ms: f64,
    pub p99_query_latency_ms: f64,
    pub slow_queries: u64,
    pub failed_queries: u64,
}

/// Storage performance metrics
#[derive(Debug, Clone)]
pub struct StorageMetrics {
    pub total_size_bytes: u64,
    pub used_size_bytes: u64,
    pub read_operations: u64,
    pub write_operations: u64,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
    pub cache_hit_rate: f64,
    pub io_latency_ms: f64,
}

/// System performance metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub cpu_usage_percent: f64,
    pub memory_usage_bytes: u64,
    pub memory_usage_percent: f64,
    pub disk_usage_bytes: u64,
    pub disk_usage_percent: f64,
    pub network_rx_bytes: u64,
    pub network_tx_bytes: u64,
}

/// Monitoring errors
#[derive(Debug, thiserror::Error)]
pub enum MonitoringError {
    #[error("Metrics collection error: {0}")]
    MetricsCollection(String),

    #[error("Health check error: {0}")]
    HealthCheck(String),

    #[error("Prometheus export error: {0}")]
    PrometheusExport(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("HTTP server error: {0}")]
    HttpServer(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_metrics);
        assert!(config.enable_health_checks);
        assert_eq!(config.collection_interval, Duration::from_secs(15));
    }

    #[test]
    fn test_prometheus_config_default() {
        let config = PrometheusConfig::default();
        assert!(config.enabled);
        assert_eq!(config.port, 9090);
        assert_eq!(config.path, "/metrics");
    }

    #[test]
    fn test_health_status_ordering() {
        assert!(HealthStatus::Healthy > HealthStatus::Degraded);
        assert!(HealthStatus::Degraded > HealthStatus::Unhealthy);
        assert!(HealthStatus::Unhealthy > HealthStatus::Unknown);
    }

    #[test]
    fn test_metric_point_creation() {
        let mut labels = HashMap::new();
        labels.insert("service".to_string(), "kotoba-db".to_string());

        let point = MetricPoint {
            name: "query_latency".to_string(),
            value: 15.5,
            timestamp: Utc::now(),
            labels,
        };

        assert_eq!(point.name, "query_latency");
        assert_eq!(point.value, 15.5);
        assert!(point.labels.contains_key("service"));
    }
}

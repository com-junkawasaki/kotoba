//! # Prometheus Exporter
//!
//! Prometheus metrics export for KotobaDB monitoring.

use crate::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{routing::get, Router, extract::State, response::IntoResponse};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};

/// Prometheus metrics exporter
pub struct PrometheusExporter {
    /// Metrics collector
    collector: Arc<MetricsCollector>,
    /// Prometheus recorder handle
    prometheus_handle: PrometheusHandle,
    /// HTTP server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
    /// Configuration
    config: PrometheusConfig,
}

impl PrometheusExporter {
    /// Create a new Prometheus exporter
    pub fn new(collector: Arc<MetricsCollector>, config: PrometheusConfig) -> Result<Self, MonitoringError> {
        if !config.enabled {
            return Err(MonitoringError::Configuration("Prometheus export is disabled".to_string()));
        }

        // Initialize Prometheus recorder
        let builder = PrometheusBuilder::new();
        let handle = builder.install_recorder()
            .map_err(|e| MonitoringError::PrometheusExport(format!("Failed to install recorder: {}", e)))?;

        Ok(Self {
            collector,
            prometheus_handle: handle,
            server_handle: None,
            config,
        })
    }

    /// Start the Prometheus HTTP server
    pub async fn start(&mut self) -> Result<(), MonitoringError> {
        let addr: SocketAddr = format!("{}:{}", self.config.address, self.config.port)
            .parse()
            .map_err(|e| MonitoringError::HttpServer(format!("Invalid address: {}", e)))?;

        let collector = Arc::clone(&self.collector);
        let global_labels = self.config.global_labels.clone();

        let app = Router::new()
            .route(&self.config.path, get(metrics_handler))
            .with_state((collector, global_labels));

        let server = axum::Server::bind(&addr)
            .serve(app.into_make_service());

        self.server_handle = Some(tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("Prometheus HTTP server error: {}", e);
            }
        }));

        println!("Prometheus exporter started on http://{}", addr);
        Ok(())
    }

    /// Stop the Prometheus HTTP server
    pub async fn stop(&mut self) -> Result<(), MonitoringError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        Ok(())
    }

    /// Get the metrics in Prometheus format
    pub async fn export_metrics(&self) -> Result<String, MonitoringError> {
        // Get metrics from our collector
        let custom_metrics = self.collector.export_prometheus().await?;

        // Get metrics from the Prometheus recorder
        let recorder_metrics = self.prometheus_handle.render();

        // Combine them
        let mut combined = String::new();
        combined.push_str("# KotobaDB Custom Metrics\n");
        combined.push_str(&custom_metrics);
        combined.push_str("\n# Prometheus Recorder Metrics\n");
        combined.push_str(&recorder_metrics);

        Ok(combined)
    }

    /// Register a custom metric with Prometheus
    pub fn register_counter(&self, name: &str, help: &str, labels: &[&str]) -> Result<(), MonitoringError> {
        use ::metrics::{counter, describe_counter};

        describe_counter!(name, help);
        // The actual counter registration happens when metrics are recorded
        Ok(())
    }

    /// Register a gauge metric with Prometheus
    pub fn register_gauge(&self, name: &str, help: &str, labels: &[&str]) -> Result<(), MonitoringError> {
        use ::metrics::{gauge, describe_gauge};

        describe_gauge!(name, help);
        // The actual gauge registration happens when metrics are recorded
        Ok(())
    }

    /// Register a histogram metric with Prometheus
    pub fn register_histogram(&self, name: &str, help: &str, labels: &[&str]) -> Result<(), MonitoringError> {
        use ::metrics::{histogram, describe_histogram};

        describe_histogram!(name, help);
        // The actual histogram registration happens when metrics are recorded
        Ok(())
    }
}

/// Format metrics as Prometheus text format
fn format_prometheus_metrics(metrics: &str) -> String {
    // For now, just return the metrics as-is
    // In a real implementation, this would parse and format properly
    metrics.to_string()
}

/// Axum handler for the /metrics endpoint
async fn metrics_handler(
    State((collector, global_labels)): State<(Arc<MetricsCollector>, HashMap<String, String>)>,
) -> impl IntoResponse {
    match collector.export_prometheus().await {
        Ok(mut metrics) => {
            // Add global labels if present
            if !global_labels.is_empty() {
                let labels_str = global_labels.iter()
                    .map(|(k, v)| format!("{}=\"{}\"", k, v))
                    .collect::<Vec<_>>()
                    .join(",");

                // This is a simplified approach - in practice, you'd need to parse
                // and modify each metric line to include global labels
                metrics = format!("# Global labels: {{{}}}\n{}", labels_str, metrics);
            }

            // Convert to Prometheus format
            let prometheus_metrics = format_prometheus_metrics(&metrics);

            (
                axum::http::StatusCode::OK,
                [("content-type", "text/plain; version=0.0.4; charset=utf-8")],
                prometheus_metrics
            )
        }
        Err(e) => {
            eprintln!("Error generating metrics: {:?}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                [("content-type", "text/plain")],
                "# Error generating metrics\n".to_string()
            )
        }
    }
}

/// Helper functions for recording metrics

/// Record a counter metric
pub fn record_counter(name: &str, value: u64, labels: &[(&str, &str)]) {
    use ::metrics::counter;

    let mut labels_vec = Vec::new();
    for (key, value) in labels {
        labels_vec.push((*key, *value));
    }

    counter!(name, &labels_vec).increment(value);
}

/// Record a gauge metric
pub fn record_gauge(name: &str, value: f64, labels: &[(&str, &str)]) {
    use ::metrics::gauge;

    let mut labels_vec = Vec::new();
    for (key, value) in labels {
        labels_vec.push((*key, *value));
    }

    gauge!(name, &labels_vec).set(value);
}

/// Record a histogram metric
pub fn record_histogram(name: &str, value: f64, labels: &[(&str, &str)]) {
    use ::metrics::histogram;

    let mut labels_vec = Vec::new();
    for (key, value) in labels {
        labels_vec.push((*key, *value));
    }

    histogram!(name, &labels_vec).record(value);
}

/// Pre-configured metric names for KotobaDB
pub mod metrics {
    pub const DB_CONNECTIONS_ACTIVE: &str = "kotoba_db_connections_active";
    pub const DB_CONNECTIONS_TOTAL: &str = "kotoba_db_connections_total";
    pub const DB_QUERIES_TOTAL: &str = "kotoba_db_queries_total";
    pub const DB_QUERY_LATENCY: &str = "kotoba_db_query_latency_seconds";
    pub const DB_STORAGE_SIZE: &str = "kotoba_db_storage_size_bytes";
    pub const DB_STORAGE_USED: &str = "kotoba_db_storage_used_bytes";

    pub const CLUSTER_NODES_TOTAL: &str = "kotoba_cluster_nodes_total";
    pub const CLUSTER_NODES_ACTIVE: &str = "kotoba_cluster_nodes_active";
    pub const CLUSTER_LEADER_CHANGES: &str = "kotoba_cluster_leader_changes_total";

    pub const BACKUP_DURATION: &str = "kotoba_backup_duration_seconds";
    pub const BACKUP_SIZE: &str = "kotoba_backup_size_bytes";
    pub const BACKUP_SUCCESS: &str = "kotoba_backup_success_total";

    pub const HEALTH_CHECK_DURATION: &str = "kotoba_health_check_duration_seconds";
    pub const HEALTH_CHECK_STATUS: &str = "kotoba_health_check_status";
}

/// Metric labels
pub mod labels {
    pub const NODE_ID: &str = "node_id";
    pub const OPERATION: &str = "operation";
    pub const STATUS: &str = "status";
    pub const BACKUP_TYPE: &str = "backup_type";
    pub const CHECK_NAME: &str = "check_name";
}

/// Example metrics setup for KotobaDB
pub fn setup_kotoba_metrics(exporter: &PrometheusExporter) -> Result<(), MonitoringError> {
    // Database metrics
    exporter.register_counter(
        metrics::DB_CONNECTIONS_TOTAL,
        "Total number of database connections",
        &[labels::NODE_ID],
    )?;

    exporter.register_gauge(
        metrics::DB_CONNECTIONS_ACTIVE,
        "Number of active database connections",
        &[labels::NODE_ID],
    )?;

    exporter.register_counter(
        metrics::DB_QUERIES_TOTAL,
        "Total number of database queries",
        &[labels::NODE_ID, labels::OPERATION],
    )?;

    exporter.register_histogram(
        metrics::DB_QUERY_LATENCY,
        "Database query latency in seconds",
        &[labels::NODE_ID, labels::OPERATION],
    )?;

    exporter.register_gauge(
        metrics::DB_STORAGE_SIZE,
        "Total database storage size in bytes",
        &[labels::NODE_ID],
    )?;

    exporter.register_gauge(
        metrics::DB_STORAGE_USED,
        "Used database storage size in bytes",
        &[labels::NODE_ID],
    )?;

    // Cluster metrics
    exporter.register_gauge(
        metrics::CLUSTER_NODES_TOTAL,
        "Total number of cluster nodes",
        &[],
    )?;

    exporter.register_gauge(
        metrics::CLUSTER_NODES_ACTIVE,
        "Number of active cluster nodes",
        &[],
    )?;

    exporter.register_counter(
        metrics::CLUSTER_LEADER_CHANGES,
        "Total number of cluster leader changes",
        &[],
    )?;

    // Backup metrics
    exporter.register_histogram(
        metrics::BACKUP_DURATION,
        "Backup operation duration in seconds",
        &[labels::BACKUP_TYPE],
    )?;

    exporter.register_gauge(
        metrics::BACKUP_SIZE,
        "Backup size in bytes",
        &[labels::BACKUP_TYPE],
    )?;

    exporter.register_counter(
        metrics::BACKUP_SUCCESS,
        "Number of successful backup operations",
        &[labels::BACKUP_TYPE, labels::STATUS],
    )?;

    // Health check metrics
    exporter.register_histogram(
        metrics::HEALTH_CHECK_DURATION,
        "Health check duration in seconds",
        &[labels::CHECK_NAME],
    )?;

    exporter.register_gauge(
        metrics::HEALTH_CHECK_STATUS,
        "Health check status (0=unknown, 1=healthy, 2=degraded, 3=unhealthy)",
        &[labels::CHECK_NAME],
    )?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockDatabase {
        query_count: Arc<Mutex<u64>>,
    }

    impl MockDatabase {
        fn new() -> Self {
            Self {
                query_count: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait::async_trait]
    impl MonitoredDatabase for MockDatabase {
        async fn get_database_metrics(&self) -> Result<DatabaseMetrics, MonitoringError> {
            Ok(DatabaseMetrics {
                active_connections: 5,
                total_connections: 10,
                uptime_seconds: 3600,
                version: "1.0.0".to_string(),
            })
        }

        async fn get_query_metrics(&self) -> Result<QueryMetrics, MonitoringError> {
            let mut count = self.query_count.lock().await;
            *count += 1;

            Ok(QueryMetrics {
                total_queries: *count,
                queries_per_second: 10.0,
                avg_query_latency_ms: 15.0,
                p95_query_latency_ms: 25.0,
                p99_query_latency_ms: 50.0,
                slow_queries: 1,
                failed_queries: 0,
            })
        }

        async fn get_storage_metrics(&self) -> Result<StorageMetrics, MonitoringError> {
            Ok(StorageMetrics {
                total_size_bytes: 1_000_000,
                used_size_bytes: 500_000,
                read_operations: 1000,
                write_operations: 500,
                read_bytes_per_sec: 100_000.0,
                write_bytes_per_sec: 50_000.0,
                cache_hit_rate: 0.95,
                io_latency_ms: 5.0,
            })
        }
    }

    #[tokio::test]
    async fn test_prometheus_exporter_creation() {
        let db = Arc::new(MockDatabase::new());
        let config = MonitoringConfig::default();
        let collector = Arc::new(MetricsCollector::new(db, config));

        let prometheus_config = PrometheusConfig::default();
        let exporter = PrometheusExporter::new(Arc::clone(&collector), prometheus_config);

        assert!(exporter.is_ok());
    }

    #[test]
    fn test_metric_constants() {
        assert_eq!(metrics::DB_CONNECTIONS_ACTIVE, "kotoba_db_connections_active");
        assert_eq!(metrics::DB_QUERIES_TOTAL, "kotoba_db_queries_total");
        assert_eq!(labels::NODE_ID, "node_id");
    }

    #[tokio::test]
    async fn test_metrics_recording() {
        // Test counter recording
        record_counter("test_counter", 5, &[("label", "value")]);

        // Test gauge recording
        record_gauge("test_gauge", 42.0, &[("label", "value")]);

        // Test histogram recording
        record_histogram("test_histogram", 1.5, &[("label", "value")]);

        // Note: In a real test, we'd verify the metrics were recorded
        // but that requires access to the Prometheus recorder internals
    }
}

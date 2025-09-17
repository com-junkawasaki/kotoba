//! # Metrics Collector
//!
//! Automatic collection and aggregation of system and application metrics.

use crate::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use tokio::time::{self, Duration, Instant};
use chrono::{DateTime, Utc};
use metrics::{counter, gauge, histogram};

/// Metrics collector for automatic metric collection
pub struct MetricsCollector {
    /// Database instance to monitor
    db: Arc<dyn MonitoredDatabase>,
    /// Metrics storage
    metrics_store: Arc<RwLock<MetricsStore>>,
    /// Collection configuration
    config: MonitoringConfig,
    /// Running collection tasks
    collection_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// Metrics event channel
    metrics_tx: mpsc::Sender<MetricsEvent>,
    metrics_rx: mpsc::Receiver<MetricsEvent>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(db: Arc<dyn MonitoredDatabase>, config: MonitoringConfig) -> Self {
        let (metrics_tx, metrics_rx) = mpsc::channel(1000);

        Self {
            db,
            metrics_store: Arc::new(RwLock::new(MetricsStore::new(config.max_metrics_points))),
            config,
            collection_tasks: Arc::new(RwLock::new(HashMap::new())),
            metrics_tx,
            metrics_rx,
        }
    }

    /// Start metrics collection
    pub async fn start(&mut self) -> Result<(), MonitoringError> {
        if !self.config.enable_metrics {
            return Ok(());
        }

        // Start database metrics collection
        self.start_database_metrics_collection().await?;

        // Start system metrics collection
        #[cfg(feature = "system")]
        self.start_system_metrics_collection().await?;

        // Start metrics processing
        self.start_metrics_processing().await?;

        println!("Metrics collector started");
        Ok(())
    }

    /// Stop metrics collection
    pub async fn stop(&self) -> Result<(), MonitoringError> {
        let mut tasks = self.collection_tasks.write().await;
        for (name, handle) in tasks.drain() {
            handle.abort();
            println!("Stopped metrics collection task: {}", name);
        }
        Ok(())
    }

    /// Record a custom metric
    pub async fn record_metric(&self, name: &str, value: f64, labels: HashMap<String, String>) -> Result<(), MonitoringError> {
        let point = MetricPoint {
            name: name.to_string(),
            value,
            timestamp: Utc::now(),
            labels,
        };

        // Store metric
        let mut store = self.metrics_store.write().await;
        store.add_metric(point.clone())?;

        // Send to Prometheus
        self.metrics_tx.send(MetricsEvent::NewMetric(point)).await
            .map_err(|_| MonitoringError::MetricsCollection("Failed to send metric event".to_string()))?;

        Ok(())
    }

    /// Get metrics for a time range
    pub async fn get_metrics(&self, name: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Result<Vec<MetricPoint>, MonitoringError> {
        let store = self.metrics_store.read().await;
        Ok(store.get_metrics(name, from, to))
    }

    /// Get current performance metrics
    pub async fn get_performance_metrics(&self) -> Result<PerformanceMetrics, MonitoringError> {
        let store = self.metrics_store.read().await;
        store.get_performance_metrics()
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> Result<String, MonitoringError> {
        let store = self.metrics_store.read().await;
        store.export_prometheus()
    }

    // Internal methods

    async fn start_database_metrics_collection(&mut self) -> Result<(), MonitoringError> {
        let db = Arc::clone(&self.db);
        let tx = self.metrics_tx.clone();
        let interval = self.config.collection_interval;

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(interval);

            loop {
                ticker.tick().await;

                // Collect database metrics
                if let Ok(db_metrics) = db.get_database_metrics().await {
                    // Send metrics events
                    let _ = tx.send(MetricsEvent::DatabaseMetrics(db_metrics)).await;
                }

                // Collect query metrics
                if let Ok(query_metrics) = db.get_query_metrics().await {
                    let _ = tx.send(MetricsEvent::QueryMetrics(query_metrics)).await;
                }

                // Collect storage metrics
                if let Ok(storage_metrics) = db.get_storage_metrics().await {
                    let _ = tx.send(MetricsEvent::StorageMetrics(storage_metrics)).await;
                }
            }
        });

        self.collection_tasks.write().await.insert("database_metrics".to_string(), handle);
        Ok(())
    }

    #[cfg(feature = "system")]
    async fn start_system_metrics_collection(&mut self) -> Result<(), MonitoringError> {
        let tx = self.metrics_tx.clone();
        let interval = self.config.collection_interval;

        let handle = tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            let system = sysinfo::System::new_all();

            loop {
                ticker.tick().await;

                // Refresh system information
                system.refresh_all();

                // Collect CPU metrics
                let cpu_usage = system.global_cpu_info().cpu_usage() as f64;
                let _ = tx.send(MetricsEvent::SystemMetric {
                    name: "cpu_usage_percent".to_string(),
                    value: cpu_usage,
                    labels: HashMap::new(),
                }).await;

                // Collect memory metrics
                let total_memory = system.total_memory() as f64;
                let used_memory = system.used_memory() as f64;
                let memory_usage_percent = (used_memory / total_memory) * 100.0;

                let _ = tx.send(MetricsEvent::SystemMetric {
                    name: "memory_usage_bytes".to_string(),
                    value: used_memory,
                    labels: HashMap::new(),
                }).await;

                let _ = tx.send(MetricsEvent::SystemMetric {
                    name: "memory_usage_percent".to_string(),
                    value: memory_usage_percent,
                    labels: HashMap::new(),
                }).await;
            }
        });

        self.collection_tasks.write().await.insert("system_metrics".to_string(), handle);
        Ok(())
    }

    #[cfg(not(feature = "system"))]
    async fn start_system_metrics_collection(&mut self) -> Result<(), MonitoringError> {
        // System metrics not available
        Ok(())
    }

    async fn start_metrics_processing(&mut self) -> Result<(), MonitoringError> {
        let metrics_store = Arc::clone(&self.metrics_store);
        let mut rx = self.metrics_rx;

        let handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    MetricsEvent::NewMetric(point) => {
                        let mut store = metrics_store.write().await;
                        let _ = store.add_metric(point);
                    }
                    MetricsEvent::DatabaseMetrics(db_metrics) => {
                        let mut store = metrics_store.write().await;
                        store.update_database_metrics(db_metrics);
                    }
                    MetricsEvent::QueryMetrics(query_metrics) => {
                        let mut store = metrics_store.write().await;
                        store.update_query_metrics(query_metrics);
                    }
                    MetricsEvent::StorageMetrics(storage_metrics) => {
                        let mut store = metrics_store.write().await;
                        store.update_storage_metrics(storage_metrics);
                    }
                    MetricsEvent::SystemMetric { name, value, labels } => {
                        let point = MetricPoint {
                            name,
                            value,
                            timestamp: Utc::now(),
                            labels,
                        };
                        let mut store = metrics_store.write().await;
                        let _ = store.add_metric(point);
                    }
                }
            }
        });

        self.collection_tasks.write().await.insert("metrics_processing".to_string(), handle);
        Ok(())
    }
}

/// Metrics events
#[derive(Debug, Clone)]
pub enum MetricsEvent {
    NewMetric(MetricPoint),
    DatabaseMetrics(DatabaseMetrics),
    QueryMetrics(QueryMetrics),
    StorageMetrics(StorageMetrics),
    SystemMetric { name: String, value: f64, labels: HashMap<String, String> },
}

/// Database metrics
#[derive(Debug, Clone)]
pub struct DatabaseMetrics {
    pub active_connections: u64,
    pub total_connections: u64,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Trait for monitored database
#[async_trait::async_trait]
pub trait MonitoredDatabase: Send + Sync {
    /// Get database-specific metrics
    async fn get_database_metrics(&self) -> Result<DatabaseMetrics, MonitoringError>;

    /// Get query performance metrics
    async fn get_query_metrics(&self) -> Result<QueryMetrics, MonitoringError>;

    /// Get storage performance metrics
    async fn get_storage_metrics(&self) -> Result<StorageMetrics, MonitoringError>;
}

/// Metrics storage and retrieval
pub struct MetricsStore {
    /// Stored metrics points
    metrics: HashMap<String, Vec<MetricPoint>>,
    /// Current performance metrics
    performance_metrics: PerformanceMetrics,
    /// Maximum points per metric
    max_points_per_metric: usize,
}

impl MetricsStore {
    pub fn new(max_points_per_metric: usize) -> Self {
        Self {
            metrics: HashMap::new(),
            performance_metrics: PerformanceMetrics {
                query_metrics: QueryMetrics {
                    total_queries: 0,
                    queries_per_second: 0.0,
                    avg_query_latency_ms: 0.0,
                    p95_query_latency_ms: 0.0,
                    p99_query_latency_ms: 0.0,
                    slow_queries: 0,
                    failed_queries: 0,
                },
                storage_metrics: StorageMetrics {
                    total_size_bytes: 0,
                    used_size_bytes: 0,
                    read_operations: 0,
                    write_operations: 0,
                    read_bytes_per_sec: 0.0,
                    write_bytes_per_sec: 0.0,
                    cache_hit_rate: 0.0,
                    io_latency_ms: 0.0,
                },
                system_metrics: SystemMetrics {
                    cpu_usage_percent: 0.0,
                    memory_usage_bytes: 0,
                    memory_usage_percent: 0.0,
                    disk_usage_bytes: 0,
                    disk_usage_percent: 0.0,
                    network_rx_bytes: 0,
                    network_tx_bytes: 0,
                },
                timestamp: Utc::now(),
            },
            max_points_per_metric,
        }
    }

    pub fn add_metric(&mut self, point: MetricPoint) -> Result<(), MonitoringError> {
        let points = self.metrics.entry(point.name.clone()).or_insert_with(Vec::new);

        points.push(point);

        // Maintain size limit
        if points.len() > self.max_points_per_metric {
            let remove_count = points.len() - self.max_points_per_metric;
            points.drain(0..remove_count);
        }

        Ok(())
    }

    pub fn get_metrics(&self, name: &str, from: DateTime<Utc>, to: DateTime<Utc>) -> Vec<MetricPoint> {
        if let Some(points) = self.metrics.get(name) {
            points.iter()
                .filter(|p| p.timestamp >= from && p.timestamp <= to)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn update_database_metrics(&mut self, metrics: DatabaseMetrics) {
        // Store as individual metrics
        let timestamp = Utc::now();
        let mut labels = HashMap::new();
        labels.insert("component".to_string(), "database".to_string());

        // Create metric points
        let active_connections = MetricPoint {
            name: "database_active_connections".to_string(),
            value: metrics.active_connections as f64,
            timestamp,
            labels: labels.clone(),
        };

        let total_connections = MetricPoint {
            name: "database_total_connections".to_string(),
            value: metrics.total_connections as f64,
            timestamp,
            labels,
        };

        let _ = self.add_metric(active_connections);
        let _ = self.add_metric(total_connections);
    }

    pub fn update_query_metrics(&mut self, metrics: QueryMetrics) {
        self.performance_metrics.query_metrics = metrics.clone();
        self.performance_metrics.timestamp = Utc::now();

        // Store as individual metrics
        let timestamp = Utc::now();
        let mut labels = HashMap::new();
        labels.insert("component".to_string(), "query".to_string());

        let metrics_list = vec![
            ("query_total", metrics.total_queries as f64),
            ("query_per_second", metrics.queries_per_second),
            ("query_avg_latency_ms", metrics.avg_query_latency_ms),
            ("query_p95_latency_ms", metrics.p95_query_latency_ms),
            ("query_p99_latency_ms", metrics.p99_query_latency_ms),
            ("query_slow", metrics.slow_queries as f64),
            ("query_failed", metrics.failed_queries as f64),
        ];

        for (name, value) in metrics_list {
            let point = MetricPoint {
                name: format!("database_{}", name),
                value,
                timestamp,
                labels: labels.clone(),
            };
            let _ = self.add_metric(point);
        }
    }

    pub fn update_storage_metrics(&mut self, metrics: StorageMetrics) {
        self.performance_metrics.storage_metrics = metrics.clone();

        // Store as individual metrics
        let timestamp = Utc::now();
        let mut labels = HashMap::new();
        labels.insert("component".to_string(), "storage".to_string());

        let metrics_list = vec![
            ("storage_total_bytes", metrics.total_size_bytes as f64),
            ("storage_used_bytes", metrics.used_size_bytes as f64),
            ("storage_read_ops", metrics.read_operations as f64),
            ("storage_write_ops", metrics.write_operations as f64),
            ("storage_read_bytes_per_sec", metrics.read_bytes_per_sec),
            ("storage_write_bytes_per_sec", metrics.write_bytes_per_sec),
            ("storage_cache_hit_rate", metrics.cache_hit_rate),
            ("storage_io_latency_ms", metrics.io_latency_ms),
        ];

        for (name, value) in metrics_list {
            let point = MetricPoint {
                name: format!("database_{}", name),
                value,
                timestamp,
                labels: labels.clone(),
            };
            let _ = self.add_metric(point);
        }
    }

    pub fn get_performance_metrics(&self) -> Result<PerformanceMetrics, MonitoringError> {
        Ok(self.performance_metrics.clone())
    }

    pub fn export_prometheus(&self) -> Result<String, MonitoringError> {
        let mut output = String::new();

        // Export stored metrics
        for (metric_name, points) in &self.metrics {
            if let Some(latest_point) = points.last() {
                output.push_str(&format!("# HELP {} {}\n", metric_name, metric_name));
                output.push_str(&format!("# TYPE {} gauge\n", metric_name));

                // Add labels if present
                if latest_point.labels.is_empty() {
                    output.push_str(&format!("{} {}\n", metric_name, latest_point.value));
                } else {
                    let labels_str = latest_point.labels.iter()
                        .map(|(k, v)| format!("{}=\"{}\"", k, v))
                        .collect::<Vec<_>>()
                        .join(",");
                    output.push_str(&format!("{}{{{}}} {}\n", metric_name, labels_str, latest_point.value));
                }
            }
        }

        Ok(output)
    }
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
    async fn test_metrics_collector_creation() {
        let db = Arc::new(MockDatabase::new());
        let config = MonitoringConfig::default();
        let collector = MetricsCollector::new(db, config);

        assert!(collector.config.enable_metrics);
    }

    #[tokio::test]
    async fn test_custom_metric_recording() {
        let db = Arc::new(MockDatabase::new());
        let config = MonitoringConfig::default();
        let collector = MetricsCollector::new(db, config);

        let mut labels = HashMap::new();
        labels.insert("test".to_string(), "value".to_string());

        collector.record_metric("test_metric", 42.0, labels).await.unwrap();

        let metrics = collector.get_metrics("test_metric", Utc::now() - chrono::Duration::hours(1), Utc::now()).await.unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].value, 42.0);
    }

    #[test]
    fn test_metrics_store() {
        let store = MetricsStore::new(10);

        let point = MetricPoint {
            name: "test".to_string(),
            value: 1.0,
            timestamp: Utc::now(),
            labels: HashMap::new(),
        };

        store.add_metric(point.clone()).unwrap();
        let metrics = store.get_metrics("test", Utc::now() - chrono::Duration::hours(1), Utc::now());
        assert_eq!(metrics.len(), 1);
    }
}

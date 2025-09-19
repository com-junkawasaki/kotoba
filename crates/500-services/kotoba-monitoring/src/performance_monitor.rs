//! # Performance Monitor
//!
//! Real-time performance monitoring and analysis for KotobaDB.

use crate::*;

/// Performance monitor for real-time performance tracking
pub struct PerformanceMonitor {
    /// Metrics collector
    collector: Arc<MetricsCollector>,
    /// Performance thresholds
    thresholds: PerformanceThresholds,
    /// Performance history
    history: Arc<RwLock<Vec<PerformanceSnapshot>>>,
}

impl PerformanceMonitor {
    /// Create a new performance monitor
    pub fn new(collector: Arc<MetricsCollector>) -> Self {
        Self {
            collector,
            thresholds: PerformanceThresholds::default(),
            history: Arc<RwLock::new(Vec::new())>,
        }
    }

    /// Get current performance analysis
    pub async fn analyze_performance(&self) -> Result<PerformanceAnalysis, MonitoringError> {
        let metrics = self.collector.get_performance_metrics().await?;

        let analysis = PerformanceAnalysis {
            overall_score: self.calculate_performance_score(&metrics),
            bottlenecks: self.identify_bottlenecks(&metrics),
            recommendations: self.generate_recommendations(&metrics),
            metrics,
            timestamp: Utc::now(),
        };

        // Store in history
        {
            let mut history = self.history.write().await;
            history.push(PerformanceSnapshot {
                analysis: analysis.clone(),
                timestamp: Utc::now(),
            });

            // Keep only last 100 snapshots
            if history.len() > 100 {
                history.remove(0);
            }
        }

        Ok(analysis)
    }

    /// Get performance history
    pub async fn get_performance_history(&self, limit: usize) -> Vec<PerformanceSnapshot> {
        let history = self.history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Set performance thresholds
    pub fn set_thresholds(&mut self, thresholds: PerformanceThresholds) {
        self.thresholds = thresholds;
    }

    /// Calculate overall performance score (0-100)
    fn calculate_performance_score(&self, metrics: &PerformanceMetrics) -> f64 {
        let mut score = 100.0;

        // Query latency penalties
        if metrics.query_metrics.avg_query_latency_ms > self.thresholds.max_query_latency_ms {
            let penalty = (metrics.query_metrics.avg_query_latency_ms - self.thresholds.max_query_latency_ms) / 10.0;
            score -= penalty.min(30.0);
        }

        // Cache hit rate penalties
        if metrics.storage_metrics.cache_hit_rate < self.thresholds.min_cache_hit_rate {
            let penalty = (self.thresholds.min_cache_hit_rate - metrics.storage_metrics.cache_hit_rate) * 50.0;
            score -= penalty.min(20.0);
        }

        // Connection usage penalties
        if metrics.query_metrics.failed_queries > 0 {
            score -= 10.0;
        }

        score.max(0.0)
    }

    /// Identify performance bottlenecks
    fn identify_bottlenecks(&self, metrics: &PerformanceMetrics) -> Vec<PerformanceBottleneck> {
        let mut bottlenecks = Vec::new();

        // Query latency bottleneck
        if metrics.query_metrics.avg_query_latency_ms > self.thresholds.max_query_latency_ms {
            bottlenecks.push(PerformanceBottleneck {
                component: "Query Engine".to_string(),
                issue: "High query latency".to_string(),
                severity: if metrics.query_metrics.avg_query_latency_ms > self.thresholds.max_query_latency_ms * 2.0 {
                    BottleneckSeverity::Critical
                } else {
                    BottleneckSeverity::Warning
                },
                impact: format!("{:.1}ms average latency", metrics.query_metrics.avg_query_latency_ms),
                suggestion: "Consider optimizing queries or adding indexes".to_string(),
            });
        }

        // Cache bottleneck
        if metrics.storage_metrics.cache_hit_rate < self.thresholds.min_cache_hit_rate {
            bottlenecks.push(PerformanceBottleneck {
                component: "Storage Cache".to_string(),
                issue: "Low cache hit rate".to_string(),
                severity: BottleneckSeverity::Warning,
                impact: format!("{:.1}% cache hit rate", metrics.storage_metrics.cache_hit_rate * 100.0),
                suggestion: "Increase cache size or review access patterns".to_string(),
            });
        }

        // I/O bottleneck
        if metrics.storage_metrics.io_latency_ms > self.thresholds.max_io_latency_ms {
            bottlenecks.push(PerformanceBottleneck {
                component: "Storage I/O".to_string(),
                issue: "High I/O latency".to_string(),
                severity: if metrics.storage_metrics.io_latency_ms > self.thresholds.max_io_latency_ms * 2.0 {
                    BottleneckSeverity::Critical
                } else {
                    BottleneckSeverity::Warning
                },
                impact: format!("{:.1}ms I/O latency", metrics.storage_metrics.io_latency_ms),
                suggestion: "Consider faster storage or I/O optimization".to_string(),
            });
        }

        bottlenecks
    }

    /// Generate performance recommendations
    fn generate_recommendations(&self, metrics: &PerformanceMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();

        if metrics.query_metrics.avg_query_latency_ms > self.thresholds.max_query_latency_ms {
            recommendations.push("Consider query optimization and indexing strategies".to_string());
        }

        if metrics.storage_metrics.cache_hit_rate < self.thresholds.min_cache_hit_rate {
            recommendations.push("Increase database cache size".to_string());
            recommendations.push("Review data access patterns for better cache utilization".to_string());
        }

        if metrics.query_metrics.failed_queries > 0 {
            recommendations.push("Investigate and fix query failures".to_string());
        }

        if metrics.storage_metrics.used_size_bytes as f64 / metrics.storage_metrics.total_size_bytes as f64 > 0.9 {
            recommendations.push("Consider expanding storage capacity".to_string());
        }

        recommendations
    }
}

/// Performance thresholds for monitoring
#[derive(Debug, Clone)]
pub struct PerformanceThresholds {
    pub max_query_latency_ms: f64,
    pub min_cache_hit_rate: f64,
    pub max_io_latency_ms: f64,
    pub max_memory_usage_percent: f64,
    pub max_cpu_usage_percent: f64,
}

impl Default for PerformanceThresholds {
    fn default() -> Self {
        Self {
            max_query_latency_ms: 50.0,
            min_cache_hit_rate: 0.8,
            max_io_latency_ms: 20.0,
            max_memory_usage_percent: 85.0,
            max_cpu_usage_percent: 80.0,
        }
    }
}

/// Performance analysis result
#[derive(Debug, Clone)]
pub struct PerformanceAnalysis {
    pub overall_score: f64,
    pub bottlenecks: Vec<PerformanceBottleneck>,
    pub recommendations: Vec<String>,
    pub metrics: PerformanceMetrics,
    pub timestamp: DateTime<Utc>,
}

/// Performance bottleneck identification
#[derive(Debug, Clone)]
pub struct PerformanceBottleneck {
    pub component: String,
    pub issue: String,
    pub severity: BottleneckSeverity,
    pub impact: String,
    pub suggestion: String,
}

/// Bottleneck severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BottleneckSeverity {
    Info,
    Warning,
    Critical,
}

/// Performance snapshot for historical tracking
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    pub analysis: PerformanceAnalysis,
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_thresholds_default() {
        let thresholds = PerformanceThresholds::default();
        assert_eq!(thresholds.max_query_latency_ms, 50.0);
        assert_eq!(thresholds.min_cache_hit_rate, 0.8);
    }

    #[test]
    fn test_performance_score_calculation() {
        let monitor = PerformanceMonitor::new(Arc::new(MetricsCollector::new(
            Arc::new(MockDatabase::new()),
            MonitoringConfig::default()
        )));

        let metrics = PerformanceMetrics {
            query_metrics: QueryMetrics {
                total_queries: 100,
                queries_per_second: 10.0,
                avg_query_latency_ms: 100.0, // Above threshold
                p95_query_latency_ms: 150.0,
                p99_query_latency_ms: 200.0,
                slow_queries: 10,
                failed_queries: 1,
            },
            storage_metrics: StorageMetrics {
                total_size_bytes: 1000000,
                used_size_bytes: 500000,
                read_operations: 1000,
                write_operations: 500,
                read_bytes_per_sec: 100000.0,
                write_bytes_per_sec: 50000.0,
                cache_hit_rate: 0.7, // Below threshold
                io_latency_ms: 10.0,
            },
            system_metrics: SystemMetrics {
                cpu_usage_percent: 50.0,
                memory_usage_bytes: 500000000,
                memory_usage_percent: 50.0,
                disk_usage_bytes: 500000000,
                disk_usage_percent: 50.0,
                network_rx_bytes: 1000000,
                network_tx_bytes: 500000,
            },
            timestamp: Utc::now(),
        };

        let score = monitor.calculate_performance_score(&metrics);
        assert!(score < 100.0); // Should be reduced due to high latency and low cache hit rate

        let bottlenecks = monitor.identify_bottlenecks(&metrics);
        assert!(!bottlenecks.is_empty());
    }
}

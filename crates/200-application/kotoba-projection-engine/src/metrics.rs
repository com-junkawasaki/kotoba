//! Metrics Collection
//!
//! Metrics collection for projection engine performance monitoring.

use std::sync::Arc;
use tokio::sync::RwLock;
use metrics::{counter, histogram, gauge};
use serde::{Deserialize, Serialize};

/// Metrics collector for projection engine
pub struct MetricsCollector {
    stats: Arc<RwLock<ProjectionMetrics>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(ProjectionMetrics::default())),
        }
    }

    /// Record event processing
    pub async fn record_event_processed(&self, processing_time_ms: f64) {
        let mut stats = self.stats.write().await;
        stats.total_events_processed += 1;
        stats.total_processing_time_ms += processing_time_ms;

        // Update metrics
        // counter!("projection_engine.events_processed", 1);
        // histogram!("projection_engine.processing_time", processing_time_ms);

        // Update average
        if stats.total_events_processed > 0 {
            stats.avg_processing_time_ms = stats.total_processing_time_ms / stats.total_events_processed as f64;
        }
    }

    /// Record cache hit
    pub async fn record_cache_hit(&self) {
        let mut stats = self.stats.write().await;
        stats.cache_hits += 1;
        // counter!("projection_engine.cache_hits", 1);
    }

    /// Record cache miss
    pub async fn record_cache_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.cache_misses += 1.0;
        // counter!("projection_engine.cache_misses", 1);
    }

    /// Record projection creation
    pub async fn record_projection_created(&self) {
        let mut stats = self.stats.write().await;
        stats.active_projections += 1;
        // gauge!("projection_engine.active_projections", stats.active_projections as f64);
        // counter!("projection_engine.projections_created", 1);
    }

    /// Record projection deletion
    pub async fn record_projection_deleted(&self) {
        let mut stats = self.stats.write().await;
        if stats.active_projections > 0 {
            stats.active_projections -= 1;
        }
        // gauge!("projection_engine.active_projections", stats.active_projections as f64);
        // counter!("projection_engine.projections_deleted", 1);
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> ProjectionMetrics {
        self.stats.read().await.clone()
    }

    /// Reset metrics
    pub async fn reset_metrics(&self) {
        let mut stats = self.stats.write().await;
        *stats = ProjectionMetrics::default();
    }
}

/// Projection metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionMetrics {
    pub total_events_processed: u64,
    pub total_processing_time_ms: f64,
    pub avg_processing_time_ms: f64,
    pub cache_hits: u64,
    pub cache_misses: f64,
    pub active_projections: u64,
    pub total_projections_created: u64,
    pub total_projections_deleted: u64,
}

impl Default for ProjectionMetrics {
    fn default() -> Self {
        Self {
            total_events_processed: 0,
            total_processing_time_ms: 0.0,
            avg_processing_time_ms: 0.0,
            cache_hits: 0,
            cache_misses: 0.0,
            active_projections: 0,
            total_projections_created: 0,
            total_projections_deleted: 0,
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

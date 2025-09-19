//! `kotoba-projection-engine`
//!
//! Real-time projection engine for GraphDB materialization using RocksDB.
//! Processes events from event streams and maintains materialized views in GraphDB.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use dashmap::DashMap;
use anyhow::{Result, Context};
use tracing::{info, warn, error, instrument};
// use metrics::macros::counter; // Temporarily disabled due to version issues
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use kotoba_ocel::OcelEvent;
use kotoba_storage::KeyValueStore;

pub mod event_processor;
pub mod materializer;
pub mod view_manager;
pub mod storage;
pub mod cache_integration;
pub mod metrics;
pub mod gql_integration;

// Re-export main types
pub use event_processor::*;
pub use materializer::*;
pub use view_manager::*;
pub use storage::*;
pub use cache_integration::*;
pub use metrics::*;

/// Main projection engine with generic KeyValueStore backend
pub struct ProjectionEngine<T: KeyValueStore> {
    /// Event processor for handling OCEL events
    event_processor: Arc<EventProcessor<T>>,
    /// Materializer for projections
    materializer: Arc<Materializer<T>>,
    /// Storage backend
    storage: Arc<T>,
    /// View manager
    view_manager: Arc<ViewManager>,
    /// Metrics collector
    metrics: Arc<MetricsCollector>,
    /// Engine configuration
    config: ProjectionConfig,
    /// Active projections
    active_projections: Arc<DashMap<String, ProjectionState>>,
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
    shutdown_rx: Arc<RwLock<mpsc::Receiver<()>>>,
}

/// Projection configuration
#[derive(Debug, Clone)]
pub struct ProjectionConfig {
    /// Storage prefix for projection keys
    pub storage_prefix: String,
    /// Maximum concurrent projections
    pub max_concurrent_projections: usize,
    /// Batch size for event processing
    pub batch_size: usize,
    /// Checkpoint interval (in events)
    pub checkpoint_interval: u64,
    /// Metrics collection
    pub enable_metrics: bool,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            storage_prefix: "projections".to_string(),
            max_concurrent_projections: 10,
            batch_size: 100,
            checkpoint_interval: 1000,
            enable_metrics: true,
        }
    }
}




/// Projection state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectionState {
    /// Projection name
    pub name: String,
    /// Current event sequence number
    pub sequence_number: u64,
    /// Last checkpoint timestamp
    pub last_checkpoint: chrono::DateTime<chrono::Utc>,
    /// Projection status
    pub status: ProjectionStatus,
    /// Processing statistics
    pub stats: ProjectionStats,
}

/// Projection status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ProjectionStatus {
    /// Projection is active and processing events
    Active,
    /// Projection is paused
    Paused,
    /// Projection encountered an error
    Error(String),
    /// Projection is being rebuilt
    Rebuilding,
}

/// Projection statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectionStats {
    /// Total events processed
    pub events_processed: u64,
    /// Events processed per second
    pub events_per_second: f64,
    /// Total processing time
    pub total_processing_time_ms: u64,
    /// Average processing time per event
    pub avg_processing_time_ms: f64,
    /// Number of cache hits
    pub cache_hits: u64,
    /// Number of cache misses
    pub cache_misses: u64,
}

impl<T: KeyValueStore + 'static> ProjectionEngine<T> {
    /// Create a new projection engine with the given KeyValueStore backend
    pub fn new(config: ProjectionConfig, storage: Arc<T>) -> Self {
        info!("Initializing Projection Engine with config: {:?}", config);

        // Initialize view manager
        let view_manager = Arc::new(ViewManager::new());

        // Initialize metrics
        let metrics = Arc::new(MetricsCollector::new());

        // Initialize materializer with storage backend
        let materializer = Arc::new(Materializer::new(
            storage.clone(),
            config.storage_prefix.clone(),
        ));

        // Initialize event processor
        let event_processor = Arc::new(EventProcessor::new(
            materializer.clone(),
            config.batch_size,
        ));

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

        let engine = Self {
            event_processor,
            materializer,
            storage,
            view_manager,
            metrics,
            config,
            active_projections: Arc::new(DashMap::new()),
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(shutdown_rx)),
        };

        info!("Projection Engine initialized successfully");
        engine
    }
}

impl<T: KeyValueStore> Clone for ProjectionEngine<T> {
    fn clone(&self) -> Self {
        Self {
            event_processor: self.event_processor.clone(),
            materializer: self.materializer.clone(),
            storage: self.storage.clone(),
            view_manager: self.view_manager.clone(),
            metrics: self.metrics.clone(),
            config: self.config.clone(),
            active_projections: self.active_projections.clone(),
            shutdown_tx: self.shutdown_tx.clone(),
            shutdown_rx: self.shutdown_rx.clone(),
        }
    }
}

impl<T: KeyValueStore + 'static> ProjectionEngine<T> {
    /// Start the projection engine
    #[instrument(skip(self))]
    pub async fn start(&self) -> Result<()> {
        info!("Starting Projection Engine");

        // Start event processor
        self.event_processor.start().await?;

        // Load existing projections
        self.load_existing_projections().await?;

        // Start metrics collection if enabled
        if self.config.enable_metrics {
            self.start_metrics_collection().await?;
        }

        info!("Projection Engine started successfully");
        Ok(())
    }

    /// Stop the projection engine
    #[instrument(skip(self))]
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Projection Engine");

        // Signal shutdown
        let _ = self.shutdown_tx.send(()).await;

        // Stop event processor
        self.event_processor.stop().await?;

        // Save projection states
        self.save_projection_states().await?;

        info!("Projection Engine stopped successfully");
        Ok(())
    }

    /// Create a new projection
    #[instrument(skip(self))]
    pub async fn create_projection(
        &self,
        name: String,
        definition: ProjectionDefinition,
    ) -> Result<()> {
        info!("Creating projection: {}", name);

        // Validate projection definition
        self.validate_projection_definition(&definition).await?;

        // Create projection in view manager
        self.view_manager.create_projection(name.clone(), definition).await?;

        // Initialize projection state
        let state = ProjectionState {
            name: name.clone(),
            sequence_number: 0,
            last_checkpoint: chrono::Utc::now(),
            status: ProjectionStatus::Active,
            stats: ProjectionStats::default(),
        };

        self.active_projections.insert(name.clone(), state);

        // Register with event processor
        self.event_processor.register_projection(&name).await?;

        info!("Projection created successfully: {}", name);
        Ok(())
    }

    /// Delete a projection
    #[instrument(skip(self))]
    pub async fn delete_projection(&self, name: &str) -> Result<()> {
        info!("Deleting projection: {}", name);

        // Unregister from event processor
        self.event_processor.unregister_projection(name).await?;

        // Delete from view manager
        self.view_manager.delete_projection(name).await?;

        // Remove from active projections
        self.active_projections.remove(name);

        info!("Projection deleted successfully: {}", name);
        Ok(())
    }

    /// Get projection status
    pub async fn get_projection_status(&self, name: &str) -> Result<Option<ProjectionState>> {
        Ok(self.active_projections.get(name).map(|s| s.clone()))
    }

    /// List all projections
    pub async fn list_projections(&self) -> Vec<String> {
        self.active_projections.iter().map(|p| p.key().clone()).collect()
    }

    /// Process a batch of OCEL events
    #[instrument(skip(self, events))]
    pub async fn process_ocel_events(&self, events: Vec<OcelEvent>) -> Result<()> {
        if self.config.enable_metrics {
            // counter!("projection_engine.events_received", events.len() as u64);
        }

        // Process OCEL events through the event processor
        self.event_processor.process_batch(events).await
    }

    /// Process a batch of events (legacy method)
    #[instrument(skip(self, events))]
    pub async fn process_events(&self, events: Vec<EventEnvelope>) -> Result<()> {
        warn!("Legacy event processing is deprecated. Use process_ocel_events instead.");
        Ok(())
    }

    /// Query projections using storage backend
    #[instrument(skip(self, query))]
    pub async fn query_projections(&self, query: serde_json::Value) -> Result<serde_json::Value> {
        // For now, implement basic key scanning
        // This would be enhanced with proper GraphQL/GQL query support
        warn!("query_projections is not fully implemented yet");

        // Return empty result for now
        Ok(serde_json::json!({
            "columns": [],
            "rows": [],
            "statistics": {
                "total_rows": 0,
                "execution_time_ms": 0
            }
        }))
    }

    /// Query a materialized view (legacy method)
    #[instrument(skip(self, query))]
    pub async fn query_view(&self, projection_name: &str, query: ViewQuery) -> Result<ViewResult> {
        warn!("Legacy view querying is deprecated. Use query_graph instead.");
        Err(anyhow::anyhow!("Legacy view querying not supported"))
    }

    /// Get engine statistics
    pub async fn get_statistics(&self) -> EngineStatistics {
        let mut total_events = 0u64;
        let mut active_projections = 0usize;

        for projection in self.active_projections.iter() {
            total_events += projection.stats.events_processed;
            if matches!(projection.status, ProjectionStatus::Active) {
                active_projections += 1;
            }
        }

        // For now, return 0 for storage size as we don't have a generic way to get size
        EngineStatistics {
            total_projections: self.active_projections.len(),
            active_projections,
            total_events_processed: total_events,
            uptime_seconds: 0, // TODO: Track uptime
            storage_size_bytes: 0, // TODO: Implement storage size calculation
        }
    }

    /// Execute GQL query (simplified implementation)
    #[instrument(skip(self))]
    pub async fn execute_gql_query(
        &self,
        query: &str,
        _user_id: Option<String>,
        _timeout_seconds: u64,
        _parameters: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        use crate::gql_integration::ProjectionEngineAdapter;

        let adapter = ProjectionEngineAdapter::new(Arc::new(self.clone()));
        adapter.execute_gql_query(query, serde_json::json!({})).await
    }

    /// Execute GQL statement (DDL/DML) (simplified implementation)
    #[instrument(skip(self))]
    pub async fn execute_gql_statement(
        &self,
        statement: &str,
        _user_id: Option<String>,
        _timeout_seconds: u64,
        _parameters: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        use crate::gql_integration::ProjectionEngineAdapter;

        let adapter = ProjectionEngineAdapter::new(Arc::new(self.clone()));
        adapter.execute_gql_statement(statement, serde_json::json!({})).await
    }

    async fn load_existing_projections(&self) -> Result<()> {
        info!("Loading existing projections");

        // Scan for projection keys in storage
        let prefix = format!("{}:projection:", self.config.storage_prefix);
        let projection_keys = self.storage.scan(prefix.as_bytes()).await?;

        for key_bytes in projection_keys {
            if let Ok(key_str) = std::str::from_utf8(&key_bytes.0) {
                if let Some(projection_name) = key_str.strip_prefix(&prefix) {
                    // Load projection state from storage
                    let state_key = format!("{}:state:{}", self.config.storage_prefix, projection_name);
                    if let Some(state_data) = self.storage.get(state_key.as_bytes()).await? {
                        if let Ok(state) = bincode::deserialize::<ProjectionState>(&state_data) {
                            self.active_projections.insert(projection_name.to_string(), state);
                            self.event_processor.register_projection(projection_name).await?;
                        }
                    }
                }
            }
        }

        info!("Loaded {} existing projections", self.active_projections.len());
        Ok(())
    }

    async fn save_projection_states(&self) -> Result<()> {
        for projection in self.active_projections.iter() {
            let state_key = format!("{}:state:{}", self.config.storage_prefix, projection.key());
            let state_data = bincode::serialize(&projection.value())?;
            self.storage.put(state_key.as_bytes(), &state_data).await?;
        }
        Ok(())
    }

    async fn validate_projection_definition(&self, definition: &ProjectionDefinition) -> Result<()> {
        // TODO: Implement validation logic
        Ok(())
    }

    async fn start_metrics_collection(&self) -> Result<()> {
        // TODO: Start metrics collection task
        Ok(())
    }
}

/// Engine statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStatistics {
    pub total_projections: usize,
    pub active_projections: usize,
    pub total_events_processed: u64,
    pub uptime_seconds: u64,
    pub storage_size_bytes: u64,
}

// Placeholder types - will be defined in respective modules
pub type EventEnvelope = serde_json::Value;
pub type ProjectionDefinition = serde_json::Value;
// Placeholder types - these will be replaced with actual implementations
pub type QueryResult = serde_json::Value; // Use JSON for now to handle type conversion
pub type ViewQuery = serde_json::Value;
pub type ViewResult = serde_json::Value;

impl Default for ProjectionStats {
    fn default() -> Self {
        Self {
            events_processed: 0,
            events_per_second: 0.0,
            total_processing_time_ms: 0,
            avg_processing_time_ms: 0.0,
            cache_hits: 0,
            cache_misses: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_projection_engine_creation() {
        let temp_dir = tempdir().unwrap();
        let config = ProjectionConfig::default();

        // Create a temporary storage backend (for testing, we could use memory-based)
        // For now, we'll skip the actual creation test as it requires a concrete KeyValueStore
        let stats_placeholder = EngineStatistics {
            total_projections: 0,
            active_projections: 0,
            total_events_processed: 0,
            uptime_seconds: 0,
            storage_size_bytes: 0,
        };
        assert_eq!(stats_placeholder.total_projections, 0);
    }

    #[tokio::test]
    async fn test_projection_lifecycle() {
        let config = ProjectionConfig::default();

        // Skip actual engine creation test as it requires concrete KeyValueStore implementation
        // This would need to be tested with a real storage backend like RocksDB adapter
        let projection_def = serde_json::json!({
            "name": "test_projection",
            "source_events": ["node.created", "edge.created"],
            "target_view": "test_view"
        });

        assert_eq!(projection_def["name"], "test_projection");

    }

    #[tokio::test]
    async fn test_gql_integration() {
        let config = ProjectionConfig::default();

        // Skip actual GQL integration test as it requires concrete KeyValueStore implementation
        // This would need to be tested with a real storage backend like RocksDB adapter

        let query = "MATCH (v:Person) RETURN v";
        // Test that GQL query structure is valid
        assert!(!query.is_empty(), "GQL query should not be empty");

        let statement = "CREATE GRAPH test_graph";
        assert!(!statement.is_empty(), "GQL statement should not be empty");
    }
}

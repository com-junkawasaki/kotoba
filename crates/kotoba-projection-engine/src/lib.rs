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
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

use kotoba_ocel::OcelEvent;
use kotoba_graphdb::GraphDB;

pub mod event_processor;
pub mod materializer;
pub mod view_manager;
pub mod storage;
pub mod cache_integration;
pub mod metrics;

// Re-export main types
pub use event_processor::*;
pub use materializer::*;
pub use view_manager::*;
pub use storage::*;
pub use cache_integration::*;
pub use metrics::*;

/// Main projection engine
pub struct ProjectionEngine {
    /// Event processor for handling OCEL events
    event_processor: Arc<EventProcessor>,
    /// Materializer for direct GraphDB projections
    materializer: Arc<Materializer>,
    /// GraphDB instance
    graphdb: Arc<GraphDB>,
    /// Cache integration
    cache_integration: Arc<CacheIntegration>,
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
    /// RocksDB data directory
    pub rocksdb_path: String,
    /// Maximum concurrent projections
    pub max_concurrent_projections: usize,
    /// Batch size for event processing
    pub batch_size: usize,
    /// Checkpoint interval (in events)
    pub checkpoint_interval: u64,
    /// Cache integration settings
    pub cache_config: CacheConfig,
    /// Metrics collection
    pub enable_metrics: bool,
}

impl Default for ProjectionConfig {
    fn default() -> Self {
        Self {
            rocksdb_path: "./data/projections".to_string(),
            max_concurrent_projections: 10,
            batch_size: 100,
            checkpoint_interval: 1000,
            cache_config: CacheConfig::default(),
            enable_metrics: true,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache TTL for projection results
    pub ttl_seconds: u64,
    /// Maximum cache size
    pub max_size: usize,
    /// Cache invalidation strategy
    pub invalidation_strategy: InvalidationStrategy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 3600, // 1 hour
            max_size: 10000,
            invalidation_strategy: InvalidationStrategy::TimeBased,
        }
    }
}

/// Cache invalidation strategy
#[derive(Debug, Clone)]
pub enum InvalidationStrategy {
    /// Time-based invalidation
    TimeBased,
    /// Event-based invalidation
    EventBased,
    /// Hybrid approach
    Hybrid,
}

/// Projection state
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
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

impl ProjectionEngine {
    /// Create a new projection engine
    pub async fn new(config: ProjectionConfig) -> Result<Self> {
        info!("Initializing Projection Engine with config: {:?}", config);

        // Initialize GraphDB
        let graphdb_path = format!("{}/graphdb", config.rocksdb_path);
        let graphdb = Arc::new(GraphDB::new(&graphdb_path).await?);

        // Initialize cache integration
        let cache_integration = Arc::new(CacheIntegration::new(config.cache_config.clone()));

        // Initialize materializer
        let materializer = Arc::new(Materializer::new(
            &graphdb_path,
            cache_integration.clone(),
        ).await?);

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
            graphdb,
            cache_integration,
            config,
            active_projections: Arc::new(DashMap::new()),
            shutdown_tx,
            shutdown_rx: Arc::new(RwLock::new(shutdown_rx)),
        };

        info!("Projection Engine initialized successfully");
        Ok(engine)
    }

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

        self.active_projections.insert(name, state);

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
            counter!("projection_engine.events_received", events.len() as u64);
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

    /// Query the GraphDB directly
    #[instrument(skip(self, query))]
    pub async fn query_graph(&self, query: GraphQuery) -> Result<QueryResult> {
        // Check cache first
        let cache_key = format!("graph_query:{:?}", query);
        if let Some(cached_result) = self.cache_integration.get_cached_result("graphdb", &serde_json::json!(query)).await? {
            if self.config.enable_metrics {
                counter!("projection_engine.cache_hits");
            }
            return Ok(cached_result);
        }

        if self.config.enable_metrics {
            counter!("projection_engine.cache_misses");
        }

        // Query the GraphDB directly
        let result = self.graphdb.execute_query(query.clone()).await?;

        // Cache the result
        self.cache_integration.cache_result("graphdb", &serde_json::json!(query), &serde_json::json!(result)).await?;

        Ok(result)
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

        EngineStatistics {
            total_projections: self.active_projections.len(),
            active_projections,
            total_events_processed: total_events,
            uptime_seconds: 0, // TODO: Track uptime
            storage_size_bytes: self.storage.get_size().unwrap_or(0),
        }
    }

    async fn load_existing_projections(&self) -> Result<()> {
        info!("Loading existing projections");

        let projections = self.view_manager.list_projections().await?;
        for projection_name in projections {
            // Load projection state from storage
            if let Some(state) = self.storage.load_projection_state(&projection_name).await? {
                self.active_projections.insert(projection_name.clone(), state);
                self.event_processor.register_projection(&projection_name).await?;
            }
        }

        info!("Loaded {} existing projections", self.active_projections.len());
        Ok(())
    }

    async fn save_projection_states(&self) -> Result<()> {
        for projection in self.active_projections.iter() {
            self.storage.save_projection_state(&projection.key(), &projection.value()).await?;
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
pub type ViewQuery = serde_json::Value;
pub type ViewResult = serde_json::Value;

// Placeholder types - these will be replaced with actual implementations
pub type GraphQuery = kotoba_graphdb::GraphQuery;
pub type QueryResult = kotoba_graphdb::QueryResult;
pub type ViewQuery = serde_json::Value;
pub type ViewResult = serde_json::Value;
pub type EventEnvelope = serde_json::Value;

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
        let config = ProjectionConfig {
            rocksdb_path: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };

        let engine = ProjectionEngine::new(config).await;
        assert!(engine.is_ok(), "Projection engine should be created successfully");

        let engine = engine.unwrap();
        let stats = engine.get_statistics().await;
        assert_eq!(stats.total_projections, 0);
    }

    #[tokio::test]
    async fn test_projection_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let config = ProjectionConfig {
            rocksdb_path: temp_dir.path().to_string_lossy().to_string(),
            ..Default::default()
        };

        let engine = ProjectionEngine::new(config).await.unwrap();

        // Test projection creation
        let projection_def = serde_json::json!({
            "name": "test_projection",
            "source_events": ["node.created", "edge.created"],
            "target_view": "test_view"
        });

        let result = engine.create_projection("test_projection".to_string(), projection_def).await;
        assert!(result.is_ok(), "Projection should be created successfully");

        // Test projection listing
        let projections = engine.list_projections().await;
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0], "test_projection");

        // Test projection status
        let status = engine.get_projection_status("test_projection").await.unwrap();
        assert!(status.is_some());
        assert_eq!(status.unwrap().name, "test_projection");

        // Test projection deletion
        let result = engine.delete_projection("test_projection").await;
        assert!(result.is_ok(), "Projection should be deleted successfully");

        let projections = engine.list_projections().await;
        assert_eq!(projections.len(), 0);
    }
}

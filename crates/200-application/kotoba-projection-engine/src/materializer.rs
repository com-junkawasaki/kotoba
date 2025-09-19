//! Materializer
//!
//! Materializes OCEL v2 events using KeyValueStore interface.

use std::sync::Arc;
use anyhow::Result;
use tracing::{info, warn};
use dashmap::DashMap;
use chrono::Utc;

use kotoba_ocel::OcelEvent;
use kotoba_storage::KeyValueStore;

use crate::EventEnvelope;

/// Materializer for projections using KeyValueStore
pub struct Materializer<T: KeyValueStore> {
    /// Storage backend
    storage: Arc<T>,
    /// Storage prefix for materialized data
    prefix: String,
    /// Active materialization tasks
    active_tasks: Arc<DashMap<String, MaterializationTask>>,
}

/// Materialization task
#[derive(Debug, Clone)]
struct MaterializationTask {
    /// Task ID
    id: String,
    /// Projection name
    projection_name: String,
    /// Task status
    status: TaskStatus,
    /// Start time
    start_time: chrono::DateTime<chrono::Utc>,
}

/// Task status
#[derive(Debug, Clone)]
enum TaskStatus {
    Running,
    Completed,
    Failed(String),
}

/// Materialization result
#[derive(Debug, Clone)]
pub struct MaterializationResult {
    /// Projection name
    pub projection_name: String,
    /// Events processed
    pub events_processed: u64,
    /// Processing time
    pub processing_time_ms: u64,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl<T: KeyValueStore + 'static> Materializer<T> {
    /// Create a new materializer
    pub fn new(
        storage: Arc<T>,
        prefix: String,
    ) -> Self {
        Self {
            storage,
            prefix,
            active_tasks: Arc::new(DashMap::new()),
        }
    }

    /// Process an OCEL event for materialization
    pub async fn process_ocel_event(&self, ocel_event: OcelEvent) -> Result<()> {
        info!("Processing OCEL event: {} ({})", ocel_event.id, ocel_event.activity);

        // Store event data in KeyValueStore
        let event_key = format!("{}:event:{}", self.prefix, ocel_event.id);
        let event_data = serde_json::json!({
            "id": ocel_event.id,
            "activity": ocel_event.activity,
            "timestamp": ocel_event.timestamp,
            "omap": ocel_event.omap,
            "vmap": ocel_event.vmap
        });

        let serialized_data = serde_json::to_vec(&event_data)?;
        self.storage.put(event_key.as_bytes(), &serialized_data).await?;

        info!("Successfully processed OCEL event: {}", ocel_event.id);
        Ok(())
    }

    /// Process an event for materialization (legacy method)
    pub async fn process_event(&self, event_type: String, event: EventEnvelope) -> Result<()> {
        warn!("Legacy event processing is deprecated. Use process_ocel_event instead.");
        Ok(())
    }
}
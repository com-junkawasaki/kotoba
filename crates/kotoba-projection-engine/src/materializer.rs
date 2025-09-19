//! Materializer
//!
//! Materializes OCEL v2 events into RocksDB-based GraphDB with real-time updates.

use std::collections::{HashMap, BTreeMap};
use std::sync::Arc;
use anyhow::{Result, Context};
use tracing::{info, warn, error, instrument};
use serde::{Deserialize, Serialize};
use dashmap::DashMap;
use chrono::Utc;

use kotoba_ocel::{OcelEvent, OcelObject, OcelValue, ValueMap};
use kotoba_graphdb::{GraphDB, Node, Edge, PropertyValue};
use crate::cache_integration::CacheIntegration;

/// Materializer for GraphDB projections
pub struct Materializer {
    /// GraphDB instance
    graphdb: Arc<GraphDB>,
    /// Cache integration
    cache: Arc<CacheIntegration>,
    /// Active materialization tasks
    active_tasks: Arc<DashMap<String, MaterializationTask>>,
    /// Object type mappings
    object_mappings: Arc<DashMap<String, String>>,
    /// Activity to edge label mappings
    activity_mappings: Arc<DashMap<String, String>>,
}

/// Materialization task
#[derive(Debug)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterializationResult {
    /// Projection name
    pub projection_name: String,
    /// Events processed
    pub events_processed: u64,
    /// Processing time
    pub processing_time_ms: u64,
    /// Cache invalidations
    pub cache_invalidations: u64,
    /// Errors encountered
    pub errors: Vec<String>,
}

impl Default for Materializer {
    fn default() -> Self {
        // Placeholder default implementation
        Self {
            graphdb: Arc::new(GraphDB::new("./data/graphdb").await.unwrap()),
            cache: Arc::new(CacheIntegration::default()),
            active_tasks: Arc::new(DashMap::new()),
            object_mappings: Arc::new(DashMap::new()),
            activity_mappings: Arc::new(DashMap::new()),
        }
    }
}

impl Materializer {
    /// Create a new materializer
    pub async fn new(
        graphdb_path: &str,
        cache: Arc<CacheIntegration>,
    ) -> Result<Self> {
        let graphdb = Arc::new(GraphDB::new(graphdb_path).await?);

        // Initialize default mappings
        let object_mappings = Arc::new(DashMap::new());
        object_mappings.insert("order".to_string(), "Order".to_string());
        object_mappings.insert("item".to_string(), "Item".to_string());
        object_mappings.insert("customer".to_string(), "Customer".to_string());

        let activity_mappings = Arc::new(DashMap::new());
        activity_mappings.insert("create_order".to_string(), "CREATED".to_string());
        activity_mappings.insert("add_item".to_string(), "CONTAINS".to_string());
        activity_mappings.insert("place_order".to_string(), "PLACED_BY".to_string());

        Ok(Self {
            graphdb,
            cache,
            active_tasks: Arc::new(DashMap::new()),
            object_mappings,
            activity_mappings,
        })
    }

    /// Process an OCEL event for materialization
    #[instrument(skip(self, ocel_event))]
    pub async fn process_ocel_event(&self, ocel_event: OcelEvent) -> Result<()> {
        info!("Processing OCEL event: {} ({})", ocel_event.id, ocel_event.activity);

        // Start materialization task
        let task_id = format!("ocel-{}-{}", ocel_event.id, uuid::Uuid::new_v4());
        let task = MaterializationTask {
            id: task_id.clone(),
            projection_name: "default".to_string(), // Default projection
            status: TaskStatus::Running,
            start_time: Utc::now(),
        };

        self.active_tasks.insert(task_id.clone(), task);

        // Materialize the OCEL event
        let result = self.materialize_ocel_event(&ocel_event).await;

        // Update task status
        if let Some(mut task) = self.active_tasks.get_mut(&task_id) {
            match &result {
                Ok(_) => task.status = TaskStatus::Completed,
                Err(e) => task.status = TaskStatus::Failed(e.to_string()),
            }
        }

        result
    }

    /// Process an event for materialization (legacy method)
    #[instrument(skip(self, event))]
    pub async fn process_event(&self, event_type: String, event: EventEnvelope) -> Result<()> {
        // Convert legacy event to OCEL format if possible
        warn!("Legacy event processing is deprecated. Use process_ocel_event instead.");
        Ok(())
    }

    /// Materialize an OCEL event into GraphDB
    #[instrument(skip(self, ocel_event))]
    async fn materialize_ocel_event(&self, ocel_event: &OcelEvent) -> Result<()> {
        info!("Materializing OCEL event: {} with {} objects", ocel_event.id, ocel_event.omap.len());

        // Start a transaction for atomic materialization
        let mut tx = self.graphdb.begin_transaction().await;

        // Create or update event node
        let event_node_id = self.materialize_event_node(&mut tx, ocel_event).await?;

        // Process all objects related to this event
        for object_id in &ocel_event.omap {
            // Get or create object node
            let object_node_id = self.materialize_object_node(&mut tx, object_id).await?;

            // Create relationship edge between event and object
            self.materialize_event_object_relationship(&mut tx, &event_node_id, &object_node_id, ocel_event).await?;
        }

        // Commit the transaction
        tx.commit().await?;

        // Invalidate relevant cache entries
        self.invalidate_event_cache(ocel_event).await?;

        info!("Successfully materialized OCEL event: {}", ocel_event.id);
        Ok(())
    }

    /// Materialize an event node
    async fn materialize_event_node(&self, tx: &mut kotoba_graphdb::GraphTransaction<'_>, ocel_event: &OcelEvent) -> Result<String> {
        let event_node_id = format!("event:{}", ocel_event.id);

        // Convert OCEL event properties to GraphDB properties
        let mut properties = BTreeMap::new();
        properties.insert("activity".to_string(), PropertyValue::String(ocel_event.activity.clone()));
        properties.insert("timestamp".to_string(), PropertyValue::Date(ocel_event.timestamp));

        // Add event attributes
        for (key, value) in &ocel_event.vmap {
            properties.insert(key.clone(), self.ocel_value_to_property_value(value));
        }

        // Create or update event node
        let labels = vec!["Event".to_string(), "OcelEvent".to_string()];

        tx.create_node(
            Some(event_node_id.clone()),
            labels,
            properties,
        ).await?;

        Ok(event_node_id)
    }

    /// Materialize an object node
    async fn materialize_object_node(&self, tx: &mut kotoba_graphdb::GraphTransaction<'_>, object_id: &str) -> Result<String> {
        let object_node_id = format!("object:{}", object_id);

        // For now, create a basic object node
        // In a real implementation, you'd fetch the actual OCEL object data
        let mut properties = BTreeMap::new();
        properties.insert("object_id".to_string(), PropertyValue::String(object_id.to_string()));

        let labels = vec!["Object".to_string(), "OcelObject".to_string()];

        tx.create_node(
            Some(object_node_id.clone()),
            labels,
            properties,
        ).await?;

        Ok(object_node_id)
    }

    /// Materialize relationship between event and object
    async fn materialize_event_object_relationship(
        &self,
        tx: &mut kotoba_graphdb::GraphTransaction<'_>,
        event_node_id: &str,
        object_node_id: &str,
        ocel_event: &OcelEvent,
    ) -> Result<()> {
        // Determine edge label based on activity
        let edge_label = self.activity_mappings
            .get(&ocel_event.activity)
            .map(|l| l.clone())
            .unwrap_or_else(|| "RELATED_TO".to_string());

        // Create edge properties
        let mut properties = BTreeMap::new();
        properties.insert("event_id".to_string(), PropertyValue::String(ocel_event.id.clone()));
        properties.insert("activity".to_string(), PropertyValue::String(ocel_event.activity.clone()));
        properties.insert("timestamp".to_string(), PropertyValue::Date(ocel_event.timestamp));

        // Create the relationship edge
        let edge_id = format!("rel:{}_{}", event_node_id, object_node_id);

        tx.create_edge(
            Some(edge_id),
            event_node_id,
            object_node_id,
            edge_label,
            properties,
        ).await?;

        Ok(())
    }

    /// Convert OCEL value to GraphDB property value
    fn ocel_value_to_property_value(&self, ocel_value: &OcelValue) -> PropertyValue {
        match ocel_value {
            OcelValue::String(s) => PropertyValue::String(s.clone()),
            OcelValue::Integer(i) => PropertyValue::Integer(*i),
            OcelValue::Float(f) => PropertyValue::Float(*f),
            OcelValue::Boolean(b) => PropertyValue::Boolean(*b),
            OcelValue::Date(dt) => PropertyValue::Date(*dt),
            OcelValue::List(values) => {
                // Convert to string representation for now
                let str_values: Vec<String> = values.iter()
                    .map(|v| match v {
                        OcelValue::String(s) => s.clone(),
                        _ => format!("{:?}", v),
                    })
                    .collect();
                PropertyValue::String(format!("{:?}", str_values))
            }
            OcelValue::Map(map) => {
                // Convert to string representation for now
                PropertyValue::String(format!("{:?}", map))
            }
        }
    }

    /// Invalidate cache entries related to the event
    async fn invalidate_event_cache(&self, ocel_event: &OcelEvent) -> Result<()> {
        // Invalidate cache for the event
        let event_cache_key = format!("event:{}", ocel_event.id);
        self.cache.invalidate_node_cache(&event_cache_key).await?;

        // Invalidate cache for related objects
        for object_id in &ocel_event.omap {
            let object_cache_key = format!("object:{}", object_id);
            self.cache.invalidate_node_cache(&object_cache_key).await?;
        }

        // Invalidate activity-based cache
        let activity_cache_key = format!("activity:{}", ocel_event.activity);
        self.cache.invalidate_node_cache(&activity_cache_key).await?;

        Ok(())
    }


    /// Get active tasks
    pub fn get_active_tasks(&self) -> Vec<MaterializationTask> {
        self.active_tasks.iter().map(|t| t.clone()).collect()
    }

    /// Get task status
    pub fn get_task_status(&self, task_id: &str) -> Option<MaterializationTask> {
        self.active_tasks.get(task_id).map(|t| t.clone())
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_data_extraction() {
        let event = serde_json::json!({
            "event_type": "node.created",
            "data": {
                "id": "node123",
                "labels": ["Person"],
                "properties": {
                    "name": "Alice",
                    "age": 30
                }
            }
        });

        let materializer = Materializer::default();
        let event_data = materializer.extract_event_data(&event).unwrap();

        assert_eq!(event_data.event_type, "node.created");
        assert_eq!(event_data.get_string("id").unwrap(), "node123");
        assert_eq!(event_data.get_array("labels").unwrap(), vec!["Person"]);
    }

    #[tokio::test]
    async fn test_materializer_creation() {
        let materializer = Materializer::default();
        let tasks = materializer.get_active_tasks();
        assert_eq!(tasks.len(), 0);
    }
}

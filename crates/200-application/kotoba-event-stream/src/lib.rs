//! `kotoba-event-stream`
//!
//! Event streaming component for KotobaDB.
//! Provides publish/subscribe functionality for event sourcing using KeyValueStore interface.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error, instrument};
use dashmap::DashMap;
use bincode;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use kotoba_storage::KeyValueStore;

/// Core event types for the event sourcing system
pub mod event;
pub use event::*;

/// Event storage and retrieval
pub mod storage;
pub use storage::*;

// Re-export EventStorage and TopicMetadata for convenience
pub use storage::{EventStorage, TopicMetadata};

/// Configuration for the event stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamConfig {
    /// Storage prefix for event keys
    pub storage_prefix: String,
    /// Maximum number of topics (column families)
    pub max_topics: usize,
    /// Maximum events per batch
    pub max_batch_size: usize,
    /// Retention period for events (in hours)
    pub retention_hours: u64,
    /// Enable compression
    pub enable_compression: bool,
    /// Enable metrics collection
    pub enable_metrics: bool,
}

impl Default for EventStreamConfig {
    fn default() -> Self {
        Self {
            storage_prefix: "events".to_string(),
            max_topics: 100,
            max_batch_size: 1000,
            retention_hours: 168, // 7 days
            enable_compression: true,
            enable_metrics: true,
        }
    }
}

/// Main event stream interface
#[async_trait]
pub trait EventStreamPort {
    /// Publish an event to the stream
    async fn publish(&self, event: EventEnvelope) -> Result<EventId>;

    /// Subscribe to events from the stream
    async fn subscribe(&self, topic: &str, handler: EventHandler) -> Result<()>;

    /// Get event by ID
    async fn get_event(&self, event_id: &EventId) -> Result<Option<EventEnvelope>>;

    /// Get events by aggregate ID
    async fn get_events_by_aggregate(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>>;

    /// Create a new topic
    async fn create_topic(&self, topic: &str) -> Result<()>;

    /// Delete a topic
    async fn delete_topic(&self, topic: &str) -> Result<()>;

    /// Get topic statistics
    async fn get_topic_stats(&self, topic: &str) -> Result<TopicStats>;

    /// List all topics
    async fn list_topics(&self) -> Result<Vec<String>>;
}

/// Event handler function type
pub type EventHandler = Box<dyn Fn(EventEnvelope) -> Result<()> + Send + Sync>;

/// Topic statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicStats {
    pub topic_name: String,
    pub event_count: u64,
    pub first_offset: u64,
    pub last_offset: u64,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}

/// Consumer offset information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsumerOffset {
    pub consumer_group: String,
    pub topic: String,
    pub partition: u32,
    pub offset: u64,
    pub last_updated: DateTime<Utc>,
}

/// Main event stream implementation using KeyValueStore
pub struct EventStream<T: KeyValueStore> {
    config: EventStreamConfig,
    storage: Arc<EventStorage<T>>,
    subscribers: Arc<DashMap<String, Vec<EventHandler>>>,
    consumer_offsets: Arc<DashMap<String, ConsumerOffset>>,
}

impl<T: KeyValueStore> EventStream<T> {
    /// Create a new event stream with the given KeyValueStore backend
    pub fn new(config: EventStreamConfig, storage: Arc<T>) -> Self {
        info!("Created event stream with storage backend");

        Self {
            config: config.clone(),
            storage: Arc::new(EventStorage::new(
                storage,
                config.storage_prefix,
                config.max_topics
            )),
            subscribers: Arc::new(DashMap::new()),
            consumer_offsets: Arc::new(DashMap::new()),
        }
    }

    /// Create topic name with validation
    fn validate_topic_name(&self, topic: &str) -> Result<String> {
        if topic.is_empty() {
            return Err(anyhow::anyhow!("Topic name cannot be empty"));
        }
        if topic.len() > 255 {
            return Err(anyhow::anyhow!("Topic name too long"));
        }
        Ok(topic.to_string())
    }
}

#[async_trait]
impl<T: KeyValueStore> EventStreamPort for EventStream<T> {
    async fn publish(&self, event: EventEnvelope) -> Result<EventId> {
        // Default topic if none specified
        let topic = "all".to_string();

        // Store event using EventStorage
        self.storage.store_event(&topic, &event).await?;

        // Notify subscribers
        if let Some(handlers) = self.subscribers.get(&topic) {
            for handler in handlers.iter() {
                if let Err(e) = handler(event.clone()) {
                    error!("Event handler error: {}", e);
                }
            }
        }

        info!("Published event: {} to topic: {}", event.id.0, topic);
        Ok(event.id)
    }

    async fn subscribe(&self, topic: &str, handler: EventHandler) -> Result<()> {
        let topic_name = self.validate_topic_name(topic)?;

        // Add handler to subscribers
        self.subscribers
            .entry(topic_name.clone())
            .or_insert_with(Vec::new)
            .push(handler);

        info!("Subscribed to topic: {}", topic_name);
        Ok(())
    }

    async fn get_event(&self, event_id: &EventId) -> Result<Option<EventEnvelope>> {
        self.storage.get_event(event_id).await
    }

    async fn get_events_by_aggregate(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>> {
        // For now, return empty vec - need to implement aggregate-based querying
        // This would require scanning keys with the aggregate prefix
        warn!("get_events_by_aggregate not fully implemented yet");
        Ok(Vec::new())
    }

    async fn create_topic(&self, topic: &str) -> Result<()> {
        let topic_name = self.validate_topic_name(topic)?;
        self.storage.create_topic(&topic_name).await?;
        info!("Created topic: {}", topic_name);
        Ok(())
    }

    async fn delete_topic(&self, topic: &str) -> Result<()> {
        let topic_name = self.validate_topic_name(topic)?;
        self.storage.delete_topic(&topic_name).await?;
        self.subscribers.remove(&topic_name);
        info!("Deleted topic: {}", topic_name);
        Ok(())
    }

    async fn get_topic_stats(&self, topic: &str) -> Result<TopicStats> {
        let topic_name = self.validate_topic_name(topic)?;
        self.storage.get_topic_stats(&topic_name).await
    }

    async fn list_topics(&self) -> Result<Vec<String>> {
        self.storage.list_topics().await
    }
}

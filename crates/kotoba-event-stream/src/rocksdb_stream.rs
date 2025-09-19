//! RocksDB-based event stream implementation
//!
//! Provides Kafka-like functionality using RocksDB for persistent event streaming.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use dashmap::DashMap;
use anyhow::{Result, Context};
use tracing::{info, warn, error, instrument};
use bincode;
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::event::*;
use crate::storage::*;
use crate::{EventEnvelope, EventHandler, EventStreamConfig};

/// RocksDB-based event stream implementation
pub struct RocksDBStream {
    /// Configuration
    config: EventStreamConfig,
    /// Storage backend
    storage: Arc<RocksDBStorage>,
    /// Active subscribers
    subscribers: Arc<DashMap<String, Vec<EventHandler>>>,
    /// Consumer group offsets
    consumer_offsets: Arc<DashMap<String, HashMap<String, u64>>>,
    /// Event notification channels
    notification_channels: Arc<DashMap<String, Vec<mpsc::Sender<EventEnvelope>>>>,
}

impl RocksDBStream {
    /// Create a new RocksDB event stream
    pub async fn new(config: EventStreamConfig) -> Result<Self> {
        let storage = Arc::new(RocksDBStorage::new(&config.rocksdb_path, config.max_topics).await?);

        info!("Created RocksDB event stream with config: {:?}", config);

        Ok(Self {
            config,
            storage,
            subscribers: Arc::new(DashMap::new()),
            consumer_offsets: Arc::new(DashMap::new()),
            notification_channels: Arc::new(DashMap::new()),
        })
    }

    /// Publish an event to a topic
    #[instrument(skip(self, event))]
    pub async fn publish_event(&self, topic: &str, event: EventEnvelope) -> Result<()> {
        // Store event
        self.storage.store_event(topic, &event).await?;

        // Notify subscribers
        self.notify_subscribers(topic, &event).await?;

        info!("Published event {} to topic {}", event.id.0, topic);
        Ok(())
    }

    /// Subscribe to a topic with a handler
    #[instrument(skip(self, handler))]
    pub async fn subscribe_to_topic(&self, topic: &str, handler: EventHandler) -> Result<()> {
        // Ensure topic exists
        if !self.storage.topic_exists(topic).await? {
            self.storage.create_topic(topic).await?;
        }

        // Add handler to subscribers
        self.subscribers
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(handler);

        info!("Subscribed to topic: {}", topic);
        Ok(())
    }

    /// Consume events from a topic with consumer group
    #[instrument(skip(self))]
    pub async fn consume_events(
        &self,
        topic: &str,
        consumer_group: &str,
        batch_size: usize,
    ) -> Result<Vec<EventEnvelope>> {
        // Get current offset for consumer group
        let current_offset = self.get_consumer_offset(consumer_group, topic).await?;

        // Get events from current offset
        let events = self.get_events_from_offset(topic, current_offset, batch_size).await?;

        // Update consumer offset
        if let Some(last_event) = events.last() {
            self.update_consumer_offset(consumer_group, topic, last_event.sequence_number).await?;
        }

        Ok(events)
    }

    /// Get events from a specific offset
    #[instrument(skip(self))]
    pub async fn get_events_from_offset(
        &self,
        topic: &str,
        offset: u64,
        max_count: usize,
    ) -> Result<Vec<EventEnvelope>> {
        // This is a simplified implementation
        // In a real implementation, you'd query the storage layer efficiently
        let mut events = Vec::new();
        let mut current_offset = offset;

        // Get topic stats to know the range
        let stats = self.storage.get_topic_stats(topic).await?;
        let end_offset = stats.last_offset;

        while current_offset <= end_offset && events.len() < max_count {
            if let Some(event) = self.storage.get_event_at_offset(topic, current_offset).await? {
                events.push(event);
            }
            current_offset += 1;
        }

        Ok(events)
    }

    /// Create a consumer group
    #[instrument(skip(self))]
    pub async fn create_consumer_group(&self, group_name: &str) -> Result<()> {
        if self.consumer_offsets.contains_key(group_name) {
            return Err(anyhow::anyhow!("Consumer group '{}' already exists", group_name));
        }

        self.consumer_offsets.insert(group_name.to_string(), HashMap::new());
        info!("Created consumer group: {}", group_name);
        Ok(())
    }

    /// Delete a consumer group
    #[instrument(skip(self))]
    pub async fn delete_consumer_group(&self, group_name: &str) -> Result<()> {
        self.consumer_offsets.remove(group_name);
        info!("Deleted consumer group: {}", group_name);
        Ok(())
    }

    /// Get consumer offset for a topic
    async fn get_consumer_offset(&self, consumer_group: &str, topic: &str) -> Result<u64> {
        if let Some(group_offsets) = self.consumer_offsets.get(consumer_group) {
            Ok(*group_offsets.get(topic).unwrap_or(&0))
        } else {
            Ok(0) // Start from beginning
        }
    }

    /// Update consumer offset
    async fn update_consumer_offset(&self, consumer_group: &str, topic: &str, offset: u64) -> Result<()> {
        if let Some(mut group_offsets) = self.consumer_offsets.get_mut(consumer_group) {
            group_offsets.insert(topic.to_string(), offset);
        }
        Ok(())
    }

    /// Notify subscribers of new events
    async fn notify_subscribers(&self, topic: &str, event: &EventEnvelope) -> Result<()> {
        if let Some(handlers) = self.subscribers.get(topic) {
            for handler in handlers.iter() {
                if let Err(e) = handler(event.clone()) {
                    error!("Event handler error for topic {}: {}", topic, e);
                }
            }
        }

        // Notify notification channels
        if let Some(channels) = self.notification_channels.get(topic) {
            for channel in channels.iter() {
                if let Err(e) = channel.send(event.clone()).await {
                    warn!("Failed to send event notification: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Create a notification channel for a topic
    #[instrument(skip(self))]
    pub async fn create_notification_channel(&self, topic: &str) -> Result<mpsc::Receiver<EventEnvelope>> {
        let (tx, rx) = mpsc::channel(1000);

        self.notification_channels
            .entry(topic.to_string())
            .or_insert_with(Vec::new)
            .push(tx);

        info!("Created notification channel for topic: {}", topic);
        Ok(rx)
    }

    /// Get topic information
    #[instrument(skip(self))]
    pub async fn get_topic_info(&self, topic: &str) -> Result<TopicInfo> {
        let stats = self.storage.get_topic_stats(topic).await?;
        let consumer_groups = self.get_consumer_groups_for_topic(topic).await?;

        Ok(TopicInfo {
            name: stats.topic_name,
            event_count: stats.event_count,
            first_offset: stats.first_offset,
            last_offset: stats.last_offset,
            created_at: stats.created_at,
            size_bytes: stats.size_bytes,
            consumer_groups,
        })
    }

    /// Get consumer groups for a topic
    async fn get_consumer_groups_for_topic(&self, topic: &str) -> Result<Vec<String>> {
        let mut groups = Vec::new();

        for entry in self.consumer_offsets.iter() {
            if entry.value().contains_key(topic) {
                groups.push(entry.key().clone());
            }
        }

        Ok(groups)
    }

    /// List all topics
    #[instrument(skip(self))]
    pub async fn list_topics(&self) -> Result<Vec<String>> {
        self.storage.list_topics().await
    }

    /// List all consumer groups
    #[instrument(skip(self))]
    pub async fn list_consumer_groups(&self) -> Result<Vec<String>> {
        Ok(self.consumer_offsets.iter().map(|e| e.key().clone()).collect())
    }

    /// Get stream statistics
    #[instrument(skip(self))]
    pub async fn get_statistics(&self) -> Result<StreamStatistics> {
        let topics = self.storage.list_topics().await?;
        let consumer_groups = self.consumer_offsets.len();

        let mut total_events = 0u64;
        let mut total_size = 0u64;

        for topic in &topics {
            let stats = self.storage.get_topic_stats(topic).await?;
            total_events += stats.event_count;
            total_size += stats.size_bytes;
        }

        Ok(StreamStatistics {
            topic_count: topics.len(),
            consumer_group_count: consumer_groups,
            total_events,
            total_size_bytes: total_size,
        })
    }

    /// Compact old events based on retention policy
    #[instrument(skip(self))]
    pub async fn compact_events(&self) -> Result<()> {
        // This is a placeholder for event compaction
        // In a real implementation, you'd remove events older than retention period
        info!("Event compaction placeholder - not implemented yet");
        Ok(())
    }
}

/// Topic information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TopicInfo {
    pub name: String,
    pub event_count: u64,
    pub first_offset: u64,
    pub last_offset: u64,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub consumer_groups: Vec<String>,
}

/// Stream statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StreamStatistics {
    pub topic_count: usize,
    pub consumer_group_count: usize,
    pub total_events: u64,
    pub total_size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rocksdb_stream_creation() {
        let temp_dir = tempdir().unwrap();
        let config = EventStreamConfig {
            rocksdb_path: temp_dir.path().to_str().unwrap().to_string(),
            max_topics: 10,
            max_batch_size: 100,
            retention_hours: 24,
            enable_compression: true,
            enable_metrics: false,
        };

        let stream = RocksDBStream::new(config).await;
        assert!(stream.is_ok(), "RocksDB stream should be created successfully");
    }

    #[tokio::test]
    async fn test_topic_operations() {
        let temp_dir = tempdir().unwrap();
        let config = EventStreamConfig {
            rocksdb_path: temp_dir.path().to_str().unwrap().to_string(),
            max_topics: 10,
            max_batch_size: 100,
            retention_hours: 24,
            enable_compression: true,
            enable_metrics: false,
        };

        let stream = RocksDBStream::new(config).await.unwrap();

        // Subscribe to topic (creates topic if it doesn't exist)
        let handler = Box::new(|event: EventEnvelope| {
            println!("Received event: {}", event.id.0);
            Ok(())
        });

        let result = stream.subscribe_to_topic("test_topic", handler).await;
        assert!(result.is_ok(), "Should be able to subscribe to topic");

        // List topics
        let topics = stream.list_topics().await.unwrap();
        assert!(topics.contains(&"test_topic".to_string()));
    }

    #[tokio::test]
    async fn test_event_publish_and_consume() {
        let temp_dir = tempdir().unwrap();
        let config = EventStreamConfig {
            rocksdb_path: temp_dir.path().to_str().unwrap().to_string(),
            max_topics: 10,
            max_batch_size: 100,
            retention_hours: 24,
            enable_compression: true,
            enable_metrics: false,
        };

        let stream = RocksDBStream::new(config).await.unwrap();

        // Create test event
        let aggregate_id = AggregateId("test-aggregate".to_string());
        let event = EventEnvelope {
            id: EventId(Uuid::new_v4().to_string()),
            aggregate_id: aggregate_id.clone(),
            aggregate_type: "TestAggregate".to_string(),
            event_type: EventType("node.created".to_string()),
            sequence_number: 1,
            timestamp: Utc::now(),
            data: EventData {
                event_type: EventType("node.created".to_string()),
                payload: serde_json::json!({"name": "test_node"}),
                metadata: HashMap::new(),
            },
            metadata: HashMap::new(),
        };

        // Publish event
        let result = stream.publish_event("test_topic", event.clone()).await;
        assert!(result.is_ok(), "Event should be published successfully");

        // Consume events
        let events = stream.consume_events("test_topic", "test_group", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_consumer_groups() {
        let temp_dir = tempdir().unwrap();
        let config = EventStreamConfig {
            rocksdb_path: temp_dir.path().to_str().unwrap().to_string(),
            max_topics: 10,
            max_batch_size: 100,
            retention_hours: 24,
            enable_compression: true,
            enable_metrics: false,
        };

        let stream = RocksDBStream::new(config).await.unwrap();

        // Create consumer group
        let result = stream.create_consumer_group("test_group").await;
        assert!(result.is_ok(), "Consumer group should be created successfully");

        // List consumer groups
        let groups = stream.list_consumer_groups().await.unwrap();
        assert!(groups.contains(&"test_group".to_string()));
    }
}

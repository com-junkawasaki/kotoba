//! Storage layer for event streaming
//!
//! Provides persistent storage for events with topic-based organization
//! and efficient retrieval capabilities. Uses KeyValueStore trait for storage backend abstraction.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use async_trait::async_trait;
use tracing::{info, warn, error, instrument};
use bincode;
use chrono::{DateTime, Utc};

use kotoba_storage::KeyValueStore;
use crate::event::*;
use crate::{TopicStats, EventStreamConfig};

/// Topic metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TopicMetadata {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub event_count: u64,
    pub first_offset: u64,
    pub last_offset: u64,
    pub size_bytes: u64,
}

/// Generic event storage that works with any KeyValueStore implementation
pub struct EventStorage<T: KeyValueStore> {
    /// Storage backend
    storage: Arc<T>,
    /// Storage prefix for keys
    prefix: String,
    /// Maximum number of topics
    max_topics: usize,
    /// Topic metadata cache
    topic_metadata: Arc<std::sync::RwLock<HashMap<String, TopicMetadata>>>,
}

impl<T: KeyValueStore> EventStorage<T> {
    /// Create a new event storage with the given KeyValueStore backend
    pub fn new(storage: Arc<T>, prefix: String, max_topics: usize) -> Self {
        info!("Initializing event storage with backend, prefix: {}", prefix);

        Self {
            storage,
            prefix,
            max_topics,
            topic_metadata: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Store an event
    pub async fn store_event(&self, topic: &str, event: &EventEnvelope) -> Result<()> {
        let key = format!("{}:{}:{}", self.prefix, topic, event.id.0);
        let value = bincode::serialize(event)?;
        self.storage.put(key.as_bytes(), &value).await?;
        Ok(())
    }

    /// Get an event by ID
    pub async fn get_event(&self, event_id: &crate::event::EventId) -> Result<Option<EventEnvelope>> {
        let key = format!("{}:all:{}", self.prefix, event_id.0);
        match self.storage.get(key.as_bytes()).await? {
            Some(data) => {
                let event: EventEnvelope = bincode::deserialize(&data)?;
                Ok(Some(event))
            }
            None => Ok(None)
        }
    }

    /// Create a topic
    pub async fn create_topic(&self, topic_name: &str) -> Result<()> {
        let key = format!("{}:topic:{}", self.prefix, topic_name);
        let metadata = TopicMetadata {
            name: topic_name.to_string(),
            created_at: Utc::now(),
            event_count: 0,
            first_offset: 0,
            last_offset: 0,
            size_bytes: 0,
        };
        let value = bincode::serialize(&metadata)?;
        self.storage.put(key.as_bytes(), &value).await?;
        Ok(())
    }

    /// Delete a topic
    pub async fn delete_topic(&self, topic_name: &str) -> Result<()> {
        let key = format!("{}:topic:{}", self.prefix, topic_name);
        self.storage.delete(key.as_bytes()).await?;
        Ok(())
    }

    /// Get topic statistics
    pub async fn get_topic_stats(&self, topic_name: &str) -> Result<TopicStats> {
        let key = format!("{}:topic:{}", self.prefix, topic_name);
        match self.storage.get(key.as_bytes()).await? {
            Some(data) => {
                let metadata: TopicMetadata = bincode::deserialize(&data)?;
                Ok(TopicStats {
                    topic_name: metadata.name,
                    event_count: metadata.event_count,
                    first_offset: metadata.first_offset,
                    last_offset: metadata.last_offset,
                    created_at: metadata.created_at,
                    size_bytes: metadata.size_bytes,
                })
            }
            None => Err(anyhow::anyhow!("Topic not found: {}", topic_name))
        }
    }

    /// List all topics
    pub async fn list_topics(&self) -> Result<Vec<String>> {
        let prefix = format!("{}:topic:", self.prefix);
        let results = self.storage.scan(prefix.as_bytes()).await?;
        let mut topics = Vec::new();

        for (key, _) in results {
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Some(topic_name) = key_str.strip_prefix(&prefix) {
                    topics.push(topic_name.to_string());
                }
            }
        }

        Ok(topics)
    }
}
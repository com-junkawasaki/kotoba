//! RocksDB-based storage for event streaming
//!
//! Provides persistent storage for events with topic-based organization
//! and efficient retrieval capabilities.

use std::collections::HashMap;
use std::sync::Arc;
use anyhow::{Result, Context};
use rocksdb::{DB, ColumnFamilyDescriptor, Options, WriteBatch, IteratorMode};
use tracing::{info, warn, error, instrument};
use bincode;
use chrono::{DateTime, Utc};

use crate::event::*;
use crate::{TopicStats, EventStreamConfig};

/// RocksDB-based event storage
pub struct RocksDBStorage {
    /// RocksDB instance
    db: Arc<DB>,
    /// Maximum number of topics
    max_topics: usize,
    /// Topic metadata cache
    topic_metadata: Arc<std::sync::RwLock<HashMap<String, TopicMetadata>>>,
}

/// Topic metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TopicMetadata {
    name: String,
    created_at: DateTime<Utc>,
    event_count: u64,
    first_offset: u64,
    last_offset: u64,
    size_bytes: u64,
}

impl RocksDBStorage {
    /// Create a new RocksDB storage
    pub async fn new(path: &str, max_topics: usize) -> Result<Self> {
        info!("Initializing RocksDB storage at: {}", path);

        // Configure RocksDB options
        let mut db_opts = Options::default();
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
        db_opts.set_max_background_jobs(4);
        db_opts.set_compaction_style(rocksdb::DBCompactionStyle::Level);
        db_opts.set_target_file_size_base(64 * 1024 * 1024); // 64MB
        db_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);

        // Define column families
        let cf_names = vec![
            "default",
            "events",
            "topics",
            "offsets",
            "metadata",
        ];

        let cf_descriptors: Vec<ColumnFamilyDescriptor> = cf_names
            .iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                cf_opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect();

        // Open database
        let db = DB::open_cf_descriptors(&db_opts, path, cf_descriptors)
            .context("Failed to open RocksDB")?;

        let storage = Self {
            db: Arc::new(db),
            max_topics,
            topic_metadata: Arc::new(std::sync::RwLock::new(HashMap::new())),
        };

        // Load existing topic metadata
        storage.load_topic_metadata().await?;

        info!("RocksDB storage initialized successfully");
        Ok(storage)
    }

    /// Store an event in a topic
    #[instrument(skip(self, event))]
    pub async fn store_event(&self, topic: &str, event: &EventEnvelope) -> Result<()> {
        // Ensure topic exists
        self.ensure_topic_exists(topic).await?;

        // Serialize event
        let event_data = bincode::serialize(event)
            .context("Failed to serialize event")?;

        // Generate key: topic/sequence_number
        let sequence_number = self.get_next_sequence_number(topic).await?;
        let key = self.make_event_key(topic, sequence_number);

        // Store in events column family
        let cf_events = self.db.cf_handle("events")
            .context("Events CF not found")?;

        self.db.put_cf(cf_events, &key, event_data)
            .context("Failed to store event")?;

        // Store event ID mapping for fast lookup
        let id_key = self.make_event_id_key(&event.id);
        let id_value = self.make_event_pointer(topic, sequence_number);
        self.db.put_cf(cf_events, &id_key, id_value)
            .context("Failed to store event ID mapping")?;

        // Store aggregate ID mapping
        let aggregate_key = self.make_aggregate_key(&event.aggregate_id, sequence_number);
        let aggregate_value = self.make_event_pointer(topic, sequence_number);
        let cf_metadata = self.db.cf_handle("metadata")
            .context("Metadata CF not found")?;
        self.db.put_cf(cf_metadata, &aggregate_key, aggregate_value)
            .context("Failed to store aggregate mapping")?;

        // Update topic metadata
        self.update_topic_metadata(topic, sequence_number, event_data.len() as u64).await?;

        info!("Stored event: {} in topic: {} at offset: {}", event.id.0, topic, sequence_number);
        Ok(())
    }

    /// Get an event by ID
    #[instrument(skip(self))]
    pub async fn get_event(&self, event_id: &EventId) -> Result<Option<EventEnvelope>> {
        let cf_events = self.db.cf_handle("events")
            .context("Events CF not found")?;

        let id_key = self.make_event_id_key(event_id);

        match self.db.get_cf(cf_events, &id_key)? {
            Some(pointer_data) => {
                let (topic, offset) = self.parse_event_pointer(&pointer_data)?;
                let event_key = self.make_event_key(&topic, offset);

                match self.db.get_cf(cf_events, &event_key)? {
                    Some(event_data) => {
                        let event: EventEnvelope = bincode::deserialize(&event_data)
                            .context("Failed to deserialize event")?;
                        Ok(Some(event))
                    }
                    None => Ok(None),
                }
            }
            None => Ok(None),
        }
    }

    /// Get events by aggregate ID
    #[instrument(skip(self))]
    pub async fn get_events_by_aggregate(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>> {
        let cf_metadata = self.db.cf_handle("metadata")
            .context("Metadata CF not found")?;

        let mut events = Vec::new();
        let prefix = format!("aggregate/{}/", aggregate_id.0);

        let iter = self.db.iterator_cf(cf_metadata, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        for item in iter {
            let (key, value) = item.context("Iterator error")?;
            if key.starts_with(prefix.as_bytes()) {
                let (topic, offset) = self.parse_event_pointer(&value)?;
                if let Some(event) = self.get_event_at_offset(&topic, offset).await? {
                    events.push(event);
                }
            }
        }

        // Sort by sequence number
        events.sort_by_key(|e| e.sequence_number);
        Ok(events)
    }

    /// Create a new topic
    #[instrument(skip(self))]
    pub async fn create_topic(&self, topic: &str) -> Result<()> {
        // Check topic limit
        let current_topics = self.topic_metadata.read().unwrap().len();
        if current_topics >= self.max_topics {
            return Err(anyhow::anyhow!("Maximum number of topics ({}) reached", self.max_topics));
        }

        // Check if topic already exists
        if self.topic_exists(topic).await? {
            return Err(anyhow::anyhow!("Topic '{}' already exists", topic));
        }

        // Create topic metadata
        let metadata = TopicMetadata {
            name: topic.to_string(),
            created_at: Utc::now(),
            event_count: 0,
            first_offset: 0,
            last_offset: 0,
            size_bytes: 0,
        };

        // Store topic metadata
        let cf_topics = self.db.cf_handle("topics")
            .context("Topics CF not found")?;

        let metadata_key = self.make_topic_metadata_key(topic);
        let metadata_data = bincode::serialize(&metadata)
            .context("Failed to serialize topic metadata")?;

        self.db.put_cf(cf_topics, &metadata_key, metadata_data)
            .context("Failed to store topic metadata")?;

        // Update cache
        self.topic_metadata.write().unwrap().insert(topic.to_string(), metadata);

        info!("Created topic: {}", topic);
        Ok(())
    }

    /// Delete a topic
    #[instrument(skip(self))]
    pub async fn delete_topic(&self, topic: &str) -> Result<()> {
        // Remove all events for this topic
        let cf_events = self.db.cf_handle("events")
            .context("Events CF not found")?;

        let prefix = format!("{}/", topic);
        let mut batch = WriteBatch::default();

        let iter = self.db.iterator_cf(cf_events, IteratorMode::From(prefix.as_bytes(), rocksdb::Direction::Forward));
        for item in iter {
            let (key, _) = item.context("Iterator error")?;
            if key.starts_with(prefix.as_bytes()) {
                batch.delete_cf(cf_events, &key);
            }
        }

        self.db.write(batch).context("Failed to delete topic events")?;

        // Remove topic metadata
        let cf_topics = self.db.cf_handle("topics")
            .context("Topics CF not found")?;

        let metadata_key = self.make_topic_metadata_key(topic);
        self.db.delete_cf(cf_topics, &metadata_key)
            .context("Failed to delete topic metadata")?;

        // Update cache
        self.topic_metadata.write().unwrap().remove(topic);

        info!("Deleted topic: {}", topic);
        Ok(())
    }

    /// Get topic statistics
    #[instrument(skip(self))]
    pub async fn get_topic_stats(&self, topic: &str) -> Result<TopicStats> {
        let metadata = self.topic_metadata.read().unwrap()
            .get(topic)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Topic '{}' not found", topic))?;

        Ok(TopicStats {
            topic_name: metadata.name,
            event_count: metadata.event_count,
            first_offset: metadata.first_offset,
            last_offset: metadata.last_offset,
            created_at: metadata.created_at,
            size_bytes: metadata.size_bytes,
        })
    }

    /// List all topics
    #[instrument(skip(self))]
    pub async fn list_topics(&self) -> Result<Vec<String>> {
        Ok(self.topic_metadata.read().unwrap().keys().cloned().collect())
    }

    /// Get event at specific offset
    async fn get_event_at_offset(&self, topic: &str, offset: u64) -> Result<Option<EventEnvelope>> {
        let cf_events = self.db.cf_handle("events")
            .context("Events CF not found")?;

        let event_key = self.make_event_key(topic, offset);

        match self.db.get_cf(cf_events, &event_key)? {
            Some(event_data) => {
                let event: EventEnvelope = bincode::deserialize(&event_data)
                    .context("Failed to deserialize event")?;
                Ok(Some(event))
            }
            None => Ok(None),
        }
    }

    /// Ensure topic exists
    async fn ensure_topic_exists(&self, topic: &str) -> Result<()> {
        if !self.topic_exists(topic).await? {
            self.create_topic(topic).await?;
        }
        Ok(())
    }

    /// Check if topic exists
    async fn topic_exists(&self, topic: &str) -> Result<bool> {
        Ok(self.topic_metadata.read().unwrap().contains_key(topic))
    }

    /// Get next sequence number for a topic
    async fn get_next_sequence_number(&self, topic: &str) -> Result<u64> {
        let metadata = self.topic_metadata.read().unwrap();
        if let Some(meta) = metadata.get(topic) {
            Ok(meta.last_offset + 1)
        } else {
            Ok(1) // First event
        }
    }

    /// Update topic metadata
    async fn update_topic_metadata(&self, topic: &str, offset: u64, size_bytes: u64) -> Result<()> {
        let mut metadata = self.topic_metadata.write().unwrap();
        if let Some(meta) = metadata.get_mut(topic) {
            meta.event_count += 1;
            meta.last_offset = offset;
            meta.size_bytes += size_bytes;

            if meta.event_count == 1 {
                meta.first_offset = offset;
            }

            // Persist updated metadata
            let cf_topics = self.db.cf_handle("topics")
                .context("Topics CF not found")?;

            let metadata_key = self.make_topic_metadata_key(topic);
            let metadata_data = bincode::serialize(meta)
                .context("Failed to serialize topic metadata")?;

            self.db.put_cf(cf_topics, &metadata_key, metadata_data)
                .context("Failed to update topic metadata")?;
        }

        Ok(())
    }

    /// Load topic metadata from RocksDB
    async fn load_topic_metadata(&self) -> Result<()> {
        let cf_topics = self.db.cf_handle("topics")
            .context("Topics CF not found")?;

        let iter = self.db.iterator_cf(cf_topics, IteratorMode::Start);
        for item in iter {
            let (_, value) = item.context("Iterator error")?;
            let metadata: TopicMetadata = bincode::deserialize(&value)
                .context("Failed to deserialize topic metadata")?;

            self.topic_metadata.write().unwrap().insert(metadata.name.clone(), metadata);
        }

        info!("Loaded metadata for {} topics", self.topic_metadata.read().unwrap().len());
        Ok(())
    }

    /// Make event key: topic/offset
    fn make_event_key(&self, topic: &str, offset: u64) -> String {
        format!("{}/{}", topic, offset)
    }

    /// Make event ID key
    fn make_event_id_key(&self, event_id: &EventId) -> String {
        format!("id/{}", event_id.0)
    }

    /// Make aggregate key: aggregate_id/sequence
    fn make_aggregate_key(&self, aggregate_id: &AggregateId, sequence: u64) -> String {
        format!("aggregate/{}/{}", aggregate_id.0, sequence)
    }

    /// Make topic metadata key
    fn make_topic_metadata_key(&self, topic: &str) -> String {
        format!("topic/{}", topic)
    }

    /// Make event pointer: topic:offset
    fn make_event_pointer(&self, topic: &str, offset: u64) -> Vec<u8> {
        format!("{}:{}", topic, offset).into_bytes()
    }

    /// Parse event pointer
    fn parse_event_pointer(&self, data: &[u8]) -> Result<(String, u64)> {
        let pointer_str = String::from_utf8(data.to_vec())
            .context("Invalid event pointer format")?;

        let parts: Vec<&str> = pointer_str.split(':').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!("Invalid event pointer format"));
        }

        let topic = parts[0].to_string();
        let offset = parts[1].parse()
            .context("Invalid offset in event pointer")?;

        Ok((topic, offset))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_rocksdb_storage_creation() {
        let temp_dir = tempdir().unwrap();
        let storage = RocksDBStorage::new(temp_dir.path().to_str().unwrap(), 10).await;
        assert!(storage.is_ok(), "RocksDB storage should be created successfully");
    }

    #[tokio::test]
    async fn test_topic_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = RocksDBStorage::new(temp_dir.path().to_str().unwrap(), 10).await.unwrap();

        // Create topic
        let result = storage.create_topic("test_topic").await;
        assert!(result.is_ok(), "Topic should be created successfully");

        // Check topic exists
        let topics = storage.list_topics().await.unwrap();
        assert!(topics.contains(&"test_topic".to_string()));

        // Get topic stats
        let stats = storage.get_topic_stats("test_topic").await.unwrap();
        assert_eq!(stats.topic_name, "test_topic");
        assert_eq!(stats.event_count, 0);
    }

    #[tokio::test]
    async fn test_event_storage_and_retrieval() {
        let temp_dir = tempdir().unwrap();
        let storage = RocksDBStorage::new(temp_dir.path().to_str().unwrap(), 10).await.unwrap();

        // Create topic
        storage.create_topic("test_topic").await.unwrap();

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

        // Store event
        let result = storage.store_event("test_topic", &event).await;
        assert!(result.is_ok(), "Event should be stored successfully");

        // Retrieve event by ID
        let retrieved = storage.get_event(&event.id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved_event = retrieved.unwrap();
        assert_eq!(retrieved_event.id, event.id);
        assert_eq!(retrieved_event.aggregate_id, event.aggregate_id);

        // Retrieve events by aggregate
        let aggregate_events = storage.get_events_by_aggregate(&aggregate_id).await.unwrap();
        assert_eq!(aggregate_events.len(), 1);
        assert_eq!(aggregate_events[0].id, event.id);

        // Check topic stats updated
        let stats = storage.get_topic_stats("test_topic").await.unwrap();
        assert_eq!(stats.event_count, 1);
        assert_eq!(stats.first_offset, 1);
        assert_eq!(stats.last_offset, 1);
    }
}

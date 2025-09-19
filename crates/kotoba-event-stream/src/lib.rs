//! `kotoba-event-stream`
//!
//! Kafka-based event streaming component for KotobaDB.
//! Provides publish/subscribe functionality for event sourcing.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::message::Message;
use serde::{Deserialize, Serialize};
use anyhow::{Result, Context};
use tracing::{info, warn, error};

/// Core event types for the event sourcing system
pub mod event;
pub use event::*;

/// Event producer for publishing events to Kafka
pub mod producer;
pub use producer::*;

/// Event consumer for subscribing to events from Kafka
pub mod consumer;
pub use consumer::*;

/// Configuration for the event stream
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStreamConfig {
    pub bootstrap_servers: String,
    pub group_id: String,
    pub topic_prefix: String,
    pub producer_config: HashMap<String, String>,
    pub consumer_config: HashMap<String, String>,
}

impl Default for EventStreamConfig {
    fn default() -> Self {
        let mut producer_config = HashMap::new();
        producer_config.insert("compression.type".to_string(), "gzip".to_string());
        producer_config.insert("acks".to_string(), "all".to_string());

        let mut consumer_config = HashMap::new();
        consumer_config.insert("enable.auto.commit".to_string(), "false".to_string());
        consumer_config.insert("auto.offset.reset".to_string(), "earliest".to_string());

        Self {
            bootstrap_servers: "localhost:9092".to_string(),
            group_id: "kotoba-event-stream".to_string(),
            topic_prefix: "kotoba.events".to_string(),
            producer_config,
            consumer_config,
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
}

/// Event handler function type
pub type EventHandler = Box<dyn Fn(EventEnvelope) -> Result<()> + Send + Sync>;

/// Main event stream implementation using Kafka
pub struct KafkaEventStream {
    config: EventStreamConfig,
    producer: FutureProducer,
    consumers: Arc<Mutex<HashMap<String, StreamConsumer>>>,
}

impl KafkaEventStream {
    /// Create a new Kafka event stream
    pub fn new(config: EventStreamConfig) -> Result<Self> {
        let mut producer_config = ClientConfig::new();
        producer_config
            .set("bootstrap.servers", &config.bootstrap_servers)
            .set("message.timeout.ms", "5000");

        for (key, value) in &config.producer_config {
            producer_config.set(key, value);
        }

        let producer: FutureProducer = producer_config
            .create()
            .context("Failed to create Kafka producer")?;

        info!("Created Kafka event stream with brokers: {}", config.bootstrap_servers);

        Ok(Self {
            config,
            producer,
            consumers: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Create topic name with prefix
    fn topic_name(&self, topic: &str) -> String {
        format!("{}.{}", self.config.topic_prefix, topic)
    }
}

#[async_trait]
impl EventStreamPort for KafkaEventStream {
    async fn publish(&self, event: EventEnvelope) -> Result<EventId> {
        let topic = self.topic_name("all");
        let payload = serde_json::to_string(&event)
            .context("Failed to serialize event")?;

        let record = FutureRecord::to(&topic)
            .key(&event.aggregate_id.0)
            .payload(&payload);

        let delivery_status = self.producer.send(record, Duration::from_secs(5)).await;
        match delivery_status {
            Ok(_) => {
                info!("Published event: {} to topic: {}", event.id.0, topic);
                Ok(event.id)
            }
            Err((err, _)) => {
                error!("Failed to publish event: {}", err);
                Err(anyhow::anyhow!("Failed to publish event: {}", err))
            }
        }
    }

    async fn subscribe(&self, topic: &str, handler: EventHandler) -> Result<()> {
        let topic_name = self.topic_name(topic);

        let mut consumer_config = ClientConfig::new();
        consumer_config
            .set("bootstrap.servers", &self.config.bootstrap_servers)
            .set("group.id", &self.config.group_id)
            .set("enable.auto.commit", "false");

        for (key, value) in &self.config.consumer_config {
            consumer_config.set(key, value);
        }

        let consumer: StreamConsumer = consumer_config
            .create()
            .context("Failed to create Kafka consumer")?;

        consumer
            .subscribe(&[&topic_name])
            .context("Failed to subscribe to topic")?;

        let mut consumers = self.consumers.lock().await;
        consumers.insert(topic_name.clone(), consumer);

        info!("Subscribed to topic: {}", topic_name);

        // Start consumer loop
        tokio::spawn(async move {
            if let Some(consumer) = consumers.get(&topic_name) {
                loop {
                    match consumer.recv().await {
                        Ok(message) => {
                            match message.payload_view::<str>() {
                                Some(Ok(payload)) => {
                                    match serde_json::from_str::<EventEnvelope>(payload) {
                                        Ok(event) => {
                                            if let Err(e) = handler(event) {
                                                error!("Event handler error: {}", e);
                                            }
                                        }
                                        Err(e) => {
                                            error!("Failed to deserialize event: {}", e);
                                        }
                                    }
                                }
                                _ => {
                                    warn!("Received message without payload");
                                }
                            }

                            // Manual commit
                            if let Err(e) = consumer.commit_message(&message, rdkafka::consumer::CommitMode::Async) {
                                error!("Failed to commit message: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("Consumer error: {}", e);
                            break;
                        }
                    }
                }
            }
        });

        Ok(())
    }

    async fn get_event(&self, event_id: &EventId) -> Result<Option<EventEnvelope>> {
        // In a real implementation, this would query from event store
        // For now, return None
        warn!("get_event not implemented yet");
        Ok(None)
    }

    async fn get_events_by_aggregate(&self, aggregate_id: &AggregateId) -> Result<Vec<EventEnvelope>> {
        // In a real implementation, this would query from event store
        // For now, return empty vector
        warn!("get_events_by_aggregate not implemented yet");
        Ok(Vec::new())
    }
}

use std::time::Duration;

//! Event Processor
//!
//! Processes OCEL v2 events from the event stream and coordinates projection updates.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use dashmap::DashMap;
use anyhow::{Result, Context};
use tracing::{info, warn, error, instrument};
use futures::stream::{self, StreamExt};
use metrics::{counter, histogram};

use kotoba_ocel::OcelEvent;
use crate::materializer::Materializer;

/// Event processor for handling incoming events
pub struct EventProcessor {
    /// Materializer for processing events
    materializer: Arc<Materializer>,
    /// Registered projections
    projections: Arc<DashMap<String, ProjectionHandler>>,
    /// Event processing queue
    event_queue: mpsc::Sender<Vec<OcelEvent>>,
    /// Batch size for processing
    batch_size: usize,
    /// Processing statistics
    stats: Arc<RwLock<ProcessorStats>>,
}

/// Projection handler
struct ProjectionHandler {
    /// Projection name
    name: String,
    /// Event types this projection handles
    event_types: Vec<String>,
    /// Processing function
    processor: Box<dyn Fn(EventEnvelope) -> Result<()> + Send + Sync>,
}

/// Processing statistics
#[derive(Debug, Clone)]
pub struct ProcessorStats {
    /// Total events received
    pub events_received: u64,
    /// Total events processed
    pub events_processed: u64,
    /// Total processing errors
    pub processing_errors: u64,
    /// Average processing time
    pub avg_processing_time_ms: f64,
    /// Current queue size
    pub queue_size: usize,
}

impl EventProcessor {
    /// Create a new event processor
    pub fn new(materializer: Arc<Materializer>, batch_size: usize) -> Self {
        let (tx, rx) = mpsc::channel(1000); // Event queue

        // Clone materializer for the async task
        let materializer_clone = materializer.clone();

        let processor = Self {
            materializer,
            projections: Arc::new(DashMap::new()),
            event_queue: tx,
            batch_size,
            stats: Arc::new(RwLock::new(ProcessorStats::default())),
        };

        // Start event processing task
        tokio::spawn(Self::process_events_task(processor.stats.clone(), rx, materializer_clone));

        processor
    }

    /// Start the event processor
    pub async fn start(&self) -> Result<()> {
        info!("Starting Event Processor");
        Ok(())
    }

    /// Stop the event processor
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping Event Processor");
        // Close the event queue
        drop(self.event_queue.clone());
        Ok(())
    }

    /// Register a projection
    #[instrument(skip(self))]
    pub async fn register_projection(&self, name: &str) -> Result<()> {
        info!("Registering projection: {}", name);

        // Create projection handler
        let handler = ProjectionHandler {
            name: name.to_string(),
            event_types: vec![
                "node.created".to_string(),
                "node.updated".to_string(),
                "node.deleted".to_string(),
                "edge.created".to_string(),
                "edge.updated".to_string(),
                "edge.deleted".to_string(),
            ],
            processor: Box::new(move |event: EventEnvelope| {
                // Process event for this projection
                // This will be called by the materializer
                Ok(())
            }),
        };

        self.projections.insert(name.to_string(), handler);
        info!("Projection registered: {}", name);
        Ok(())
    }

    /// Unregister a projection
    #[instrument(skip(self))]
    pub async fn unregister_projection(&self, name: &str) -> Result<()> {
        info!("Unregistering projection: {}", name);
        self.projections.remove(name);
        info!("Projection unregistered: {}", name);
        Ok(())
    }

    /// Process a batch of OCEL events
    #[instrument(skip(self, events))]
    pub async fn process_batch(&self, events: Vec<OcelEvent>) -> Result<()> {
        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.events_received += events.len() as u64;
            stats.queue_size += events.len();
        }

        // Send events to processing queue
        if let Err(e) = self.event_queue.send(events).await {
            error!("Failed to send events to processing queue: {}", e);
            return Err(anyhow::anyhow!("Failed to queue events: {}", e));
        }

        Ok(())
    }

    /// Get processing statistics
    pub async fn get_statistics(&self) -> ProcessorStats {
        self.stats.read().await.clone()
    }

    /// Background task for processing OCEL events
    async fn process_events_task(
        stats: Arc<RwLock<ProcessorStats>>,
        mut rx: mpsc::Receiver<Vec<OcelEvent>>,
        materializer: Arc<Materializer>,
    ) {
        info!("Starting OCEL event processing task");

        while let Some(events) = rx.recv().await {
            let start_time = std::time::Instant::now();

            // Process events in batches
            let batch_size = events.len();
            let result = Self::process_ocel_event_batch(&materializer, events).await;

            let processing_time = start_time.elapsed();

            // Update statistics
            let mut stats = stats.write().await;
            stats.events_processed += batch_size as u64;
            stats.queue_size = stats.queue_size.saturating_sub(batch_size);

            if let Err(e) = result {
                stats.processing_errors += 1;
                error!("Error processing OCEL event batch: {}", e);
            } else {
                // Update average processing time
                let total_time_ms = processing_time.as_millis() as f64;
                let avg_time = (stats.avg_processing_time_ms + total_time_ms / batch_size as f64) / 2.0;
                stats.avg_processing_time_ms = avg_time;

                if batch_size > 0 {
                    // histogram!("projection_engine.event_processing_time", total_time_ms / batch_size as f64);
                }
            }
        }

        info!("OCEL event processing task stopped");
    }

    /// Process a batch of OCEL events
    async fn process_ocel_event_batch(
        materializer: &Arc<Materializer>,
        events: Vec<OcelEvent>,
    ) -> Result<()> {
        // Process events in parallel
        let results = stream::iter(events)
            .map(|event| async {
                // Route OCEL event to materializer
                Self::route_ocel_event(materializer, event).await
            })
            .buffer_unordered(10) // Process up to 10 events concurrently
            .collect::<Vec<Result<()>>>()
            .await;

        // Check for errors
        for result in results {
            result?;
        }

        Ok(())
    }

    /// Route an OCEL event to the materializer
    async fn route_ocel_event(materializer: &Arc<Materializer>, event: OcelEvent) -> Result<()> {
        // Send OCEL event to materializer for direct GraphDB materialization
        materializer.process_ocel_event(event).await?;
        Ok(())
    }
}

impl Default for ProcessorStats {
    fn default() -> Self {
        Self {
            events_received: 0,
            events_processed: 0,
            processing_errors: 0,
            avg_processing_time_ms: 0.0,
            queue_size: 0,
        }
    }
}

// Placeholder type
pub type EventEnvelope = serde_json::Value;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materializer::Materializer;

    #[tokio::test]
    async fn test_event_processor_creation() {
        // Create a mock materializer
        let materializer = Arc::new(Materializer::default());
        let processor = EventProcessor::new(materializer, 10);

        let stats = processor.get_statistics().await;
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.events_processed, 0);
    }

    #[tokio::test]
    async fn test_projection_registration() {
        let materializer = Arc::new(Materializer::default());
        let processor = EventProcessor::new(materializer, 10);

        let result = processor.register_projection("test_projection").await;
        assert!(result.is_ok(), "Projection should be registered successfully");

        let result = processor.unregister_projection("test_projection").await;
        assert!(result.is_ok(), "Projection should be unregistered successfully");
    }

    #[tokio::test]
    async fn test_event_processing() {
        let materializer = Arc::new(Materializer::default());
        let processor = EventProcessor::new(materializer, 10);

        // Create a test event
        let event = serde_json::json!({
            "event_type": "node.created",
            "aggregate_id": "test-123",
            "data": {
                "name": "Test Node"
            }
        });

        let result = processor.process_batch(vec![event]).await;
        assert!(result.is_ok(), "Event batch should be processed successfully");

        // Check statistics
        let stats = processor.get_statistics().await;
        assert_eq!(stats.events_received, 1);
    }
}

//! Event types and structures for the event sourcing system

use serde::{Deserialize, Serialize};
use std::fmt;

/// Unique identifier for events
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventId(pub String);

impl fmt::Display for EventId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for aggregates
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateId(pub String);

impl fmt::Display for AggregateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Event type identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventType(pub String);

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Core event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    pub event_type: EventType,
    pub payload: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Event envelope containing all event information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub id: EventId,
    pub aggregate_id: AggregateId,
    pub aggregate_type: String,
    pub event_type: EventType,
    pub sequence_number: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: EventData,
    pub metadata: HashMap<String, serde_json::Value>,
}

use std::collections::HashMap;
use chrono;

/// Common event types for KotobaDB operations
pub mod event_types {
    use super::EventType;

    pub const NODE_CREATED: &str = "node.created";
    pub const NODE_UPDATED: &str = "node.updated";
    pub const NODE_DELETED: &str = "node.deleted";

    pub const EDGE_CREATED: &str = "edge.created";
    pub const EDGE_UPDATED: &str = "edge.updated";
    pub const EDGE_DELETED: &str = "edge.deleted";

    pub const PROPERTY_SET: &str = "property.set";
    pub const PROPERTY_REMOVED: &str = "property.removed";

    pub const TRANSACTION_STARTED: &str = "transaction.started";
    pub const TRANSACTION_COMMITTED: &str = "transaction.committed";
    pub const TRANSACTION_ROLLED_BACK: &str = "transaction.rolled_back";

    pub fn node_created() -> EventType {
        EventType(NODE_CREATED.to_string())
    }

    pub fn node_updated() -> EventType {
        EventType(NODE_UPDATED.to_string())
    }

    pub fn node_deleted() -> EventType {
        EventType(NODE_DELETED.to_string())
    }

    pub fn edge_created() -> EventType {
        EventType(EDGE_CREATED.to_string())
    }

    pub fn edge_updated() -> EventType {
        EventType(EDGE_UPDATED.to_string())
    }

    pub fn edge_deleted() -> EventType {
        EventType(EDGE_DELETED.to_string())
    }

    pub fn property_set() -> EventType {
        EventType(PROPERTY_SET.to_string())
    }

    pub fn property_removed() -> EventType {
        EventType(PROPERTY_REMOVED.to_string())
    }

    pub fn transaction_started() -> EventType {
        EventType(TRANSACTION_STARTED.to_string())
    }

    pub fn transaction_committed() -> EventType {
        EventType(TRANSACTION_COMMITTED.to_string())
    }

    pub fn transaction_rolled_back() -> EventType {
        EventType(TRANSACTION_ROLLED_BACK.to_string())
    }
}

/// Event builder for creating events
pub struct EventBuilder {
    aggregate_id: AggregateId,
    aggregate_type: String,
    event_type: EventType,
    sequence_number: u64,
    data: EventData,
    metadata: HashMap<String, serde_json::Value>,
}

impl EventBuilder {
    pub fn new(aggregate_id: AggregateId, aggregate_type: String, event_type: EventType) -> Self {
        Self {
            aggregate_id,
            aggregate_type,
            event_type: event_type.clone(),
            sequence_number: 0,
            data: EventData {
                event_type,
                payload: serde_json::Value::Null,
                metadata: HashMap::new(),
            },
            metadata: HashMap::new(),
        }
    }

    pub fn sequence_number(mut self, seq: u64) -> Self {
        self.sequence_number = seq;
        self
    }

    pub fn payload(mut self, payload: serde_json::Value) -> Self {
        self.data.payload = payload;
        self
    }

    pub fn data_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.data.metadata.insert(key.to_string(), value);
        self
    }

    pub fn event_metadata(mut self, key: &str, value: serde_json::Value) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }

    pub fn build(self) -> EventEnvelope {
        let id = EventId(format!("{}-{}-{}", self.aggregate_id.0, self.event_type.0, self.sequence_number));

        EventEnvelope {
            id,
            aggregate_id: self.aggregate_id,
            aggregate_type: self.aggregate_type,
            event_type: self.data.event_type.clone(),
            sequence_number: self.sequence_number,
            timestamp: chrono::Utc::now(),
            data: self.data,
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_builder() {
        let aggregate_id = AggregateId("test-aggregate".to_string());
        let event = EventBuilder::new(
            aggregate_id.clone(),
            "TestAggregate".to_string(),
            event_types::node_created(),
        )
        .sequence_number(1)
        .payload(serde_json::json!({"name": "test"}))
        .data_metadata("version", serde_json::json!("1.0"))
        .build();

        assert_eq!(event.aggregate_id, aggregate_id);
        assert_eq!(event.sequence_number, 1);
        assert_eq!(event.event_type.0, "node.created");
        assert_eq!(event.data.payload["name"], "test");
        assert_eq!(event.data.metadata["version"], "1.0");
    }

    #[test]
    fn test_event_serialization() {
        let aggregate_id = AggregateId("test-aggregate".to_string());
        let event = EventBuilder::new(
            aggregate_id,
            "TestAggregate".to_string(),
            event_types::node_created(),
        )
        .sequence_number(1)
        .payload(serde_json::json!({"name": "test"}))
        .build();

        let serialized = serde_json::to_string(&event).unwrap();
        let deserialized: EventEnvelope = serde_json::from_str(&serialized).unwrap();

        assert_eq!(event.id, deserialized.id);
        assert_eq!(event.aggregate_id, deserialized.aggregate_id);
        assert_eq!(event.event_type, deserialized.event_type);
    }
}

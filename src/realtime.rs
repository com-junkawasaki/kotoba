//! Real-time functionality for Kotoba
//!
//! WebSocket and Server-Sent Events support for real-time updates
//! between server and connected clients.

use crate::{engidb::EngiDB, Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

/// Real-time event types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RealtimeEvent {
    TodoAdded { id: u64, title: String },
    TodoCompleted { id: u64 },
    TodoDeleted { id: u64 },
    TodoUpdated { id: u64, changes: HashMap<String, serde_json::Value> },
}

/// WebSocket message from client
#[derive(Debug, Deserialize)]
pub struct ClientMessage {
    pub action: String,
    pub data: serde_json::Value,
}

/// WebSocket message to client
#[derive(Debug, Serialize)]
pub struct ServerMessage {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

/// Global event broadcaster
pub type EventBroadcaster = broadcast::Sender<RealtimeEvent>;

/// Initialize event broadcaster
pub fn create_event_broadcaster() -> EventBroadcaster {
    let (tx, _) = broadcast::channel(100);
    tx
}

/// Broadcast an event to all connected clients
pub fn broadcast_event(broadcaster: &EventBroadcaster, event: RealtimeEvent) -> Result<()> {
    let _ = broadcaster.send(event);
    Ok(())
}

/// HTMX integration helpers
pub mod htmx_integration {
    use super::*;

    /// Generate HTMX attributes for real-time updates
    pub fn realtime_htmx_attrs(endpoint: &str, events: &[&str]) -> String {
        let events_str = events.join(", ");
        format!("hx-sse=\"connect:{}/events\" sse-swap=\"{}\"", endpoint, events_str)
    }

    /// Generate HTMX trigger for custom events
    pub fn trigger_event(event_name: &str, data: &serde_json::Value) -> String {
        format!("hx-trigger=\"{} from:body\" hx-vals='{}'",
               event_name,
               serde_json::to_string(data).unwrap_or_default())
    }
}



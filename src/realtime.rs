//! Real-time functionality for Kotoba
//!
//! WebSocket and Server-Sent Events support for real-time updates
//! between server and connected clients.

use crate::{engidb::EngiDB, Error, Result};
use actix::{Actor, StreamHandler};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
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
pub type EventBroadcaster = Arc<Mutex<broadcast::Sender<RealtimeEvent>>>;

/// Initialize event broadcaster
pub fn create_event_broadcaster() -> EventBroadcaster {
    let (tx, _) = broadcast::channel(100);
    Arc::new(Mutex::new(tx))
}

/// Broadcast an event to all connected clients
pub fn broadcast_event(broadcaster: &EventBroadcaster, event: RealtimeEvent) -> Result<()> {
    if let Ok(tx) = broadcaster.lock() {
        let _ = tx.send(event);
    }
    Ok(())
}

/// WebSocket actor for handling client connections
pub struct RealtimeWebSocket {
    broadcaster: EventBroadcaster,
    engidb: EngiDB,
}

impl RealtimeWebSocket {
    pub fn new(broadcaster: EventBroadcaster, engidb: EngiDB) -> Self {
        RealtimeWebSocket {
            broadcaster,
            engidb,
        }
    }

    /// Handle client messages
    fn handle_client_message(&self, msg: ClientMessage) -> Result<Option<ServerMessage>> {
        match msg.action.as_str() {
            "subscribe" => {
                // Client subscribed to events
                Ok(Some(ServerMessage {
                    event_type: "subscribed".to_string(),
                    data: serde_json::json!({"message": "Successfully subscribed to real-time events"}),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
            "ping" => {
                // Heartbeat
                Ok(Some(ServerMessage {
                    event_type: "pong".to_string(),
                    data: serde_json::json!({"timestamp": chrono::Utc::now().timestamp()}),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                }))
            }
            _ => {
                println!("⚠️ Unknown client action: {}", msg.action);
                Ok(None)
            }
        }
    }
}

impl Actor for RealtimeWebSocket {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        println!("🌐 New WebSocket client connected");

        // Subscribe to broadcast events
        let mut rx = {
            if let Ok(tx) = self.broadcaster.lock() {
                tx.subscribe()
            } else {
                return;
            }
        };

        // Spawn task to listen for broadcast events
        let addr = ctx.address();
        actix::spawn(async move {
            while let Ok(event) = rx.recv().await {
                // Send event to client
                let message = ServerMessage {
                    event_type: match &event {
                        RealtimeEvent::TodoAdded { .. } => "todo_added".to_string(),
                        RealtimeEvent::TodoCompleted { .. } => "todo_completed".to_string(),
                        RealtimeEvent::TodoDeleted { .. } => "todo_deleted".to_string(),
                        RealtimeEvent::TodoUpdated { .. } => "todo_updated".to_string(),
                    },
                    data: match event {
                        RealtimeEvent::TodoAdded { id, title } => {
                            serde_json::json!({"id": id, "title": title})
                        }
                        RealtimeEvent::TodoCompleted { id } => {
                            serde_json::json!({"id": id})
                        }
                        RealtimeEvent::TodoDeleted { id } => {
                            serde_json::json!({"id": id})
                        }
                        RealtimeEvent::TodoUpdated { id, changes } => {
                            serde_json::json!({"id": id, "changes": changes})
                        }
                    },
                    timestamp: chrono::Utc::now().to_rfc3339(),
                };

                if let Ok(json) = serde_json::to_string(&message) {
                    let _ = addr.send(ws::Message::Text(json.into())).await;
                }
            }
        });

        // Send welcome message
        let welcome = ServerMessage {
            event_type: "connected".to_string(),
            data: serde_json::json!({"message": "Connected to Kotoba real-time server"}),
            timestamp: chrono::Utc::now().to_rfc3339(),
        };

        if let Ok(json) = serde_json::to_string(&welcome) {
            ctx.text(json);
        }
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        println!("🌐 WebSocket client disconnected");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for RealtimeWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Parse client message
                if let Ok(client_msg) = serde_json::from_str::<ClientMessage>(&text) {
                    match self.handle_client_message(client_msg) {
                        Ok(Some(response)) => {
                            if let Ok(json) = serde_json::to_string(&response) {
                                ctx.text(json);
                            }
                        }
                        Ok(None) => {} // No response needed
                        Err(e) => {
                            eprintln!("❌ Error handling client message: {}", e);
                            let error_msg = ServerMessage {
                                event_type: "error".to_string(),
                                data: serde_json::json!({"message": format!("Error: {}", e)}),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                ctx.text(json);
                            }
                        }
                    }
                } else {
                    // Invalid JSON
                    let error_msg = ServerMessage {
                        event_type: "error".to_string(),
                        data: serde_json::json!({"message": "Invalid JSON message"}),
                        timestamp: chrono::Utc::now().to_rfc3339(),
                    };
                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        ctx.text(json);
                    }
                }
            }
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

/// WebSocket endpoint handler
pub async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    broadcaster: web::Data<EventBroadcaster>,
    engidb: web::Data<EngiDB>,
) -> Result<HttpResponse, actix_web::Error> {
    let broadcaster = broadcaster.get_ref().clone();
    let engidb = engidb.get_ref().clone();

    ws::start(RealtimeWebSocket::new(broadcaster, engidb), &req, stream)
}

/// Server-Sent Events endpoint for older browsers
pub async fn sse_handler(
    broadcaster: web::Data<EventBroadcaster>,
) -> HttpResponse {
    let (tx, rx) = tokio::sync::mpsc::channel(100);

    // Subscribe to broadcast events
    let mut broadcast_rx = {
        if let Ok(broadcaster) = broadcaster.lock() {
            broadcaster.subscribe()
        } else {
            return HttpResponse::InternalServerError().finish();
        }
    };

    // Spawn task to listen for events
    actix::spawn(async move {
        while let Ok(event) = broadcast_rx.recv().await {
            let event_data = match event {
                RealtimeEvent::TodoAdded { id, title } => {
                    format!("event: todo_added\ndata: {{\"id\": {}, \"title\": \"{}\", \"timestamp\": \"{}\"}}}\n\n",
                           id, title, chrono::Utc::now().to_rfc3339())
                }
                RealtimeEvent::TodoCompleted { id } => {
                    format!("event: todo_completed\ndata: {{\"id\": {}, \"timestamp\": \"{}\"}}}\n\n",
                           id, chrono::Utc::now().to_rfc3339())
                }
                RealtimeEvent::TodoDeleted { id } => {
                    format!("event: todo_deleted\ndata: {{\"id\": {}, \"timestamp\": \"{}\"}}}\n\n",
                           id, chrono::Utc::now().to_rfc3339())
                }
                RealtimeEvent::TodoUpdated { id, changes } => {
                    format!("event: todo_updated\ndata: {{\"id\": {}, \"changes\": {}, \"timestamp\": \"{}\"}}}\n\n",
                           id, serde_json::to_string(&changes).unwrap_or_default(), chrono::Utc::now().to_rfc3339())
                }
            };

            let _ = tx.send(event_data).await;
        }
    });

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .streaming(rx)
}

/// Add real-time routes to Actix Web app
pub fn configure_realtime_routes(
    cfg: &mut web::ServiceConfig,
    broadcaster: EventBroadcaster,
) {
    cfg
        .app_data(web::Data::new(broadcaster))
        .route("/ws", web::get().to(websocket_handler))
        .route("/events", web::get().to(sse_handler));
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

// Re-export actix for WebSocket support
extern crate actix;
extern crate actix_web_actors;

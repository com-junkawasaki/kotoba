//! HTTP API Server for Kotoba
//!
//! Provides REST API endpoints for the Todo application,
//! connecting HTMX frontend with EngiDB backend.
//!
//! Pure Rust implementation using Axum/Hyper.

use crate::{engidb::EngiDB, Error, Result, realtime::{create_event_broadcaster, broadcast_event, RealtimeEvent}};
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{delete, get, post},
    Router,
};
use axum::response::Sse;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tokio::net::TcpListener;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::broadcast;
use futures::stream::{self, Stream};
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub engidb: Arc<EngiDB>,
    pub event_broadcaster: crate::realtime::EventBroadcaster,
}

/// Todo item representation for API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: u64,
    pub title: String,
    pub description: String,
    pub completed: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// Todo creation request
#[derive(Debug, Deserialize)]
pub struct CreateTodoRequest {
    pub title: String,
    pub description: Option<String>,
}

/// Start the HTTP server
pub async fn start_server(db_path: PathBuf, port: u16) -> Result<()> {
    let engidb = EngiDB::open(&db_path)?;
    let event_broadcaster = create_event_broadcaster();

    let app_state = AppState {
        engidb: Arc::new(engidb),
        event_broadcaster: event_broadcaster.clone(),
    };

    let app = build_router(app_state);

    println!("🚀 Starting Kotoba API Server on port {}", port);
    println!("📊 Database: {}", db_path.display());
    println!("🌐 API endpoints:");
    println!("  POST /api/todo/add     - Add new todo");
    println!("  GET  /api/todo/list    - List all todos");
    println!("  POST /api/todo/{{id}}/complete - Mark todo as completed");
    println!("  DELETE /api/todo/{{id}}  - Delete todo");
    println!("  GET  /ws               - WebSocket real-time");
    println!("  GET  /events           - Server-Sent Events");

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await
        .map_err(|e| Error::Storage(format!("Failed to bind to port {}: {}", port, e)))?;

    println!("✅ Server listening on http://127.0.0.1:{}", port);

    axum::serve(listener, app)
        .await
        .map_err(|e| Error::Storage(format!("HTTP server error: {}", e)))
}

/// Build the application router
fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/app", get(todo_app))
        .route("/api/todo/add", post(add_todo))
        .route("/api/todo/list", get(list_todos))
        .route("/api/todo/:id/complete", post(complete_todo))
        .route("/api/todo/:id", delete(delete_todo))
        .route("/ws", get(ws_handler))
        .route("/events", get(sse_handler))
        .nest_service("/static", tower_http::services::ServeDir::new("examples"))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

/// Root endpoint - serve the Todo UI
async fn index() -> Html<&'static str> {
    Html(r#"<!DOCTYPE html>
<html>
<head>
    <title>Kotoba Todo API</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 40px; }
        .endpoint { margin: 10px 0; padding: 10px; border: 1px solid #ccc; }
        .method { font-weight: bold; color: #007acc; }
    </style>
</head>
<body>
    <h1>🚀 Kotoba Todo API Server</h1>
    <p>Server is running! Use the HTMX frontend for full functionality.</p>

    <h2>📋 API Endpoints</h2>
    <div class="endpoint">
        <span class="method">POST</span> /api/todo/add - Add new todo
    </div>
    <div class="endpoint">
        <span class="method">GET</span> /api/todo/list - List all todos
    </div>
    <div class="endpoint">
        <span class="method">POST</span> /api/todo/{id}/complete - Mark todo as completed
    </div>
    <div class="endpoint">
        <span class="method">DELETE</span> /api/todo/{id} - Delete todo
    </div>

    <p><a href="/app">Open Full Todo App</a> | <a href="/static/todo_app_full.html">Static Version</a></p>
</body>
</html>"#)
}

/// Serve the full Todo app
async fn todo_app() -> Html<&'static str> {
    Html(include_str!("../examples/todo_app_full.html"))
}

/// Add a new todo item
async fn add_todo(
    State(state): State<AppState>,
    Form(req): Form<CreateTodoRequest>,
) -> impl IntoResponse {
    println!("📝 Adding todo: {}", req.title);

    // Generate ID and timestamp
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as u64;

    let now = chrono::Utc::now().to_rfc3339();

    // Create EAF-IPG node for the todo item
    use kotoba_types::{Node, Layer};
    use indexmap::IndexMap;

    let todo_node = Node {
        id: format!("todo_{}", id),
        kind: "TodoItem".to_string(),
        properties: {
            let mut props = IndexMap::new();
            props.insert("id".to_string(), serde_json::json!(id));
            props.insert("title".to_string(), serde_json::json!(req.title));
            props.insert("description".to_string(), serde_json::json!(req.description.as_deref().unwrap_or("")));
            props.insert("completed".to_string(), serde_json::json!(false));
            props.insert("created_at".to_string(), serde_json::json!(now.clone()));
            props.insert("updated_at".to_string(), serde_json::json!(now));
            props
        },
    };

    // Store in EngiDB
    match state.engidb.store_todo_item(&todo_node) {
        Ok(_) => {
            println!("✅ Todo stored in EngiDB: {} (ID: {})", req.title, id);

            // Commit the change
            let _ = state.engidb.commit("main", "api-server".to_string(), format!("Add todo: {}", req.title));

            // Broadcast real-time event
            let _ = broadcast_event(&state.event_broadcaster, RealtimeEvent::TodoAdded {
                id,
                title: req.title.clone(),
            });

            // Return HTMX-compatible response
            (
                StatusCode::CREATED,
                [("HX-Trigger", "todoAdded"), ("Content-Type", "application/json")],
                serde_json::json!({
                    "success": true,
                    "id": id,
                    "message": "Todo added successfully"
                }).to_string()
            )
        }
        Err(e) => {
            eprintln!("❌ Failed to store todo: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                [("Content-Type", "application/json"), ("", "")],
                serde_json::json!({
                    "error": "Failed to store todo",
                    "details": e.to_string()
                }).to_string()
            )
        }
    }
}

/// List all todo items (HTMX HTML response)
async fn list_todos(State(state): State<AppState>) -> impl IntoResponse {
    println!("📋 Listing todos for HTMX");

    // Query todos from EngiDB
    match state.engidb.scan_todo_items() {
        Ok(nodes) => {
            println!("✅ Found {} todo nodes", nodes.len());

            // Convert nodes to TodoItems
            let mut todos = Vec::new();
            for node in nodes {
                if let (Some(id), Some(title), Some(completed)) = (
                    node.properties.get("id").and_then(|v| v.as_u64()),
                    node.properties.get("title").and_then(|v| v.as_str()),
                    node.properties.get("completed").and_then(|v| v.as_bool()),
                ) {
                    todos.push(TodoItem {
                        id,
                        title: title.to_string(),
                        description: node.properties.get("description")
                            .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        completed,
                        created_at: node.properties.get("created_at")
                            .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                        updated_at: node.properties.get("updated_at")
                            .and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    });
                }
            }

            // Sort by creation time (newest first)
            todos.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            // Generate HTML for HTMX
            let html = generate_todo_list_html(&todos);
            Html(html)
        }
        Err(e) => {
            eprintln!("❌ Failed to scan todos: {}", e);
            Html("<div class=\"text-red-500\">Failed to load todos</div>".to_string())
        }
    }
}

/// Generate HTML for todo list (HTMX response)
fn generate_todo_list_html(todos: &[TodoItem]) -> String {
    if todos.is_empty() {
        return r#"<div class="text-center text-gray-500 py-8">
            <p class="text-lg mb-2">📝 No todos yet</p>
            <p class="text-sm">Add your first todo above!</p>
        </div>"#.to_string();
    }

    let mut html = String::new();

    for todo in todos {
        let completed_class = if todo.completed { "completed line-through text-gray-500" } else { "" };
        let checkbox_checked = if todo.completed { "checked" } else { "" };

        html.push_str(&format!(r#"<div class="flex items-center justify-between p-4 border border-gray-200 rounded-lg mb-3 bg-white shadow-sm hover:shadow-md transition-shadow">
            <div class="flex items-center space-x-3">
                <input type="checkbox" {checked} class="w-5 h-5 text-blue-600 bg-gray-100 border-gray-300 rounded focus:ring-blue-500 focus:ring-2"
                       hx-post="/api/todo/{id}/complete" hx-swap="none">
                <span class="text-lg {completed}">{title}</span>
            </div>
            <div class="flex items-center space-x-2">
                <span class="text-xs text-gray-400">{created}</span>
                <button class="text-red-500 hover:text-red-700 p-1"
                        hx-delete="/api/todo/{id}" hx-confirm="Delete this todo?"
                        hx-target="closest div" hx-swap="outerHTML">
                    🗑️
                </button>
            </div>
        </div>"#,
            checked = checkbox_checked,
            id = todo.id,
            completed = completed_class,
            title = html_escape(&todo.title),
            created = &todo.created_at[..10] // Just the date part
        ));
    }

    html
}

/// Simple HTML escaping
fn html_escape(s: &str) -> String {
    s.replace("&", "&amp;")
     .replace("<", "&lt;")
     .replace(">", "&gt;")
     .replace("\"", "&quot;")
     .replace("'", "&#x27;")
}

/// Mark a todo as completed
async fn complete_todo(
    Path(id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    println!("✅ Completing todo #{}", id);

    // For HTMX, we just return success without content
    // The checkbox state change is handled client-side
    (StatusCode::OK, "")
}

/// Delete a todo item
async fn delete_todo(
    Path(id): Path<u64>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    println!("🗑️ Deleting todo #{}", id);

    // For HTMX, we return empty content to remove the element
    // The hx-target="closest div" and hx-swap="outerHTML" will remove the todo item
    (StatusCode::OK, "")
}

/// WebSocket handler for real-time updates
async fn ws_handler(
    ws: axum::extract::ws::WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state.event_broadcaster))
}

/// Handle WebSocket connection
async fn handle_socket(mut socket: axum::extract::ws::WebSocket, broadcaster: crate::realtime::EventBroadcaster) {
    println!("🌐 New WebSocket client connected");

    // Send welcome message
    let welcome = serde_json::json!({
        "event_type": "connected",
        "data": {"message": "Connected to Kotoba real-time server"},
        "timestamp": chrono::Utc::now().to_rfc3339()
    });

    if let Ok(json) = serde_json::to_string(&welcome) {
        let _ = socket.send(axum::extract::ws::Message::Text(json.into())).await;
    }

    // Subscribe to broadcast events
    let mut rx = broadcaster.subscribe();

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = socket.recv() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Text(text))) => {
                        // Handle client messages if needed
                        println!("📨 WebSocket message: {}", text);
                    }
                    Some(Ok(axum::extract::ws::Message::Close(_))) => {
                        println!("🌐 WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        eprintln!("❌ WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
            // Handle broadcast events
            event = rx.recv() => {
                match event {
                    Ok(realtime_event) => {
                        let message = crate::realtime::ServerMessage {
                            event_type: match &realtime_event {
                                RealtimeEvent::TodoAdded { .. } => "todo_added".to_string(),
                                RealtimeEvent::TodoCompleted { .. } => "todo_completed".to_string(),
                                RealtimeEvent::TodoDeleted { .. } => "todo_deleted".to_string(),
                                RealtimeEvent::TodoUpdated { .. } => "todo_updated".to_string(),
                            },
                            data: match realtime_event {
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
                            if socket.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
}

/// Server-Sent Events handler for older browsers
async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<axum::response::sse::Event>>> {
    let mut rx = state.event_broadcaster.subscribe();

    let stream = stream::unfold(rx, |mut rx| async move {
        match rx.recv().await {
            Ok(event) => {
                let event_data = match event {
                    RealtimeEvent::TodoAdded { id, title } => {
                        format!("event: todo_added\\ndata: {{\"id\": {}, \"title\": \"{}\", \"timestamp\": \"{}\"}}}}}}\\n\\n",
                               id, title, chrono::Utc::now().to_rfc3339())
                    }
                    RealtimeEvent::TodoCompleted { id } => {
                        format!("event: todo_completed\\ndata: {{\"id\": {}, \"timestamp\": \"{}\"}}}}}}\\n\\n",
                               id, chrono::Utc::now().to_rfc3339())
                    }
                    RealtimeEvent::TodoDeleted { id } => {
                        format!("event: todo_deleted\\ndata: {{\"id\": {}, \"timestamp\": \"{}\"}}}}}}\\n\\n",
                               id, chrono::Utc::now().to_rfc3339())
                    }
                    RealtimeEvent::TodoUpdated { id, changes } => {
                        format!("event: todo_updated\\ndata: {{\"id\": {}, \"changes\": {}, \"timestamp\": \"{}\"}}}}}}\\n\\n",
                               id, serde_json::to_string(&changes).unwrap_or_default(), chrono::Utc::now().to_rfc3339())
                    }
                };

                Some((Ok(axum::response::sse::Event::default().data(event_data)), rx))
            }
            Err(_) => None,
        }
    });

    Sse::new(stream)
}

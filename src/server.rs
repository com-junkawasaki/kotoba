//! HTTP API Server for Kotoba
//!
//! Provides REST API endpoints for the Todo application,
//! connecting HTMX frontend with EngiDB backend.

use crate::{engidb::EngiDB, Error, Result, realtime::{create_event_broadcaster, broadcast_event, RealtimeEvent, configure_realtime_routes}};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use actix_files as fs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Shared application state
pub struct AppState {
    pub engidb: Mutex<EngiDB>,
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

    let app_state = web::Data::new(AppState {
        engidb: Mutex::new(engidb.clone()),
        event_broadcaster: event_broadcaster.clone(),
    });

    println!("🚀 Starting Kotoba API Server on port {}", port);
    println!("📊 Database: {}", db_path.display());
    println!("🌐 API endpoints:");
    println!("  POST /api/todo/add     - Add new todo");
    println!("  GET  /api/todo/list    - List all todos");
    println!("  POST /api/todo/{id}/complete - Mark todo as completed");
    println!("  DELETE /api/todo/{id}  - Delete todo");

    HttpServer::new(move || {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .app_data(app_state.clone())
            .wrap(cors)
            .wrap(Logger::default())
            .service(
                web::scope("/api")
                    .service(
                        web::scope("/todo")
                            .route("/add", web::post().to(add_todo))
                            .route("/list", web::get().to(list_todos))
                            .route("/{id}/complete", web::post().to(complete_todo))
                            .route("/{id}", web::delete().to(delete_todo))
                    )
            )
            .service(fs::Files::new("/static", "examples/").show_files_listing())
            .route("/", web::get().to(index))
            .route("/app", web::get().to(todo_app))
            .configure(|cfg| configure_realtime_routes(cfg, event_broadcaster, engidb))
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
    .map_err(|e| Error::Storage(format!("HTTP server error: {}", e)))
}

/// Root endpoint - serve the Todo UI
async fn index() -> impl Responder {
    // For now, return a simple HTML. In production, this would serve the UI-IR generated HTML
    HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<!DOCTYPE html>
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
async fn todo_app() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../examples/todo_app_full.html"))
}

/// Add a new todo item
async fn add_todo(
    req: web::Json<CreateTodoRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("📝 Adding todo: {}", req.title);

    let engidb = match data.engidb.lock() {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "error": "Database lock error"
        })),
    };

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
    match engidb.store_todo_item(&todo_node) {
        Ok(_) => {
            println!("✅ Todo stored in EngiDB: {} (ID: {})", req.title, id);

            // Commit the change
            let _ = engidb.commit("main", "api-server".to_string(), format!("Add todo: {}", req.title));

            // Broadcast real-time event
            let _ = broadcast_event(&data.event_broadcaster, RealtimeEvent::TodoAdded {
                id,
                title: req.title.clone(),
            });

            // Return HTMX-compatible response
            // HTMX will trigger the "todoAdded" event to update the list
            HttpResponse::Created()
                .insert_header(("HX-Trigger", "todoAdded"))
                .json(serde_json::json!({
                    "success": true,
                    "id": id,
                    "message": "Todo added successfully"
                }))
        }
        Err(e) => {
            eprintln!("❌ Failed to store todo: {}", e);
            HttpResponse::InternalServerError().json(serde_json::json!({
                "error": "Failed to store todo",
                "details": e.to_string()
            }))
        }
    }
}

/// List all todo items (HTMX HTML response)
async fn list_todos(data: web::Data<AppState>) -> impl Responder {
    println!("📋 Listing todos for HTMX");

    let engidb = match data.engidb.lock() {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError()
            .content_type("text/html")
            .body("<div class=\"text-red-500\">Database error</div>"),
    };

    // Query todos from EngiDB
    match engidb.scan_todo_items() {
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
            HttpResponse::Ok()
                .content_type("text/html")
                .body(html)
        }
        Err(e) => {
            eprintln!("❌ Failed to scan todos: {}", e);
            HttpResponse::InternalServerError()
                .content_type("text/html")
                .body("<div class=\"text-red-500\">Failed to load todos</div>")
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
    path: web::Path<u64>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    println!("✅ Completing todo #{}", id);

    // For HTMX, we just return success without content
    // The checkbox state change is handled client-side
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("")
}

/// Delete a todo item
async fn delete_todo(
    path: web::Path<u64>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    println!("🗑️ Deleting todo #{}", id);

    // For HTMX, we return empty content to remove the element
    // The hx-target="closest div" and hx-swap="outerHTML" will remove the todo item
    HttpResponse::Ok()
        .content_type("text/plain")
        .body("")
}

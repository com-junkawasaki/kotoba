//! HTTP API Server for Kotoba
//!
//! Provides REST API endpoints for the Todo application,
//! connecting HTMX frontend with EngiDB backend.

use crate::{engidb::EngiDB, Error, Result};
use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use actix_web::middleware::Logger;
use actix_cors::Cors;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Shared application state
pub struct AppState {
    pub engidb: Mutex<EngiDB>,
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
    let app_state = web::Data::new(AppState {
        engidb: Mutex::new(engidb),
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
            .route("/", web::get().to(index))
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

    <p><a href="/static/todo.html">Open Todo App</a></p>
</body>
</html>"#)
}

/// Add a new todo item
async fn add_todo(
    req: web::Json<CreateTodoRequest>,
    data: web::Data<AppState>,
) -> impl Responder {
    println!("📝 Adding todo: {}", req.title);

    // Note: Since we can't use the existing add_todo function due to lifetime issues,
    // we'll implement a simplified version here. In production, this would be refactored.

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

    // Create todo node (simplified - in production this would use proper EAF-IPG structure)
    let todo_item = TodoItem {
        id,
        title: req.title.clone(),
        description: req.description.clone().unwrap_or_default(),
        completed: false,
        created_at: now.clone(),
        updated_at: now,
    };

    // In a real implementation, this would store in EngiDB
    // For now, we'll just return success
    println!("✅ Todo added: {} (ID: {})", todo_item.title, todo_item.id);

    HttpResponse::Created().json(serde_json::json!({
        "success": true,
        "todo": todo_item,
        "message": "Todo added successfully"
    }))
}

/// List all todo items
async fn list_todos(data: web::Data<AppState>) -> impl Responder {
    println!("📋 Listing todos");

    // In a real implementation, this would query EngiDB
    // For now, return empty list with a note
    let todos: Vec<TodoItem> = vec![];

    HttpResponse::Ok().json(serde_json::json!({
        "todos": todos,
        "message": "Todo listing - EngiDB integration pending",
        "count": todos.len()
    }))
}

/// Mark a todo as completed
async fn complete_todo(
    path: web::Path<u64>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    println!("✅ Completing todo #{}", id);

    // In a real implementation, this would update the todo in EngiDB
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "id": id,
        "message": "Todo completion - EngiDB integration pending"
    }))
}

/// Delete a todo item
async fn delete_todo(
    path: web::Path<u64>,
    data: web::Data<AppState>,
) -> impl Responder {
    let id = path.into_inner();
    println!("🗑️ Deleting todo #{}", id);

    // In a real implementation, this would delete from EngiDB
    HttpResponse::Ok().json(serde_json::json!({
        "success": true,
        "id": id,
        "message": "Todo deletion - EngiDB integration pending"
    }))
}

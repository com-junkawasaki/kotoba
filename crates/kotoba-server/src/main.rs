use std::sync::Arc;
use std::path::PathBuf;
use std::collections::HashMap;
use clap::Parser;
use axum::{
    routing::{any, get, post},
    extract::{Path as AxumPath, State, RawBody},
    http::{Request, StatusCode, HeaderMap},
    response::{Html, IntoResponse, Json as AxumJson, Response},
    Router,
};
use kotoba_workflow::prelude::*;
use kotoba_routing::engine::{HttpRoutingEngine, HttpRequest, HttpResponse};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use kotoba_errors::KotobaError;

/// Command line arguments for kotoba-server
#[derive(Parser)]
#[command(name = "kotoba-server")]
#[command(about = "Kotoba HTTP Server")]
struct Args {
    /// Host address to bind to
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    /// Port number to bind to
    #[arg(long, default_value = "8100")]
    port: u16,

    /// Configuration file path
    #[arg(long)]
    config: Option<PathBuf>,

    /// Enable development mode
    #[arg(long)]
    dev: bool,
}

// Application state holds both the workflow engine and the new routing engine.
struct AppStateInt {
    // Both engines are now wrapped in an Arc for shared ownership.
    workflow_engine: Arc<WorkflowEngine>,
    routing_engine: Arc<HttpRoutingEngine>,
}
type AppState = Arc<AppStateInt>;

// Make our custom error type convertible into a response.
impl IntoResponse for KotobaError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            KotobaError::NotFound(ressource) => (StatusCode::NOT_FOUND, format!("Resource not found: {}", ressource)),
            KotobaError::Validation(details) => (StatusCode::BAD_REQUEST, format!("Validation failed: {}", details)),
            KotobaError::Security(details) => (StatusCode::FORBIDDEN, format!("Forbidden: {}", details)),
            KotobaError::InvalidArgument(details) => (StatusCode::BAD_REQUEST, format!("Invalid argument: {}", details)),
            // For other errors, we don't want to leak internal details to the client.
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "An internal server error occurred".to_string()),
        };

        // Log the full error for debugging, regardless of what is sent to the client.
        tracing::error!("An error occurred: {:?}", self);

        (status, message).into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kotoba_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create workflow engine instance
    let workflow_engine = Arc::new(WorkflowEngine::builder()
            .with_memory_storage()
            .build()
            .await?);

    // Create and initialize the routing engine, passing the workflow engine to it.
    let app_dir = std::env::current_dir()?.join("src").join("app");
    let routing_engine = Arc::new(HttpRoutingEngine::new(&app_dir, Arc::clone(&workflow_engine)).await?);

    // Combine into a single AppState
    let app_state = Arc::new(AppStateInt {
        workflow_engine,
        routing_engine,
    });

    // Build our application with a fallback to the routing engine
    let app = Router::new()
        .route("/api/v1/workflows", post(start_workflow))
        .route("/api/v1/workflows/:id", get(get_workflow_status))
        // The fallback handler will process all other requests
        .fallback(routing_handler)
        .with_state(app_state)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let addr = format!("{}:{}", args.host, args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("🌐 Kotoba HTTP Server listening on {}", listener.local_addr()?);

    if args.dev {
        tracing::info!("🚀 Development mode enabled");
    }

    axum::serve(listener, app).await?;

    Ok(())
}

/// The main handler that delegates requests to the kotoba-routing engine.
async fn routing_handler(
    State(state): State<AppState>,
    request: Request<RawBody>,
) -> Response {
    // 1. Convert Axum request into our HttpRequest
    let (parts, body) = request.into_parts();
    let body_bytes = body.collect().await.unwrap().to_bytes();
    let json_body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap_or(serde_json::Value::Null);
    
    let kotoba_req = HttpRequest {
        method: parts.method.to_string(),
        path: parts.uri.path().to_string(),
        headers: parts.headers.iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect(),
        body: json_body,
    };

    // 2. Process the request with the engine
    let kotoba_res = state.routing_engine.handle_request(kotoba_req).await;
    
    // 3. Convert our HttpResponse back into an Axum Response
    match kotoba_res {
        Ok(res) => {
            let mut headers = HeaderMap::new();
            for (key, val) in res.headers {
                headers.insert(key.parse().unwrap(), val.parse().unwrap());
            }

            if res.headers.get("Content-Type").map(|v| v == "text/html").unwrap_or(false) {
                (StatusCode::from_u16(res.status_code).unwrap(), headers, Html(res.body.to_string())).into_response()
            } else {
                (StatusCode::from_u16(res.status_code).unwrap(), headers, AxumJson(res.body)).into_response()
            }
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response()
        }
    }
}

async fn start_workflow(
    State(state): State<AppState>,
    Json(payload): Json<WorkflowIR>,
) -> Result<Json<StartWorkflowResponse>, (StatusCode, String)> {
    let execution_id = state.workflow_engine
        .start_workflow(&payload, Default::default())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StartWorkflowResponse {
        execution_id: execution_id.0,
    }))
}

async fn get_workflow_status(
    State(state): State<AppState>,
    AxumPath(execution_id): AxumPath<String>,
) -> Result<Json<WorkflowExecution>, (StatusCode, String)> {
    let exec_id = WorkflowExecutionId(execution_id);
    let execution = state.workflow_engine
        .get_execution(&exec_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "Workflow execution not found".to_string()))?;

    Ok(Json(execution))
}

#[derive(Serialize)]
struct StartWorkflowResponse {
    execution_id: String,
}

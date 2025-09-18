use std::sync::Arc;
use axum::{
    routing::{get, post},
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    Router,
};
use kotoba_workflow::prelude::*;
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Application state
type AppState = Arc<WorkflowEngine>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "kotoba_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Create workflow engine instance
    let workflow_engine = Arc::new(
        WorkflowEngine::builder()
            .with_memory_storage()
            .build()
            .await?,
    );

    // build our application with a route
    let app = Router::new()
        .route("/api/v1/workflows", post(start_workflow))
        .route("/api/v1/workflows/:id", get(get_workflow_status))
        .with_state(workflow_engine)
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    tracing::debug!("listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}

async fn start_workflow(
    State(engine): State<AppState>,
    Json(payload): Json<WorkflowIR>,
) -> Result<Json<StartWorkflowResponse>, (StatusCode, String)> {
    let execution_id = engine
        .start_workflow(&payload, Default::default())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    Ok(Json(StartWorkflowResponse {
        execution_id: execution_id.0,
    }))
}

async fn get_workflow_status(
    State(engine): State<AppState>,
    Path(execution_id): Path<String>,
) -> Result<Json<WorkflowExecution>, (StatusCode, String)> {
    let exec_id = WorkflowExecutionId(execution_id);
    let execution = engine
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

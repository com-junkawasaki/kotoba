//! Workflow HTTP handlers

use axum::{
    extract::{Path as AxumPath, State},
    response::Json,
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[cfg(feature = "workflow")]
use crate::{WorkflowEngineInterface, WorkflowExecutionId, WorkflowIR, WorkflowExecution};

/// Shared state for workflow handlers
#[cfg(feature = "workflow")]
#[derive(Clone)]
pub struct WorkflowState<E> {
    pub engine: Arc<E>,
}

#[cfg(feature = "workflow")]
impl<E> WorkflowState<E> {
    pub fn new(engine: E) -> Self {
        Self {
            engine: Arc::new(engine),
        }
    }
}

/// Start workflow response
#[derive(Debug, Serialize, Deserialize)]
pub struct StartWorkflowResponse {
    pub execution_id: String,
}

/// Workflow API handler
#[cfg(feature = "workflow")]
#[derive(Clone)]
pub struct WorkflowApiHandler<E> {
    state: WorkflowState<E>,
}

#[cfg(feature = "workflow")]
impl<E> WorkflowApiHandler<E>
where
    E: WorkflowEngineInterface + Send + Sync + 'static,
{
    pub fn new(engine: E) -> Self {
        Self {
            state: WorkflowState::new(engine),
        }
    }

    pub async fn start_workflow(
        State(state): State<WorkflowState<E>>,
        Json(payload): Json<WorkflowIR>,
    ) -> Result<Json<StartWorkflowResponse>, (StatusCode, String)> {
        let execution_id = state.engine
            .start_workflow(&payload, serde_json::Value::Null)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(StartWorkflowResponse {
            execution_id: execution_id.0,
        }))
    }

    pub async fn get_workflow_status(
        State(state): State<WorkflowState<E>>,
        AxumPath(execution_id): AxumPath<String>,
    ) -> Result<Json<WorkflowExecution>, (StatusCode, String)> {
        let exec_id = WorkflowExecutionId(execution_id);
        let execution = state.engine
            .get_execution(&exec_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or_else(|| (StatusCode::NOT_FOUND, "Workflow execution not found".to_string()))?;

        Ok(Json(execution))
    }

    pub async fn list_workflows(
        State(state): State<WorkflowState<E>>,
    ) -> Result<Json<Vec<WorkflowExecution>>, (StatusCode, String)> {
        let executions = state.engine
            .list_executions()
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(Json(executions))
    }

    pub async fn cancel_workflow(
        State(state): State<WorkflowState<E>>,
        AxumPath(execution_id): AxumPath<String>,
    ) -> Result<StatusCode, (StatusCode, String)> {
        let exec_id = WorkflowExecutionId(execution_id);
        state.engine
            .cancel_execution(&exec_id)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        Ok(StatusCode::NO_CONTENT)
    }
}

/// Workflow status handler for simpler use cases
#[derive(Clone)]
pub struct WorkflowStatusHandler;

impl WorkflowStatusHandler {
    pub fn new() -> Self {
        Self
    }

    pub async fn health() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
        Ok(Json(serde_json::json!({
            "status": "workflow_integration_available",
            "version": env!("CARGO_PKG_VERSION")
        })))
    }
}

impl Default for WorkflowStatusHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Workflow API response types
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowListResponse {
    pub workflows: Vec<WorkflowSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowSummary {
    pub execution_id: String,
    pub status: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

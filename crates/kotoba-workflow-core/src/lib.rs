//! # Kotoba Workflow Core
//!
//! Serverless Workflow specification compliant workflow engine.
//! Based on https://serverlessworkflow.io/specification

pub mod engine;
pub mod parser;
pub mod types;

/// Re-export main types
pub use engine::{WorkflowEngine, WorkflowEngineBuilder};
pub use types::*;

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        WorkflowEngine,
        WorkflowDocument,
        WorkflowState,
        ExecutionContext,
        ExecutionResult,
        ExecutionStatus,
        StateResult,
        StateStatus,
        WorkflowError,
    };
}

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Workflow execution identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionId(pub String);

impl fmt::Display for WorkflowExecutionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for WorkflowExecutionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for WorkflowExecutionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}


/// Workflow execution state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub execution_id: WorkflowExecutionId,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// Start workflow response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartWorkflowResponse {
    pub execution_id: String,
}

/// Minimal workflow IR representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIR {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub activities: Vec<ActivityIR>,
    pub connections: Vec<Connection>,
}

/// Activity IR representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIR {
    pub id: String,
    pub name: String,
    pub activity_type: String,
    pub config: serde_json::Value,
}

/// Connection between activities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}

/// Workflow engine interface trait
#[async_trait]
pub trait WorkflowEngineInterface: Send + Sync {
    /// Start workflow execution
    async fn start_workflow(
        &self,
        workflow: &WorkflowDocument,
        input: serde_json::Value,
    ) -> Result<String, WorkflowError>;

    /// Get execution status
    async fn get_execution_status(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionStatus>, WorkflowError>;

    /// Get execution result
    async fn get_execution_result(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionResult>, WorkflowError>;

    /// List all executions
    async fn list_executions(&self) -> Result<Vec<String>, WorkflowError>;

    /// Cancel execution
    async fn cancel_execution(
        &self,
        execution_id: &str,
    ) -> Result<(), WorkflowError>;
}

/// Workflow-specific error type
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    NotFound(String),

    #[error("Workflow validation failed: {0}")]
    Validation(String),

    #[error("Workflow execution failed: {0}")]
    Execution(String),

    #[error("Workflow execution error: {0}")]
    ExecutionError(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<kotoba_errors::KotobaError> for WorkflowError {
    fn from(err: kotoba_errors::KotobaError) -> Self {
        match err {
            kotoba_errors::KotobaError::NotFound(msg) => WorkflowError::NotFound(msg),
            kotoba_errors::KotobaError::Validation(msg) => WorkflowError::Validation(msg),
            _ => WorkflowError::Unknown(err.to_string()),
        }
    }
}


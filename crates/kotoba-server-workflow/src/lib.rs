//! # Kotoba Server Workflow Integration
//!
//! Workflow integration components for the Kotoba HTTP server.
//! Provides workflow API endpoints and routing integration.

pub mod handlers;
pub mod router;

#[cfg(feature = "workflow")]
pub mod workflow;

pub use handlers::{WorkflowApiHandler, WorkflowStatusHandler};
pub use router::WorkflowRouter;

#[cfg(feature = "workflow")]
pub use workflow::{WorkflowEngine, WorkflowExecutionId, WorkflowIR, StartWorkflowResponse};

use axum::Router;
use kotoba_server_core::{AppRouter, ServerError};
use std::sync::Arc;

/// Workflow server extension trait
pub trait WorkflowServerExt {
    fn with_workflow_routes(self) -> Self;
    fn with_workflow_engine<E>(self, engine: E) -> Self
    where
        E: WorkflowEngineInterface + Send + Sync + 'static;
}

impl WorkflowServerExt for AppRouter {
    fn with_workflow_routes(self) -> Self {
        self.merge(WorkflowRouter::new().build())
    }

    fn with_workflow_engine<E>(self, engine: E) -> Self
    where
        E: WorkflowEngineInterface + Send + Sync + 'static,
    {
        let workflow_router = WorkflowRouter::new().with_engine(engine);
        self.merge(workflow_router.build())
    }
}

/// Workflow engine interface for dependency injection
#[cfg(feature = "workflow")]
#[async_trait::async_trait]
pub trait WorkflowEngineInterface: Send + Sync {
    async fn start_workflow(
        &self,
        workflow: &WorkflowIR,
        context: serde_json::Value,
    ) -> Result<WorkflowExecutionId, kotoba_errors::KotobaError>;

    async fn get_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, kotoba_errors::KotobaError>;

    async fn list_executions(&self) -> Result<Vec<WorkflowExecution>, kotoba_errors::KotobaError>;

    async fn cancel_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), kotoba_errors::KotobaError>;
}

#[cfg(feature = "workflow")]
pub use kotoba_workflow_core::prelude::*;

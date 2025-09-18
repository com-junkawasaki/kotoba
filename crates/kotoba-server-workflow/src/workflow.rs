//! Workflow engine re-exports and utilities

#[cfg(feature = "workflow")]
pub use kotoba_workflow_core::{
    WorkflowEngine,
    WorkflowExecutionId,
    WorkflowIR,
    WorkflowExecution,
    StartWorkflowResponse,
};

#[cfg(feature = "workflow")]
use async_trait::async_trait;

/// Workflow engine interface re-export
#[cfg(feature = "workflow")]
#[async_trait]
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
#[async_trait]
impl WorkflowEngineInterface for WorkflowEngine {
    async fn start_workflow(
        &self,
        workflow: &WorkflowIR,
        context: serde_json::Value,
    ) -> Result<WorkflowExecutionId, kotoba_errors::KotobaError> {
        self.start_workflow(workflow, context).await
    }

    async fn get_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, kotoba_errors::KotobaError> {
        self.get_execution(execution_id).await
    }

    async fn list_executions(&self) -> Result<Vec<WorkflowExecution>, kotoba_errors::KotobaError> {
        // TODO: Implement list_executions in WorkflowEngine
        Err(kotoba_errors::KotobaError::Unknown("list_executions not implemented".to_string()))
    }

    async fn cancel_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), kotoba_errors::KotobaError> {
        // TODO: Implement cancel_execution in WorkflowEngine
        Err(kotoba_errors::KotobaError::Unknown("cancel_execution not implemented".to_string()))
    }
}

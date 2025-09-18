//! Core workflow engine implementation

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    WorkflowEngineInterface,
    WorkflowIR,
    WorkflowExecution,
    WorkflowExecutionId,
    ExecutionStatus,
    WorkflowError,
};

/// In-memory workflow engine implementation
#[derive(Debug)]
pub struct WorkflowEngine {
    executions: Arc<RwLock<HashMap<WorkflowExecutionId, WorkflowExecution>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn builder() -> WorkflowEngineBuilder {
        WorkflowEngineBuilder::new()
    }
}

impl Default for WorkflowEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl WorkflowEngineInterface for WorkflowEngine {
    async fn start_workflow(
        &self,
        workflow: &WorkflowIR,
        _context: serde_json::Value,
    ) -> Result<WorkflowExecutionId, WorkflowError> {
        let execution_id = WorkflowExecutionId(Uuid::new_v4().to_string());
        let now = Utc::now();

        let execution = WorkflowExecution {
            execution_id: execution_id.clone(),
            workflow_id: workflow.id.clone(),
            status: ExecutionStatus::Running,
            created_at: now,
            updated_at: now,
            result: None,
            error: None,
        };

        // Simulate workflow execution (in a real implementation, this would spawn tasks)
        let execution_clone = execution.clone();
        let executions = Arc::clone(&self.executions);

        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let mut executions = executions.write().await;
            if let Some(mut exec) = executions.get_mut(&execution_clone.execution_id) {
                exec.status = ExecutionStatus::Completed;
                exec.updated_at = Utc::now();
                exec.result = Some(serde_json::json!({"message": "Workflow completed successfully"}));
            }
        });

        let mut executions = self.executions.write().await;
        executions.insert(execution_id.clone(), execution);

        Ok(execution_id)
    }

    async fn get_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, WorkflowError> {
        let executions = self.executions.read().await;
        Ok(executions.get(execution_id).cloned())
    }

    async fn list_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        let executions = self.executions.read().await;
        Ok(executions.values().cloned().collect())
    }

    async fn cancel_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        if let Some(mut execution) = executions.get_mut(execution_id) {
            execution.status = ExecutionStatus::Cancelled;
            execution.updated_at = Utc::now();
            Ok(())
        } else {
            Err(WorkflowError::NotFound(format!("Execution {} not found", execution_id)))
        }
    }
}

/// Workflow engine builder
#[derive(Debug, Default)]
pub struct WorkflowEngineBuilder {
    // Future: add configuration options
}

impl WorkflowEngineBuilder {
    pub fn new() -> Self {
        Self {}
    }

    pub fn build(self) -> WorkflowEngine {
        WorkflowEngine::new()
    }

    /// Memory storage (default)
    pub fn with_memory_storage(self) -> Self {
        // For now, just return self - future versions may support different backends
        self
    }
}


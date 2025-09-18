//! Core workflow engine implementation
//!
//! Based on https://serverlessworkflow.io/specification

use async_trait::async_trait;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{
    WorkflowDocument,
    WorkflowState,
    ExecutionContext,
    ExecutionResult,
    ExecutionStatus,
    StateResult,
    StateStatus,
    WorkflowError,
    WorkflowEngineInterface,
};

/// In-memory workflow engine implementation
#[derive(Debug)]
pub struct WorkflowEngine {
    executions: Arc<RwLock<HashMap<String, ExecutionContext>>>,
    results: Arc<RwLock<HashMap<String, ExecutionResult>>>,
    state_results: Arc<RwLock<HashMap<String, Vec<StateResult>>>>,
}

impl WorkflowEngine {
    pub fn new() -> Self {
        Self {
            executions: Arc::new(RwLock::new(HashMap::new())),
            results: Arc::new(RwLock::new(HashMap::new())),
            state_results: Arc::new(RwLock::new(HashMap::new())),
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
        workflow: &WorkflowDocument,
        input: serde_json::Value,
    ) -> Result<String, WorkflowError> {
        let execution_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let context = ExecutionContext {
            execution_id: execution_id.clone(),
            data: input,
            start_time: now,
        };

        // Initialize execution
        {
            let mut executions = self.executions.write().await;
            executions.insert(execution_id.clone(), context.clone());
        }

        // Initialize state results
        {
            let mut state_results = self.state_results.write().await;
            state_results.insert(execution_id.clone(), Vec::new());
        }

        // Start workflow execution in background
        let executions = Arc::clone(&self.executions);
        let results = Arc::clone(&self.results);
        let state_results = Arc::clone(&self.state_results);
        let workflow_clone = workflow.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            Self::execute_workflow(
                workflow_clone,
                &execution_id_clone,
                context,
                executions,
                results,
                state_results,
            ).await;
        });

        Ok(execution_id)
    }

    async fn get_execution_status(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionStatus>, WorkflowError> {
        let results = self.results.read().await;
        Ok(results.get(execution_id).map(|r| r.status.clone()))
    }

    async fn get_execution_result(
        &self,
        execution_id: &str,
    ) -> Result<Option<ExecutionResult>, WorkflowError> {
        let results = self.results.read().await;
        Ok(results.get(execution_id).cloned())
    }

    async fn list_executions(&self) -> Result<Vec<String>, WorkflowError> {
        let executions = self.executions.read().await;
        Ok(executions.keys().cloned().collect())
    }

    async fn cancel_execution(
        &self,
        execution_id: &str,
    ) -> Result<(), WorkflowError> {
        // Mark execution as cancelled
        let mut results = self.results.write().await;
        if let Some(result) = results.get_mut(execution_id) {
            result.status = ExecutionStatus::Cancelled;
            result.completed_at = Utc::now();
        }
        Ok(())
    }
}

impl WorkflowEngine {
    /// Execute workflow states sequentially
    async fn execute_workflow(
        workflow: WorkflowDocument,
        execution_id: &str,
        mut context: ExecutionContext,
        executions: Arc<RwLock<HashMap<String, ExecutionContext>>>,
        results: Arc<RwLock<HashMap<String, ExecutionResult>>>,
        state_results: Arc<RwLock<HashMap<String, Vec<StateResult>>>>,
    ) {
        let start_time = Utc::now();
        let mut state_index = 0;

        // Execute each state in sequence
        while state_index < workflow.r#do.len() {
            let state = &workflow.r#do[state_index];
            let state_name = Self::get_state_name(state, state_index);

            let state_start = Utc::now();
            let state_result = Self::execute_state(state, &mut context).await;
            let state_end = Utc::now();

            let state_status = match &state_result {
                Ok(_) => StateStatus::Succeeded,
                Err(_) => StateStatus::Failed,
            };

            let state_result_record = StateResult {
                state_name: state_name.clone(),
                status: state_status,
                output: state_result.as_ref().ok().cloned(),
                execution_time_ms: (state_end - state_start).num_milliseconds() as u64,
                started_at: state_start,
                completed_at: state_end,
                error: state_result.as_ref().err().map(|e| e.to_string()),
            };

            // Record state result
            {
                let mut state_results_lock = state_results.write().await;
                if let Some(results) = state_results_lock.get_mut(execution_id) {
                    results.push(state_result_record);
                }
            }

            match state_result {
                Ok(_) => {
                    state_index += 1; // Move to next state
                }
                Err(error) => {
                    // Create failed execution result
                    let result = ExecutionResult {
                        execution_id: execution_id.to_string(),
                        status: ExecutionStatus::Failed,
                        output: None,
                        duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
                        completed_at: Utc::now(),
                        errors: vec![error.to_string()],
                    };

                    let mut results_lock = results.write().await;
                    results_lock.insert(execution_id.to_string(), result);
                    return;
                }
            }
        }

        // Create successful execution result
        let result = ExecutionResult {
            execution_id: execution_id.to_string(),
            status: ExecutionStatus::Succeeded,
            output: Some(context.data),
            duration_ms: (Utc::now() - start_time).num_milliseconds() as u64,
            completed_at: Utc::now(),
            errors: Vec::new(),
        };

        let mut results_lock = results.write().await;
        results_lock.insert(execution_id.to_string(), result);
    }

    /// Execute a single workflow state
    async fn execute_state(
        state: &WorkflowState,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        match state {
            WorkflowState::CallHttp(http_state) => {
                Self::execute_http_call(http_state, context).await
            }
            WorkflowState::Set(set_state) => {
                Self::execute_set_variables(set_state, context).await
            }
            WorkflowState::Wait(wait_state) => {
                Self::execute_wait(wait_state, context).await
            }
            WorkflowState::Raise(raise_state) => {
                Self::execute_raise_error(raise_state, context).await
            }
            _ => {
                // For now, skip unimplemented states
                Ok(serde_json::Value::Null)
            }
        }
    }

    /// Execute HTTP call state
    async fn execute_http_call(
        http_state: &crate::CallHttpState,
        _context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        // This is a simplified implementation
        // In a real implementation, this would make actual HTTP calls
        println!("Executing HTTP call to: {}", http_state.endpoint);
        Ok(serde_json::json!({"status": "simulated", "endpoint": http_state.endpoint}))
    }

    /// Execute set variables state
    async fn execute_set_variables(
        set_state: &crate::SetState,
        context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        for (key, value) in &set_state.variables {
            context.data[key] = value.clone();
        }
        Ok(serde_json::Value::Null)
    }

    /// Execute wait state
    async fn execute_wait(
        wait_state: &crate::WaitState,
        _context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        match &wait_state.wait {
            crate::WaitDefinition::Seconds { seconds } => {
                tokio::time::sleep(tokio::time::Duration::from_secs(*seconds)).await;
            }
            _ => {
                // For now, only seconds are supported
            }
        }
        Ok(serde_json::Value::Null)
    }

    /// Execute raise error state
    async fn execute_raise_error(
        raise_state: &crate::RaiseState,
        _context: &mut ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        Err(WorkflowError::ExecutionError(format!(
            "Raised error: {}",
            raise_state.raise.detail.as_deref().unwrap_or("Unknown error")
        )))
    }

    /// Get state name for logging
    fn get_state_name(state: &WorkflowState, index: usize) -> String {
        match state {
            WorkflowState::CallHttp(s) => s.name.clone().unwrap_or_else(|| format!("http-{}", index)),
            WorkflowState::CallGrpc(s) => s.name.clone().unwrap_or_else(|| format!("grpc-{}", index)),
            WorkflowState::CallOpenApi(s) => s.name.clone().unwrap_or_else(|| format!("openapi-{}", index)),
            WorkflowState::CallAsyncApi(s) => s.name.clone().unwrap_or_else(|| format!("asyncapi-{}", index)),
            WorkflowState::Emit(s) => s.name.clone().unwrap_or_else(|| format!("emit-{}", index)),
            WorkflowState::Listen(s) => s.name.clone().unwrap_or_else(|| format!("listen-{}", index)),
            WorkflowState::RunScript(s) => s.name.clone().unwrap_or_else(|| format!("script-{}", index)),
            WorkflowState::RunContainer(s) => s.name.clone().unwrap_or_else(|| format!("container-{}", index)),
            WorkflowState::RunWorkflow(s) => s.name.clone().unwrap_or_else(|| format!("workflow-{}", index)),
            WorkflowState::Switch(s) => s.name.clone().unwrap_or_else(|| format!("switch-{}", index)),
            WorkflowState::For(s) => s.name.clone().unwrap_or_else(|| format!("for-{}", index)),
            WorkflowState::Fork(s) => s.name.clone().unwrap_or_else(|| format!("fork-{}", index)),
            WorkflowState::Try(s) => s.name.clone().unwrap_or_else(|| format!("try-{}", index)),
            WorkflowState::Wait(s) => s.name.clone().unwrap_or_else(|| format!("wait-{}", index)),
            WorkflowState::Raise(s) => s.name.clone().unwrap_or_else(|| format!("raise-{}", index)),
            WorkflowState::Set(s) => s.name.clone().unwrap_or_else(|| format!("set-{}", index)),
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


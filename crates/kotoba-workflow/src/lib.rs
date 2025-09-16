//! # Kotoba Workflow Engine (Itonami)
//!
//! Temporal-inspired workflow engine built on top of Kotoba's graph rewriting system.
//!
//! ## Features
//!
//! - **Temporal Patterns**: Sequence, Parallel, Decision, Wait, Saga, Activity, Sub-workflow
//! - **MVCC Persistence**: Workflow state management with Merkle DAG
//! - **Graph-based Execution**: Declarative workflow definition using graph transformations
//! - **Activity System**: Extensible activity execution framework
//! - **Event Sourcing**: Complete audit trail of workflow execution
//!
//! ## Example
//!
//! ```rust
//! use kotoba_workflow::prelude::*;
//! use std::collections::HashMap;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create activity registry
//!     let registry = ActivityRegistry::new();
//!
//!     // Activity system is ready!
//!     println!("Workflow activity system ready!");
//!     Ok(())
//! }
//! ```

pub mod ir;
pub mod executor;
pub mod store;
pub mod activity;

// Re-export main types
pub use ir::{WorkflowIR, WorkflowExecution, WorkflowExecutionId, ActivityIR, DummyGraphRef, ExecutionStatus};
pub use executor::{ActivityRegistry, Activity, WorkflowExecutor, WorkflowStateManager, WorkflowError};
pub use store::{WorkflowStore, StorageBackend, StorageFactory};
pub use activity::prelude::*;

/// Workflow engine builder
pub struct WorkflowEngineBuilder {
    storage_backend: Option<StorageBackend>,
}

impl WorkflowEngineBuilder {
    pub fn new() -> Self {
        Self {
            storage_backend: Some(StorageBackend::Memory), // Default to memory
        }
    }

    /// Configure storage backend
    pub fn with_storage_backend(mut self, backend: StorageBackend) -> Self {
        self.storage_backend = Some(backend);
        self
    }

    /// Use memory storage (default)
    pub fn with_memory_storage(mut self) -> Self {
        self.storage_backend = Some(StorageBackend::Memory);
        self
    }

    /// Use RocksDB storage (requires 'rocksdb' feature)
    #[cfg(feature = "rocksdb")]
    pub fn with_rocksdb_storage(mut self, path: impl Into<String>) -> Self {
        self.storage_backend = Some(StorageBackend::RocksDB { path: path.into() });
        self
    }

    /// Use SQLite storage (requires 'sqlite' feature)
    #[cfg(feature = "sqlite")]
    pub fn with_sqlite_storage(mut self, path: impl Into<String>) -> Self {
        self.storage_backend = Some(StorageBackend::SQLite { path: path.into() });
        self
    }

    pub async fn build(self) -> Result<WorkflowEngine, WorkflowError> {
        let storage = if let Some(backend) = self.storage_backend {
            StorageFactory::create(backend).await?
        } else {
            return Err(WorkflowError::StorageError("No storage backend configured".to_string()));
        };

        let activity_registry = std::sync::Arc::new(ActivityRegistry::new());
        let state_manager = std::sync::Arc::new(WorkflowStateManager::new());

        Ok(WorkflowEngine {
            storage,
            activity_registry,
            state_manager,
            executor: None,
        })
    }
}

/// Main workflow engine
pub struct WorkflowEngine {
    storage: std::sync::Arc<dyn WorkflowStore>,
    activity_registry: std::sync::Arc<ActivityRegistry>,
    state_manager: std::sync::Arc<WorkflowStateManager>,
    executor: Option<std::sync::Arc<WorkflowExecutor>>,
}

impl WorkflowEngine {
    pub fn builder() -> WorkflowEngineBuilder {
        WorkflowEngineBuilder::new()
    }

    /// Get activity registry for registering activities
    pub fn activity_registry(&self) -> &std::sync::Arc<ActivityRegistry> {
        &self.activity_registry
    }

    /// Start workflow execution
    pub async fn start_workflow(
        &mut self,
        workflow_ir: &WorkflowIR,
        inputs: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<WorkflowExecutionId, WorkflowError> {
        let executor = self.executor.get_or_insert_with(|| {
            std::sync::Arc::new(WorkflowExecutor::new(
                std::sync::Arc::clone(&self.activity_registry),
                std::sync::Arc::clone(&self.state_manager),
            ))
        });

        executor.start_workflow(workflow_ir, inputs).await
    }

    /// Wait for workflow completion
    pub async fn wait_for_completion(
        &self,
        execution_id: WorkflowExecutionId,
        timeout: Option<std::time::Duration>,
    ) -> Result<WorkflowResult, WorkflowError> {
        // TODO: Implement actual completion waiting logic with timeout
        // For now, just poll the execution status
        match self.storage.get_execution(&execution_id).await? {
            Some(execution) => Ok(WorkflowResult {
                execution_id,
                status: execution.status,
                outputs: execution.outputs,
                execution_time: execution.start_time.signed_duration_since(chrono::Utc::now()).to_std()
                    .unwrap_or(std::time::Duration::from_secs(0)),
            }),
            None => Err(WorkflowError::WorkflowNotFound(execution_id.0)),
        }
    }

    /// Get workflow execution status
    pub async fn get_execution_status(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<ExecutionStatus>, WorkflowError> {
        match self.storage.get_execution(execution_id).await? {
            Some(execution) => Ok(Some(execution.status)),
            None => Ok(None),
        }
    }

    /// Get workflow execution details
    pub async fn get_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, WorkflowError> {
        self.storage.get_execution(execution_id).await
    }

    /// List running executions
    pub async fn list_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
        self.storage.get_running_executions().await
    }

    /// Cancel workflow execution
    pub async fn cancel_execution(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), WorkflowError> {
        // TODO: Implement cancellation logic
        // For now, just update status
        if let Some(mut execution) = self.storage.get_execution(execution_id).await? {
            execution.status = ExecutionStatus::Cancelled;
            execution.end_time = Some(chrono::Utc::now());
            self.storage.update_execution(&execution).await?;
        }
        Ok(())
    }
}

/// Workflow execution result
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub execution_id: WorkflowExecutionId,
    pub status: ExecutionStatus,
    pub outputs: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub execution_time: std::time::Duration,
}

// WorkflowError is re-exported from executor module

/// Prelude for convenient imports
pub mod prelude {
    pub use super::{
        WorkflowEngine, WorkflowIR, WorkflowExecution, WorkflowExecutionId,
        ActivityRegistry, Activity, WorkflowStore, ExecutionStatus,
    };
}

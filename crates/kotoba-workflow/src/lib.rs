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

use std::collections::HashMap;
use std::sync::Arc;
use crate::ir::{ExecutionEventType, ExecutionEvent, WorkflowExecution, WorkflowExecutionId};
use crate::store::KotobaStorageBridge;
use crate::distributed::{LoadBalancer, DistributedExecutionManager, DistributedWorkflowExecutor};
use kotoba_core::prelude::TxId;

pub mod ir;
pub mod executor;
pub mod store;
pub mod activity;
pub mod parser;
pub mod distributed;
pub mod saga;
pub mod monitoring;
pub mod optimization;
pub mod integrations;

// Re-export main types
pub use ir::{WorkflowIR, WorkflowExecution, WorkflowExecutionId, ActivityIR, ExecutionStatus};
pub use executor::{ActivityRegistry, Activity, WorkflowExecutor, WorkflowStateManager, WorkflowError};
pub use store::{WorkflowStore, StorageBackend, StorageFactory, EventSourcingManager, SnapshotManager};
pub use parser::WorkflowParser;
pub use activity::prelude::*;
pub use distributed::{
    DistributedCoordinator, DistributedExecutionManager, DistributedWorkflowExecutor,
    LoadBalancer, RoundRobinBalancer, LeastLoadedBalancer, NodeInfo, ClusterHealth
};
// Phase 3: Advanced Features
pub use saga::{SagaManager, SagaExecutionEngine, AdvancedSagaPattern, SagaContext};
pub use monitoring::{MonitoringManager, MonitoringConfig, WorkflowStats, ActivityStats, SystemHealth};
pub use optimization::{WorkflowOptimizer, OptimizationStrategy, OptimizationResult, ParallelExecutionPlan};
pub use integrations::{IntegrationManager, Integration, HttpIntegration, DatabaseIntegration};

/// Workflow engine builder
pub struct WorkflowEngineBuilder {
    storage_backend: Option<StorageBackend>,
    kotoba_backend: Option<std::sync::Arc<dyn kotoba_storage::storage::backend::StorageBackend>>,
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

    /// Use Kotoba storage backend for full integration
    pub fn with_kotoba_storage(mut self, backend: std::sync::Arc<dyn kotoba_storage::storage::backend::StorageBackend>) -> Self {
        self.kotoba_backend = Some(backend);
        // When using Kotoba backend, disable internal storage
        self.storage_backend = None;
        self
    }

    pub async fn build(self) -> Result<WorkflowEngine, WorkflowError> {
        let storage = if let Some(backend) = self.storage_backend {
            StorageFactory::create(backend).await?
        } else if let Some(kotoba_backend) = self.kotoba_backend {
            // Create a bridge to Kotoba storage
            std::sync::Arc::new(KotobaStorageBridge::new(kotoba_backend))
        } else {
            return Err(WorkflowError::StorageError("No storage backend configured".to_string()));
        };

        let activity_registry = std::sync::Arc::new(ActivityRegistry::new());
        let state_manager = std::sync::Arc::new(WorkflowStateManager::new());

        // Phase 2: Initialize event sourcing and snapshot management
        let event_sourcing = std::sync::Arc::new(EventSourcingManager::new(std::sync::Arc::clone(&storage))
            .with_snapshot_config(100, 10));
        let snapshot_manager = std::sync::Arc::new(SnapshotManager::new(
            std::sync::Arc::clone(&storage),
            std::sync::Arc::clone(&event_sourcing)
        ).with_config(50, 5));

        Ok(WorkflowEngine {
            storage,
            activity_registry,
            state_manager,
            event_sourcing,
            snapshot_manager,
            executor: None,
            distributed_executor: None,
        })
    }
}

/// Main workflow engine - Phase 2: MVCC + Event Sourcing + Distributed
pub struct WorkflowEngine {
    storage: std::sync::Arc<dyn WorkflowStore>,
    activity_registry: std::sync::Arc<ActivityRegistry>,
    state_manager: std::sync::Arc<WorkflowStateManager>,
    event_sourcing: std::sync::Arc<EventSourcingManager>,
    snapshot_manager: std::sync::Arc<SnapshotManager>,
    executor: Option<std::sync::Arc<WorkflowExecutor>>,
    /// Phase 2: Distributed execution support
    distributed_executor: Option<std::sync::Arc<DistributedWorkflowExecutor>>,
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
        // Phase 2: Use MVCC-based state management
        if let Some(mut execution) = self.state_manager.get_execution(execution_id).await {
            execution.status = ExecutionStatus::Cancelled;
            execution.end_time = Some(chrono::Utc::now());
            self.state_manager.update_execution(execution).await?;

            // Record cancellation event
            self.event_sourcing.record_event(
                execution_id,
                ExecutionEventType::WorkflowCancelled,
                HashMap::new(),
            ).await?;
        }
        Ok(())
    }

    /// Phase 2: Get workflow execution at specific transaction
    pub async fn get_execution_at_tx(
        &self,
        execution_id: &WorkflowExecutionId,
        tx_id: TxId,
    ) -> Option<WorkflowExecution> {
        self.state_manager.get_execution_at(execution_id, Some(tx_id)).await
    }

    /// Phase 2: Get execution history (all versions)
    pub async fn get_execution_history(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Vec<(TxId, WorkflowExecution)> {
        self.state_manager.get_execution_history(execution_id).await
    }

    /// Phase 2: Get full event history
    pub async fn get_event_history(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Vec<ExecutionEvent>, WorkflowError> {
        self.event_sourcing.get_full_event_history(execution_id).await
    }

    /// Phase 2: Rebuild execution from events (for recovery)
    pub async fn rebuild_execution_from_events(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, WorkflowError> {
        self.event_sourcing.rebuild_execution_from_events(execution_id).await
    }

    /// Phase 2: Create manual snapshot
    pub async fn create_snapshot(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), WorkflowError> {
        if let Some(execution) = self.state_manager.get_execution(execution_id).await {
            self.snapshot_manager.create_snapshot(&execution).await?;
        }
        Ok(())
    }

    /// Phase 2: Restore from snapshot
    pub async fn restore_from_snapshot(
        &self,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<WorkflowExecution>, WorkflowError> {
        self.snapshot_manager.restore_from_snapshot(execution_id).await
    }

    /// Phase 2: Get performance statistics
    pub async fn get_performance_stats(&self) -> HashMap<String, usize> {
        self.snapshot_manager.get_performance_stats().await
    }

    /// Phase 2: Access event sourcing manager
    pub fn event_sourcing(&self) -> &std::sync::Arc<EventSourcingManager> {
        &self.event_sourcing
    }

    /// Phase 2: Access snapshot manager
    pub fn snapshot_manager(&self) -> &std::sync::Arc<SnapshotManager> {
        &self.snapshot_manager
    }

    /// Phase 2: Enable distributed execution
    pub fn enable_distributed_execution(
        &mut self,
        local_node_id: String,
        load_balancer: Arc<dyn LoadBalancer>,
    ) {
        let execution_manager = Arc::new(DistributedExecutionManager::new(
            local_node_id,
            load_balancer,
        ));
        self.distributed_executor = Some(Arc::new(DistributedWorkflowExecutor::new(
            Arc::clone(&execution_manager)
        )));
    }

    /// Phase 2: Submit workflow for distributed execution
    pub async fn submit_distributed_workflow(
        &self,
        execution_id: WorkflowExecutionId,
    ) -> Result<String, WorkflowError> {
        if let Some(distributed) = &self.distributed_executor {
            distributed.execution_manager.submit_execution(execution_id).await
        } else {
            Err(WorkflowError::InvalidStrategy("Distributed execution not enabled".to_string()))
        }
    }

    /// Phase 2: Get cluster health
    pub async fn get_cluster_health(&self) -> Result<ClusterHealth, WorkflowError> {
        if let Some(distributed) = &self.distributed_executor {
            Ok(distributed.cluster_health_check().await)
        } else {
            Err(WorkflowError::InvalidStrategy("Distributed execution not enabled".to_string()))
        }
    }

    /// Phase 2: Get distributed execution manager
    pub fn distributed_execution_manager(&self) -> Option<&std::sync::Arc<DistributedExecutionManager>> {
        self.distributed_executor.as_ref()
            .map(|d| &d.execution_manager)
    }

    /// Phase 2: Check if distributed execution is enabled
    pub fn is_distributed_enabled(&self) -> bool {
        self.distributed_executor.is_some()
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
        WorkflowParser, EventSourcingManager, SnapshotManager,
        // Phase 2 distributed types
        DistributedCoordinator, DistributedExecutionManager, DistributedWorkflowExecutor,
        LoadBalancer, RoundRobinBalancer, LeastLoadedBalancer,
        // Phase 3 advanced features
        SagaManager, SagaExecutionEngine, MonitoringManager, WorkflowOptimizer,
        IntegrationManager, HttpIntegration, DatabaseIntegration,
    };
}

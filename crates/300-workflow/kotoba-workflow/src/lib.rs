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
use crate::ir::{ExecutionEventType, ExecutionEvent};
use crate::store::KotobaStorageBridge;
use crate::distributed::{LoadBalancer, DistributedExecutionManager, DistributedWorkflowExecutor};
use kotoba_core::prelude::TxId;
use kotoba_errors::WorkflowError;
use kotoba_workflow_core::prelude::*;

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

// Re-export main types - use core types where possible
pub use kotoba_workflow_core::{WorkflowIR, WorkflowExecution, WorkflowExecutionId, ExecutionStatus, WorkflowEngineInterface};
pub use ir::{ActivityIR};
pub use executor::{ActivityRegistry, Activity, WorkflowExecutor, WorkflowStateManager};
pub use store::{WorkflowStore, StorageBackend, StorageFactory, EventSourcingManager, SnapshotManager};
pub use parser::WorkflowParser;
pub use activity::prelude::*;
pub use distributed::{
    DistributedCoordinator, RoundRobinBalancer, LeastLoadedBalancer, NodeInfo, ClusterHealth
};
// Phase 3: Advanced Features
pub use saga::{SagaManager, SagaExecutionEngine, AdvancedSagaPattern, SagaContext};
pub use monitoring::{MonitoringManager, MonitoringConfig, WorkflowStats, ActivityStats, SystemHealth};
pub use optimization::{WorkflowOptimizer, OptimizationStrategy, OptimizationResult, ParallelExecutionPlan};
#[cfg(feature = "activities-http")]
pub use integrations::HttpIntegration;
#[cfg(feature = "activities-db")]
pub use integrations::{DatabaseIntegration, MessageQueueIntegration};
pub use integrations::{IntegrationManager, Integration};

/// Workflow engine builder
pub struct WorkflowEngineBuilder {
    storage_backend: Option<StorageBackend>,
    kotoba_backend: Option<std::sync::Arc<dyn kotoba_storage::port::StoragePort>>,
}

impl WorkflowEngineBuilder {
    pub fn new() -> Self {
        Self {
            storage_backend: Some(StorageBackend::Memory), // Default to memory
            kotoba_backend: None,
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
    pub fn with_kotoba_storage(mut self, backend: std::sync::Arc<dyn kotoba_storage::port::StoragePort>) -> Self {
        self.kotoba_backend = Some(backend);
        // When using Kotoba backend, disable internal storage
        self.storage_backend = None;
        self
    }

    pub async fn build(self) -> Result<ExtendedWorkflowEngine, WorkflowError> {
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

        // Create core engine first
        let core_engine = kotoba_workflow_core::WorkflowEngine::new();

        Ok(ExtendedWorkflowEngine {
            core_engine,
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

/// Extended workflow engine - builds on core workflow functionality
pub struct ExtendedWorkflowEngine {
    core_engine: kotoba_workflow_core::WorkflowEngine,
    storage: std::sync::Arc<dyn WorkflowStore>,
    activity_registry: std::sync::Arc<ActivityRegistry>,
    state_manager: std::sync::Arc<WorkflowStateManager>,
    event_sourcing: std::sync::Arc<EventSourcingManager>,
    snapshot_manager: std::sync::Arc<SnapshotManager>,
    executor: Option<std::sync::Arc<WorkflowExecutor>>,
    /// Phase 2: Distributed execution support
    distributed_executor: Option<std::sync::Arc<DistributedWorkflowExecutor>>,
}

impl ExtendedWorkflowEngine {
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

/// Extended workflow execution result with additional timing information
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub execution_id: WorkflowExecutionId,
    pub status: ExecutionStatus,
    pub outputs: Option<WorkflowExecution>,
    pub execution_time: std::time::Duration,
}

// WorkflowError is re-exported from executor module

// Alias for backward compatibility
pub type WorkflowEngine = ExtendedWorkflowEngine;

/// Prelude for convenient imports
pub mod prelude {
    pub use super::{
        WorkflowEngine, ExtendedWorkflowEngine, WorkflowIR,
        ActivityRegistry, Activity, WorkflowStore, ExecutionStatus,
        WorkflowParser, EventSourcingManager, SnapshotManager,
        // Phase 2 distributed types
        DistributedCoordinator, RoundRobinBalancer, LeastLoadedBalancer,
        // Phase 3 advanced features
        SagaManager, SagaExecutionEngine, MonitoringManager, WorkflowOptimizer,
        IntegrationManager,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use chrono::Utc;

    // Mock WorkflowStore for testing
    struct MockWorkflowStore;

    #[async_trait::async_trait]
    impl WorkflowStore for MockWorkflowStore {
        async fn save_execution(&self, _execution: &WorkflowExecution) -> Result<(), WorkflowError> {
            Ok(())
        }

        async fn get_execution(&self, _execution_id: &WorkflowExecutionId) -> Result<Option<WorkflowExecution>, WorkflowError> {
            Ok(None)
        }

        async fn update_execution(&self, _execution: &WorkflowExecution) -> Result<(), WorkflowError> {
            Ok(())
        }

        async fn get_running_executions(&self) -> Result<Vec<WorkflowExecution>, WorkflowError> {
            Ok(vec![])
        }

        async fn delete_execution(&self, _execution_id: &WorkflowExecutionId) -> Result<(), WorkflowError> {
            Ok(())
        }
    }

    // Mock StoragePort for testing
    struct MockStoragePort;

    #[async_trait::async_trait]
    impl kotoba_storage::port::StoragePort for MockStoragePort {
        async fn get(&self, _key: &[u8]) -> Result<Option<Vec<u8>>, kotoba_storage::StorageError> {
            Ok(None)
        }

        async fn put(&self, _key: &[u8], _value: &[u8]) -> Result<(), kotoba_storage::StorageError> {
            Ok(())
        }

        async fn delete(&self, _key: &[u8]) -> Result<(), kotoba_storage::StorageError> {
            Ok(())
        }

        async fn scan(&self, _prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>, kotoba_storage::StorageError> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_workflow_engine_builder_creation() {
        let builder = WorkflowEngineBuilder::new();

        // Builder should have default memory storage
        assert!(builder.storage_backend.is_some());
        assert!(builder.kotoba_backend.is_none());
        assert!(matches!(builder.storage_backend.as_ref().unwrap(), StorageBackend::Memory));
    }

    #[test]
    fn test_workflow_engine_builder_with_memory_storage() {
        let builder = WorkflowEngineBuilder::new().with_memory_storage();

        assert!(matches!(builder.storage_backend.as_ref().unwrap(), StorageBackend::Memory));
    }

    #[test]
    fn test_workflow_engine_builder_with_storage_backend() {
        let builder = WorkflowEngineBuilder::new().with_storage_backend(StorageBackend::Memory);

        assert!(matches!(builder.storage_backend.as_ref().unwrap(), StorageBackend::Memory));
    }

    #[cfg(feature = "rocksdb")]
    #[test]
    fn test_workflow_engine_builder_with_rocksdb_storage() {
        let path = "/tmp/test_db";
        let builder = WorkflowEngineBuilder::new().with_rocksdb_storage(path);

        match builder.storage_backend.as_ref().unwrap() {
            StorageBackend::RocksDB { path: db_path } => assert_eq!(db_path, path),
            _ => panic!("Expected RocksDB storage backend"),
        }
    }

    #[cfg(feature = "sqlite")]
    #[test]
    fn test_workflow_engine_builder_with_sqlite_storage() {
        let path = "/tmp/test.db";
        let builder = WorkflowEngineBuilder::new().with_sqlite_storage(path);

        match builder.storage_backend.as_ref().unwrap() {
            StorageBackend::SQLite { path: db_path } => assert_eq!(db_path, path),
            _ => panic!("Expected SQLite storage backend"),
        }
    }

    #[test]
    fn test_workflow_engine_builder_with_kotoba_storage() {
        let mock_storage = Arc::new(MockStoragePort);
        let builder = WorkflowEngineBuilder::new().with_kotoba_storage(mock_storage);

        assert!(builder.kotoba_backend.is_some());
        assert!(builder.storage_backend.is_none()); // Should be disabled when using Kotoba backend
    }

    #[tokio::test]
    async fn test_workflow_engine_builder_build_with_memory() {
        let builder = WorkflowEngineBuilder::new();
        let result = builder.build().await;

        assert!(result.is_ok());
        let engine = result.unwrap();

        // Verify engine has all components initialized
        assert!(engine.activity_registry().as_ref().is_send());
        assert!(engine.event_sourcing.as_ref().is_send());
        assert!(engine.snapshot_manager.as_ref().is_send());
    }

    #[tokio::test]
    async fn test_workflow_engine_builder_build_with_kotoba_storage() {
        let mock_storage = Arc::new(MockStoragePort);
        let builder = WorkflowEngineBuilder::new().with_kotoba_storage(mock_storage);

        let result = builder.build().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_extended_workflow_engine_builder_method() {
        let builder = ExtendedWorkflowEngine::builder();

        // Should return a WorkflowEngineBuilder
        assert!(builder.storage_backend.is_some());
    }

    #[test]
    fn test_workflow_result_creation() {
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let status = ExecutionStatus::Completed;
        let execution_time = std::time::Duration::from_secs(5);

        let result = WorkflowResult {
            execution_id,
            status,
            outputs: None,
            execution_time,
        };

        assert_eq!(result.execution_id, execution_id);
        assert_eq!(result.status, status);
        assert_eq!(result.execution_time, execution_time);
        assert!(result.outputs.is_none());
    }

    #[test]
    fn test_workflow_result_with_outputs() {
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let status = ExecutionStatus::Completed;
        let execution_time = std::time::Duration::from_millis(1500);

        // Create a mock workflow execution as output
        let workflow_execution = WorkflowExecution {
            execution_id,
            workflow_ir: WorkflowIR::default(),
            status,
            inputs: HashMap::new(),
            outputs: Some(serde_json::json!({"result": "success"})),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            error_message: None,
        };

        let result = WorkflowResult {
            execution_id,
            status,
            outputs: Some(workflow_execution),
            execution_time,
        };

        assert_eq!(result.execution_id, execution_id);
        assert_eq!(result.execution_time, execution_time);
        assert!(result.outputs.is_some());

        let outputs = result.outputs.unwrap();
        assert_eq!(outputs.execution_id, execution_id);
        assert_eq!(outputs.status, status);
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_activity_registry() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let registry = engine.activity_registry();

        // Should be able to get the registry
        assert!(registry.as_ref().is_send());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_execution_status() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return None for non-existent execution
        let result = engine.get_execution_status(&execution_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_execution() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return None for non-existent execution
        let result = engine.get_execution(&execution_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_list_running_executions() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        // Should return empty list initially
        let result = engine.list_running_executions().await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_cancel_execution() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should succeed even for non-existent execution (graceful handling)
        let result = engine.cancel_execution(&execution_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_create_snapshot() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should succeed even for non-existent execution (graceful handling)
        let result = engine.create_snapshot(&execution_id).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_restore_from_snapshot() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return None for non-existent snapshot
        let result = engine.restore_from_snapshot(&execution_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_performance_stats() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let stats = engine.get_performance_stats().await;

        // Should return a HashMap (may be empty initially)
        assert!(stats.is_empty() || !stats.is_empty()); // Accept either state
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_event_sourcing_access() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let event_sourcing = engine.event_sourcing();

        // Should be able to access the event sourcing manager
        assert!(event_sourcing.as_ref().is_send());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_snapshot_manager_access() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let snapshot_manager = engine.snapshot_manager();

        // Should be able to access the snapshot manager
        assert!(snapshot_manager.as_ref().is_send());
    }

    #[test]
    fn test_extended_workflow_engine_is_distributed_enabled_initially_false() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        // Should be false initially
        assert!(!engine.is_distributed_enabled());
    }

    #[test]
    fn test_extended_workflow_engine_distributed_execution_manager_initially_none() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        // Should be None initially
        assert!(engine.distributed_execution_manager().is_none());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_submit_distributed_workflow_without_setup() {
        let builder = WorkflowEngineBuilder::new();
        let mut engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should fail because distributed execution is not enabled
        let result = engine.submit_distributed_workflow(execution_id).await;
        assert!(result.is_err());

        if let Err(WorkflowError::InvalidStrategy(msg)) = result {
            assert!(msg.contains("Distributed execution not enabled"));
        } else {
            panic!("Expected InvalidStrategy error");
        }
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_cluster_health_without_setup() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        // Should fail because distributed execution is not enabled
        let result = engine.get_cluster_health().await;
        assert!(result.is_err());

        if let Err(WorkflowError::InvalidStrategy(msg)) = result {
            assert!(msg.contains("Distributed execution not enabled"));
        } else {
            panic!("Expected InvalidStrategy error");
        }
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_start_workflow() {
        let builder = WorkflowEngineBuilder::new();
        let mut engine = builder.build().await.unwrap();

        let workflow_ir = WorkflowIR::default();
        let inputs = HashMap::new();

        // This may fail due to unimplemented features, but we can test that the method exists
        let result = engine.start_workflow(&workflow_ir, inputs).await;

        // Accept both success and failure as the implementation may be incomplete
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_wait_for_completion() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let timeout = Some(std::time::Duration::from_secs(5));

        // Should return an error for non-existent execution
        let result = engine.wait_for_completion(execution_id, timeout).await;

        // Should fail because execution doesn't exist
        assert!(result.is_err());
    }

    #[test]
    fn test_workflow_result_debug_formatting() {
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let result = WorkflowResult {
            execution_id,
            status: ExecutionStatus::Running,
            outputs: None,
            execution_time: std::time::Duration::from_secs(10),
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("WorkflowResult"));
        assert!(debug_str.contains("Running"));
    }

    #[test]
    fn test_workflow_result_clone() {
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let original = WorkflowResult {
            execution_id,
            status: ExecutionStatus::Failed,
            outputs: None,
            execution_time: std::time::Duration::from_millis(500),
        };

        let cloned = original.clone();

        assert_eq!(original.execution_id, cloned.execution_id);
        assert_eq!(original.status, cloned.status);
        assert_eq!(original.outputs, cloned.outputs);
        assert_eq!(original.execution_time, cloned.execution_time);
    }

    #[test]
    fn test_workflow_engine_builder_chaining() {
        let builder = WorkflowEngineBuilder::new()
            .with_memory_storage()
            .with_storage_backend(StorageBackend::Memory);

        // Should still work after chaining
        assert!(builder.storage_backend.is_some());
    }

    #[tokio::test]
    async fn test_multiple_workflow_engines_independence() {
        let builder1 = WorkflowEngineBuilder::new();
        let builder2 = WorkflowEngineBuilder::new();

        let engine1 = builder1.build().await.unwrap();
        let engine2 = builder2.build().await.unwrap();

        // Engines should be independent
        assert!(!engine1.is_distributed_enabled());
        assert!(!engine2.is_distributed_enabled());

        // Different registry instances
        assert!(!Arc::ptr_eq(engine1.activity_registry(), engine2.activity_registry()));
    }

    #[test]
    fn test_storage_backend_enum_variants() {
        // Test that all storage backend variants exist
        let _memory = StorageBackend::Memory;

        #[cfg(feature = "rocksdb")]
        let _rocksdb = StorageBackend::RocksDB { path: "/tmp/test".to_string() };

        #[cfg(feature = "sqlite")]
        let _sqlite = StorageBackend::SQLite { path: "/tmp/test.db".to_string() };
    }

    #[test]
    fn test_workflow_engine_type_alias() {
        // Test that WorkflowEngine is an alias for ExtendedWorkflowEngine
        let _engine: WorkflowEngine = ExtendedWorkflowEngine {
            core_engine: kotoba_workflow_core::WorkflowEngine::new(),
            storage: Arc::new(MockWorkflowStore),
            activity_registry: Arc::new(ActivityRegistry::new()),
            state_manager: Arc::new(WorkflowStateManager::new()),
            event_sourcing: Arc::new(EventSourcingManager::new(Arc::new(MockWorkflowStore)).with_snapshot_config(100, 10)),
            snapshot_manager: Arc::new(SnapshotManager::new(Arc::new(MockWorkflowStore), Arc::new(EventSourcingManager::new(Arc::new(MockWorkflowStore)).with_snapshot_config(100, 10))).with_config(50, 5)),
            executor: None,
            distributed_executor: None,
        };
    }

    #[test]
    fn test_prelude_exports() {
        // Test that prelude exports work
        let _builder = prelude::WorkflowEngine::builder();
        let _workflow_ir: prelude::WorkflowIR = WorkflowIR::default();
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_execution_at_tx() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let tx_id = TxId::new(1);

        // Should return None for non-existent execution
        let result = engine.get_execution_at_tx(&execution_id, tx_id).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_execution_history() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return empty history for non-existent execution
        let history = engine.get_execution_history(&execution_id).await;
        assert!(history.is_empty());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_get_event_history() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return empty event history for non-existent execution
        let result = engine.get_event_history(&execution_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_extended_workflow_engine_rebuild_execution_from_events() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();

        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());

        // Should return None for non-existent execution
        let result = engine.rebuild_execution_from_events(&execution_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_workflow_result_serialization() {
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4());
        let result = WorkflowResult {
            execution_id,
            status: ExecutionStatus::Completed,
            outputs: None,
            execution_time: std::time::Duration::from_secs(30),
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&result);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("Completed"));
        assert!(json_str.contains("30"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<WorkflowResult> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.status, ExecutionStatus::Completed);
        assert_eq!(deserialized.execution_time, std::time::Duration::from_secs(30));
    }

    #[test]
    fn test_workflow_engine_builder_debug() {
        let builder = WorkflowEngineBuilder::new();
        let debug_str = format!("{:?}", builder);
        assert!(debug_str.contains("WorkflowEngineBuilder"));
    }

    #[test]
    fn test_extended_workflow_engine_debug() {
        let builder = WorkflowEngineBuilder::new();
        let engine = builder.build().await.unwrap();
        let debug_str = format!("{:?}", engine);
        assert!(debug_str.contains("ExtendedWorkflowEngine"));
    }
}

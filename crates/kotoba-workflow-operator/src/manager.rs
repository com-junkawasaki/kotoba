//! Workflow Manager for Kubernetes Operator
//!
//! Manages the lifecycle of workflows and executions in the Kubernetes cluster.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use kube::Client;

use crate::crds::{Workflow, WorkflowExecution, WorkflowPhase, ExecutionPhase};
use kotoba_workflow::{WorkflowEngine, WorkflowExecutionId};

/// Workflow Manager
pub struct WorkflowManager {
    client: Client,
    engines: RwLock<HashMap<String, WorkflowEngineInstance>>,
    workflow_cache: RwLock<HashMap<String, Workflow>>,
}

struct WorkflowEngineInstance {
    engine: Arc<WorkflowEngine>,
    status: EngineStatus,
    last_heartbeat: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq)]
enum EngineStatus {
    Starting,
    Running,
    Stopping,
    Failed,
}

impl WorkflowManager {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            engines: RwLock::new(HashMap::new()),
            workflow_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Deploy a workflow
    pub async fn deploy_workflow(&self, workflow: &Workflow) -> Result<(), Box<dyn std::error::Error>> {
        info!("Deploying workflow: {}", workflow.spec.name);

        let workflow_key = self.get_workflow_key(workflow);

        // Create workflow engine instance
        let engine = self.create_workflow_engine(workflow).await?;
        let instance = WorkflowEngineInstance {
            engine,
            status: EngineStatus::Starting,
            last_heartbeat: chrono::Utc::now(),
        };

        // Store in cache
        {
            let mut engines = self.engines.write().await;
            engines.insert(workflow_key.clone(), instance);
        }

        {
            let mut cache = self.workflow_cache.write().await;
            cache.insert(workflow_key, workflow.clone());
        }

        // Start the workflow engine
        self.start_workflow_engine(&workflow_key).await?;

        Ok(())
    }

    /// Update a deployed workflow
    pub async fn update_workflow(&self, workflow: &Workflow) -> Result<(), Box<dyn std::error::Error>> {
        info!("Updating workflow: {}", workflow.spec.name);

        let workflow_key = self.get_workflow_key(workflow);

        // Check if workflow is already deployed
        let mut engines = self.engines.write().await;
        if let Some(instance) = engines.get_mut(&workflow_key) {
            // Update workflow definition
            // TODO: Implement hot-reload of workflow definition
            warn!("Workflow hot-reload not yet implemented for: {}", workflow_key);
        } else {
            // Deploy if not exists
            drop(engines);
            self.deploy_workflow(workflow).await?;
        }

        Ok(())
    }

    /// Delete a deployed workflow
    pub async fn delete_workflow(&self, workflow: &Workflow) -> Result<(), Box<dyn std::error::Error>> {
        info!("Deleting workflow: {}", workflow.spec.name);

        let workflow_key = self.get_workflow_key(workflow);

        // Stop and remove workflow engine
        self.stop_workflow_engine(&workflow_key).await?;

        // Remove from cache
        {
            let mut engines = self.engines.write().await;
            engines.remove(&workflow_key);
        }

        {
            let mut cache = self.workflow_cache.write().await;
            cache.remove(&workflow_key);
        }

        Ok(())
    }

    /// Execute a workflow
    pub async fn execute_workflow(
        &self,
        execution: &WorkflowExecution,
    ) -> Result<WorkflowExecutionId, Box<dyn std::error::Error>> {
        info!("Executing workflow: {}", execution.spec.workflow_ref.name);

        let workflow_key = format!(
            "{}/{}",
            execution.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            execution.spec.workflow_ref.name
        );

        let engines = self.engines.read().await;
        let instance = engines.get(&workflow_key)
            .ok_or(format!("Workflow engine not found for: {}", workflow_key))?;

        if instance.status != EngineStatus::Running {
            return Err(format!("Workflow engine not ready: {:?}", instance.status).into());
        }

        // Convert execution inputs to workflow format
        let inputs = execution.spec.inputs.clone();

        // Start workflow execution
        let execution_id = instance.engine.start_workflow(
            &self.convert_workflow_definition(&execution.spec.workflow_ref.name).await?,
            inputs
        ).await?;

        Ok(execution_id)
    }

    /// Get workflow execution status
    pub async fn get_execution_status(
        &self,
        workflow_key: &str,
        execution_id: &WorkflowExecutionId,
    ) -> Result<Option<ExecutionPhase>, Box<dyn std::error::Error>> {
        let engines = self.engines.read().await;
        let instance = engines.get(workflow_key)
            .ok_or(format!("Workflow engine not found for: {}", workflow_key))?;

        let status = instance.engine.get_execution_status(execution_id).await?;
        let phase = status.map(|s| match s {
            kotoba_workflow::ExecutionStatus::Running => ExecutionPhase::Running,
            kotoba_workflow::ExecutionStatus::Completed => ExecutionPhase::Succeeded,
            kotoba_workflow::ExecutionStatus::Failed => ExecutionPhase::Failed,
            kotoba_workflow::ExecutionStatus::Cancelled => ExecutionPhase::Cancelled,
            kotoba_workflow::ExecutionStatus::TimedOut => ExecutionPhase::Failed,
            kotoba_workflow::ExecutionStatus::Compensating => ExecutionPhase::Running,
        });

        Ok(phase)
    }

    /// Cancel workflow execution
    pub async fn cancel_execution(
        &self,
        workflow_key: &str,
        execution_id: &WorkflowExecutionId,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let engines = self.engines.read().await;
        let instance = engines.get(workflow_key)
            .ok_or(format!("Workflow engine not found for: {}", workflow_key))?;

        instance.engine.cancel_execution(execution_id).await?;
        Ok(())
    }

    /// Get workflow statistics
    pub async fn get_workflow_stats(&self, workflow_key: &str) -> Result<WorkflowStats, Box<dyn std::error::Error>> {
        let engines = self.engines.read().await;
        let instance = engines.get(workflow_key)
            .ok_or(format!("Workflow engine not found for: {}", workflow_key))?;

        let stats = instance.engine.get_performance_stats().await;

        Ok(WorkflowStats {
            total_executions: stats.get("total_executions").copied().unwrap_or(0),
            active_executions: stats.get("active_executions").copied().unwrap_or(0),
            successful_executions: stats.get("successful_executions").copied().unwrap_or(0),
            failed_executions: stats.get("failed_executions").copied().unwrap_or(0),
        })
    }

    /// Health check for workflow engines
    pub async fn health_check(&self) -> Vec<EngineHealth> {
        let engines = self.engines.read().await;
        let mut results = Vec::new();

        for (key, instance) in engines.iter() {
            let health = EngineHealth {
                workflow_key: key.clone(),
                status: match instance.status {
                    EngineStatus::Running => HealthStatus::Healthy,
                    EngineStatus::Starting => HealthStatus::Starting,
                    EngineStatus::Stopping => HealthStatus::Stopping,
                    EngineStatus::Failed => HealthStatus::Unhealthy,
                },
                last_heartbeat: instance.last_heartbeat,
                active_executions: 0, // TODO: Get actual count
            };
            results.push(health);
        }

        results
    }

    /// Scale workflow
    pub async fn scale_workflow(
        &self,
        workflow_key: &str,
        replicas: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Scaling workflow {} to {} replicas", workflow_key, replicas);

        // TODO: Implement actual scaling logic
        // This would involve updating Kubernetes deployment replicas

        Ok(())
    }

    // Private helper methods

    fn get_workflow_key(&self, workflow: &Workflow) -> String {
        format!(
            "{}/{}",
            workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            workflow.spec.name
        )
    }

    async fn create_workflow_engine(&self, workflow: &Workflow) -> Result<Arc<WorkflowEngine>, Box<dyn std::error::Error>> {
        // TODO: Create workflow engine with proper configuration
        // For now, create a basic engine
        let engine = WorkflowEngine::builder()
            .with_memory_storage()
            .build()
            .await?;

        Ok(Arc::new(engine))
    }

    async fn start_workflow_engine(&self, workflow_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut engines = self.engines.write().await;
        if let Some(instance) = engines.get_mut(workflow_key) {
            instance.status = EngineStatus::Running;
            instance.last_heartbeat = chrono::Utc::now();
        }
        Ok(())
    }

    async fn stop_workflow_engine(&self, workflow_key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut engines = self.engines.write().await;
        if let Some(instance) = engines.get_mut(workflow_key) {
            instance.status = EngineStatus::Stopping;
            // TODO: Gracefully shutdown the engine
        }
        Ok(())
    }

    async fn convert_workflow_definition(&self, workflow_name: &str) -> Result<kotoba_workflow::WorkflowIR, Box<dyn std::error::Error>> {
        // TODO: Convert Kubernetes workflow definition to Kotoba WorkflowIR
        // For now, return a placeholder
        Err("Workflow definition conversion not implemented".into())
    }
}

/// Workflow statistics
#[derive(Debug, Clone)]
pub struct WorkflowStats {
    pub total_executions: u64,
    pub active_executions: u64,
    pub successful_executions: u64,
    pub failed_executions: u64,
}

/// Engine health information
#[derive(Debug, Clone)]
pub struct EngineHealth {
    pub workflow_key: String,
    pub status: HealthStatus,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub active_executions: u64,
}

/// Health status
#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Starting,
    Stopping,
    Unhealthy,
}

/// Workflow Manager Error
#[derive(Debug, thiserror::Error)]
pub enum ManagerError {
    #[error("Workflow engine error: {0}")]
    EngineError(#[from] kotoba_workflow::WorkflowError),
    #[error("Kubernetes error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Execution not found: {0}")]
    ExecutionNotFound(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

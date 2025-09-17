//! Reconcilers for Workflow Custom Resources
//!
//! Handles the reconciliation logic for Workflow and WorkflowExecution resources.

use std::sync::Arc;
use kube::runtime::controller::Action;
use tokio::time::Duration;
use tracing::{info, error, warn};

use crate::crds::{Workflow, WorkflowExecution, WorkflowPhase, ExecutionPhase, WorkflowCondition, WorkflowConditionType, ConditionStatus, ExecutionCondition, ExecutionConditionType};
use crate::manager::WorkflowManager;

/// Workflow Reconciler
pub struct WorkflowReconciler {
    manager: Arc<WorkflowManager>,
}

impl WorkflowReconciler {
    pub fn new(manager: Arc<WorkflowManager>) -> Self {
        Self { manager }
    }

    /// Reconcile workflow resource
    pub async fn reconcile(&self, workflow: Arc<Workflow>) -> Result<Action, Box<dyn std::error::Error>> {
        info!("Reconciling workflow: {}", workflow.spec.name);

        let name = workflow.metadata.name.as_ref().ok_or("Workflow name not found")?;
        let namespace = workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string());

        // Check current status
        let current_phase = workflow.status.as_ref().map(|s| s.phase.clone()).unwrap_or(WorkflowPhase::Pending);

        match current_phase {
            WorkflowPhase::Pending => {
                // Deploy the workflow
                match self.manager.deploy_workflow(&workflow).await {
                    Ok(_) => {
                        info!("Successfully deployed workflow: {}", name);
                        Ok(Action::requeue(Duration::from_secs(5)))
                    }
                    Err(e) => {
                        error!("Failed to deploy workflow {}: {}", name, e);
                        Ok(Action::requeue(Duration::from_secs(30)))
                    }
                }
            }
            WorkflowPhase::Running => {
                // Check if workflow is still healthy
                let health = self.manager.health_check().await;
                let workflow_key = format!("{}/{}", namespace, workflow.spec.name);

                if let Some(engine_health) = health.iter().find(|h| h.workflow_key == workflow_key) {
                    match engine_health.status {
                        crate::manager::HealthStatus::Healthy => {
                            // Workflow is healthy, continue monitoring
                            Ok(Action::requeue(Duration::from_secs(30)))
                        }
                        crate::manager::HealthStatus::Unhealthy => {
                            warn!("Workflow {} is unhealthy", name);
                            Ok(Action::requeue(Duration::from_secs(10)))
                        }
                        _ => {
                            // Starting or stopping, check again soon
                            Ok(Action::requeue(Duration::from_secs(5)))
                        }
                    }
                } else {
                    warn!("No health information found for workflow: {}", name);
                    Ok(Action::requeue(Duration::from_secs(10)))
                }
            }
            WorkflowPhase::Terminating => {
                // Workflow is being deleted
                match self.manager.delete_workflow(&workflow).await {
                    Ok(_) => {
                        info!("Successfully deleted workflow: {}", name);
                        Ok(Action::await_change())
                    }
                    Err(e) => {
                        error!("Failed to delete workflow {}: {}", name, e);
                        Ok(Action::requeue(Duration::from_secs(30)))
                    }
                }
            }
            _ => {
                // Other phases don't require action
                Ok(Action::requeue(Duration::from_secs(60)))
            }
        }
    }
}

/// WorkflowExecution Reconciler
pub struct WorkflowExecutionReconciler {
    manager: Arc<WorkflowManager>,
}

impl WorkflowExecutionReconciler {
    pub fn new(manager: Arc<WorkflowManager>) -> Self {
        Self { manager }
    }

    /// Reconcile workflow execution resource
    pub async fn reconcile(&self, execution: Arc<WorkflowExecution>) -> Result<Action, Box<dyn std::error::Error>> {
        info!("Reconciling workflow execution: {}", execution.metadata.name.as_ref().unwrap_or(&"unknown".to_string()));

        let name = execution.metadata.name.as_ref().ok_or("Execution name not found")?;
        let namespace = execution.metadata.namespace.as_ref().unwrap_or(&"default".to_string());

        // Check current status
        let current_phase = execution.status.as_ref().map(|s| s.phase.clone()).unwrap_or(ExecutionPhase::Pending);

        match current_phase {
            ExecutionPhase::Pending => {
                // Start the workflow execution
                let workflow_key = format!("{}/{}", namespace, execution.spec.workflow_ref.name);

                match self.manager.execute_workflow(&execution).await {
                    Ok(execution_id) => {
                        info!("Successfully started execution {} for workflow {}", name, workflow_key);
                        // TODO: Store execution_id in status
                        Ok(Action::requeue(Duration::from_secs(5)))
                    }
                    Err(e) => {
                        error!("Failed to start execution {}: {}", name, e);
                        Ok(Action::requeue(Duration::from_secs(30)))
                    }
                }
            }
            ExecutionPhase::Running => {
                // Check execution status
                let workflow_key = format!("{}/{}", namespace, execution.spec.workflow_ref.name);

                if let Some(execution_id_str) = execution.status.as_ref().and_then(|s| s.execution_id.as_ref()) {
                    if let Ok(execution_id) = execution_id_str.parse::<uuid::Uuid>() {
                        let exec_id = kotoba_workflow::WorkflowExecutionId(execution_id.to_string());

                        match self.manager.get_execution_status(&workflow_key, &exec_id).await {
                            Ok(Some(phase)) => {
                                match phase {
                                    ExecutionPhase::Succeeded => {
                                        info!("Execution {} completed successfully", name);
                                        Ok(Action::await_change())
                                    }
                                    ExecutionPhase::Failed => {
                                        warn!("Execution {} failed", name);
                                        Ok(Action::await_change())
                                    }
                                    ExecutionPhase::Running => {
                                        // Still running, continue monitoring
                                        Ok(Action::requeue(Duration::from_secs(10)))
                                    }
                                    _ => {
                                        Ok(Action::requeue(Duration::from_secs(10)))
                                    }
                                }
                            }
                            Ok(None) => {
                                warn!("Execution {} not found", name);
                                Ok(Action::await_change())
                            }
                            Err(e) => {
                                error!("Failed to get execution status for {}: {}", name, e);
                                Ok(Action::requeue(Duration::from_secs(30)))
                            }
                        }
                    } else {
                        error!("Invalid execution ID format: {}", execution_id_str);
                        Ok(Action::requeue(Duration::from_secs(30)))
                    }
                } else {
                    error!("No execution ID found for execution: {}", name);
                    Ok(Action::requeue(Duration::from_secs(30)))
                }
            }
            ExecutionPhase::Succeeded | ExecutionPhase::Failed | ExecutionPhase::Cancelled => {
                // Execution is complete, no further action needed
                Ok(Action::await_change())
            }
        }
    }
}

/// Common reconciliation utilities
pub struct ReconciliationUtils;

impl ReconciliationUtils {
    /// Create workflow condition
    pub fn create_workflow_condition(
        condition_type: WorkflowConditionType,
        status: ConditionStatus,
        message: Option<String>,
        reason: Option<String>,
    ) -> WorkflowCondition {
        WorkflowCondition {
            type_: condition_type,
            status,
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            message,
            reason,
        }
    }

    /// Create execution condition
    pub fn create_execution_condition(
        condition_type: ExecutionConditionType,
        status: ConditionStatus,
        message: Option<String>,
        reason: Option<String>,
    ) -> ExecutionCondition {
        ExecutionCondition {
            type_: condition_type,
            status,
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            message,
            reason,
        }
    }

    /// Check if workflow needs scaling
    pub fn should_scale_workflow(
        current_replicas: u32,
        target_cpu: Option<u32>,
        target_memory: Option<u32>,
        current_cpu: f64,
        current_memory: f64,
    ) -> Option<u32> {
        let cpu_threshold = target_cpu.unwrap_or(70) as f64;
        let memory_threshold = target_memory.unwrap_or(80) as f64;

        if current_cpu > cpu_threshold || current_memory > memory_threshold {
            // Scale up
            Some((current_replicas as f64 * 1.5).ceil() as u32)
        } else if current_cpu < cpu_threshold * 0.5 && current_memory < memory_threshold * 0.5 && current_replicas > 1 {
            // Scale down
            Some((current_replicas as f64 * 0.75).floor() as u32)
        } else {
            None
        }
    }

    /// Calculate backoff duration for retries
    pub fn calculate_backoff(attempt: u32, initial_delay: Duration, max_delay: Duration) -> Duration {
        let delay = initial_delay * 2_u32.pow(attempt.saturating_sub(1));
        std::cmp::min(delay, max_delay)
    }

    /// Check if resource is expired based on timeout
    pub fn is_expired(start_time: &str, timeout_seconds: Option<u32>) -> bool {
        if let Some(timeout) = timeout_seconds {
            if let Ok(start) = chrono::DateTime::parse_from_rfc3339(start_time) {
                let now = chrono::Utc::now();
                let elapsed = now.signed_duration_since(start);
                elapsed.num_seconds() > timeout as i64
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Validate workflow specification
    pub fn validate_workflow_spec(workflow: &Workflow) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if workflow.spec.name.is_empty() {
            errors.push("Workflow name cannot be empty".to_string());
        }

        if workflow.spec.version.is_empty() {
            errors.push("Workflow version cannot be empty".to_string());
        }

        // Validate scaling configuration
        if let Some(min_replicas) = workflow.spec.scaling.min_replicas {
            if let Some(max_replicas) = workflow.spec.scaling.max_replicas {
                if min_replicas > max_replicas {
                    errors.push("Minimum replicas cannot be greater than maximum replicas".to_string());
                }
            }
        }

        // Validate resource requirements
        if let Some(requests) = &workflow.spec.resources.requests {
            if let Some(limits) = &workflow.spec.resources.limits {
                // TODO: Add resource validation logic
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Validate execution specification
    pub fn validate_execution_spec(execution: &WorkflowExecution) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if execution.spec.workflow_ref.name.is_empty() {
            errors.push("Workflow reference name cannot be empty".to_string());
        }

        // Validate timeout
        if let Some(timeout) = &execution.spec.timeout {
            if timeout.is_empty() {
                errors.push("Timeout cannot be empty if specified".to_string());
            }
        }

        // Validate inputs
        for (key, value) in &execution.spec.inputs {
            if key.is_empty() {
                errors.push("Input parameter name cannot be empty".to_string());
                break;
            }
            // TODO: Add more input validation based on workflow definition
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Reconciliation Error
#[derive(Debug, thiserror::Error)]
pub enum ReconciliationError {
    #[error("Validation error: {0:?}")]
    ValidationError(Vec<String>),
    #[error("Workflow manager error: {0}")]
    ManagerError(#[from] Box<dyn std::error::Error>),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Resource conflict")]
    ResourceConflict,
    #[error("Invalid state transition")]
    InvalidStateTransition,
}

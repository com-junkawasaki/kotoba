//! Kubernetes Controller for Workflow Resources
//!
//! Implements the reconciliation logic for workflow custom resources.

use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, error, warn};
use kube::{
    api::{Api, ListParams, Patch, PatchParams},
    client::Client,
    runtime::{controller::Action, Controller},
};

use crate::crds::{Workflow, WorkflowExecution, WorkflowStatus, WorkflowPhase, WorkflowCondition, WorkflowConditionType, ConditionStatus, ExecutionPhase};
use crate::manager::WorkflowManager;
use crate::reconciler::{WorkflowReconciler, WorkflowExecutionReconciler};

/// Workflow Controller
pub struct WorkflowController {
    client: Client,
    workflow_api: Api<Workflow>,
    execution_api: Api<WorkflowExecution>,
    manager: Arc<WorkflowManager>,
}

impl WorkflowController {
    pub fn new(client: Client, manager: Arc<WorkflowManager>) -> Self {
        let workflow_api = Api::all(client.clone());
        let execution_api = Api::all(client.clone());

        Self {
            client,
            workflow_api,
            execution_api,
            manager,
        }
    }

    /// Run the controller
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting workflow controller");

        let workflow_controller = Controller::new(self.workflow_api.clone(), ListParams::default())
            .owns(self.execution_api.clone(), ListParams::default())
            .run(
                WorkflowReconciler::new(self.manager.clone()),
                |obj| obj.status.as_ref().map(|s| s.phase.to_string()),
                |_ctx| Action::requeue(Duration::from_secs(30)),
            )
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("Reconciled workflow: {:?}", o),
                    Err(e) => error!("Reconcile failed: {}", e),
                }
            });

        let execution_controller = Controller::new(self.execution_api.clone(), ListParams::default())
            .run(
                WorkflowExecutionReconciler::new(self.manager.clone()),
                |obj| obj.status.as_ref().map(|s| s.phase.to_string()),
                |_ctx| Action::requeue(Duration::from_secs(10)),
            )
            .for_each(|res| async move {
                match res {
                    Ok(o) => info!("Reconciled execution: {:?}", o),
                    Err(e) => error!("Execution reconcile failed: {}", e),
                }
            });

        // Run both controllers concurrently
        tokio::try_join!(workflow_controller, execution_controller)?;

        Ok(())
    }

    /// Create workflow deployment
    pub async fn create_workflow_deployment(
        &self,
        workflow: &Workflow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating deployment for workflow: {}", workflow.spec.name);

        // Create Kubernetes deployment for workflow engine
        let deployment = self.create_deployment_spec(workflow).await?;
        let deployment_api: Api<k8s_openapi::api::apps::v1::Deployment> = Api::namespaced(
            self.client.clone(),
            &workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        deployment_api.patch(
            &format!("{}-engine", workflow.spec.name),
            &PatchParams::apply("kotoba-workflow-operator"),
            &Patch::Apply(deployment),
        ).await?;

        Ok(())
    }

    /// Update workflow status
    pub async fn update_workflow_status(
        &self,
        workflow: &Workflow,
        phase: WorkflowPhase,
        message: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let name = &workflow.metadata.name.as_ref().ok_or("Workflow name not found")?;
        let namespace = &workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string());

        let mut status = workflow.status.clone().unwrap_or_default();
        status.phase = phase;
        status.message = message;
        status.observed_generation = workflow.metadata.generation;

        // Update conditions
        let condition = WorkflowCondition {
            type_: match phase {
                WorkflowPhase::Running => WorkflowConditionType::Executing,
                WorkflowPhase::Succeeded => WorkflowConditionType::Complete,
                WorkflowPhase::Failed => WorkflowConditionType::Failed,
                _ => WorkflowConditionType::Initialized,
            },
            status: match phase {
                WorkflowPhase::Failed => ConditionStatus::True,
                _ => ConditionStatus::False,
            },
            last_transition_time: Some(chrono::Utc::now().to_rfc3339()),
            message: status.message.clone(),
            reason: Some(format!("{:?}", phase)),
        };

        status.conditions.push(condition);

        let patch = serde_json::json!({
            "status": status
        });

        self.workflow_api.patch_status(
            name,
            &PatchParams::apply("kotoba-workflow-operator"),
            &Patch::Merge(&patch),
        ).await?;

        Ok(())
    }

    /// Create Kubernetes deployment specification
    async fn create_deployment_spec(
        &self,
        workflow: &Workflow,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let spec = &workflow.spec;

        // Create deployment spec based on workflow requirements
        let deployment_spec = serde_json::json!({
            "apiVersion": "apps/v1",
            "kind": "Deployment",
            "metadata": {
                "name": format!("{}-engine", spec.name),
                "labels": {
                    "app": "kotoba-workflow",
                    "workflow": spec.name,
                }
            },
            "spec": {
                "replicas": spec.scaling.min_replicas.unwrap_or(1),
                "selector": {
                    "matchLabels": {
                        "app": "kotoba-workflow",
                        "workflow": spec.name,
                    }
                },
                "template": {
                    "metadata": {
                        "labels": {
                            "app": "kotoba-workflow",
                            "workflow": spec.name,
                        }
                    },
                    "spec": {
                        "containers": [{
                            "name": "workflow-engine",
                            "image": "kotoba/workflow-engine:latest", // TODO: Use configurable image
                            "env": spec.env.iter().map(|env| {
                                serde_json::json!({
                                    "name": env.name,
                                    "value": env.value
                                })
                            }).collect::<Vec<_>>(),
                            "resources": {
                                "requests": {
                                    "cpu": spec.resources.requests.as_ref().and_then(|r| r.cpu.as_ref()).unwrap_or("100m"),
                                    "memory": spec.resources.requests.as_ref().and_then(|r| r.memory.as_ref()).unwrap_or("128Mi")
                                },
                                "limits": {
                                    "cpu": spec.resources.limits.as_ref().and_then(|r| r.cpu.as_ref()).unwrap_or("500m"),
                                    "memory": spec.resources.limits.as_ref().and_then(|r| r.memory.as_ref()).unwrap_or("512Mi")
                                }
                            },
                            "ports": [{
                                "containerPort": 8080,
                                "name": "http"
                            }]
                        }],
                        "serviceAccountName": spec.config.monitoring.metrics_endpoint.as_ref().unwrap_or(&"default".to_string())
                    }
                }
            }
        });

        Ok(deployment_spec)
    }

    /// Create workflow service
    pub async fn create_workflow_service(
        &self,
        workflow: &Workflow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating service for workflow: {}", workflow.spec.name);

        let service_spec = serde_json::json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": {
                "name": format!("{}-service", workflow.spec.name),
                "labels": {
                    "app": "kotoba-workflow",
                    "workflow": workflow.spec.name,
                }
            },
            "spec": {
                "selector": {
                    "app": "kotoba-workflow",
                    "workflow": workflow.spec.name,
                },
                "ports": [{
                    "port": 80,
                    "targetPort": 8080,
                    "protocol": "TCP",
                    "name": "http"
                }],
                "type": "ClusterIP"
            }
        });

        let service_api: Api<k8s_openapi::api::core::v1::Service> = Api::namespaced(
            self.client.clone(),
            &workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        service_api.patch(
            &format!("{}-service", workflow.spec.name),
            &PatchParams::apply("kotoba-workflow-operator"),
            &Patch::Apply(service_spec),
        ).await?;

        Ok(())
    }

    /// Scale workflow deployment
    pub async fn scale_workflow(
        &self,
        workflow: &Workflow,
        replicas: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Scaling workflow {} to {} replicas", workflow.spec.name, replicas);

        let scale_spec = serde_json::json!({
            "spec": {
                "replicas": replicas
            }
        });

        let scale_api: Api<k8s_openapi::api::apps::v1::Scale> = Api::namespaced(
            self.client.clone(),
            &workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        scale_api.patch(
            &format!("{}-engine", workflow.spec.name),
            &PatchParams::apply("kotoba-workflow-operator"),
            &Patch::Merge(&scale_spec),
        ).await?;

        Ok(())
    }

    /// Delete workflow resources
    pub async fn delete_workflow_resources(
        &self,
        workflow: &Workflow,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Deleting resources for workflow: {}", workflow.spec.name);

        let namespace = &workflow.metadata.namespace.as_ref().unwrap_or(&"default".to_string());

        // Delete deployment
        let deployment_api: Api<k8s_openapi::api::apps::v1::Deployment> = Api::namespaced(
            self.client.clone(),
            namespace,
        );
        deployment_api.delete(&format!("{}-engine", workflow.spec.name), &Default::default()).await.ok();

        // Delete service
        let service_api: Api<k8s_openapi::api::core::v1::Service> = Api::namespaced(
            self.client.clone(),
            namespace,
        );
        service_api.delete(&format!("{}-service", workflow.spec.name), &Default::default()).await.ok();

        Ok(())
    }
}

/// Error types for the controller
#[derive(Debug, thiserror::Error)]
pub enum ControllerError {
    #[error("Kubernetes API error: {0}")]
    KubeError(#[from] kube::Error),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Workflow error: {0}")]
    WorkflowError(String),
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
}

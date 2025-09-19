//! Custom Resource Definitions for Kotoba Workflow Operator
//!
//! Defines the Kubernetes custom resources for workflow management.

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Workflow custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kotoba.io",
    version = "v1",
    kind = "Workflow",
    plural = "workflows",
    derive = "Default"
)]
#[kube(status = "WorkflowStatus")]
#[serde(rename_all = "camelCase")]
pub struct WorkflowSpec {
    /// Workflow name
    pub name: String,
    /// Workflow description
    #[serde(default)]
    pub description: Option<String>,
    /// Workflow version
    pub version: String,
    /// Workflow definition in JSON format
    pub definition: serde_json::Value,
    /// Workflow configuration
    #[serde(default)]
    pub config: WorkflowConfig,
    /// Environment variables
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// Resource requirements
    #[serde(default)]
    pub resources: ResourceRequirements,
    /// Scaling configuration
    #[serde(default)]
    pub scaling: ScalingConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowConfig {
    /// Timeout in seconds
    #[serde(default)]
    pub timeout: Option<u32>,
    /// Retry policy
    #[serde(default)]
    pub retry_policy: Option<RetryPolicy>,
    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RetryPolicy {
    /// Maximum number of attempts
    pub max_attempts: u32,
    /// Initial delay in seconds
    pub initial_delay: u32,
    /// Backoff multiplier
    pub backoff_multiplier: f64,
    /// Maximum delay in seconds
    pub max_delay: u32,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringConfig {
    /// Enable metrics collection
    #[serde(default)]
    pub enable_metrics: bool,
    /// Enable tracing
    #[serde(default)]
    pub enable_tracing: bool,
    /// Metrics endpoint
    pub metrics_endpoint: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvVar {
    /// Environment variable name
    pub name: String,
    /// Environment variable value
    pub value: Option<String>,
    /// Source of the environment variable's value
    pub value_from: Option<EnvVarSource>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarSource {
    /// Selects a key of a ConfigMap
    pub config_map_key_ref: Option<ConfigMapKeySelector>,
    /// Selects a key of a Secret
    pub secret_key_ref: Option<SecretKeySelector>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ConfigMapKeySelector {
    /// The name of the ConfigMap
    pub name: String,
    /// The key to select
    pub key: String,
    /// Specify whether the ConfigMap or its key must be defined
    #[serde(default)]
    pub optional: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SecretKeySelector {
    /// The name of the Secret
    pub name: String,
    /// The key of the secret to select from
    pub key: String,
    /// Specify whether the Secret or its key must be defined
    #[serde(default)]
    pub optional: Option<bool>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    /// Minimum amount of compute resources required
    pub requests: Option<ResourceList>,
    /// Maximum amount of compute resources allowed
    pub limits: Option<ResourceList>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceList {
    /// CPU requirement
    pub cpu: Option<String>,
    /// Memory requirement
    pub memory: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScalingConfig {
    /// Minimum number of replicas
    #[serde(default)]
    pub min_replicas: Option<u32>,
    /// Maximum number of replicas
    pub max_replicas: Option<u32>,
    /// Target CPU utilization percentage
    pub target_cpu_utilization_percentage: Option<u32>,
    /// Target memory utilization percentage
    pub target_memory_utilization_percentage: Option<u32>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowStatus {
    /// Current phase of the workflow
    pub phase: WorkflowPhase,
    /// Human-readable message indicating details about why the workflow is in this phase
    #[serde(default)]
    pub message: Option<String>,
    /// Conditions represent the latest available observations of the workflow's state
    #[serde(default)]
    pub conditions: Vec<WorkflowCondition>,
    /// Start time of the workflow
    pub start_time: Option<String>,
    /// Completion time of the workflow
    pub completion_time: Option<String>,
    /// Number of active executions
    #[serde(default)]
    pub active_executions: u32,
    /// Number of failed executions
    #[serde(default)]
    pub failed_executions: u32,
    /// Number of successful executions
    #[serde(default)]
    pub successful_executions: u32,
    /// Observed generation
    pub observed_generation: Option<i64>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum WorkflowPhase {
    /// Workflow is being created
    Pending,
    /// Workflow is running
    Running,
    /// Workflow has completed successfully
    Succeeded,
    /// Workflow has failed
    Failed,
    /// Workflow is being deleted
    Terminating,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowCondition {
    /// Type of workflow condition
    pub type_: WorkflowConditionType,
    /// Status of the condition
    pub status: ConditionStatus,
    /// Last time the condition transitioned from one status to another
    pub last_transition_time: Option<String>,
    /// Human-readable message indicating details about last transition
    pub message: Option<String>,
    /// Unique, one-word, CamelCase reason for the condition's last transition
    pub reason: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum WorkflowConditionType {
    /// Workflow has been initialized
    Initialized,
    /// Workflow is being executed
    Executing,
    /// Workflow execution completed
    Complete,
    /// Workflow failed
    Failed,
    /// Workflow is being scaled
    Scaling,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ConditionStatus {
    /// Condition is true
    True,
    /// Condition is false
    False,
    /// Condition status is unknown
    Unknown,
}

/// WorkflowExecution custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kotoba.io",
    version = "v1",
    kind = "WorkflowExecution",
    plural = "workflowexecutions",
    derive = "Default"
)]
#[kube(status = "WorkflowExecutionStatus")]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionSpec {
    /// Reference to the workflow to execute
    pub workflow_ref: WorkflowReference,
    /// Execution inputs
    #[serde(default)]
    pub inputs: HashMap<String, serde_json::Value>,
    /// Execution configuration
    #[serde(default)]
    pub config: ExecutionConfig,
    /// Execution timeout
    pub timeout: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowReference {
    /// Name of the workflow
    pub name: String,
    /// Namespace of the workflow
    pub namespace: Option<String>,
    /// Version of the workflow to execute
    pub version: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionConfig {
    /// Priority of the execution
    #[serde(default)]
    pub priority: ExecutionPriority,
    /// Labels for the execution
    #[serde(default)]
    pub labels: HashMap<String, String>,
    /// Annotations for the execution
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ExecutionPriority {
    /// Low priority
    Low,
    /// Normal priority (default)
    Normal,
    /// High priority
    High,
    /// Critical priority
    Critical,
}

impl Default for ExecutionPriority {
    fn default() -> Self {
        ExecutionPriority::Normal
    }
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowExecutionStatus {
    /// Current phase of the execution
    pub phase: ExecutionPhase,
    /// Human-readable message indicating details about why the execution is in this phase
    #[serde(default)]
    pub message: Option<String>,
    /// Start time of the execution
    pub start_time: Option<String>,
    /// Completion time of the execution
    pub completion_time: Option<String>,
    /// Execution ID assigned by the workflow engine
    pub execution_id: Option<String>,
    /// Execution results
    pub results: Option<serde_json::Value>,
    /// Conditions represent the latest available observations of the execution's state
    #[serde(default)]
    pub conditions: Vec<ExecutionCondition>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ExecutionPhase {
    /// Execution is pending
    Pending,
    /// Execution is running
    Running,
    /// Execution has completed successfully
    Succeeded,
    /// Execution has failed
    Failed,
    /// Execution has been cancelled
    Cancelled,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionCondition {
    /// Type of execution condition
    pub type_: ExecutionConditionType,
    /// Status of the condition
    pub status: ConditionStatus,
    /// Last time the condition transitioned from one status to another
    pub last_transition_time: Option<String>,
    /// Human-readable message indicating details about last transition
    pub message: Option<String>,
    /// Unique, one-word, CamelCase reason for the condition's last transition
    pub reason: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, PartialEq)]
pub enum ExecutionConditionType {
    /// Execution has been scheduled
    Scheduled,
    /// Execution is running
    Running,
    /// Execution completed
    Complete,
    /// Execution failed
    Failed,
}

/// WorkflowTemplate custom resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kotoba.io",
    version = "v1",
    kind = "WorkflowTemplate",
    plural = "workflowtemplates",
    derive = "Default"
)]
pub struct WorkflowTemplateSpec {
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Template parameters
    #[serde(default)]
    pub parameters: Vec<TemplateParameter>,
    /// Template definition
    pub template: serde_json::Value,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateMetadata {
    /// Template name
    pub name: String,
    /// Template description
    pub description: Option<String>,
    /// Template version
    pub version: String,
    /// Template category
    pub category: String,
    /// Template tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Template author
    pub author: Option<String>,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateParameter {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub type_: String,
    /// Parameter description
    pub description: Option<String>,
    /// Default value
    pub default: Option<serde_json::Value>,
    /// Whether the parameter is required
    #[serde(default)]
    pub required: bool,
}

/// WorkflowCluster custom resource for cluster-wide configuration
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "kotoba.io",
    version = "v1",
    kind = "WorkflowCluster",
    plural = "workflowclusters",
    derive = "Default"
)]
pub struct WorkflowClusterSpec {
    /// Cluster configuration
    pub config: ClusterConfig,
    /// Default resource requirements
    #[serde(default)]
    pub default_resources: ResourceRequirements,
    /// Cluster scaling configuration
    #[serde(default)]
    pub scaling: ClusterScalingConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClusterConfig {
    /// Workflow engine image
    pub image: String,
    /// Image pull policy
    #[serde(default)]
    pub image_pull_policy: String,
    /// Service account for workflows
    pub service_account: Option<String>,
    /// Cluster domain
    pub cluster_domain: Option<String>,
    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringConfig,
}

#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ClusterScalingConfig {
    /// Minimum number of workflow engines
    #[serde(default)]
    pub min_engines: u32,
    /// Maximum number of workflow engines
    pub max_engines: Option<u32>,
    /// Target CPU utilization for scaling
    pub target_cpu_utilization: Option<u32>,
    /// Target memory utilization for scaling
    pub target_memory_utilization: Option<u32>,
}

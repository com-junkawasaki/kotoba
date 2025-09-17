//! # Kotoba Workflow Operator
//!
//! Kubernetes operator for managing Kotoba Workflow Engine deployments and executions.
//!
//! ## Features
//!
//! - **Automated Deployment**: Automatically deploy workflow engines when Workflow resources are created
//! - **Lifecycle Management**: Handle workflow scaling, updates, and deletion
//! - **Execution Management**: Manage workflow executions with proper resource allocation
//! - **Monitoring Integration**: Integrate with Kubernetes monitoring and logging
//! - **High Availability**: Support for multi-zone deployments and failover
//!
//! ## Custom Resources
//!
//! ### Workflow
//! Defines a workflow and its execution environment:
//! ```yaml
//! apiVersion: kotoba.io/v1
//! kind: Workflow
//! metadata:
//!   name: order-processing
//! spec:
//!   name: order-processing
//!   version: "1.0.0"
//!   definition: {...}
//!   resources:
//!     requests:
//!       cpu: "100m"
//!       memory: "128Mi"
//!     limits:
//!       cpu: "500m"
//!       memory: "512Mi"
//!   scaling:
//!     minReplicas: 1
//!     maxReplicas: 10
//! ```
//!
//! ### WorkflowExecution
//! Triggers execution of a workflow:
//! ```yaml
//! apiVersion: kotoba.io/v1
//! kind: WorkflowExecution
//! metadata:
//!   name: order-123-execution
//! spec:
//!   workflowRef:
//!     name: order-processing
//!     version: "1.0.0"
//!   inputs:
//!     orderId: "123"
//!     customerId: "456"
//!   timeout: "30m"
//! ```
//!
//! ## Usage
//!
//! 1. Install the operator in your Kubernetes cluster
//! 2. Create Workflow custom resources
//! 3. Create WorkflowExecution resources to trigger executions
//! 4. Monitor executions through Kubernetes events and logs

pub mod controller;
pub mod crds;
pub mod manager;
pub mod reconciler;

// Re-export main components
pub use controller::WorkflowController;
pub use crds::*;
pub use manager::WorkflowManager;
pub use reconciler::{WorkflowReconciler, WorkflowExecutionReconciler, ReconciliationUtils};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Operator configuration
#[derive(Debug, Clone)]
pub struct OperatorConfig {
    /// Kubernetes namespace to watch
    pub namespace: String,
    /// Leader election configuration
    pub leader_election: LeaderElectionConfig,
    /// Reconciliation intervals
    pub reconciliation: ReconciliationConfig,
    /// Resource limits
    pub resources: ResourceConfig,
}

#[derive(Debug, Clone)]
pub struct LeaderElectionConfig {
    /// Enable leader election
    pub enabled: bool,
    /// Lease duration in seconds
    pub lease_duration: u32,
    /// Renew deadline in seconds
    pub renew_deadline: u32,
    /// Retry period in seconds
    pub retry_period: u32,
}

#[derive(Debug, Clone)]
pub struct ReconciliationConfig {
    /// Workflow reconciliation interval
    pub workflow_interval: std::time::Duration,
    /// Execution reconciliation interval
    pub execution_interval: std::time::Duration,
    /// Health check interval
    pub health_check_interval: std::time::Duration,
}

#[derive(Debug, Clone)]
pub struct ResourceConfig {
    /// Maximum workflows per node
    pub max_workflows_per_node: u32,
    /// CPU overhead per workflow
    pub cpu_overhead_per_workflow: String,
    /// Memory overhead per workflow
    pub memory_overhead_per_workflow: String,
}

impl Default for OperatorConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            leader_election: LeaderElectionConfig {
                enabled: true,
                lease_duration: 15,
                renew_deadline: 10,
                retry_period: 2,
            },
            reconciliation: ReconciliationConfig {
                workflow_interval: std::time::Duration::from_secs(30),
                execution_interval: std::time::Duration::from_secs(10),
                health_check_interval: std::time::Duration::from_secs(60),
            },
            resources: ResourceConfig {
                max_workflows_per_node: 10,
                cpu_overhead_per_workflow: "50m".to_string(),
                memory_overhead_per_workflow: "64Mi".to_string(),
            },
        }
    }
}

/// Operator metrics and health
pub struct OperatorMetrics {
    pub workflows_total: u64,
    pub executions_total: u64,
    pub reconciliations_total: u64,
    pub errors_total: u64,
    pub last_reconciliation: Option<chrono::DateTime<chrono::Utc>>,
}

impl Default for OperatorMetrics {
    fn default() -> Self {
        Self {
            workflows_total: 0,
            executions_total: 0,
            reconciliations_total: 0,
            errors_total: 0,
            last_reconciliation: None,
        }
    }
}

impl OperatorMetrics {
    pub fn record_workflow_operation(&mut self) {
        self.workflows_total += 1;
        self.reconciliations_total += 1;
        self.last_reconciliation = Some(chrono::Utc::now());
    }

    pub fn record_execution_operation(&mut self) {
        self.executions_total += 1;
        self.reconciliations_total += 1;
        self.last_reconciliation = Some(chrono::Utc::now());
    }

    pub fn record_error(&mut self) {
        self.errors_total += 1;
    }

    pub fn get_health_status(&self) -> OperatorHealth {
        let now = chrono::Utc::now();

        if let Some(last_rec) = self.last_reconciliation {
            let time_since_last_rec = now.signed_duration_since(last_rec);
            if time_since_last_rec.num_minutes() > 5 {
                return OperatorHealth::Unhealthy;
            }
        } else {
            return OperatorHealth::Starting;
        }

        if self.errors_total > self.reconciliations_total / 10 {
            OperatorHealth::Degraded
        } else {
            OperatorHealth::Healthy
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperatorHealth {
    Starting,
    Healthy,
    Degraded,
    Unhealthy,
}

/// Kubernetes manifest generation utilities
pub mod manifests {
    use super::*;

    /// Generate RBAC manifests
    pub fn generate_rbac_manifests(namespace: &str) -> String {
        format!(
            r#"---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: kotoba-workflow-operator
  namespace: {namespace}
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: kotoba-workflow-operator
rules:
- apiGroups: ["kotoba.io"]
  resources: ["workflows", "workflowexecutions", "workflowtemplates", "workflowclusters"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["apps"]
  resources: ["deployments", "replicasets"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: [""]
  resources: ["services", "configmaps", "secrets", "pods", "nodes"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["networking.k8s.io"]
  resources: ["ingresses"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRoleBinding
metadata:
  name: kotoba-workflow-operator
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: ClusterRole
  name: kotoba-workflow-operator
subjects:
- kind: ServiceAccount
  name: kotoba-workflow-operator
  namespace: {namespace}
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: kotoba-workflow-crds
  namespace: {namespace}
data:
  crds.yaml: |
    # CRD definitions would go here
"#,
            namespace = namespace
        )
    }

    /// Generate operator deployment manifest
    pub fn generate_operator_deployment(namespace: &str, image: &str) -> String {
        format!(
            r#"---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: kotoba-workflow-operator
  namespace: {namespace}
  labels:
    app: kotoba-workflow-operator
spec:
  replicas: 1
  selector:
    matchLabels:
      app: kotoba-workflow-operator
  template:
    metadata:
      labels:
        app: kotoba-workflow-operator
    spec:
      serviceAccountName: kotoba-workflow-operator
      containers:
      - name: operator
        image: {image}
        imagePullPolicy: Always
        env:
        - name: RUST_LOG
          value: info
        - name: WATCH_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        resources:
          requests:
            cpu: 100m
            memory: 128Mi
          limits:
            cpu: 500m
            memory: 512Mi
        livenessProbe:
          httpGet:
            path: /health
            port: 8080
          initialDelaySeconds: 30
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /ready
            port: 8080
          initialDelaySeconds: 5
          periodSeconds: 10
---
apiVersion: v1
kind: Service
metadata:
  name: kotoba-workflow-operator-service
  namespace: {namespace}
spec:
  selector:
    app: kotoba-workflow-operator
  ports:
  - port: 8080
    targetPort: 8080
    protocol: TCP
    name: http
  type: ClusterIP
"#,
            namespace = namespace,
            image = image
        )
    }

    /// Generate CRD manifests
    pub fn generate_crd_manifests() -> String {
        r#"---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: workflows.kotoba.io
spec:
  group: kotoba.io
  versions:
  - name: v1
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        type: object
        properties:
          spec:
            type: object
            properties:
              name:
                type: string
              description:
                type: string
              version:
                type: string
              definition:
                type: object
              config:
                type: object
              env:
                type: array
                items:
                  type: object
              resources:
                type: object
              scaling:
                type: object
          status:
            type: object
            properties:
              phase:
                type: string
                enum: ["Pending", "Running", "Succeeded", "Failed", "Terminating"]
              message:
                type: string
              conditions:
                type: array
                items:
                  type: object
              startTime:
                type: string
              completionTime:
                type: string
              activeExecutions:
                type: integer
              failedExecutions:
                type: integer
              successfulExecutions:
                type: integer
              observedGeneration:
                type: integer
  scope: Namespaced
  names:
    plural: workflows
    singular: workflow
    kind: Workflow
    shortNames:
    - wf
---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: workflowexecutions.kotoba.io
spec:
  group: kotoba.io
  versions:
  - name: v1
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        type: object
        properties:
          spec:
            type: object
            properties:
              workflowRef:
                type: object
                properties:
                  name:
                    type: string
                  namespace:
                    type: string
                  version:
                    type: string
              inputs:
                type: object
              config:
                type: object
              timeout:
                type: string
          status:
            type: object
            properties:
              phase:
                type: string
                enum: ["Pending", "Running", "Succeeded", "Failed", "Cancelled"]
              message:
                type: string
              startTime:
                type: string
              completionTime:
                type: string
              executionId:
                type: string
              results:
                type: object
              conditions:
                type: array
                items:
                  type: object
  scope: Namespaced
  names:
    plural: workflowexecutions
    singular: workflowexecution
    kind: WorkflowExecution
    shortNames:
    - wfe
---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: workflowtemplates.kotoba.io
spec:
  group: kotoba.io
  versions:
  - name: v1
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        type: object
        properties:
          spec:
            type: object
            properties:
              metadata:
                type: object
                properties:
                  name:
                    type: string
                  description:
                    type: string
                  version:
                    type: string
                  category:
                    type: string
                  tags:
                    type: array
                    items:
                      type: string
                  author:
                    type: string
              parameters:
                type: array
                items:
                  type: object
              template:
                type: object
  scope: Namespaced
  names:
    plural: workflowtemplates
    singular: workflowtemplate
    kind: WorkflowTemplate
    shortNames:
    - wft
---
apiVersion: apiextensions.k8s.io/v1
kind: CustomResourceDefinition
metadata:
  name: workflowclusters.kotoba.io
spec:
  group: kotoba.io
  versions:
  - name: v1
    served: true
    storage: true
    schema:
      openAPIV3Schema:
        type: object
        properties:
          spec:
            type: object
            properties:
              config:
                type: object
              defaultResources:
                type: object
              scaling:
                type: object
  scope: Cluster
  names:
    plural: workflowclusters
    singular: workflowcluster
    kind: WorkflowCluster
    shortNames:
    - wfc
"#.to_string()
    }
}

/// Installation utilities
pub mod install {
    use super::*;

    /// Generate complete installation manifest
    pub fn generate_install_manifest(namespace: &str, operator_image: &str) -> String {
        format!(
            "{}\n---\n{}\n---\n{}",
            manifests::generate_rbac_manifests(namespace),
            manifests::generate_crd_manifests(),
            manifests::generate_operator_deployment(namespace, operator_image)
        )
    }

    /// Validate installation prerequisites
    pub async fn validate_prerequisites() -> Result<(), Vec<String>> {
        let mut issues = Vec::new();

        // Check if CRDs exist
        // TODO: Implement actual CRD validation

        // Check operator permissions
        // TODO: Implement permission validation

        if issues.is_empty() {
            Ok(())
        } else {
            Err(issues)
        }
    }
}

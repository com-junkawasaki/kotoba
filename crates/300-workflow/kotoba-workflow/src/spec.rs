//! Serverless Workflow Specification Implementation
//!
//! This module implements the Serverless Workflow specification (https://serverlessworkflow.io/)
//! providing JSON/YAML-based workflow definitions compliant with the SW DSL.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Serverless Workflow Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDocument {
    /// DSL version (e.g., "1.0.0")
    pub dsl: String,
    /// Workflow namespace
    pub namespace: String,
    /// Workflow name
    pub name: String,
    /// Workflow version
    pub version: String,
    /// Optional title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Optional summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Serverless Workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerlessWorkflow {
    /// Workflow metadata
    pub document: WorkflowDocument,
    /// Input schema (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<WorkflowInput>,
    /// Workflow steps
    #[serde(rename = "do")]
    pub r#do: Vec<WorkflowStep>,
    /// Timeouts (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeouts: Option<WorkflowTimeouts>,
    /// Events (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events: Option<Vec<EventDefinition>>,
    /// Functions (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub functions: Option<Vec<FunctionDefinition>>,
}

/// Workflow input schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    /// Input schema definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Value>,
}

/// Workflow timeouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTimeouts {
    /// Workflow execution timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workflow: Option<TimeoutDefinition>,
    /// State execution timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<TimeoutDefinition>,
    /// Action execution timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<TimeoutDefinition>,
}

/// Timeout definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutDefinition {
    /// Duration in seconds
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u64>,
    /// ISO 8601 duration string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration: Option<String>,
}

/// Event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Event name
    pub name: String,
    /// Event source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Event type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Event data schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Function definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    /// Function name
    pub name: String,
    /// Function type
    #[serde(rename = "type")]
    pub function_type: String,
    /// Function operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
}

/// Workflow step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorkflowStep {
    /// Call step (HTTP, gRPC, OpenAPI, etc.)
    Call {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Call definition
        call: CallDefinition,
        /// Optional output mapping
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    /// Emit event step
    Emit {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Emit definition
        emit: EmitDefinition,
    },
    /// Listen for events step
    Listen {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Listen definition
        listen: ListenDefinition,
        /// Optional output mapping
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    /// Wait step
    Wait {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Wait definition
        wait: WaitDefinition,
    },
    /// Run container/script step
    Run {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Run definition
        run: RunDefinition,
        /// Optional output mapping
        #[serde(skip_serializing_if = "Option::is_none")]
        output: Option<String>,
    },
    /// Switch/decision step
    Switch {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Switch definition
        switch: Vec<SwitchCase>,
    },
    /// For loop step
    For {
        /// Step name
        #[serde(skip)]
        name: String,
        /// For definition
        r#for: ForDefinition,
    },
    /// Fork parallel execution step
    Fork {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Fork definition
        fork: ForkDefinition,
    },
    /// Try-catch error handling step
    Try {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Try definition
        r#try: TryDefinition,
    },
    /// Raise error step
    Raise {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Raise definition
        raise: RaiseDefinition,
    },
    /// Set variable step
    Set {
        /// Step name
        #[serde(skip)]
        name: String,
        /// Set definition
        set: HashMap<String, Value>,
    },
}

/// Call step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CallDefinition {
    /// HTTP call
    Http {
        /// HTTP method
        method: String,
        /// Endpoint URL
        endpoint: String,
        /// Optional headers
        #[serde(skip_serializing_if = "Option::is_none")]
        headers: Option<HashMap<String, String>>,
        /// Optional body
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
        /// Optional authentication
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<Authentication>,
    },
    /// gRPC call
    Grpc {
        /// Proto file
        proto: ProtoDefinition,
        /// Service definition
        service: ServiceDefinition,
        /// Method name
        method: String,
        /// Method arguments
        arguments: HashMap<String, Value>,
        /// Optional authentication
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<Authentication>,
    },
    /// OpenAPI call
    OpenApi {
        /// OpenAPI document
        document: ApiDocument,
        /// Operation ID
        operation_id: String,
        /// Parameters
        #[serde(skip_serializing_if = "Option::is_none")]
        parameters: Option<HashMap<String, Value>>,
        /// Request body
        #[serde(skip_serializing_if = "Option::is_none")]
        body: Option<Value>,
        /// Optional authentication
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<Authentication>,
    },
    /// AsyncAPI call
    AsyncApi {
        /// AsyncAPI document
        document: ApiDocument,
        /// Operation reference
        operation_ref: String,
        /// Server name
        server: String,
        /// Message definition
        message: MessageDefinition,
        /// Optional authentication
        #[serde(skip_serializing_if = "Option::is_none")]
        auth: Option<Authentication>,
    },
}

/// Authentication definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Authentication {
    /// Basic authentication
    Basic {
        /// Username
        username: String,
        /// Password
        password: String,
    },
    /// Bearer token authentication
    Bearer {
        /// Token
        token: String,
    },
    /// OAuth2 authentication
    OAuth2 {
        /// Authority URL
        authority: String,
        /// Grant type
        grant_type: String,
        /// Client ID
        client_id: String,
        /// Client secret
        client_secret: String,
        /// Scopes
        #[serde(skip_serializing_if = "Option::is_none")]
        scopes: Option<Vec<String>>,
    },
}

/// Proto definition for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtoDefinition {
    /// Proto file endpoint or inline content
    pub endpoint: String,
}

/// Service definition for gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceDefinition {
    /// Service name
    pub name: String,
    /// Service host
    pub host: String,
    /// Service port
    pub port: u16,
}

/// API document definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocument {
    /// Document endpoint
    pub endpoint: String,
}

/// Message definition for AsyncAPI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDefinition {
    /// Message payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
}

/// Emit step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitDefinition {
    /// Event definition
    pub event: EventInstance,
}

/// Event instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventInstance {
    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with: Option<EventData>,
}

/// Event data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventData {
    /// Event source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Event type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Event data payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Listen step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenDefinition {
    /// Events to listen for
    pub to: ListenTarget,
}

/// Listen target
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListenTarget {
    /// Listen for one event
    One {
        /// Event filter
        with: EventFilter,
    },
    /// Listen for all matching events
    All {
        /// Event filters
        with: Vec<EventFilter>,
    },
}

/// Event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Event source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Event data filter (expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Wait step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WaitDefinition {
    /// Wait for duration
    Duration {
        /// Seconds to wait
        seconds: u64,
    },
    /// Wait until specific time
    Until {
        /// Timestamp to wait until
        timestamp: String,
    },
    /// Wait for event
    Event {
        /// Event to wait for
        event: EventFilter,
    },
}

/// Run step definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RunDefinition {
    /// Run container
    Container {
        /// Container definition
        container: ContainerDefinition,
    },
    /// Run script
    Script {
        /// Script definition
        script: ScriptDefinition,
    },
    /// Run workflow
    Workflow {
        /// Workflow definition
        workflow: SubWorkflowDefinition,
    },
}

/// Container definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerDefinition {
    /// Container image
    pub image: String,
    /// Command to run
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<Vec<String>>,
    /// Environment variables
    #[serde(skip_serializing_if = "Option::is_none")]
    pub env: Option<HashMap<String, String>>,
    /// Volumes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volumes: Option<Vec<VolumeDefinition>>,
}

/// Script definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDefinition {
    /// Script language
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Script code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// Script arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, Value>>,
}

/// Sub-workflow definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubWorkflowDefinition {
    /// Sub-workflow namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Sub-workflow name
    pub name: String,
    /// Sub-workflow version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Input data for sub-workflow
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Value>,
}

/// Volume definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeDefinition {
    /// Host path
    pub host_path: String,
    /// Container path
    pub container_path: String,
}

/// Switch case definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCase {
    /// Case name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Condition expression
    #[serde(skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    /// Steps to execute
    #[serde(skip_serializing_if = "Option::is_none")]
    pub then: Option<String>,
    /// Default case (no condition)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,
}

/// For loop definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForDefinition {
    /// Loop variable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub each: Option<String>,
    /// Collection to iterate over (expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_expr: Option<String>,
    /// Loop index variable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Loop condition (while)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub while_expr: Option<String>,
    /// Steps to execute in loop
    pub do_steps: Vec<WorkflowStep>,
}

/// Fork definition for parallel execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkDefinition {
    /// Fork branches
    pub branches: Vec<ForkBranch>,
    /// Whether to compete (first to complete wins)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compete: Option<bool>,
}

/// Fork branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkBranch {
    /// Branch name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Steps to execute in this branch
    pub do_steps: Vec<WorkflowStep>,
}

/// Try-catch definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryDefinition {
    /// Steps to try
    pub do_steps: Vec<WorkflowStep>,
    /// Catch definitions
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catch: Option<Vec<CatchDefinition>>,
}

/// Catch definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchDefinition {
    /// Error filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<ErrorFilter>,
    /// Error variable name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub as_var: Option<String>,
    /// Steps to execute on error
    pub do_steps: Vec<WorkflowStep>,
}

/// Error filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorFilter {
    /// Error type filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Error status filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
}

/// Raise error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaiseDefinition {
    /// Error definition
    pub error: ErrorDefinition,
}

/// Error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDefinition {
    /// Error type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Error title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Error detail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// Error instance
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
}

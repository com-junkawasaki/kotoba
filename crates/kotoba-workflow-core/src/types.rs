//! Serverless Workflow specification types
//!
//! Based on https://serverlessworkflow.io/specification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Serverless Workflow Document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDocument {
    /// DSL version
    pub dsl: String,
    /// Workflow namespace
    pub namespace: String,
    /// Workflow name
    pub name: String,
    /// Workflow version
    pub version: String,
    /// Workflow title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Workflow summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Workflow description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Input schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<Schema>,
    /// Output schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Schema>,
    /// Workflow execution timeout
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<Timeout>,
    /// Workflow states
    pub r#do: Vec<WorkflowState>,
}

/// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Schema {
    /// JSON Schema reference
    Ref { schema: serde_json::Value },
    /// Inline schema
    Inline(serde_json::Value),
}

/// Timeout definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Timeout {
    /// Duration in seconds
    Seconds(u64),
    /// ISO 8601 duration string
    Duration(String),
}

/// Workflow state - the core execution unit
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum WorkflowState {
    /// HTTP call
    CallHttp(CallHttpState),
    /// gRPC call
    CallGrpc(CallGrpcState),
    /// OpenAPI call
    CallOpenApi(CallOpenApiState),
    /// AsyncAPI call
    CallAsyncApi(CallAsyncApiState),
    /// Emit event
    Emit(EmitState),
    /// Listen for events
    Listen(ListenState),
    /// Run script
    RunScript(RunScriptState),
    /// Run container
    RunContainer(RunContainerState),
    /// Run sub-workflow
    RunWorkflow(RunWorkflowState),
    /// Switch/conditional branching
    Switch(SwitchState),
    /// For loop
    For(ForState),
    /// Fork/parallel execution
    Fork(ForkState),
    /// Try-catch error handling
    Try(TryState),
    /// Wait/delay
    Wait(WaitState),
    /// Raise error
    Raise(RaiseState),
    /// Set variables
    Set(SetState),
}

/// HTTP call state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallHttpState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// HTTP method
    pub method: String,
    /// HTTP endpoint
    pub endpoint: String,
    /// HTTP headers
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    /// Request body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Query parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<HashMap<String, String>>,
    /// Authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Authentication>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// gRPC call state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGrpcState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// gRPC service name
    pub service: GrpcService,
    /// Method name
    pub method: String,
    /// Method arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// gRPC service definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcService {
    /// Service name
    pub name: String,
    /// Host
    pub host: String,
    /// Port
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// TLS enabled
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
}

/// OpenAPI call state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallOpenApiState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// OpenAPI document
    pub document: OpenApiDocument,
    /// Operation ID
    pub operation_id: String,
    /// Operation parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    /// Request body
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<serde_json::Value>,
    /// Authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Authentication>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// OpenAPI document reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OpenApiDocument {
    /// URI reference
    Uri { uri: String },
    /// Inline document
    Inline(serde_json::Value),
}

/// AsyncAPI call state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallAsyncApiState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// AsyncAPI document
    pub document: AsyncApiDocument,
    /// Operation reference
    pub operation_ref: String,
    /// Server name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    /// Message payload
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<AsyncApiMessage>,
    /// Authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Authentication>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// AsyncAPI document reference
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AsyncApiDocument {
    /// URI reference
    Uri { uri: String },
    /// Inline document
    Inline(serde_json::Value),
}

/// AsyncAPI message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncApiMessage {
    /// Message payload
    pub payload: serde_json::Value,
}

/// Emit event state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmitState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Event to emit
    pub event: EventDefinition,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Listen state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListenState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Events to listen for
    pub to: ListenDefinition,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Listen definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ListenDefinition {
    /// Listen for one event
    One { one: EventFilter },
    /// Listen for all events
    All { all: Vec<EventFilter> },
}

/// Event filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventFilter {
    /// Event type filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Source filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Data filter (JSONPath expression)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Run script state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunScriptState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Script definition
    pub script: ScriptDefinition,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Script definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDefinition {
    /// Script language
    pub language: String,
    /// Script code
    pub code: String,
    /// Script arguments
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<HashMap<String, serde_json::Value>>,
}

/// Run container state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunContainerState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Container definition
    pub container: ContainerDefinition,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
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
    pub volumes: Option<Vec<String>>,
}

/// Run workflow state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunWorkflowState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Workflow reference
    pub workflow: WorkflowReference,
    /// Input data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input: Option<serde_json::Value>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Workflow reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowReference {
    /// Workflow namespace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub namespace: Option<String>,
    /// Workflow name
    pub name: String,
    /// Workflow version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

/// Switch state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Switch expression
    pub switch: Vec<SwitchCase>,
    /// Default case
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
}

/// Switch case
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwitchCase {
    /// Case name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Condition expression
    pub when: String,
    /// Target state
    pub then: String,
}

/// For loop state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Loop variable name
    pub each: String,
    /// Collection to iterate over
    pub in_expr: String,
    /// Index variable name (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Loop condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub while_expr: Option<String>,
    /// Loop body
    pub do_steps: Vec<WorkflowState>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Fork state (parallel execution)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForkState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Parallel branches
    pub branches: Vec<WorkflowState>,
    /// Completion condition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completion: Option<CompletionCondition>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Completion condition for fork
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CompletionCondition {
    /// All branches must complete
    All,
    /// At least N branches must complete
    AtLeast { count: usize },
    /// Any branch completion
    Any,
}

/// Try-catch state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Try block
    pub try_steps: Vec<WorkflowState>,
    /// Catch blocks
    #[serde(skip_serializing_if = "Option::is_none")]
    pub catch: Option<Vec<CatchDefinition>>,
    /// Output mapping
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputMapping>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Catch definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatchDefinition {
    /// Error filter
    #[serde(skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<String>>,
    /// Error variable name
    pub as_var: String,
    /// Catch block steps
    pub do_steps: Vec<WorkflowState>,
}

/// Wait state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Wait duration
    pub wait: WaitDefinition,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Wait definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WaitDefinition {
    /// Duration in seconds
    Seconds { seconds: u64 },
    /// ISO 8601 duration string
    Duration { duration: String },
    /// Event-based wait
    Event { event: EventFilter },
}

/// Raise error state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaiseState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Error to raise
    pub raise: ErrorDefinition,
}

/// Error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDefinition {
    /// Error type
    pub r#type: String,
    /// HTTP status code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<u16>,
    /// Error title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Error detail
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Set variables state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetState {
    /// State name/id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Variables to set
    pub variables: HashMap<String, serde_json::Value>,
    /// Next state transition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition: Option<String>,
}

/// Event definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventDefinition {
    /// Event source
    pub source: String,
    /// Event type
    pub r#type: String,
    /// Event data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Authentication definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Authentication {
    /// Basic authentication
    Basic { username: String, password: String },
    /// Bearer token
    Bearer { token: String },
    /// OAuth2
    OAuth2 { token_url: String, client_id: String, client_secret: String },
}

/// Output mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputMapping {
    /// Simple variable assignment
    Variable { as_var: String },
    /// Complex transformation
    Transform {
        /// Target variable
        as_var: String,
        /// Transformation expression
        transform: String,
    },
}

/// Workflow execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    /// Execution ID
    pub execution_id: String,
    /// Workflow instance data
    pub data: serde_json::Value,
    /// Start time
    pub start_time: chrono::DateTime<chrono::Utc>,
}

/// Workflow execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Execution ID
    pub execution_id: String,
    /// Final status
    pub status: ExecutionStatus,
    /// Output data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Execution duration in milliseconds
    pub duration_ms: u64,
    /// Completion time
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Execution errors
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ExecutionStatus {
    /// Execution is running
    Running,
    /// Execution completed successfully
    Succeeded,
    /// Execution failed
    Failed,
    /// Execution was cancelled
    Cancelled,
    /// Execution timed out
    Timeout,
}

/// State execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateResult {
    /// State name
    pub state_name: String,
    /// Execution status
    pub status: StateStatus,
    /// Output data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<serde_json::Value>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Start time
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// End time
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Error message if failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// State execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum StateStatus {
    /// State is executing
    Executing,
    /// State completed successfully
    Succeeded,
    /// State failed
    Failed,
    /// State was skipped
    Skipped,
}


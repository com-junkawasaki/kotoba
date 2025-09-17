//! Defines the schema for declarative routing files (`.kotoba` routes).
//!
//! These structs represent the structure of a route definition file, which is
//! parsed from Jsonnet into these Rust types for processing by the routing engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents the top-level structure of a `.kotoba` route file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteFile {
    /// Metadata about the route, such as the base path.
    pub route: RouteMetadata,
    /// A map of HTTP methods to their corresponding handler workflows.
    pub handlers: HashMap<String, HandlerWorkflow>,
}

/// Metadata associated with a route definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteMetadata {
    /// The base path for this route file (e.g., "/api/agents").
    pub path: String,
    /// A list of middleware to apply to all handlers in this file.
    #[serde(default)]
    pub middleware: Vec<String>,
}

/// Defines an executable workflow for a specific HTTP method.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandlerWorkflow {
    /// A list of steps to be executed sequentially.
    pub steps: Vec<WorkflowStep>,
}

/// Represents a single step in a handler workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    /// A unique identifier for this step within the workflow.
    /// The result of this step will be available in the context under this ID.
    pub id: String,
    /// The type of operation to perform.
    #[serde(rename = "type")]
    pub step_type: WorkflowStepType,
    /// The name of a specific rewrite rule to execute (for `db_rewrite` type).
    #[serde(default)]
    pub rule: String,
    /// The GQL query to execute (for `db_query` type).
    #[serde(default)]
    pub query: String,
    /// Parameters for the operation, which can reference context variables.
    /// Example: "request.body" or { user_id: "request.params.id" }
    #[serde(default)]
    pub params: serde_json::Value,
    /// The HTTP status code to return (for `return` type).
    #[serde(rename = "statusCode", default)]
    pub status_code: u16,
    /// The response body, which can reference context variables.
    /// Example: "context.fetch_user.result"
    #[serde(default)]
    pub body: serde_json::Value,
}

/// The type of operation a workflow step can perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepType {
    /// Execute a GQL query against the Kotoba database.
    DbQuery,
    /// Execute a named graph rewrite rule.
    DbRewrite,
    /// Apply a raw graph patch.
    DbPatch,
    /// Call an external HTTP service.
    HttpCall,
    /// Return a response, terminating the workflow.
    Return,
    /// A placeholder for custom logic.
    Custom,
}

impl Default for WorkflowStepType {
    fn default() -> Self {
        WorkflowStepType::Custom
    }
}

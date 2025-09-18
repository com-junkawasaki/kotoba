//! Defines the schema for declarative application files (`route.rs`, `page.kotoba`, etc.).
//!
//! These structs represent the structure of route, page, and layout definitions,
//! which are processed by the routing engine.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// --- API Route Definitions (`route.rs`) ---

/// Represents the structure of an API route definition, typically parsed from Jsonnet.
/// This corresponds to the content of a `route.kotoba` file conceptually.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ApiRoute {
    /// A map of HTTP methods to their corresponding handler workflows.
    #[serde(default)]
    pub handlers: HashMap<String, HandlerWorkflow>,
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
    #[serde(default)]
    pub params: serde_json::Value,
    /// The HTTP status code to return (for `return` type).
    #[serde(rename = "statusCode", default = "default_status_code")]
    pub status_code: u16,
    /// The response body, which can reference context variables.
    #[serde(default)]
    pub body: serde_json::Value,
}

fn default_status_code() -> u16 { 200 }

/// The type of operation a workflow step can perform.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowStepType {
    DbQuery,
    DbRewrite,
    DbPatch,
    HttpCall,
    Return,
    #[default]
    Custom,
}


// --- UI Component Definitions (`page.kotoba` and `layout.kotoba`) ---

/// Represents a UI page definition from a `page.kotoba` file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageModule {
    /// The data loading workflow to be executed on the server-side.
    /// The result of this workflow is passed as props to the component.
    #[serde(rename = "load", default)]
    pub load_workflow: Option<HandlerWorkflow>,
    /// The definition of the UI component tree.
    #[serde(default)]
    pub component: Component,
}

/// Represents a UI layout definition from a `layout.kotoba` file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LayoutModule {
    /// The data loading workflow for the layout.
    #[serde(rename = "load", default)]
    pub load_workflow: Option<HandlerWorkflow>,
    /// The definition of the layout component tree. It must contain a `children` placeholder.
    #[serde(default)]
    pub component: Component,
}

/// A generic representation of a UI component.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Component {
    /// The type of the component (e.g., "div", "h1", "UserCard").
    #[serde(rename = "type")]
    pub component_type: String,
    /// Properties (props) to be passed to the component.
    #[serde(default)]
    pub props: HashMap<String, serde_json::Value>,
    /// Child components.
    #[serde(default)]
    pub children: Vec<ComponentOrString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ComponentOrString {
    Component(Component),
    String(String),
}

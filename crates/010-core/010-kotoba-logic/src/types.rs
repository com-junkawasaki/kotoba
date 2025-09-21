//! Core type definitions for the Kotoba system
//!
//! This module defines the fundamental types used throughout the Kotoba
//! ecosystem, including graph elements, values, and identifiers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use kotoba_types::Cid;

/// Vertex ID type
pub type VertexId = uuid::Uuid;

/// Edge ID type
pub type EdgeId = uuid::Uuid;

/// Property key type
pub type PropertyKey = String;

/// Label type
pub type Label = String;

/// Properties type (key-value pairs)
pub type Properties = HashMap<PropertyKey, Value>;

/// Value type for properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Large integer value
    Integer(i64),
    /// String value
    String(String),
    /// Array value
    Array(Vec<String>),
}

impl Value {
    /// Get the value as a string representation
    pub fn as_string(&self) -> String {
        match self {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Int(i) => i.to_string(),
            Value::Integer(i) => i.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => format!("{:?}", arr),
        }
    }

    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if the value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if the value is an integer
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_) | Value::Integer(_))
    }

    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if the value is an array
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }
}

/// Graph kind enumeration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphKind {
    /// Generic graph
    Graph,
    /// Typed graph
    TypedGraph,
    /// Process graph
    ProcessGraph,
    /// Open graph
    OpenGraph,
}

/// Graph instance representing a complete graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphInstance {
    /// Core graph data
    pub core: GraphCore,
    /// Graph kind
    pub kind: GraphKind,
    /// Content ID
    pub cid: Cid,
    /// Typing information (optional)
    pub typing: Option<Typing>,
}

/// Core graph data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCore {
    /// Nodes in the graph
    pub nodes: Vec<Node>,
    /// Edges in the graph
    pub edges: Vec<Edge>,
    /// Boundary information (optional)
    pub boundary: Option<Boundary>,
    /// Graph attributes (optional)
    pub attrs: Option<Attrs>,
}

/// Typing information for typed graphs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Typing {
    /// Type graph reference
    pub type_graph: String,
    /// Node typing map
    pub node_typing: HashMap<String, String>,
    /// Edge typing map
    pub edge_typing: HashMap<String, String>,
}

/// Node in the graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Content ID
    pub cid: String,
    /// Labels
    pub labels: Vec<Label>,
    /// Node type
    pub r#type: String,
    /// Ports
    pub ports: Vec<Port>,
    /// Attributes
    pub attrs: Option<Attrs>,
    /// Component reference
    pub component_ref: Option<String>,
}

/// Edge in the graph
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Content ID
    pub cid: String,
    /// Edge label (optional)
    pub label: Option<String>,
    /// Edge type
    pub r#type: String,
    /// Source reference
    pub src: String,
    /// Target reference
    pub tgt: String,
    /// Attributes
    pub attrs: Option<Attrs>,
}

/// Port definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Port {
    /// Port name
    pub name: String,
    /// Port direction
    pub direction: PortDirection,
    /// Port type (optional)
    pub r#type: Option<String>,
    /// Multiplicity (optional)
    pub multiplicity: Option<String>,
    /// Attributes (optional)
    pub attrs: Option<Attrs>,
}

/// Port direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PortDirection {
    /// Input port
    In,
    /// Output port
    Out,
    /// Bidirectional port
    Bidirectional,
}

/// Boundary definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Boundary {
    /// Exposed ports
    pub expose: Vec<String>,
    /// Constraints (optional)
    pub constraints: Option<Attrs>,
}

/// Attributes type
pub type Attrs = HashMap<String, Value>;

/// Type definition for schema
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeDef {
    /// Type name
    pub name: String,
    /// Type description
    pub description: String,
    /// Base types
    pub base_types: Vec<String>,
    /// Properties
    pub properties: HashMap<String, PropertyDef>,
    /// Constraints
    pub constraints: Vec<String>,
}

/// Property definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDef {
    /// Property name
    pub name: String,
    /// Property type
    pub r#type: String,
    /// Required flag
    pub required: bool,
    /// Default value
    pub default: Option<Value>,
    /// Description
    pub description: String,
}

/// Schema definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SchemaDef {
    /// Schema name
    pub name: String,
    /// Schema description
    pub description: String,
    /// Types defined in this schema
    pub types: HashMap<String, TypeDef>,
    /// Imports
    pub imports: Vec<String>,
    /// Version
    pub version: String,
}

/// Process network definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessNetwork {
    /// Network name
    pub name: String,
    /// Network description
    pub description: String,
    /// Nodes in the network
    pub nodes: Vec<ProcessNode>,
    /// Edges in the network
    pub edges: Vec<ProcessEdge>,
    /// Parameters
    pub parameters: HashMap<String, Value>,
}

/// Process node
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessNode {
    /// Node name
    pub name: String,
    /// Node type
    pub node_type: String,
    /// Configuration
    pub config: HashMap<String, Value>,
    /// Input ports
    pub inputs: HashMap<String, String>,
    /// Output ports
    pub outputs: HashMap<String, String>,
}

/// Process edge
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProcessEdge {
    /// Edge name
    pub name: String,
    /// Source node
    pub src: String,
    /// Source port
    pub src_port: String,
    /// Target node
    pub tgt: String,
    /// Target port
    pub tgt_port: String,
    /// Edge type
    pub edge_type: String,
}

/// Pattern matching result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatchResult {
    /// Matched nodes
    pub nodes: HashMap<String, VertexId>,
    /// Matched edges
    pub edges: HashMap<String, EdgeId>,
    /// Bindings
    pub bindings: HashMap<String, Value>,
    /// Score
    pub score: f64,
}

/// Rewrite rule result
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RewriteResult {
    /// Success flag
    pub success: bool,
    /// New graph
    pub graph: GraphInstance,
    /// Applied rules
    pub applied_rules: Vec<String>,
    /// Statistics
    pub stats: ExecutionStats,
}

/// Execution statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExecutionStats {
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Memory usage in MB
    pub memory_usage_mb: f64,
    /// Number of operations
    pub operations_count: usize,
    /// Success rate
    pub success_rate: f64,
}

impl Default for ExecutionStats {
    fn default() -> Self {
        Self {
            execution_time_ms: 0,
            memory_usage_mb: 0.0,
            operations_count: 0,
            success_rate: 0.0,
        }
    }
}

/// Error type for type operations
#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
    /// Type not found
    TypeNotFound(String),
    /// Property not found
    PropertyNotFound(String),
    /// Type mismatch
    TypeMismatch(String),
    /// Invalid value
    InvalidValue(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::TypeNotFound(name) => write!(f, "Type not found: {}", name),
            TypeError::PropertyNotFound(name) => write!(f, "Property not found: {}", name),
            TypeError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
            TypeError::InvalidValue(msg) => write!(f, "Invalid value: {}", msg),
        }
    }
}

impl std::error::Error for TypeError {}

/// Result type for type operations
pub type TypeResult<T> = std::result::Result<T, TypeError>;

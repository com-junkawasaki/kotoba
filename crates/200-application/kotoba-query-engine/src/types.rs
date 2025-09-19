//! Common types for the GQL query engine

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Vertex identifier
pub type VertexId = String;

/// Edge identifier
pub type EdgeId = String;

/// Graph vertex
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vertex {
    pub id: VertexId,
    pub labels: Vec<String>,
    pub properties: HashMap<String, Value>,
}

/// Graph edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub id: EdgeId,
    pub label: String,
    pub from_vertex: VertexId,
    pub to_vertex: VertexId,
    pub properties: HashMap<String, Value>,
}

/// Graph path (sequence of vertices and edges)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub vertices: Vec<Vertex>,
    pub edges: Vec<Edge>,
}

/// Vertex filter for scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexFilter {
    pub labels: Option<Vec<String>>,
    pub property_filters: HashMap<String, PropertyFilter>,
}

/// Edge filter for scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFilter {
    pub labels: Option<Vec<String>>,
    pub from_vertex: Option<VertexId>,
    pub to_vertex: Option<VertexId>,
    pub property_filters: HashMap<String, PropertyFilter>,
}

/// Property filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyFilter {
    pub operator: FilterOperator,
    pub value: Value,
}

/// Filter operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Like,
    Regex,
    In,
    Contains,
}

/// Path pattern for graph traversals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathPattern {
    pub start_vertex: VertexId,
    pub edge_labels: Option<Vec<String>>,
    pub direction: PathDirection,
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
}

/// Path direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathDirection {
    Outgoing,
    Incoming,
    Both,
}

/// Query execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryExecutionResult {
    pub query_id: String,
    pub status: ExecutionStatus,
    pub result: Option<QueryResult>,
    pub error: Option<String>,
    pub execution_time_ms: u64,
    pub rows_processed: u64,
}

/// Execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Error,
    Timeout,
    Cancelled,
}

/// Query result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub statistics: QueryStatistics,
}

/// Query execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryStatistics {
    pub total_time_ms: u64,
    pub planning_time_ms: u64,
    pub execution_time_ms: u64,
    pub rows_scanned: u64,
    pub rows_returned: u64,
    pub indices_used: Vec<String>,
}

/// Statement execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementResult {
    pub success: bool,
    pub message: String,
    pub affected_rows: Option<u64>,
    pub execution_time_ms: u64,
}

/// Index lookup result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexLookupResult {
    pub index_name: String,
    pub keys_searched: Vec<Value>,
    pub results_found: u64,
    pub lookup_time_ms: u64,
}

/// Cache entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub key: String,
    pub value: Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub ttl: Option<std::time::Duration>,
}

/// Query plan cost
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanCost {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub network_cost: f64,
    pub memory_cost: f64,
    pub total_cost: f64,
}

/// Query optimization hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationHint {
    pub hint_type: HintType,
    pub description: String,
    pub suggested_action: String,
}

/// Hint types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HintType {
    IndexSuggestion,
    JoinOptimization,
    FilterPushdown,
    QueryRewrite,
}

/// Query execution context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionContext {
    pub user_id: Option<String>,
    pub session_id: String,
    pub database_name: String,
    pub query_timeout: std::time::Duration,
    pub max_memory_mb: u64,
    pub enable_tracing: bool,
}

/// Query compilation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilationResult {
    pub success: bool,
    pub optimized_query: Option<String>,
    pub execution_plan: Option<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Schema information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaInfo {
    pub vertex_labels: Vec<String>,
    pub edge_labels: Vec<String>,
    pub property_keys: Vec<String>,
    pub indices: Vec<IndexInfo>,
}

/// Index information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexInfo {
    pub name: String,
    pub target_type: IndexTargetType,
    pub properties: Vec<String>,
    pub index_type: IndexType,
}

/// Index target type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexTargetType {
    Vertex,
    Edge,
}

/// Index type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    FullText,
    Spatial,
}

/// Query metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryMetrics {
    pub query_type: String,
    pub execution_count: u64,
    pub total_execution_time_ms: u64,
    pub average_execution_time_ms: f64,
    pub max_execution_time_ms: u64,
    pub min_execution_time_ms: u64,
    pub cache_hit_rate: f64,
    pub error_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertex_creation() {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), Value::String("Alice".to_string()));
        properties.insert("age".to_string(), Value::Number(30.into()));

        let vertex = Vertex {
            id: "v1".to_string(),
            labels: vec!["Person".to_string()],
            properties,
        };

        assert_eq!(vertex.id, "v1");
        assert_eq!(vertex.labels, vec!["Person"]);
        assert_eq!(vertex.properties["name"], Value::String("Alice".to_string()));
    }

    #[test]
    fn test_edge_creation() {
        let mut properties = HashMap::new();
        properties.insert("since".to_string(), Value::Number(2020.into()));

        let edge = Edge {
            id: "e1".to_string(),
            label: "FRIENDS_WITH".to_string(),
            from_vertex: "v1".to_string(),
            to_vertex: "v2".to_string(),
            properties,
        };

        assert_eq!(edge.id, "e1");
        assert_eq!(edge.label, "FRIENDS_WITH");
        assert_eq!(edge.from_vertex, "v1");
        assert_eq!(edge.to_vertex, "v2");
    }
}

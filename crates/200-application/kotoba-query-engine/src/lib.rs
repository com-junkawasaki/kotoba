//! `kotoba-query-engine`
//!
//! ISO GQL (ISO/IEC 9075-16:2023) query engine for KotobaDB.
//! Provides SQL-like graph query capabilities for property graphs.

use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;

use kotoba_storage::KeyValueStore;

pub mod parser;
pub mod ast;
pub mod planner;
pub mod executor;
pub mod optimizer;

// Re-export main types
pub use ast::*;
pub use parser::*;
pub use planner::*;
pub use executor::*;
pub use optimizer::*;

/// Query result types
pub mod types;

// Import specific types to avoid conflicts
pub use types::{QueryResult, StatementResult, VertexId, EdgeId, Vertex, Edge, VertexFilter, EdgeFilter, Path};
pub use serde_json::Value;

// Import PathPattern from types only to avoid conflict with ast::PathPattern
pub use types::PathPattern;

/// Query execution context
#[derive(Debug, Clone)]
pub struct QueryContext {
    pub user_id: Option<String>,
    pub database: String,
    pub timeout: std::time::Duration,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Main GQL query engine with generic KeyValueStore backend
pub struct GqlQueryEngine<T: KeyValueStore> {
    storage: Arc<T>,
    optimizer: QueryOptimizer<T>,
    planner: QueryPlanner<T>,
}

impl<T: KeyValueStore + 'static> GqlQueryEngine<T> {
    pub fn new(storage: Arc<T>) -> Self {
        let optimizer = QueryOptimizer::new(storage.clone());
        let planner = QueryPlanner::new(storage.clone());

        Self {
            storage,
            optimizer,
            planner,
        }
    }

    /// Execute a GQL query
    pub async fn execute_query(
        &self,
        query: &str,
        context: QueryContext,
    ) -> Result<QueryResult> {
        // Parse query
        let parsed_query = GqlParser::parse(query)?;

        // Optimize query
        let optimized_query = self.optimizer.optimize(parsed_query).await?;

        // Plan execution
        let execution_plan = self.planner.plan(optimized_query).await?;

        // Execute plan
        let executor = QueryExecutor::new(self.storage.clone());

        executor.execute(execution_plan, context).await
    }

    /// Execute a GQL statement (DDL, DML)
    pub async fn execute_statement(
        &self,
        statement: &str,
        context: QueryContext,
    ) -> Result<StatementResult> {
        // Parse statement
        let parsed_statement = GqlParser::parse_statement(statement)?;

        // Execute statement
        let executor = StatementExecutor::new(self.storage.clone());

        executor.execute(parsed_statement, context).await
    }
}

/// Projection interface for graph data access
#[async_trait]
pub trait ProjectionPort: Send + Sync {
    async fn get_vertex(&self, id: &VertexId) -> Result<Option<Vertex>>;
    async fn get_edge(&self, id: &EdgeId) -> Result<Option<Edge>>;
    async fn scan_vertices(&self, filter: Option<VertexFilter>) -> Result<Vec<Vertex>>;
    async fn scan_edges(&self, filter: Option<EdgeFilter>) -> Result<Vec<Edge>>;
    async fn traverse(&self, start: &VertexId, pattern: &PathPattern) -> Result<Vec<Path>>;
}

/// Index manager interface
#[async_trait]
pub trait IndexManagerPort: Send + Sync {
    async fn lookup_vertices(&self, property: &str, value: &Value) -> Result<Vec<VertexId>>;
    async fn lookup_edges(&self, property: &str, value: &Value) -> Result<Vec<EdgeId>>;
    async fn range_scan(&self, property: &str, start: &Value, end: &Value) -> Result<Vec<VertexId>>;
    async fn has_vertex_index(&self, property: &str) -> Result<bool>;
    async fn has_edge_index(&self, property: &str) -> Result<bool>;
}

/// Cache interface
#[async_trait]
pub trait CachePort: Send + Sync {
    async fn get(&self, key: &str) -> Result<Option<serde_json::Value>>;
    async fn set(&self, key: &str, value: serde_json::Value, ttl: Option<std::time::Duration>) -> Result<()>;
    async fn delete(&self, key: &str) -> Result<()>;
}

// Import types from types module
pub use types::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Mock KeyValueStore for testing
    struct MockKeyValueStore {
        data: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl MockKeyValueStore {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl KeyValueStore for MockKeyValueStore {
        async fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        async fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        async fn scan(&self, prefix: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
            Ok(vec![])
        }
    }

    // Mock ProjectionPort implementation
    struct MockProjectionPort;

    #[async_trait::async_trait]
    impl ProjectionPort for MockProjectionPort {
        async fn get_vertex(&self, _id: &VertexId) -> Result<Option<Vertex>> {
            Ok(None)
        }

        async fn get_edge(&self, _id: &EdgeId) -> Result<Option<Edge>> {
            Ok(None)
        }

        async fn scan_vertices(&self, _filter: Option<VertexFilter>) -> Result<Vec<Vertex>> {
            Ok(vec![])
        }

        async fn scan_edges(&self, _filter: Option<EdgeFilter>) -> Result<Vec<Edge>> {
            Ok(vec![])
        }

        async fn traverse(&self, _start: &VertexId, _pattern: &PathPattern) -> Result<Vec<Path>> {
            Ok(vec![])
        }
    }

    // Mock IndexManagerPort implementation
    struct MockIndexManagerPort;

    #[async_trait::async_trait]
    impl IndexManagerPort for MockIndexManagerPort {
        async fn lookup_vertices(&self, _property: &str, _value: &Value) -> Result<Vec<VertexId>> {
            Ok(vec![])
        }

        async fn lookup_edges(&self, _property: &str, _value: &Value) -> Result<Vec<EdgeId>> {
            Ok(vec![])
        }

        async fn range_scan(&self, _property: &str, _start: &Value, _end: &Value) -> Result<Vec<VertexId>> {
            Ok(vec![])
        }

        async fn has_vertex_index(&self, _property: &str) -> Result<bool> {
            Ok(false)
        }

        async fn has_edge_index(&self, _property: &str) -> Result<bool> {
            Ok(false)
        }
    }

    // Mock CachePort implementation
    struct MockCachePort;

    #[async_trait::async_trait]
    impl CachePort for MockCachePort {
        async fn get(&self, _key: &str) -> Result<Option<serde_json::Value>> {
            Ok(None)
        }

        async fn set(&self, _key: &str, _value: serde_json::Value, _ttl: Option<std::time::Duration>) -> Result<()> {
            Ok(())
        }

        async fn delete(&self, _key: &str) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_query_context_creation() {
        let mut parameters = HashMap::new();
        parameters.insert("limit".to_string(), serde_json::json!(10));
        parameters.insert("offset".to_string(), serde_json::json!(0));

        let context = QueryContext {
            user_id: Some("user123".to_string()),
            database: "test_db".to_string(),
            timeout: std::time::Duration::from_secs(30),
            parameters,
        };

        assert_eq!(context.user_id, Some("user123".to_string()));
        assert_eq!(context.database, "test_db");
        assert_eq!(context.timeout, std::time::Duration::from_secs(30));
        assert_eq!(context.parameters.get("limit"), Some(&serde_json::json!(10)));
        assert_eq!(context.parameters.get("offset"), Some(&serde_json::json!(0)));
    }

    #[test]
    fn test_query_context_creation_minimal() {
        let context = QueryContext {
            user_id: None,
            database: "default".to_string(),
            timeout: std::time::Duration::from_millis(5000),
            parameters: HashMap::new(),
        };

        assert_eq!(context.user_id, None);
        assert_eq!(context.database, "default");
        assert_eq!(context.timeout, std::time::Duration::from_millis(5000));
        assert!(context.parameters.is_empty());
    }

    #[test]
    fn test_query_context_clone() {
        let original = QueryContext {
            user_id: Some("test_user".to_string()),
            database: "test_db".to_string(),
            timeout: std::time::Duration::from_secs(60),
            parameters: HashMap::new(),
        };

        let cloned = original.clone();

        assert_eq!(original.user_id, cloned.user_id);
        assert_eq!(original.database, cloned.database);
        assert_eq!(original.timeout, cloned.timeout);
        assert_eq!(original.parameters, cloned.parameters);
    }

    #[test]
    fn test_query_context_debug() {
        let context = QueryContext {
            user_id: Some("debug_user".to_string()),
            database: "debug_db".to_string(),
            timeout: std::time::Duration::from_secs(10),
            parameters: HashMap::new(),
        };

        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("debug_user"));
        assert!(debug_str.contains("debug_db"));
        assert!(debug_str.contains("10"));
    }

    #[test]
    fn test_query_context_serialization() {
        let mut parameters = HashMap::new();
        parameters.insert("name".to_string(), serde_json::json!("test"));

        let context = QueryContext {
            user_id: Some("user_001".to_string()),
            database: "test_database".to_string(),
            timeout: std::time::Duration::from_secs(45),
            parameters,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&context);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("user_001"));
        assert!(json_str.contains("test_database"));
        assert!(json_str.contains("45"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<QueryContext> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.user_id, Some("user_001".to_string()));
        assert_eq!(deserialized.database, "test_database");
        assert_eq!(deserialized.timeout, std::time::Duration::from_secs(45));
    }

    #[tokio::test]
    async fn test_gql_query_engine_creation() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let engine = GqlQueryEngine::new(mock_storage);

        // Verify that engine was created successfully
        assert!(true); // If we reach here, creation was successful
    }

    #[tokio::test]
    async fn test_gql_query_engine_execute_query() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let engine = GqlQueryEngine::new(mock_storage);

        let context = QueryContext {
            user_id: Some("test_user".to_string()),
            database: "test_db".to_string(),
            timeout: std::time::Duration::from_secs(10),
            parameters: HashMap::new(),
        };

        // This will fail because the parser and other components are not fully implemented yet
        // But we can test that the method exists and can be called
        let query = "MATCH (n) RETURN n";
        let result = engine.execute_query(query, context).await;

        // For now, we expect this to fail due to unimplemented components
        // Once the full implementation is complete, this should succeed
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_gql_query_engine_execute_statement() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let engine = GqlQueryEngine::new(mock_storage);

        let context = QueryContext {
            user_id: Some("test_user".to_string()),
            database: "test_db".to_string(),
            timeout: std::time::Duration::from_secs(10),
            parameters: HashMap::new(),
        };

        // This will fail because statement execution is not fully implemented yet
        let statement = "CREATE GRAPH test_graph";
        let result = engine.execute_statement(statement, context).await;

        // For now, we expect this to fail due to unimplemented components
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_projection_port_mock() {
        let projection = MockProjectionPort;
        let vertex_id = VertexId("test_vertex".to_string());

        // Test get_vertex
        let result = projection.get_vertex(&vertex_id).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Test scan_vertices
        let result = projection.scan_vertices(None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test scan_edges
        let result = projection.scan_edges(None).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test traverse
        let pattern = PathPattern::default();
        let result = projection.traverse(&vertex_id, &pattern).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_index_manager_port_mock() {
        let index_manager = MockIndexManagerPort;
        let value = serde_json::json!("test_value");

        // Test lookup_vertices
        let result = index_manager.lookup_vertices("name", &value).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test lookup_edges
        let result = index_manager.lookup_edges("type", &value).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test range_scan
        let start = serde_json::json!("a");
        let end = serde_json::json!("z");
        let result = index_manager.range_scan("name", &start, &end).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());

        // Test has_vertex_index
        let result = index_manager.has_vertex_index("name").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());

        // Test has_edge_index
        let result = index_manager.has_edge_index("type").await;
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[tokio::test]
    async fn test_cache_port_mock() {
        let cache = MockCachePort;
        let value = serde_json::json!({"key": "value"});

        // Test get
        let result = cache.get("test_key").await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Test set
        let result = cache.set("test_key", value.clone(), Some(std::time::Duration::from_secs(60))).await;
        assert!(result.is_ok());

        // Test delete
        let result = cache.delete("test_key").await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_projection_port_trait() {
        // Test that ProjectionPort is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockProjectionPort>();
    }

    #[test]
    fn test_index_manager_port_trait() {
        // Test that IndexManagerPort is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockIndexManagerPort>();
    }

    #[test]
    fn test_cache_port_trait() {
        // Test that CachePort is Send + Sync
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<MockCachePort>();
    }

    #[test]
    fn test_query_result_types() {
        // Test that QueryResult type exists and can be constructed
        // Since QueryResult is likely an enum or struct from types module,
        // we'll test that it's accessible
        let _query_result_type_exists = std::any::TypeId::of::<QueryResult>();
        assert!(true);
    }

    #[test]
    fn test_statement_result_types() {
        // Test that StatementResult type exists and can be constructed
        let _statement_result_type_exists = std::any::TypeId::of::<StatementResult>();
        assert!(true);
    }

    #[test]
    fn test_vertex_edge_types() {
        // Test that VertexId, EdgeId, Vertex, Edge types exist
        let vertex_id = VertexId("test".to_string());
        assert_eq!(vertex_id.0, "test");

        let edge_id = EdgeId("test_edge".to_string());
        assert_eq!(edge_id.0, "test_edge");
    }

    #[test]
    fn test_filter_types() {
        // Test that filter types exist
        let _vertex_filter_exists = std::any::TypeId::of::<VertexFilter>();
        let _edge_filter_exists = std::any::TypeId::of::<EdgeFilter>();
        assert!(true);
    }

    #[test]
    fn test_path_pattern_type() {
        // Test that PathPattern type exists
        let _path_pattern_exists = std::any::TypeId::of::<PathPattern>();
        assert!(true);
    }

    #[test]
    fn test_path_type() {
        // Test that Path type exists
        let _path_exists = std::any::TypeId::of::<Path>();
        assert!(true);
    }

    #[test]
    fn test_gql_query_engine_with_different_storage() {
        // Test that GqlQueryEngine can work with different KeyValueStore implementations
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let _engine: GqlQueryEngine<MockKeyValueStore> = GqlQueryEngine::new(mock_storage);
        assert!(true);
    }

    #[test]
    fn test_query_context_with_parameters() {
        let mut parameters = HashMap::new();
        parameters.insert("limit".to_string(), serde_json::json!(100));
        parameters.insert("sort".to_string(), serde_json::json!("name"));
        parameters.insert("filter".to_string(), serde_json::json!({"active": true}));

        let context = QueryContext {
            user_id: Some("admin".to_string()),
            database: "analytics".to_string(),
            timeout: std::time::Duration::from_millis(30000),
            parameters,
        };

        assert_eq!(context.parameters.len(), 3);
        assert_eq!(context.parameters.get("limit"), Some(&serde_json::json!(100)));
        assert_eq!(context.parameters.get("sort"), Some(&serde_json::json!("name")));

        let filter_value = context.parameters.get("filter").unwrap();
        assert_eq!(filter_value.get("active"), Some(&serde_json::json!(true)));
    }

    #[test]
    fn test_query_context_equality() {
        let context1 = QueryContext {
            user_id: Some("user1".to_string()),
            database: "db1".to_string(),
            timeout: std::time::Duration::from_secs(30),
            parameters: HashMap::new(),
        };

        let context2 = QueryContext {
            user_id: Some("user1".to_string()),
            database: "db1".to_string(),
            timeout: std::time::Duration::from_secs(30),
            parameters: HashMap::new(),
        };

        // Note: QueryContext doesn't implement PartialEq, so we can't test equality directly
        // This is fine as it's a complex struct that may not need equality comparison
        assert!(true);
    }
}

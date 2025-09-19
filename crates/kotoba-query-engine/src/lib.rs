//! `kotoba-query-engine`
//!
//! ISO GQL (ISO/IEC 9075-16:2023) query engine for KotobaDB.
//! Provides SQL-like graph query capabilities for property graphs.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;

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
pub use types::*;

/// Query execution context
#[derive(Debug, Clone)]
pub struct QueryContext {
    pub user_id: Option<String>,
    pub database: String,
    pub timeout: std::time::Duration,
    pub parameters: HashMap<String, serde_json::Value>,
}

/// Main GQL query engine
pub struct GqlQueryEngine {
    projection: Arc<dyn ProjectionPort>,
    index_manager: Arc<dyn IndexManagerPort>,
    cache: Arc<dyn CachePort>,
    optimizer: QueryOptimizer,
    planner: QueryPlanner,
}

impl GqlQueryEngine {
    pub fn new(
        projection: Arc<dyn ProjectionPort>,
        index_manager: Arc<dyn IndexManagerPort>,
        cache: Arc<dyn CachePort>,
    ) -> Self {
        let optimizer = QueryOptimizer::new(index_manager.clone());
        let planner = QueryPlanner::new(projection.clone(), index_manager.clone());

        Self {
            projection,
            index_manager,
            cache,
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
        let optimized_query = self.optimizer.optimize(parsed_query)?;

        // Plan execution
        let execution_plan = self.planner.plan(optimized_query)?;

        // Execute plan
        let executor = QueryExecutor::new(
            self.projection.clone(),
            self.index_manager.clone(),
            self.cache.clone(),
        );

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
        let executor = StatementExecutor::new(
            self.projection.clone(),
            self.index_manager.clone(),
        );

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

    #[tokio::test]
    async fn test_query_engine_creation() {
        // Test that query engine can be created
        // This will be expanded once all dependencies are implemented
    }
}

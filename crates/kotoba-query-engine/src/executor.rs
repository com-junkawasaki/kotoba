//! Query Executor
//!
//! Executes optimized query plans against the graph database.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use anyhow::Result;
use futures::stream::{self, StreamExt};

use crate::ast::*;
use crate::types::*;
use crate::planner::*;
use crate::{ProjectionPort, IndexManagerPort, CachePort};

/// Query executor
pub struct QueryExecutor {
    projection: Arc<dyn ProjectionPort>,
    index_manager: Arc<dyn IndexManagerPort>,
    cache: Arc<dyn CachePort>,
}

impl QueryExecutor {
    /// Convert Vertex to serde_json::Value
    fn vertex_to_json_value(&self, vertex: Vertex) -> serde_json::Value {
        serde_json::json!({
            "id": vertex.id,
            "labels": vertex.labels,
            "properties": vertex.properties
        })
    }

    /// Convert Edge to serde_json::Value
    fn edge_to_json_value(&self, edge: Edge) -> serde_json::Value {
        serde_json::json!({
            "id": edge.id,
            "label": edge.label,
            "from_vertex": edge.from_vertex,
            "to_vertex": edge.to_vertex,
            "properties": edge.properties
        })
    }
    pub fn new(
        projection: Arc<dyn ProjectionPort>,
        index_manager: Arc<dyn IndexManagerPort>,
        cache: Arc<dyn CachePort>,
    ) -> Self {
        Self {
            projection,
            index_manager,
            cache,
        }
    }

    /// Execute a query plan
    pub async fn execute(
        &self,
        plan: ExecutionPlan,
        context: crate::QueryContext,
    ) -> Result<QueryResult> {
        let mut current_result = ExecutionResult::Empty;

        // Execute each step in order
        for step in plan.steps {
            current_result = match step {
                ExecutionStep::Match(match_plan) => {
                    self.execute_match(match_plan, current_result).await?
                }
                ExecutionStep::Filter(filter_plan) => {
                    self.execute_filter(filter_plan, current_result).await?
                }
                ExecutionStep::GroupBy(group_by_plan) => {
                    self.execute_group_by(group_by_plan, current_result).await?
                }
                ExecutionStep::Sort(sort_plan) => {
                    self.execute_sort(sort_plan, current_result).await?
                }
                ExecutionStep::Limit(limit_clause) => {
                    self.execute_limit(limit_clause, current_result).await?
                }
                ExecutionStep::Return(return_plan) => {
                    return self.execute_return(return_plan, current_result).await;
                }
            };
        }

        // If no return step was executed, return the current result
        Ok(QueryResult::from(current_result))
    }

    async fn execute_match(
        &self,
        match_plan: MatchPlan,
        _previous_result: ExecutionResult,
    ) -> Result<ExecutionResult> {
        let mut results = Vec::new();

        // Execute vertex scans
        for vertex_scan in match_plan.vertex_scans {
            let vertices = self.execute_vertex_scan(vertex_scan).await?;
            results.extend(vertices);
        }

        // Execute edge scans and joins
        for edge_scan in match_plan.edge_scans {
            let edges = self.execute_edge_scan(edge_scan).await?;
            // TODO: Implement proper joining logic
            results.extend(edges.into_iter().map(|e| vec![e]));
        }

        Ok(ExecutionResult::Rows(results))
    }

    async fn execute_vertex_scan(&self, scan_plan: VertexScanPlan) -> Result<Vec<Vec<Value>>> {
        let mut results = Vec::new();

        match scan_plan.scan_type {
            ScanType::IndexScan(property) => {
                // Use index for efficient lookup
                if let Some(value_expr) = scan_plan.properties.get(&property) {
                    if let ValueExpression::Literal(crate::ast::AstValue::String(value)) = value_expr {
                        let vertex_ids = self.index_manager.lookup_vertices(&property, &serde_json::Value::String(value.clone())).await?;
                        for vertex_id in vertex_ids {
                            if let Some(vertex) = self.projection.get_vertex(&vertex_id).await? {
                                let json_value = self.vertex_to_json_value(vertex);
                                results.push(vec![json_value]);
                            }
                        }
                    }
                }
            }
            _ => {
                // Full scan
                let vertices = self.projection.scan_vertices(None).await?;
                for vertex in vertices {
                    // TODO: Apply filters
                    let json_value = self.vertex_to_json_value(vertex);
                    results.push(vec![json_value]);
                }
            }
        }

        Ok(results)
    }

    async fn execute_edge_scan(&self, scan_plan: EdgeScanPlan) -> Result<Vec<Value>> {
        let mut results = Vec::new();

        match scan_plan.scan_type {
            ScanType::IndexScan(property) => {
                // Use index for efficient lookup
                if let Some(value_expr) = scan_plan.properties.get(&property) {
                    if let ValueExpression::Literal(crate::ast::AstValue::String(value)) = value_expr {
                        let edge_ids = self.index_manager.lookup_edges(&property, &serde_json::Value::String(value.clone())).await?;
                        for edge_id in edge_ids {
                            if let Some(edge) = self.projection.get_edge(&edge_id).await? {
                                let json_value = self.edge_to_json_value(edge);
                                results.push(json_value);
                            }
                        }
                    }
                }
            }
            _ => {
                // Full scan
                let edges = self.projection.scan_edges(None).await?;
                for edge in edges {
                    // TODO: Apply filters
                    let json_value = self.edge_to_json_value(edge);
                    results.push(json_value);
                }
            }
        }

        Ok(results)
    }

    async fn execute_filter(
        &self,
        filter_plan: FilterPlan,
        input: ExecutionResult,
    ) -> Result<ExecutionResult> {
        match input {
            ExecutionResult::Rows(rows) => {
                let mut filtered_rows = Vec::new();

                for row in rows {
                    if self.evaluate_filter(&filter_plan, &row).await? {
                        filtered_rows.push(row);
                    }
                }

                Ok(ExecutionResult::Rows(filtered_rows))
            }
            _ => Ok(input),
        }
    }

    async fn evaluate_filter(&self, _filter_plan: &FilterPlan, _row: &[Value]) -> Result<bool> {
        // TODO: Implement filter evaluation
        // For now, return true for all rows
        Ok(true)
    }

    async fn execute_group_by(
        &self,
        group_by_plan: GroupByPlan,
        input: ExecutionResult,
    ) -> Result<ExecutionResult> {
        match input {
            ExecutionResult::Rows(rows) => {
                let mut groups = std::collections::HashMap::new();

                for row in rows {
                    let key = self.compute_group_key(&group_by_plan.keys, &row).await?;
                    groups.entry(key).or_insert_with(Vec::new).push(row);
                }

                Ok(ExecutionResult::Grouped(groups))
            }
            _ => Ok(input),
        }
    }

    async fn compute_group_key(&self, _keys: &[ValueExpression], _row: &[Value]) -> Result<String> {
        // TODO: Implement group key computation
        Ok("default_group".to_string())
    }

    async fn execute_sort(
        &self,
        sort_plan: SortPlan,
        input: ExecutionResult,
    ) -> Result<ExecutionResult> {
        match input {
            ExecutionResult::Rows(mut rows) => {
                // TODO: Implement sorting based on sort keys
                // For now, just return as-is
                Ok(ExecutionResult::Rows(rows))
            }
            _ => Ok(input),
        }
    }

    async fn execute_limit(
        &self,
        limit_clause: LimitClause,
        input: ExecutionResult,
    ) -> Result<ExecutionResult> {
        match input {
            ExecutionResult::Rows(rows) => {
                let start = limit_clause.offset.unwrap_or(0) as usize;
                let end = start + limit_clause.count as usize;
                let limited_rows = rows.into_iter()
                    .skip(start)
                    .take(limit_clause.count as usize)
                    .collect();

                Ok(ExecutionResult::Rows(limited_rows))
            }
            _ => Ok(input),
        }
    }

    async fn execute_return(
        &self,
        return_plan: ReturnPlan,
        input: ExecutionResult,
    ) -> Result<QueryResult> {
        let mut results = Vec::new();

        match input {
            ExecutionResult::Rows(rows) => {
                for row in rows {
                    let mut result_row = Vec::new();

                    for item in &return_plan.items {
                        let value = self.evaluate_expression(&item.expression, &row).await?;
                        result_row.push(value);
                    }

                    results.push(result_row);
                }
            }
            ExecutionResult::Grouped(groups) => {
                // Handle grouped results
                for (_key, rows) in groups {
                    // TODO: Implement aggregation
                    if let Some(row) = rows.first() {
                        let mut result_row = Vec::new();
                        for item in &return_plan.items {
                            let value = self.evaluate_expression(&item.expression, row).await?;
                            result_row.push(value);
                        }
                        results.push(result_row);
                    }
                }
            }
            ExecutionResult::Empty => {}
        }

        // Apply DISTINCT if requested
        if return_plan.distinct {
            // TODO: Implement distinct logic
        }

        let rows_returned = results.len() as u64;
        Ok(QueryResult {
            columns: return_plan.items.iter()
                .map(|item| item.alias.clone().unwrap_or_else(|| "column".to_string()))
                .collect(),
            rows: results,
            statistics: crate::QueryStatistics {
                total_time_ms: 0,
                planning_time_ms: 0,
                execution_time_ms: 0,
                rows_scanned: 0,
                rows_returned,
                indices_used: vec![],
            },
        })
    }

    async fn evaluate_expression(&self, _expression: &ValueExpression, _row: &[serde_json::Value]) -> Result<serde_json::Value> {
        // TODO: Implement expression evaluation
        // For now, return a placeholder
        Ok(serde_json::Value::String("placeholder".to_string()))
    }
}

/// Statement executor for DDL/DML operations
pub struct StatementExecutor {
    projection: Arc<dyn ProjectionPort>,
    index_manager: Arc<dyn IndexManagerPort>,
}

impl StatementExecutor {
    pub fn new(
        projection: Arc<dyn ProjectionPort>,
        index_manager: Arc<dyn IndexManagerPort>,
    ) -> Self {
        Self {
            projection,
            index_manager,
        }
    }

    pub async fn execute(
        &self,
        _statement: GqlStatement,
        _context: crate::QueryContext,
    ) -> Result<StatementResult> {
        // TODO: Implement statement execution
        Ok(StatementResult {
            success: true,
            message: "Statement executed successfully".to_string(),
            affected_rows: None,
            execution_time_ms: 0,
        })
    }
}

/// Execution result types
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Empty,
    Rows(Vec<Vec<serde_json::Value>>),
    Grouped(std::collections::HashMap<String, Vec<Vec<serde_json::Value>>>),
}


impl From<ExecutionResult> for QueryResult {
    fn from(result: ExecutionResult) -> Self {
        match result {
            ExecutionResult::Rows(rows) => {
                let rows_returned = rows.len() as u64;
                QueryResult {
                    columns: vec!["result".to_string()], // Placeholder
                    rows,
                    statistics: crate::QueryStatistics {
                        total_time_ms: 0,
                        planning_time_ms: 0,
                        execution_time_ms: 0,
                        rows_scanned: 0,
                        rows_returned,
                        indices_used: vec![],
                    },
                }
            },
            _ => QueryResult {
                columns: Vec::new(),
                rows: Vec::new(),
                statistics: crate::QueryStatistics {
                    total_time_ms: 0,
                    planning_time_ms: 0,
                    execution_time_ms: 0,
                    rows_scanned: 0,
                    rows_returned: 0u64,
                    indices_used: vec![],
                },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_query_executor_creation() {
        // Test that executor can be created
        // This will be expanded with actual execution tests
    }
}

//! Query Planner
//!
//! Plans the execution of GQL queries by creating optimized execution plans.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;

use crate::ast::*;
use crate::types::*;
use crate::{ProjectionPort, IndexManagerPort};
use kotoba_storage::KeyValueStore;

/// Query planner with KeyValueStore backend
pub struct QueryPlanner<T: KeyValueStore> {
    storage: Arc<T>,
}

impl<T: KeyValueStore + 'static> QueryPlanner<T> {
    pub fn new(storage: Arc<T>) -> Self {
        Self { storage }
    }

    /// Create execution plan for a query
    pub async fn plan(&self, query: GqlQuery) -> Result<ExecutionPlan> {
        let mut plan = ExecutionPlan::default();

        // Process each clause
        for clause in query.clauses {
            match clause {
                QueryClause::Match(match_clause) => {
                    plan.steps.push(ExecutionStep::Match(self.plan_match(match_clause).await?));
                }
                QueryClause::Where(where_clause) => {
                    plan.steps.push(ExecutionStep::Filter(self.plan_where(where_clause)?));
                }
                QueryClause::GroupBy(group_by) => {
                    plan.steps.push(ExecutionStep::GroupBy(self.plan_group_by(group_by)?));
                }
                QueryClause::OrderBy(order_by) => {
                    plan.steps.push(ExecutionStep::Sort(self.plan_order_by(order_by)?));
                }
                QueryClause::Limit(limit) => {
                    plan.steps.push(ExecutionStep::Limit(limit));
                }
                _ => {} // Other clauses not implemented yet
            }
        }

        // Add return step
        if let Some(return_clause) = query.returning {
            plan.steps.push(ExecutionStep::Return(self.plan_return(return_clause)?));
        }

        Ok(plan)
    }

    async fn plan_match(&self, match_clause: MatchClause) -> Result<MatchPlan> {
        let mut vertex_scans = Vec::new();
        let mut edge_scans = Vec::new();

        // Analyze graph pattern for optimization opportunities
        for path_pattern in match_clause.pattern.path_patterns {
            if let PathTerm::PathElement(path_element) = path_pattern.path_term {
                // Plan vertex scans
                vertex_scans.push(self.plan_vertex_scan(&path_element.vertex_pattern).await?);

                // Plan edge scans
                for edge_pattern in path_element.edge_patterns {
                    edge_scans.push(self.plan_edge_scan(&edge_pattern).await?);
                }
            }
        }

        Ok(MatchPlan {
            vertex_scans,
            edge_scans,
            join_strategy: JoinStrategy::HashJoin, // Default strategy
        })
    }

    async fn plan_vertex_scan(&self, vertex_pattern: &VertexPattern) -> Result<VertexScanPlan> {
        // Analyze vertex pattern for index usage
        let mut index_candidates: Vec<String> = Vec::new();

        for (property, _) in &vertex_pattern.properties {
            // TODO: Check if there's an index for this property using KeyValueStore
            // For now, assume no indices exist
            // if let Ok(index_exists) = self.check_vertex_index_exists(property).await {
            //     if index_exists {
            //         index_candidates.push(property.clone());
            //     }
            // }
        }

        // Choose the best index or fallback to scan
        let scan_type = if !index_candidates.is_empty() {
            ScanType::IndexScan(index_candidates[0].clone())
        } else {
            ScanType::FullScan
        };

        Ok(VertexScanPlan {
            labels: vertex_pattern.labels.clone(),
            properties: vertex_pattern.properties.clone(),
            scan_type,
        })
    }

    async fn plan_edge_scan(&self, edge_pattern: &EdgePattern) -> Result<EdgeScanPlan> {
        // Similar logic for edge scanning
        let mut index_candidates: Vec<String> = Vec::new();

        for (property, _) in &edge_pattern.properties {
            // TODO: Check if there's an index for this property using KeyValueStore
            // For now, assume no indices exist
            // if let Ok(index_exists) = self.check_edge_index_exists(property).await {
            //     if index_exists {
            //         index_candidates.push(property.clone());
            //     }
            // }
        }

        let scan_type = if !index_candidates.is_empty() {
            ScanType::IndexScan(index_candidates[0].clone())
        } else {
            ScanType::FullScan
        };

        Ok(EdgeScanPlan {
            labels: edge_pattern.labels.clone(),
            properties: edge_pattern.properties.clone(),
            direction: edge_pattern.direction.clone(),
            scan_type,
        })
    }

    fn plan_where(&self, where_clause: WhereClause) -> Result<FilterPlan> {
        // Analyze WHERE clause for optimization
        let filter_type = match where_clause.expression {
            BooleanExpression::Comparison(comp) => {
                FilterType::Comparison(comp)
            }
            BooleanExpression::Exists(pattern) => {
                FilterType::Exists(*pattern)
            }
            _ => FilterType::Generic(where_clause.expression),
        };

        Ok(FilterPlan {
            filter_type,
        })
    }

    fn plan_group_by(&self, group_by: GroupByClause) -> Result<GroupByPlan> {
        Ok(GroupByPlan {
            keys: group_by.grouping_keys,
        })
    }

    fn plan_order_by(&self, order_by: OrderByClause) -> Result<SortPlan> {
        Ok(SortPlan {
            keys: order_by.sort_keys,
        })
    }

    fn plan_return(&self, return_clause: ReturnClause) -> Result<ReturnPlan> {
        Ok(ReturnPlan {
            items: return_clause.items,
            distinct: return_clause.distinct,
        })
    }
}

/// Execution plan types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    pub steps: Vec<ExecutionStep>,
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self { steps: Vec::new() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStep {
    Match(MatchPlan),
    Filter(FilterPlan),
    GroupBy(GroupByPlan),
    Sort(SortPlan),
    Limit(LimitClause),
    Return(ReturnPlan),
}

/// Match execution plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchPlan {
    pub vertex_scans: Vec<VertexScanPlan>,
    pub edge_scans: Vec<EdgeScanPlan>,
    pub join_strategy: JoinStrategy,
}

/// Vertex scan plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexScanPlan {
    pub labels: Vec<String>,
    pub properties: std::collections::HashMap<String, ValueExpression>,
    pub scan_type: ScanType,
}

/// Edge scan plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeScanPlan {
    pub labels: Vec<String>,
    pub properties: std::collections::HashMap<String, ValueExpression>,
    pub direction: EdgeDirection,
    pub scan_type: ScanType,
}

/// Scan types for optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScanType {
    FullScan,
    IndexScan(String),
    RangeScan { start: ValueExpression, end: ValueExpression },
}

/// Join strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JoinStrategy {
    HashJoin,
    NestedLoopJoin,
    MergeJoin,
}

/// Filter plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPlan {
    pub filter_type: FilterType,
}

/// Filter types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterType {
    Comparison(ComparisonExpression),
    Exists(GraphPattern),
    Generic(BooleanExpression),
}

/// Group by plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupByPlan {
    pub keys: Vec<ValueExpression>,
}

/// Sort plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortPlan {
    pub keys: Vec<SortKey>,
}

/// Return plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnPlan {
    pub items: Vec<ReturnItem>,
    pub distinct: bool,
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_planner_creation() {
        // Test that planner can be created
        // This will be expanded with actual planning tests
    }
}

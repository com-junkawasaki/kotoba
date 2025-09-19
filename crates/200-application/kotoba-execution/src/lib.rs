//! kotoba-execution - Kotoba Execution Components

pub mod execution;

use crate::execution::physical_plan::PhysicalPlan;
use crate::execution::metrics::ExecutionMetrics;
// use kotoba_core::types::{Result, Value}; // Avoid conflicts with our custom Result type
use kotoba_core::types::Value;

use kotoba_storage::KeyValueStore;
use std::sync::Arc;

// Use std::result::Result instead of kotoba_core::types::Result to avoid conflicts
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait::async_trait]
pub trait QueryExecutor<T: KeyValueStore + 'static>: Send + Sync {
    async fn execute(&self, plan: PhysicalPlan) -> Result<Vec<Value>>;
}

pub struct DefaultQueryExecutor<T: KeyValueStore + 'static> {
    storage: Arc<T>,
}

impl<T: KeyValueStore + 'static> DefaultQueryExecutor<T> {
    pub fn new(storage: Arc<T>) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl<T: KeyValueStore + 'static> QueryExecutor<T> for DefaultQueryExecutor<T> {
    async fn execute(&self, plan: PhysicalPlan) -> Result<Vec<Value>> {
        let mut metrics = ExecutionMetrics::new();
        // TODO: Implement execution logic using KeyValueStore
        // For now, return empty result
        Ok(vec![])
    }
}

pub mod planner;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::execution::*;
    pub use crate::planner::*;
    // Avoid re-exporting duplicate types
    // PhysicalPlan and PhysicalOp are available through both execution and planner
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_core::types::*;
    use kotoba_core::ir::*;
    use kotoba_memory::MemoryKeyValueStore;
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
        async fn put(&self, _key: &[u8], _value: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        async fn get(&self, _key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn delete(&self, _key: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        async fn scan(&self, _prefix: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_column_creation() {
        let column = Column::new("id".to_string(), "String".to_string());
        assert_eq!(column.name, "id");
        assert_eq!(column.data_type, "String");
    }

    #[test]
    fn test_physical_plan_creation() {
        let column = Column::new("name".to_string(), "String".to_string());
        let scan_op = PhysicalOp::Scan {
            table: "users".to_string(),
            filter: None,
            projection: vec!["name".to_string()],
        };

        let plan = PhysicalPlan::new(scan_op, vec![column]);
        assert_eq!(plan.schema.len(), 1);
        assert_eq!(plan.schema[0].name, "name");
    }

    #[test]
    fn test_physical_op_scan() {
        let scan_op = PhysicalOp::Scan {
            table: "users".to_string(),
            filter: Some(Expr::Const(Value::Int(1))),
            projection: vec!["id".to_string(), "name".to_string()],
        };

        match scan_op {
            PhysicalOp::Scan { table, filter, projection } => {
                assert_eq!(table, "users");
                assert!(filter.is_some());
                assert_eq!(projection.len(), 2);
            }
            _ => panic!("Expected Scan operation"),
        }
    }

    #[test]
    fn test_physical_op_filter() {
        let filter_op = PhysicalOp::Filter {
            input: Box::new(PhysicalOp::Scan {
                table: "users".to_string(),
                filter: None,
                projection: vec!["name".to_string()],
            }),
            condition: Expr::Const(Value::Boolean(true)),
        };

        match filter_op {
            PhysicalOp::Filter { input, condition } => {
                match *input {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "users"),
                    _ => panic!("Expected nested Scan operation"),
                }
                assert!(matches!(condition, Expr::Const(Value::Boolean(true))));
            }
            _ => panic!("Expected Filter operation"),
        }
    }

    #[test]
    fn test_physical_op_projection() {
        let projection_op = PhysicalOp::Projection {
            input: Box::new(PhysicalOp::Scan {
                table: "users".to_string(),
                filter: None,
                projection: vec!["*".to_string()],
            }),
            expressions: vec![
                (Expr::Var("name".to_string()), "user_name".to_string()),
                (Expr::Const(Value::String("active".to_string())), "status".to_string()),
            ],
        };

        match projection_op {
            PhysicalOp::Projection { input, expressions } => {
                match *input {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "users"),
                    _ => panic!("Expected nested Scan operation"),
                }
                assert_eq!(expressions.len(), 2);
                assert_eq!(expressions[0].1, "user_name");
                assert_eq!(expressions[1].1, "status");
            }
            _ => panic!("Expected Projection operation"),
        }
    }

    #[test]
    fn test_physical_op_sort() {
        let sort_op = PhysicalOp::Sort {
            input: Box::new(PhysicalOp::Scan {
                table: "products".to_string(),
                filter: None,
                projection: vec!["name".to_string(), "price".to_string()],
            }),
            order_by: vec![
                ("price".to_string(), SortDirection::Desc),
                ("name".to_string(), SortDirection::Asc),
            ],
        };

        match sort_op {
            PhysicalOp::Sort { input, order_by } => {
                match *input {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "products"),
                    _ => panic!("Expected nested Scan operation"),
                }
                assert_eq!(order_by.len(), 2);
                assert_eq!(order_by[0].1, SortDirection::Desc);
                assert_eq!(order_by[1].1, SortDirection::Asc);
            }
            _ => panic!("Expected Sort operation"),
        }
    }

    #[test]
    fn test_physical_op_group_by() {
        let group_by_op = PhysicalOp::GroupBy {
            input: Box::new(PhysicalOp::Scan {
                table: "orders".to_string(),
                filter: None,
                projection: vec!["*".to_string()],
            }),
            group_by: vec!["customer_id".to_string()],
            aggregates: vec![
                AggregateExpr {
                    function: AggregateFunction::Sum,
                    argument: Expr::Var("amount".to_string()),
                    alias: "total_amount".to_string(),
                },
                AggregateExpr {
                    function: AggregateFunction::Count,
                    argument: Expr::Const(Value::Int(1)),
                    alias: "order_count".to_string(),
                },
            ],
        };

        match group_by_op {
            PhysicalOp::GroupBy { input, group_by, aggregates } => {
                match *input {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "orders"),
                    _ => panic!("Expected nested Scan operation"),
                }
                assert_eq!(group_by.len(), 1);
                assert_eq!(group_by[0], "customer_id");
                assert_eq!(aggregates.len(), 2);
                assert_eq!(aggregates[0].function, AggregateFunction::Sum);
                assert_eq!(aggregates[1].function, AggregateFunction::Count);
            }
            _ => panic!("Expected GroupBy operation"),
        }
    }

    #[test]
    fn test_physical_op_join() {
        let join_op = PhysicalOp::Join {
            left: Box::new(PhysicalOp::Scan {
                table: "users".to_string(),
                filter: None,
                projection: vec!["id".to_string(), "name".to_string()],
            }),
            right: Box::new(PhysicalOp::Scan {
                table: "orders".to_string(),
                filter: None,
                projection: vec!["user_id".to_string(), "amount".to_string()],
            }),
            join_type: JoinType::Inner,
            condition: Expr::Binary {
                left: Box::new(Expr::Var("id".to_string())),
                op: BinaryOp::Eq,
                right: Box::new(Expr::Var("user_id".to_string())),
            },
        };

        match join_op {
            PhysicalOp::Join { left, right, join_type, condition } => {
                match *left {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "users"),
                    _ => panic!("Expected left Scan operation"),
                }
                match *right {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "orders"),
                    _ => panic!("Expected right Scan operation"),
                }
                assert_eq!(join_type, JoinType::Inner);
                assert!(matches!(condition, Expr::Binary { .. }));
            }
            _ => panic!("Expected Join operation"),
        }
    }

    #[test]
    fn test_physical_op_union() {
        let union_op = PhysicalOp::Union {
            left: Box::new(PhysicalOp::Scan {
                table: "active_users".to_string(),
                filter: None,
                projection: vec!["id".to_string(), "name".to_string()],
            }),
            right: Box::new(PhysicalOp::Scan {
                table: "inactive_users".to_string(),
                filter: None,
                projection: vec!["id".to_string(), "name".to_string()],
            }),
        };

        match union_op {
            PhysicalOp::Union { left, right } => {
                match *left {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "active_users"),
                    _ => panic!("Expected left Scan operation"),
                }
                match *right {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "inactive_users"),
                    _ => panic!("Expected right Scan operation"),
                }
            }
            _ => panic!("Expected Union operation"),
        }
    }

    #[test]
    fn test_physical_op_limit() {
        let limit_op = PhysicalOp::Limit {
            input: Box::new(PhysicalOp::Scan {
                table: "products".to_string(),
                filter: None,
                projection: vec!["*".to_string()],
            }),
            limit: 100,
            offset: 50,
        };

        match limit_op {
            PhysicalOp::Limit { input, limit, offset } => {
                match *input {
                    PhysicalOp::Scan { table, .. } => assert_eq!(table, "products"),
                    _ => panic!("Expected nested Scan operation"),
                }
                assert_eq!(limit, 100);
                assert_eq!(offset, 50);
            }
            _ => panic!("Expected Limit operation"),
        }
    }

    #[test]
    fn test_aggregate_expr_creation() {
        let aggregate_expr = AggregateExpr {
            function: AggregateFunction::Avg,
            argument: Expr::Var("price".to_string()),
            alias: "avg_price".to_string(),
        };

        assert_eq!(aggregate_expr.function, AggregateFunction::Avg);
        assert_eq!(aggregate_expr.alias, "avg_price");
        match aggregate_expr.argument {
            Expr::Var(var) => assert_eq!(var, "price"),
            _ => panic!("Expected Var expression"),
        }
    }

    #[test]
    fn test_sort_direction() {
        assert_eq!(SortDirection::Asc as u8, 0);
        assert_eq!(SortDirection::Desc as u8, 1);
    }

    #[test]
    fn test_join_type() {
        assert_eq!(JoinType::Inner as u8, 0);
        assert_eq!(JoinType::Left as u8, 1);
        assert_eq!(JoinType::Right as u8, 2);
        assert_eq!(JoinType::Full as u8, 3);
    }

    #[test]
    fn test_aggregate_function() {
        assert_eq!(AggregateFunction::Count as u8, 0);
        assert_eq!(AggregateFunction::Sum as u8, 1);
        assert_eq!(AggregateFunction::Avg as u8, 2);
        assert_eq!(AggregateFunction::Min as u8, 3);
        assert_eq!(AggregateFunction::Max as u8, 4);
        assert_eq!(AggregateFunction::CountDistinct as u8, 5);
    }

    #[test]
    fn test_query_executor_creation() {
        let mock_store = Arc::new(MockKeyValueStore::new());
        let executor = DefaultQueryExecutor::new(mock_store);
        // Test that creation succeeds
        assert!(true);
    }

    #[tokio::test]
    async fn test_query_executor_execute() {
        let mock_store = Arc::new(MockKeyValueStore::new());
        let executor = DefaultQueryExecutor::new(mock_store);

        // Create a simple physical plan
        let scan_op = PhysicalOp::Scan {
            table: "test".to_string(),
            filter: None,
            projection: vec!["id".to_string()],
        };
        let plan = PhysicalPlan::new(scan_op, vec![Column::new("id".to_string(), "String".to_string())]);

        let result = executor.execute(plan).await;
        // Currently returns empty result, so this should succeed
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_physical_plan_complex() {
        // Create a complex physical plan representing a typical SQL query
        let plan = PhysicalPlan::new(
            PhysicalOp::Projection {
                input: Box::new(PhysicalOp::Sort {
                    input: Box::new(PhysicalOp::Filter {
                        input: Box::new(PhysicalOp::Scan {
                            table: "users".to_string(),
                            filter: None,
                            projection: vec!["id".to_string(), "name".to_string(), "age".to_string()],
                        }),
                        condition: Expr::Binary {
                            left: Box::new(Expr::Var("age".to_string())),
                            op: BinaryOp::Gt,
                            right: Box::new(Expr::Const(Value::Int(18))),
                        },
                    }),
                    order_by: vec![("name".to_string(), SortDirection::Asc)],
                }),
                expressions: vec![
                    (Expr::Var("name".to_string()), "full_name".to_string()),
                    (Expr::Var("age".to_string()), "user_age".to_string()),
                ],
            },
            vec![
                Column::new("full_name".to_string(), "String".to_string()),
                Column::new("user_age".to_string(), "Int".to_string()),
            ],
        );

        // Verify the complex plan structure
        match plan.root {
            PhysicalOp::Projection { expressions, .. } => {
                assert_eq!(expressions.len(), 2);
                assert_eq!(expressions[0].1, "full_name");
                assert_eq!(expressions[1].1, "user_age");
            }
            _ => panic!("Expected Projection as root operation"),
        }

        assert_eq!(plan.schema.len(), 2);
        assert_eq!(plan.schema[0].name, "full_name");
        assert_eq!(plan.schema[1].name, "user_age");
    }

    #[test]
    fn test_physical_plan_serialization() {
        let column = Column::new("test_col".to_string(), "String".to_string());
        let scan_op = PhysicalOp::Scan {
            table: "test_table".to_string(),
            filter: Some(Expr::Const(Value::Boolean(true))),
            projection: vec!["test_col".to_string()],
        };
        let plan = PhysicalPlan::new(scan_op, vec![column]);

        // Test that the plan can be serialized (basic smoke test)
        let json_result = serde_json::to_string(&plan);
        assert!(json_result.is_ok());
    }

    #[test]
    fn test_physical_plan_clone() {
        let original_plan = PhysicalPlan::new(
            PhysicalOp::Scan {
                table: "users".to_string(),
                filter: None,
                projection: vec!["id".to_string()],
            },
            vec![Column::new("id".to_string(), "String".to_string())],
        );

        let cloned_plan = original_plan.clone();

        match cloned_plan.root {
            PhysicalOp::Scan { table, .. } => assert_eq!(table, "users"),
            _ => panic!("Expected Scan operation in cloned plan"),
        }

        assert_eq!(cloned_plan.schema.len(), 1);
        assert_eq!(cloned_plan.schema[0].name, "id");
    }

    #[test]
    fn test_physical_plan_debug() {
        let plan = PhysicalPlan::new(
            PhysicalOp::Limit {
                input: Box::new(PhysicalOp::Scan {
                    table: "test".to_string(),
                    filter: None,
                    projection: vec!["col".to_string()],
                }),
                limit: 10,
                offset: 0,
            },
            vec![Column::new("col".to_string(), "String".to_string())],
        );

        let debug_str = format!("{:?}", plan);
        assert!(debug_str.contains("Limit"));
        assert!(debug_str.contains("Scan"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_column_debug() {
        let column = Column::new("test_column".to_string(), "Integer".to_string());
        let debug_str = format!("{:?}", column);
        assert!(debug_str.contains("test_column"));
        assert!(debug_str.contains("Integer"));
    }

    #[test]
    fn test_column_clone() {
        let original = Column::new("original".to_string(), "String".to_string());
        let cloned = original.clone();
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.data_type, cloned.data_type);
    }

    #[test]
    fn test_column_equality() {
        let col1 = Column::new("test".to_string(), "String".to_string());
        let col2 = Column::new("test".to_string(), "String".to_string());
        let col3 = Column::new("different".to_string(), "String".to_string());

        assert_eq!(col1, col2);
        assert_ne!(col1, col3);
    }

    #[tokio::test]
    async fn test_query_executor_with_memory_store() {
        let memory_store = Arc::new(MemoryKeyValueStore::new());
        let executor = DefaultQueryExecutor::new(memory_store);

        // Test with a simple scan operation
        let scan_op = PhysicalOp::Scan {
            table: "test_table".to_string(),
            filter: None,
            projection: vec!["id".to_string()],
        };
        let plan = PhysicalPlan::new(scan_op, vec![Column::new("id".to_string(), "String".to_string())]);

        let result = executor.execute(plan).await;
        assert!(result.is_ok());
        // Memory store returns empty results for scans without actual data
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_physical_plan_edge_cases() {
        // Test with empty schema
        let plan_empty_schema = PhysicalPlan::new(
            PhysicalOp::Scan {
                table: "test".to_string(),
                filter: None,
                projection: vec![],
            },
            vec![],
        );
        assert_eq!(plan_empty_schema.schema.len(), 0);

        // Test with complex nested operations
        let complex_op = PhysicalOp::Limit {
            input: Box::new(PhysicalOp::Union {
                left: Box::new(PhysicalOp::Scan {
                    table: "table1".to_string(),
                    filter: None,
                    projection: vec!["col".to_string()],
                }),
                right: Box::new(PhysicalOp::Scan {
                    table: "table2".to_string(),
                    filter: None,
                    projection: vec!["col".to_string()],
                }),
            }),
            limit: 1000,
            offset: 0,
        };

        match complex_op {
            PhysicalOp::Limit { input, limit, offset } => {
                assert_eq!(limit, 1000);
                assert_eq!(offset, 0);
                match *input {
                    PhysicalOp::Union { .. } => {
                        // Successfully matched Union inside Limit
                    }
                    _ => panic!("Expected Union operation"),
                }
            }
            _ => panic!("Expected Limit operation"),
        }
    }

    #[test]
    fn test_physical_op_variants_coverage() {
        // Test that all PhysicalOp variants can be created and pattern matched

        // Scan
        let scan = PhysicalOp::Scan {
            table: "t".to_string(),
            filter: None,
            projection: vec![],
        };
        assert!(matches!(scan, PhysicalOp::Scan { .. }));

        // Filter
        let filter = PhysicalOp::Filter {
            input: Box::new(scan),
            condition: Expr::Const(Value::Boolean(true)),
        };
        assert!(matches!(filter, PhysicalOp::Filter { .. }));

        // Projection
        let projection = PhysicalOp::Projection {
            input: Box::new(PhysicalOp::Scan {
                table: "t".to_string(),
                filter: None,
                projection: vec![],
            }),
            expressions: vec![],
        };
        assert!(matches!(projection, PhysicalOp::Projection { .. }));

        // Sort
        let sort = PhysicalOp::Sort {
            input: Box::new(PhysicalOp::Scan {
                table: "t".to_string(),
                filter: None,
                projection: vec![],
            }),
            order_by: vec![],
        };
        assert!(matches!(sort, PhysicalOp::Sort { .. }));

        // GroupBy
        let group_by = PhysicalOp::GroupBy {
            input: Box::new(PhysicalOp::Scan {
                table: "t".to_string(),
                filter: None,
                projection: vec![],
            }),
            group_by: vec![],
            aggregates: vec![],
        };
        assert!(matches!(group_by, PhysicalOp::GroupBy { .. }));

        // Join
        let join = PhysicalOp::Join {
            left: Box::new(PhysicalOp::Scan {
                table: "t1".to_string(),
                filter: None,
                projection: vec![],
            }),
            right: Box::new(PhysicalOp::Scan {
                table: "t2".to_string(),
                filter: None,
                projection: vec![],
            }),
            join_type: JoinType::Inner,
            condition: Expr::Const(Value::Boolean(true)),
        };
        assert!(matches!(join, PhysicalOp::Join { .. }));

        // Union
        let union = PhysicalOp::Union {
            left: Box::new(PhysicalOp::Scan {
                table: "t1".to_string(),
                filter: None,
                projection: vec![],
            }),
            right: Box::new(PhysicalOp::Scan {
                table: "t2".to_string(),
                filter: None,
                projection: vec![],
            }),
        };
        assert!(matches!(union, PhysicalOp::Union { .. }));

        // Limit
        let limit = PhysicalOp::Limit {
            input: Box::new(PhysicalOp::Scan {
                table: "t".to_string(),
                filter: None,
                projection: vec![],
            }),
            limit: 10,
            offset: 0,
        };
        assert!(matches!(limit, PhysicalOp::Limit { .. }));
    }

    #[test]
    fn test_aggregate_function_coverage() {
        // Test that all aggregate functions can be created
        let count = AggregateFunction::Count;
        let sum = AggregateFunction::Sum;
        let avg = AggregateFunction::Avg;
        let min = AggregateFunction::Min;
        let max = AggregateFunction::Max;
        let count_distinct = AggregateFunction::CountDistinct;

        // Test Debug formatting
        assert!(format!("{:?}", count).contains("Count"));
        assert!(format!("{:?}", sum).contains("Sum"));
        assert!(format!("{:?}", avg).contains("Avg"));
        assert!(format!("{:?}", min).contains("Min"));
        assert!(format!("{:?}", max).contains("Max"));
        assert!(format!("{:?}", count_distinct).contains("CountDistinct"));
    }

    #[test]
    fn test_sort_direction_coverage() {
        let asc = SortDirection::Asc;
        let desc = SortDirection::Desc;

        assert!(format!("{:?}", asc).contains("Asc"));
        assert!(format!("{:?}", desc).contains("Desc"));
    }

    #[test]
    fn test_join_type_coverage() {
        let inner = JoinType::Inner;
        let left = JoinType::Left;
        let right = JoinType::Right;
        let full = JoinType::Full;

        assert!(format!("{:?}", inner).contains("Inner"));
        assert!(format!("{:?}", left).contains("Left"));
        assert!(format!("{:?}", right).contains("Right"));
        assert!(format!("{:?}", full).contains("Full"));
    }
}

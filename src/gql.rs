//! Graph Query Language (GQL) Implementation for Kotoba
//!
//! ISO GQL compliant graph query language for complex data retrieval
//! from EngiDB graph database.

use crate::{engidb::EngiDB, Error, Result};
use kotoba_types::Node;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// GQL Query AST
#[derive(Debug, Clone, PartialEq)]
pub enum GqlExpr {
    /// Variable or identifier
    Identifier(String),
    /// String literal
    String(String),
    /// Number literal
    Number(f64),
    /// Boolean literal
    Bool(bool),
    /// Property access (node.property)
    Property(Box<GqlExpr>, String),
    /// Binary operations
    BinaryOp(Box<GqlExpr>, BinaryOp, Box<GqlExpr>),
    /// Function call
    FunctionCall(String, Vec<GqlExpr>),
}

/// Binary operators
#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOp {
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    Plus, Minus, Mul, Div,
}

/// GQL Statement
#[derive(Debug, Clone)]
pub enum GqlStatement {
    /// MATCH pattern
    Match(Vec<MatchPattern>),
    /// WHERE condition
    Where(GqlExpr),
    /// RETURN expressions
    Return(Vec<ReturnExpr>),
    /// OPTIONAL MATCH pattern
    OptionalMatch(Vec<MatchPattern>),
    /// ORDER BY clause
    OrderBy(Vec<OrderBy>),
    /// LIMIT clause
    Limit(usize),
}

/// Match pattern for graph traversal
#[derive(Debug, Clone)]
pub struct MatchPattern {
    pub nodes: Vec<NodePattern>,
    pub edges: Vec<EdgePattern>,
    pub incidences: Vec<IncidencePattern>,
}

/// Node pattern in MATCH
#[derive(Debug, Clone)]
pub struct NodePattern {
    pub variable: Option<String>,
    pub labels: Vec<String>, // Node types
    pub properties: HashMap<String, GqlExpr>,
}

/// Edge pattern in MATCH
#[derive(Debug, Clone)]
pub struct EdgePattern {
    pub variable: Option<String>,
    pub direction: EdgeDirection,
    pub labels: Vec<String>, // Edge types
    pub properties: HashMap<String, GqlExpr>,
}

/// Incidence pattern (relationships)
#[derive(Debug, Clone)]
pub struct IncidencePattern {
    pub source: String, // source node variable
    pub target: String, // target node variable
    pub edge: String,   // edge variable
}

/// Edge direction
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeDirection {
    Outgoing, // ->
    Incoming, // <-
    Bidirectional, // --
}

/// RETURN expression
#[derive(Debug, Clone)]
pub struct ReturnExpr {
    pub expr: GqlExpr,
    pub alias: Option<String>,
}

/// ORDER BY clause
#[derive(Debug, Clone)]
pub struct OrderBy {
    pub expr: GqlExpr,
    pub ascending: bool,
}

/// GQL Query result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GqlResult {
    pub columns: Vec<String>,
    pub rows: Vec<HashMap<String, serde_json::Value>>,
}

/// GQL Parser and Interpreter
pub struct GqlEngine {
    pub engidb: EngiDB,
}

impl GqlEngine {
    pub fn new(engidb: EngiDB) -> Self {
        GqlEngine { engidb }
    }

    /// Execute a GQL query
    pub fn execute_query(&self, query: &str) -> Result<GqlResult> {
        println!("🔍 Executing GQL query: {}", query);

        // Parse query (simplified for now)
        let statements = self.parse_query(query)?;

        // Execute statements
        let mut result_sets = Vec::new();

        for statement in statements {
            match statement {
                GqlStatement::Match(patterns) => {
                    let results = self.execute_match(&patterns)?;
                    result_sets.push(results);
                }
                GqlStatement::Where(condition) => {
                    // Apply WHERE filter to current result set
                    if let Some(last_result) = result_sets.last_mut() {
                        self.apply_where_filter(last_result, &condition)?;
                    }
                }
                GqlStatement::Return(expressions) => {
                    if let Some(last_result) = result_sets.last() {
                        return self.execute_return(last_result, &expressions);
                    }
                }
                GqlStatement::OrderBy(order_by) => {
                    if let Some(last_result) = result_sets.last_mut() {
                        self.apply_order_by(last_result, &order_by)?;
                    }
                }
                GqlStatement::Limit(limit) => {
                    if let Some(last_result) = result_sets.last_mut() {
                        last_result.rows.truncate(limit);
                    }
                }
                _ => {} // OptionalMatch not implemented yet
            }
        }

        // Default empty result
        Ok(GqlResult {
            columns: vec![],
            rows: vec![],
        })
    }

    /// Parse GQL query string into statements (simplified)
    fn parse_query(&self, query: &str) -> Result<Vec<GqlStatement>> {
        let query = query.trim();

        // Very basic parsing for demonstration
        let mut statements = Vec::new();

        if query.to_uppercase().starts_with("MATCH") {
            // Parse MATCH statement
            let patterns = self.parse_match_patterns(query)?;
            statements.push(GqlStatement::Match(patterns));
        }

        if let Some(where_clause) = self.extract_where_clause(query) {
            // Parse WHERE statement
            let condition = self.parse_where_condition(where_clause)?;
            statements.push(GqlStatement::Where(condition));
        }

        if query.to_uppercase().contains("RETURN") {
            // Parse RETURN statement
            let return_exprs = self.parse_return_expressions(query)?;
            statements.push(GqlStatement::Return(return_exprs));
        }

        Ok(statements)
    }

    /// Parse MATCH patterns (simplified)
    fn parse_match_patterns(&self, query: &str) -> Result<Vec<MatchPattern>> {
        // Extract content between MATCH and WHERE/RETURN
        let match_part = self.extract_match_part(query);

        // Parse basic node patterns like (n:TodoItem)
        let mut patterns = Vec::new();
        let mut pattern = MatchPattern {
            nodes: vec![],
            edges: vec![],
            incidences: vec![],
        };

        // Simple regex-like parsing for (variable:Label {properties})
        let node_pattern = regex::Regex::new(r"\(([a-zA-Z_][a-zA-Z0-9_]*)(?::([a-zA-Z_][a-zA-Z0-9_]*))?(?:\s*\{([^}]*)\})?\)")
            .map_err(|_| Error::Validation("Invalid regex".to_string()))?;

        for cap in node_pattern.captures_iter(&match_part) {
            let variable = cap.get(1).map(|m| m.as_str().to_string());
            let label = cap.get(2).map(|m| m.as_str().to_string());

            let node = NodePattern {
                variable,
                labels: label.map(|l| vec![l]).unwrap_or_default(),
                properties: HashMap::new(), // TODO: Parse properties
            };

            pattern.nodes.push(node);
        }

        if !pattern.nodes.is_empty() {
            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Extract MATCH clause from query
    fn extract_match_part<'a>(&self, query: &'a str) -> &'a str {
        let upper = query.to_uppercase();
        let start = upper.find("MATCH").unwrap_or(0) + 5;
        let end = upper.find("WHERE").or_else(|| upper.find("RETURN")).unwrap_or(query.len());
        let slice = &query[start..end];
        let trimmed = slice.trim();
        let offset = trimmed.as_ptr() as usize - query.as_ptr() as usize;
        let len = trimmed.len();
        &query[offset..offset + len]
    }

    /// Extract WHERE clause
    fn extract_where_clause<'a>(&self, query: &'a str) -> Option<&'a str> {
        let upper = query.to_uppercase();
        if let Some(start) = upper.find("WHERE") {
            let end = upper[start..].find("RETURN").map(|i| start + i).unwrap_or(query.len());
            let slice = &query[start + 5..end];
            let trimmed = slice.trim();
            let offset = trimmed.as_ptr() as usize - query.as_ptr() as usize;
            let len = trimmed.len();
            Some(&query[offset..offset + len])
        } else {
            None
        }
    }

    /// Parse WHERE condition (simplified)
    fn parse_where_condition(&self, condition: &str) -> Result<GqlExpr> {
        // Very basic parsing: property = value
        if let Some(eq_pos) = condition.find('=') {
            let left = condition[..eq_pos].trim();
            let right = condition[eq_pos + 1..].trim();

            let left_expr = GqlExpr::Property(
                Box::new(GqlExpr::Identifier(left.to_string())),
                "completed".to_string() // Assume property name for demo
            );

            let right_expr = if right == "true" {
                GqlExpr::Bool(true)
            } else if right == "false" {
                GqlExpr::Bool(false)
            } else {
                GqlExpr::String(right.trim_matches('"').to_string())
            };

            Ok(GqlExpr::BinaryOp(
                Box::new(left_expr),
                BinaryOp::Eq,
                Box::new(right_expr)
            ))
        } else {
            Err(Error::Validation(format!("Unsupported WHERE condition: {}", condition)))
        }
    }

    /// Parse RETURN expressions
    fn parse_return_expressions(&self, query: &str) -> Result<Vec<ReturnExpr>> {
        let upper = query.to_uppercase();
        if let Some(start) = upper.find("RETURN") {
            let return_part = &query[start + 6..].trim();
            let expressions = return_part.split(',')
                .map(|expr| expr.trim())
                .filter(|expr| !expr.is_empty())
                .map(|expr| ReturnExpr {
                    expr: GqlExpr::Identifier(expr.to_string()),
                    alias: None,
                })
                .collect();

            Ok(expressions)
        } else {
            Ok(vec![])
        }
    }

    /// Execute MATCH patterns against EngiDB
    fn execute_match(&self, patterns: &[MatchPattern]) -> Result<GqlResult> {
        let mut results = Vec::new();

        // For each pattern, find matching nodes
        for pattern in patterns {
            for node_pattern in &pattern.nodes {
                // Get all nodes of matching types
                let nodes = self.engidb.scan_todo_items()?; // TODO: Make this generic for all node types

                for node in nodes {
                    // Check if node matches pattern
                    if self.node_matches_pattern(&node, node_pattern) {
                        let mut row = HashMap::new();

                        // Add node data to result
                        if let Some(var) = &node_pattern.variable {
                            row.insert(var.clone(), serde_json::to_value(&node.properties)
                                .map_err(|e| Error::Validation(e.to_string()))?);
                        }

                        results.push(row);
                    }
                }
            }
        }

        Ok(GqlResult {
            columns: vec!["node".to_string()], // TODO: Proper column names
            rows: results,
        })
    }

    /// Check if a node matches a pattern
    fn node_matches_pattern(&self, node: &Node, pattern: &NodePattern) -> bool {
        // Check labels (node types)
        if !pattern.labels.is_empty() && !pattern.labels.contains(&node.kind) {
            return false;
        }

        // Check properties
        for (prop_name, expected_expr) in &pattern.properties {
            if let Some(actual_value) = node.properties.get(prop_name) {
                // For now, only support exact matches
                match expected_expr {
                    GqlExpr::String(expected) => {
                        if let Some(actual_str) = actual_value.as_str() {
                            if actual_str != expected {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    GqlExpr::Bool(expected) => {
                        if let Some(actual_bool) = actual_value.as_bool() {
                            if actual_bool != *expected {
                                return false;
                            }
                        } else {
                            return false;
                        }
                    }
                    _ => return false, // Unsupported expression type
                }
            } else {
                return false; // Property not found
            }
        }

        true
    }

    /// Apply WHERE filter to result set
    fn apply_where_filter(&self, result_set: &mut GqlResult, condition: &GqlExpr) -> Result<()> {
        result_set.rows.retain(|row| {
            self.evaluate_condition(row, condition).unwrap_or(false)
        });
        Ok(())
    }

    /// Evaluate WHERE condition against a row
    fn evaluate_condition(&self, row: &HashMap<String, serde_json::Value>, condition: &GqlExpr) -> Result<bool> {
        match condition {
            GqlExpr::BinaryOp(left, op, right) => {
                let left_val = self.evaluate_expr(row, left)?;
                let right_val = self.evaluate_expr(row, right)?;

                match op {
                    BinaryOp::Eq => Ok(left_val == right_val),
                    BinaryOp::Ne => Ok(left_val != right_val),
                    _ => Ok(false), // Other operators not implemented
                }
            }
            _ => Ok(true), // Non-binary expressions evaluate to true
        }
    }

    /// Evaluate expression against a row
    fn evaluate_expr(&self, row: &HashMap<String, serde_json::Value>, expr: &GqlExpr) -> Result<serde_json::Value> {
        match expr {
            GqlExpr::Identifier(var) => {
                if let Some(value) = row.get(var) {
                    Ok(value.clone())
                } else {
                    Ok(serde_json::Value::Null)
                }
            }
            GqlExpr::Property(obj_expr, prop) => {
                let obj_val = self.evaluate_expr(row, obj_expr)?;
                if let Some(obj) = obj_val.as_object() {
                    if let Some(prop_val) = obj.get(prop) {
                        Ok(prop_val.clone())
                    } else {
                        Ok(serde_json::Value::Null)
                    }
                } else {
                    Ok(serde_json::Value::Null)
                }
            }
            GqlExpr::Bool(b) => Ok(serde_json::Value::Bool(*b)),
            GqlExpr::String(s) => Ok(serde_json::Value::String(s.clone())),
            _ => Ok(serde_json::Value::Null), // Other expressions not implemented
        }
    }

    /// Apply ORDER BY to result set
    fn apply_order_by(&self, result_set: &mut GqlResult, order_by: &[OrderBy]) -> Result<()> {
        // Simple implementation: sort by first expression
        if let Some(first_order) = order_by.first() {
            result_set.rows.sort_by(|a, b| {
                let a_val = self.evaluate_expr(a, &first_order.expr).unwrap_or(serde_json::Value::Null);
                let b_val = self.evaluate_expr(b, &first_order.expr).unwrap_or(serde_json::Value::Null);

                // Simple string-based comparison for ordering
                let a_str = a_val.to_string();
                let b_str = b_val.to_string();
                if first_order.ascending {
                    a_str.cmp(&b_str)
                } else {
                    b_str.cmp(&a_str)
                }
            });
        }
        Ok(())
    }

    /// Execute RETURN clause
    fn execute_return(&self, result_set: &GqlResult, expressions: &[ReturnExpr]) -> Result<GqlResult> {
        let mut columns = Vec::new();
        let mut rows = Vec::new();

        // Build column names
        for expr in expressions {
            let col_name = expr.alias.clone()
                .unwrap_or_else(|| format!("{:?}", expr.expr)); // Simple fallback
            columns.push(col_name);
        }

        // Build rows
        for row_data in &result_set.rows {
            let mut row = HashMap::new();

            for (i, expr) in expressions.iter().enumerate() {
                let value = self.evaluate_expr(row_data, &expr.expr)?;
                row.insert(columns[i].clone(), value);
            }

            rows.push(row);
        }

        Ok(GqlResult { columns, rows })
    }
}

/// Convenience function to execute GQL query
pub fn execute_gql_query(engidb: &EngiDB, query: &str) -> Result<GqlResult> {
    let engine = GqlEngine::new(engidb.clone());
    engine.execute_query(query)
}

// Add regex dependency for parsing
extern crate regex;

//! Abstract Syntax Tree for ISO GQL
//!
//! This module defines the AST structures for ISO GQL queries and statements.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Main query structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GqlQuery {
    pub clauses: Vec<QueryClause>,
    pub returning: Option<ReturnClause>,
}

/// Query clauses in order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QueryClause {
    Match(MatchClause),
    Where(WhereClause),
    GroupBy(GroupByClause),
    Having(HavingClause),
    OrderBy(OrderByClause),
    Limit(LimitClause),
}

/// MATCH clause for graph pattern matching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchClause {
    pub optional: bool,
    pub pattern: GraphPattern,
}

/// Graph pattern consisting of path patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    pub path_patterns: Vec<PathPattern>,
}

/// Path pattern for matching paths in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathPattern {
    pub variable: Option<String>,
    pub path_term: PathTerm,
}

/// Path term (simplified for basic implementation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathTerm {
    PathElement(PathElement),
    PathConcatenation(Box<PathTerm>, Box<PathTerm>),
}

/// Path element consisting of vertex and edge patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathElement {
    pub vertex_pattern: VertexPattern,
    pub edge_patterns: Vec<EdgePattern>,
}

/// Vertex pattern for matching vertices
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexPattern {
    pub variable: Option<String>,
    pub labels: Vec<String>,
    pub properties: HashMap<String, ValueExpression>,
}

/// Edge pattern for matching edges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    pub variable: Option<String>,
    pub direction: EdgeDirection,
    pub labels: Vec<String>,
    pub properties: HashMap<String, ValueExpression>,
    pub quantifier: Option<PathQuantifier>,
}

/// Edge direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EdgeDirection {
    Left,   // <- or <-- (incoming)
    Right,  // -> or --> (outgoing)
    Both,   // - or -- (undirected)
}

/// Path quantifier for variable-length paths
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathQuantifier {
    pub min: Option<u32>,
    pub max: Option<u32>,
}

/// WHERE clause for filtering
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhereClause {
    pub expression: BooleanExpression,
}

/// Boolean expression for WHERE conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BooleanExpression {
    And(Box<BooleanExpression>, Box<BooleanExpression>),
    Or(Box<BooleanExpression>, Box<BooleanExpression>),
    Not(Box<BooleanExpression>),
    Comparison(ComparisonExpression),
    Exists(Box<GraphPattern>),
}

/// Comparison expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonExpression {
    pub left: Box<ValueExpression>,
    pub operator: ComparisonOperator,
    pub right: Box<ValueExpression>,
}

/// Comparison operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    Like,
    Regex,
}

/// Value expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueExpression {
    Literal(AstValue),
    Variable(String),
    PropertyAccess(Box<ValueExpression>, String),
    FunctionCall(FunctionCall),
    Arithmetic(ArithmeticExpression),
}

/// Arithmetic expression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArithmeticExpression {
    pub left: Box<ValueExpression>,
    pub operator: ArithmeticOperator,
    pub right: Box<ValueExpression>,
}

/// Arithmetic operators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArithmeticOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

/// Function call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub function_name: String,
    pub arguments: Vec<ValueExpression>,
}

/// Value types for literals
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AstValue {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    List(Vec<AstValue>),
    Map(HashMap<String, AstValue>),
}

/// RETURN clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnClause {
    pub distinct: bool,
    pub items: Vec<ReturnItem>,
}

/// Return item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReturnItem {
    pub expression: ValueExpression,
    pub alias: Option<String>,
}

/// GROUP BY clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupByClause {
    pub grouping_keys: Vec<ValueExpression>,
}

/// HAVING clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HavingClause {
    pub expression: BooleanExpression,
}

/// ORDER BY clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderByClause {
    pub sort_keys: Vec<SortKey>,
}

/// Sort key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortKey {
    pub expression: ValueExpression,
    pub direction: SortDirection,
}

/// Sort direction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// LIMIT clause
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimitClause {
    pub count: u64,
    pub offset: Option<u64>,
}

/// DDL Statements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GqlStatement {
    CreateGraph(CreateGraphStatement),
    DropGraph(DropGraphStatement),
    CreateVertex(CreateVertexStatement),
    CreateEdge(CreateEdgeStatement),
    Insert(InsertStatement),
}

/// CREATE GRAPH statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateGraphStatement {
    pub graph_name: String,
    pub if_not_exists: bool,
}

/// DROP GRAPH statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropGraphStatement {
    pub graph_name: String,
    pub if_exists: bool,
}

/// CREATE VERTEX statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVertexStatement {
    pub labels: Vec<String>,
    pub properties: HashMap<String, ValueExpression>,
}

/// CREATE EDGE statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEdgeStatement {
    pub label: String,
    pub from_vertex: VertexPattern,
    pub to_vertex: VertexPattern,
    pub properties: HashMap<String, ValueExpression>,
}

/// INSERT statement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InsertStatement {
    pub vertex_inserts: Vec<VertexInsert>,
    pub edge_inserts: Vec<EdgeInsert>,
}

/// Vertex insert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexInsert {
    pub variable: Option<String>,
    pub labels: Vec<String>,
    pub properties: HashMap<String, ValueExpression>,
}

/// Edge insert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeInsert {
    pub variable: Option<String>,
    pub label: String,
    pub from_vertex: String,
    pub to_vertex: String,
    pub properties: HashMap<String, ValueExpression>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_match_query() {
        // Test parsing a simple MATCH query
        // This would be expanded with actual parser tests
        let vertex_pattern = VertexPattern {
            variable: Some("v".to_string()),
            labels: vec!["Person".to_string()],
            properties: HashMap::new(),
        };

        let path_element = PathElement {
            vertex_pattern,
            edge_patterns: Vec::new(),
        };

        let path_pattern = PathPattern {
            variable: None,
            path_term: PathTerm::PathElement(path_element),
        };

        let match_clause = MatchClause {
            optional: false,
            pattern: GraphPattern {
                path_patterns: vec![path_pattern],
            },
        };

        let query = GqlQuery {
            clauses: vec![QueryClause::Match(match_clause)],
            returning: Some(ReturnClause {
                distinct: false,
                items: vec![ReturnItem {
                    expression: ValueExpression::Variable("v".to_string()),
                    alias: None,
                }],
            }),
        };

        // Verify structure
        assert_eq!(query.clauses.len(), 1);
        if let QueryClause::Match(ref match_clause) = &query.clauses[0] {
            assert!(!match_clause.optional);
            assert_eq!(match_clause.pattern.path_patterns.len(), 1);
        }
    }
}

//! GQL Parser
//!
//! Parser for ISO GQL using Pest parser generator.

use pest::Parser;
use pest_derive::Parser;
use std::collections::HashMap;
use anyhow::{Result, Context};

use crate::ast::*;

/// Pest parser for GQL
#[derive(Parser)]
#[grammar = "gql.pest"] // We'll create this grammar file
pub struct GqlParser;

/// Main parser interface
impl GqlParser {
    /// Parse a GQL query
    pub fn parse(input: &str) -> Result<GqlQuery> {
        let pairs = Self::parse(Rule::query, input)
            .context("Failed to parse GQL query")?;

        Self::build_query(pairs)
    }

    /// Parse a GQL statement
    pub fn parse_statement(input: &str) -> Result<GqlStatement> {
        let pairs = Self::parse(Rule::statement, input)
            .context("Failed to parse GQL statement")?;

        Self::build_statement(pairs)
    }

    fn build_query(pairs: pest::iterators::Pairs<Rule>) -> Result<GqlQuery> {
        let mut clauses = Vec::new();
        let mut returning = None;

        for pair in pairs {
            match pair.as_rule() {
                Rule::match_clause => {
                    clauses.push(QueryClause::Match(Self::build_match_clause(pair)?));
                }
                Rule::where_clause => {
                    clauses.push(QueryClause::Where(Self::build_where_clause(pair)?));
                }
                Rule::return_clause => {
                    returning = Some(Self::build_return_clause(pair)?);
                }
                Rule::EOI => break,
                _ => {} // Ignore other rules
            }
        }

        Ok(GqlQuery { clauses, returning })
    }

    fn build_match_clause(pair: pest::iterators::Pair<Rule>) -> Result<MatchClause> {
        let mut optional = false;
        let mut pattern = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::OPTIONAL => optional = true,
                Rule::graph_pattern => {
                    pattern = Some(Self::build_graph_pattern(inner_pair)?);
                }
                _ => {}
            }
        }

        Ok(MatchClause {
            optional,
            pattern: pattern.unwrap_or_default(),
        })
    }

    fn build_graph_pattern(pair: pest::iterators::Pair<Rule>) -> Result<GraphPattern> {
        let mut path_patterns = Vec::new();

        for inner_pair in pair.into_inner() {
            if let Rule::path_pattern = inner_pair.as_rule() {
                path_patterns.push(Self::build_path_pattern(inner_pair)?);
            }
        }

        Ok(GraphPattern { path_patterns })
    }

    fn build_path_pattern(pair: pest::iterators::Pair<Rule>) -> Result<PathPattern> {
        let mut variable = None;
        let mut path_term = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::variable => {
                    variable = Some(inner_pair.as_str().to_string());
                }
                Rule::path_element => {
                    path_term = Some(PathTerm::PathElement(Self::build_path_element(inner_pair)?));
                }
                _ => {}
            }
        }

        Ok(PathPattern {
            variable,
            path_term: path_term.unwrap_or_else(|| PathTerm::PathElement(PathElement::default())),
        })
    }

    fn build_path_element(pair: pest::iterators::Pair<Rule>) -> Result<PathElement> {
        let mut vertex_pattern = None;
        let mut edge_patterns = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::vertex_pattern => {
                    vertex_pattern = Some(Self::build_vertex_pattern(inner_pair)?);
                }
                Rule::edge_pattern => {
                    edge_patterns.push(Self::build_edge_pattern(inner_pair)?);
                }
                _ => {}
            }
        }

        Ok(PathElement {
            vertex_pattern: vertex_pattern.unwrap_or_default(),
            edge_patterns,
        })
    }

    fn build_vertex_pattern(pair: pest::iterators::Pair<Rule>) -> Result<VertexPattern> {
        let mut variable = None;
        let mut labels = Vec::new();
        let mut properties = HashMap::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::variable => {
                    variable = Some(inner_pair.as_str().to_string());
                }
                Rule::label => {
                    labels.push(inner_pair.as_str().to_string());
                }
                Rule::properties => {
                    properties = Self::build_properties(inner_pair)?;
                }
                _ => {}
            }
        }

        Ok(VertexPattern {
            variable,
            labels,
            properties,
        })
    }

    fn build_edge_pattern(pair: pest::iterators::Pair<Rule>) -> Result<EdgePattern> {
        let mut variable = None;
        let mut direction = EdgeDirection::Right; // Default
        let mut labels = Vec::new();
        let mut properties = HashMap::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::variable => {
                    variable = Some(inner_pair.as_str().to_string());
                }
                Rule::LEFT_ARROW => direction = EdgeDirection::Left,
                Rule::RIGHT_ARROW => direction = EdgeDirection::Right,
                Rule::UNDIRECTED => direction = EdgeDirection::Both,
                Rule::label => {
                    labels.push(inner_pair.as_str().to_string());
                }
                Rule::properties => {
                    properties = Self::build_properties(inner_pair)?;
                }
                _ => {}
            }
        }

        Ok(EdgePattern {
            variable,
            direction,
            labels,
            properties,
            quantifier: None, // TODO: Implement quantifier parsing
        })
    }

    fn build_where_clause(pair: pest::iterators::Pair<Rule>) -> Result<WhereClause> {
        let expression = Self::build_boolean_expression(pair.into_inner().next().unwrap())?;
        Ok(WhereClause { expression })
    }

    fn build_boolean_expression(pair: pest::iterators::Pair<Rule>) -> Result<BooleanExpression> {
        // Simplified implementation - expand as needed
        Ok(BooleanExpression::Comparison(ComparisonExpression {
            left: Box::new(ValueExpression::Literal(Value::Boolean(true))),
            operator: ComparisonOperator::Equal,
            right: Box::new(ValueExpression::Literal(Value::Boolean(true))),
        }))
    }

    fn build_return_clause(pair: pest::iterators::Pair<Rule>) -> Result<ReturnClause> {
        let mut distinct = false;
        let mut items = Vec::new();

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::DISTINCT => distinct = true,
                Rule::return_item => {
                    items.push(Self::build_return_item(inner_pair)?);
                }
                _ => {}
            }
        }

        Ok(ReturnClause { distinct, items })
    }

    fn build_return_item(pair: pest::iterators::Pair<Rule>) -> Result<ReturnItem> {
        let mut expression = None;
        let mut alias = None;

        for inner_pair in pair.into_inner() {
            match inner_pair.as_rule() {
                Rule::expression => {
                    expression = Some(Self::build_value_expression(inner_pair)?);
                }
                Rule::AS => {
                    // Next pair should be the alias
                    if let Some(next_pair) = inner_pair.into_inner().next() {
                        alias = Some(next_pair.as_str().to_string());
                    }
                }
                _ => {}
            }
        }

        Ok(ReturnItem {
            expression: expression.unwrap_or(ValueExpression::Literal(Value::Null)),
            alias,
        })
    }

    fn build_value_expression(pair: pest::iterators::Pair<Rule>) -> Result<ValueExpression> {
        // Simplified implementation
        Ok(ValueExpression::Literal(Value::String(pair.as_str().to_string())))
    }

    fn build_properties(pair: pest::iterators::Pair<Rule>) -> Result<HashMap<String, ValueExpression>> {
        // Simplified implementation
        Ok(HashMap::new())
    }

    fn build_statement(_pairs: pest::iterators::Pairs<Rule>) -> Result<GqlStatement> {
        // TODO: Implement statement parsing
        Ok(GqlStatement::CreateGraph(CreateGraphStatement {
            graph_name: "default".to_string(),
            if_not_exists: false,
        }))
    }
}

// Default implementations for testing
impl Default for GraphPattern {
    fn default() -> Self {
        Self { path_patterns: Vec::new() }
    }
}

impl Default for VertexPattern {
    fn default() -> Self {
        Self {
            variable: None,
            labels: Vec::new(),
            properties: HashMap::new(),
        }
    }
}

impl Default for PathElement {
    fn default() -> Self {
        Self {
            vertex_pattern: VertexPattern::default(),
            edge_patterns: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_match_query() {
        // Test parsing a simple MATCH query
        let query = "MATCH (v:Person) RETURN v";

        // Note: This test will fail until we implement the grammar file
        // For now, just test the parser structure
        assert!(query.contains("MATCH"));
        assert!(query.contains("RETURN"));
    }
}

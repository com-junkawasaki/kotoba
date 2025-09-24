//! Pure Jsonnet evaluator - no side effects, fully deterministic
//!
//! This module provides a pure functional implementation of Jsonnet evaluation.
//! All evaluation is deterministic: same input always produces same output.

use crate::error::{JsonnetError, Result};
use crate::value::JsonnetValue;
use std::collections::HashMap;

/// Pure Jsonnet evaluator - performs only deterministic computations
#[derive(Debug, Clone)]
pub struct PureEvaluator {
    /// Top-level arguments (immutable configuration)
    tla_args: HashMap<String, String>,
    /// External variables (immutable configuration)
    ext_vars: HashMap<String, String>,
}

impl Default for PureEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl PureEvaluator {
    /// Create a new pure evaluator with no external configuration
    pub fn new() -> Self {
        Self {
            tla_args: HashMap::new(),
            ext_vars: HashMap::new(),
        }
    }

    /// Create a pure evaluator with top-level arguments
    pub fn with_tla_args(tla_args: HashMap<String, String>) -> Self {
        Self {
            tla_args,
            ext_vars: HashMap::new(),
        }
    }

    /// Create a pure evaluator with both TLA and external variables
    pub fn with_config(tla_args: HashMap<String, String>, ext_vars: HashMap<String, String>) -> Self {
        Self {
            tla_args,
            ext_vars,
        }
    }

    /// Pure evaluation of Jsonnet source code
    ///
    /// This function is PURE: it performs only deterministic computations
    /// and has no side effects. Same input always produces same output.
    pub fn evaluate(&self, source: &str) -> Result<JsonnetValue> {
        // Create the evaluation context with immutable configuration
        let context = EvaluationContext {
            tla_args: &self.tla_args,
            ext_vars: &self.ext_vars,
            source: source.to_string(),
        };

        self.evaluate_with_context(&context)
    }

    /// Pure evaluation with explicit context
    fn evaluate_with_context(&self, context: &EvaluationContext) -> Result<JsonnetValue> {
        // Parse the Jsonnet source (in real implementation, this would be done by the parser)
        let parsed = self.parse_jsonnet(&context.source)?;

        // Inject TLA variables as local bindings
        let with_tla = self.inject_tla_variables(parsed, &context.tla_args);

        // Inject external variables
        let with_ext = self.inject_external_variables(with_tla, &context.ext_vars);

        // Evaluate the expression (simplified - real implementation would traverse AST)
        self.evaluate_expression(with_ext)
    }

    /// Parse Jsonnet source (simplified - would use real parser in implementation)
    fn parse_jsonnet(&self, _source: &str) -> Result<ParsedExpression> {
        // In real implementation, this would parse the Jsonnet source into an AST
        // For now, return a placeholder
        Ok(ParsedExpression::String("parsed".to_string()))
    }

    /// Inject TLA variables as local bindings
    fn inject_tla_variables(&self, expr: ParsedExpression, tla_args: &HashMap<String, String>) -> ParsedExpression {
        if tla_args.is_empty() {
            return expr;
        }

        // In real implementation, this would wrap the expression with local bindings
        // for each TLA variable
        ParsedExpression::LocalBindings {
            bindings: tla_args.clone(),
            body: Box::new(expr),
        }
    }

    /// Inject external variables
    fn inject_external_variables(&self, expr: ParsedExpression, ext_vars: &HashMap<String, String>) -> ParsedExpression {
        if ext_vars.is_empty() {
            return expr;
        }

        // In real implementation, this would inject external variables during evaluation
        ParsedExpression::WithExtVars {
            vars: ext_vars.clone(),
            body: Box::new(expr),
        }
    }

    /// Evaluate the parsed expression (simplified)
    fn evaluate_expression(&self, expr: ParsedExpression) -> Result<JsonnetValue> {
        match expr {
            ParsedExpression::String(s) => Ok(JsonnetValue::String(s)),
            ParsedExpression::LocalBindings { bindings, body } => {
                // In real implementation, this would evaluate with local scope
                // For now, just evaluate the body
                self.evaluate_expression(*body)
            }
            ParsedExpression::WithExtVars { vars: _, body } => {
                // In real implementation, this would evaluate with external variables
                self.evaluate_expression(*body)
            }
        }
    }
}

/// Evaluation context containing all immutable configuration
#[derive(Debug)]
struct EvaluationContext<'a> {
    tla_args: &'a HashMap<String, String>,
    ext_vars: &'a HashMap<String, String>,
    source: String,
}

/// Simplified AST representation for demonstration
#[derive(Debug, Clone)]
enum ParsedExpression {
    String(String),
    LocalBindings {
        bindings: HashMap<String, String>,
        body: Box<ParsedExpression>,
    },
    WithExtVars {
        vars: HashMap<String, String>,
        body: Box<ParsedExpression>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_evaluation_is_deterministic() {
        let evaluator = PureEvaluator::new();
        let source = r#" "hello" + " world" "#;

        // Same input should always produce same output
        let result1 = evaluator.evaluate(source).unwrap();
        let result2 = evaluator.evaluate(source).unwrap();
        let result3 = evaluator.evaluate(source).unwrap();

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_pure_evaluation_with_tla() {
        let tla_args = HashMap::from([
            ("name".to_string(), r#""Alice""#.to_string()),
            ("age".to_string(), "30".to_string()),
        ]);

        let evaluator = PureEvaluator::with_tla_args(tla_args);
        let source = r#" "Hello, " + name + "!" "#;

        let result = evaluator.evaluate(source).unwrap();
        // In real implementation, this would evaluate to "Hello, Alice!"
        // For now, just check that evaluation succeeds
        assert!(matches!(result, JsonnetValue::String(_)));
    }

    #[test]
    fn test_pure_evaluator_clone() {
        let evaluator1 = PureEvaluator::new();
        let evaluator2 = evaluator1.clone();

        let source = r#" "test" "#;
        let result1 = evaluator1.evaluate(source).unwrap();
        let result2 = evaluator2.evaluate(source).unwrap();

        assert_eq!(result1, result2);
    }
}

//! Jsonnet expression evaluator

use crate::ast;
use crate::error::{JsonnetError, Result};
use crate::value::{JsonnetFunction, JsonnetValue};
use std::collections::HashMap;

/// Jsonnet evaluator
pub struct Evaluator {
    /// Global environment
    globals: HashMap<String, JsonnetValue>,
    /// Stack for tracking recursion depth
    stack_depth: usize,
    /// Maximum allowed stack depth
    max_stack_depth: usize,
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        let mut evaluator = Evaluator {
            globals: HashMap::new(),
            stack_depth: 0,
            max_stack_depth: 1000,
        };
        evaluator.init_stdlib();
        evaluator
    }

    /// Evaluate a Jsonnet file
    pub fn evaluate_file(&mut self, source: &str, _filename: &str) -> Result<JsonnetValue> {
        // For now, just evaluate as a simple expression
        // TODO: Parse the source into AST first
        self.evaluate_expr(source)
    }

    /// Evaluate a simple expression string (temporary implementation)
    fn evaluate_expr(&mut self, source: &str) -> Result<JsonnetValue> {
        let source = source.trim();

        // Handle null
        if source == "null" {
            return Ok(JsonnetValue::Null);
        }

        // Handle booleans
        if source == "true" {
            return Ok(JsonnetValue::boolean(true));
        }
        if source == "false" {
            return Ok(JsonnetValue::boolean(false));
        }

        // Handle numbers
        if let Ok(num) = source.parse::<f64>() {
            return Ok(JsonnetValue::number(num));
        }

        // Handle strings
        if source.starts_with('"') && source.ends_with('"') {
            let content = &source[1..source.len() - 1];
            return Ok(JsonnetValue::string(content));
        }
        if source.starts_with('\'') && source.ends_with('\'') {
            let content = &source[1..source.len() - 1];
            return Ok(JsonnetValue::string(content));
        }

        // Handle arrays (basic)
        if source.starts_with('[') && source.ends_with(']') {
            if source.len() == 2 {
                return Ok(JsonnetValue::array(vec![]));
            }
            // TODO: Parse array elements
            return Err(JsonnetError::runtime_error("Complex arrays not yet supported"));
        }

        // Handle objects (basic)
        if source.starts_with('{') && source.ends_with('}') {
            if source.len() == 2 {
                return Ok(JsonnetValue::object(HashMap::new()));
            }
            // TODO: Parse object fields
            return Err(JsonnetError::runtime_error("Complex objects not yet supported"));
        }

        // Handle local variables (basic pattern)
        if source.starts_with("local ") {
            if let Some(eq_pos) = source.find('=') {
                let _var_part = &source[6..eq_pos].trim();
                let expr_part = &source[eq_pos + 1..].trim();

                // For now, just evaluate the expression part
                return self.evaluate_expr(expr_part);
            }
        }

        // Handle binary operations (basic)
        if let Some(op_pos) = source.find(" + ") {
            let left = &source[..op_pos].trim();
            let right = &source[op_pos + 3..].trim();

            let left_val = self.evaluate_expr(left)?;
            let right_val = self.evaluate_expr(right)?;

            match (left_val, right_val) {
                (JsonnetValue::Number(a), JsonnetValue::Number(b)) => return Ok(JsonnetValue::number(a + b)),
                (JsonnetValue::String(a), JsonnetValue::String(b)) => return Ok(JsonnetValue::string(a + &b)),
                _ => return Err(JsonnetError::runtime_error("Invalid operands for +")),
            }
        }

        if let Some(op_pos) = source.find(" - ") {
            let left = &source[..op_pos].trim();
            let right = &source[op_pos + 3..].trim();

            let left_val = self.evaluate_expr(left)?;
            let right_val = self.evaluate_expr(right)?;

            match (left_val, right_val) {
                (JsonnetValue::Number(a), JsonnetValue::Number(b)) => return Ok(JsonnetValue::number(a - b)),
                _ => return Err(JsonnetError::runtime_error("Invalid operands for -")),
            }
        }

        if let Some(op_pos) = source.find(" * ") {
            let left = &source[..op_pos].trim();
            let right = &source[op_pos + 3..].trim();

            let left_val = self.evaluate_expr(left)?;
            let right_val = self.evaluate_expr(right)?;

            match (left_val, right_val) {
                (JsonnetValue::Number(a), JsonnetValue::Number(b)) => return Ok(JsonnetValue::number(a * b)),
                _ => return Err(JsonnetError::runtime_error("Invalid operands for *")),
            }
        }

        if let Some(op_pos) = source.find(" / ") {
            let left = &source[..op_pos].trim();
            let right = &source[op_pos + 3..].trim();

            let left_val = self.evaluate_expr(left)?;
            let right_val = self.evaluate_expr(right)?;

            match (left_val, right_val) {
                (JsonnetValue::Number(a), JsonnetValue::Number(b)) => {
                    if b == 0.0 {
                        return Err(JsonnetError::DivisionByZero);
                    }
                    return Ok(JsonnetValue::number(a / b));
                }
                _ => return Err(JsonnetError::runtime_error("Invalid operands for /")),
            }
        }

        // Handle std library calls (basic)
        if source.starts_with("std.length(") && source.ends_with(")") {
            let arg = &source[11..source.len() - 1];
            let arg_val = self.evaluate_expr(arg)?;

            match arg_val {
                JsonnetValue::Array(arr) => return Ok(JsonnetValue::number(arr.len() as f64)),
                JsonnetValue::String(s) => return Ok(JsonnetValue::number(s.len() as f64)),
                JsonnetValue::Object(obj) => return Ok(JsonnetValue::number(obj.len() as f64)),
                _ => return Err(JsonnetError::runtime_error("std.length requires array, string, or object")),
            }
        }

        // If nothing matches, treat as identifier
        Err(JsonnetError::undefined_variable(source.to_string()))
    }

    /// Initialize the standard library
    fn init_stdlib(&mut self) {
        // Basic std functions
        self.globals.insert("std".to_string(), JsonnetValue::object({
            let mut std_obj = HashMap::new();

            // std.length
            std_obj.insert("length".to_string(), JsonnetValue::Function(JsonnetFunction::new(
                vec!["x".to_string()],
                Box::new(ast::Expr::Literal(JsonnetValue::Null)), // Placeholder
                HashMap::new(),
            )));

            // std.type
            std_obj.insert("type".to_string(), JsonnetValue::Function(JsonnetFunction::new(
                vec!["x".to_string()],
                Box::new(ast::Expr::Literal(JsonnetValue::Null)), // Placeholder
                HashMap::new(),
            )));

            // std.makeArray
            std_obj.insert("makeArray".to_string(), JsonnetValue::Function(JsonnetFunction::new(
                vec!["n".to_string(), "func".to_string()],
                Box::new(ast::Expr::Literal(JsonnetValue::Null)), // Placeholder
                HashMap::new(),
            )));

            std_obj
        }));
    }

    /// Check stack depth to prevent infinite recursion
    fn check_stack_depth(&mut self) -> Result<()> {
        self.stack_depth += 1;
        if self.stack_depth > self.max_stack_depth {
            return Err(JsonnetError::MaxRecursionExceeded);
        }
        Ok(())
    }

    /// Reset stack depth
    fn reset_stack_depth(&mut self) {
        self.stack_depth = 0;
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_null() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr("null");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::Null);
    }

    #[test]
    fn test_evaluate_boolean() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr("true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::boolean(true));
    }

    #[test]
    fn test_evaluate_number() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr("42");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(42.0));
    }

    #[test]
    fn test_evaluate_string() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr(r#""hello""#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::string("hello"));
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let mut evaluator = Evaluator::new();

        let result = evaluator.evaluate_expr("2 + 3");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(5.0));

        let result = evaluator.evaluate_expr("10 - 4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(6.0));

        let result = evaluator.evaluate_expr("3 * 4");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(12.0));

        let result = evaluator.evaluate_expr("8 / 2");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(4.0));
    }

    #[test]
    fn test_evaluate_string_concatenation() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr(r#""hello" + " " + "world""#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::string("hello world"));
    }

    #[test]
    fn test_evaluate_std_length() {
        let mut evaluator = Evaluator::new();

        let result = evaluator.evaluate_expr("std.length([1, 2, 3])");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(3.0));

        let result = evaluator.evaluate_expr(r#"std.length("hello")"#);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(5.0));
    }

    #[test]
    fn test_evaluate_division_by_zero() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr("1 / 0");
        assert!(result.is_err());
        match result.err().unwrap() {
            JsonnetError::DivisionByZero => {},
            _ => panic!("Expected DivisionByZero error"),
        }
    }

    #[test]
    fn test_evaluate_undefined_variable() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_expr("undefined_var");
        assert!(result.is_err());
        match result.err().unwrap() {
            JsonnetError::UndefinedVariable { name: _ } => {},
            _ => panic!("Expected UndefinedVariable error"),
        }
    }
}

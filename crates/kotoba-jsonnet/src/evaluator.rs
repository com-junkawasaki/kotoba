//! Jsonnet expression evaluator

use crate::ast::{self, BinaryOp, Expr, ObjectField, Program, Stmt, UnaryOp};
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

    /// Initialize the standard library
    fn init_stdlib(&mut self) {
        use crate::stdlib::*;

        // Add std object with functions
        let mut std_object = HashMap::new();

        // std.length function
        std_object.insert("length".to_string(), JsonnetValue::Builtin(JsonnetBuiltin::Length));

        self.globals.insert("std".to_string(), JsonnetValue::Object(std_object));
    }

    /// Evaluate a Jsonnet file
    pub fn evaluate_file(&mut self, source: &str, filename: &str) -> Result<JsonnetValue> {
        use crate::parser::Parser;
        let mut parser = Parser::new();
        let program = parser.parse(source)?;
        self.evaluate_program(program)
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

    /// Evaluate a parsed program
    fn evaluate_program(&mut self, program: Program) -> Result<JsonnetValue> {
        let mut result = JsonnetValue::Null;

        for stmt in program.statements {
            result = self.evaluate_statement(&stmt)?;
        }

        Ok(result)
    }

    /// Evaluate a statement
    fn evaluate_statement(&mut self, stmt: &Stmt) -> Result<JsonnetValue> {
        match stmt {
            Stmt::Expr(expr) => self.evaluate_expression(expr),
            Stmt::Local(bindings) => {
                let mut scope = HashMap::new();

                // Evaluate bindings in order
                for (name, expr) in bindings {
                    let value = self.evaluate_expression(expr)?;
                    scope.insert(name.clone(), value);
                }

                // For now, just return the last binding value
                // TODO: Handle proper scoping
                if let Some((_, expr)) = bindings.last() {
                    self.evaluate_expression(expr)
                } else {
                    Ok(JsonnetValue::Null)
                }
            }
            Stmt::Assert { cond, message, expr } => {
                let cond_value = self.evaluate_expression(cond)?;
                if !cond_value.is_truthy() {
                    let msg = if let Some(msg_expr) = message {
                        match self.evaluate_expression(msg_expr)? {
                            JsonnetValue::String(s) => s,
                            _ => "Assertion failed".to_string(),
                        }
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(JsonnetError::runtime_error(&msg));
                }
                self.evaluate_expression(expr)
            }
        }
    }

    /// Evaluate an expression
    fn evaluate_expression(&mut self, expr: &Expr) -> Result<JsonnetValue> {
        match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::StringInterpolation(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        ast::StringInterpolationPart::Literal(s) => result.push_str(s),
                        ast::StringInterpolationPart::Interpolation(expr) => {
                            let value = self.evaluate_expression(expr)?;
                            match value {
                                JsonnetValue::String(s) => result.push_str(&s),
                                JsonnetValue::Number(n) => result.push_str(&n.to_string()),
                                JsonnetValue::Boolean(b) => result.push_str(&b.to_string()),
                                JsonnetValue::Null => result.push_str("null"),
                                _ => result.push_str(&value.to_string()),
                            }
                        }
                    }
                }
                Ok(JsonnetValue::string(result))
            }
            Expr::Var(name) => {
                if let Some(value) = self.globals.get(name) {
                    Ok(value.clone())
                } else {
                    Err(JsonnetError::runtime_error(&format!("Undefined variable: {}", name)))
                }
            }
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(left)?;
                let right_val = self.evaluate_expression(right)?;
                self.evaluate_binary_op(*op, left_val, right_val)
            }
            Expr::UnaryOp { op, expr } => {
                let val = self.evaluate_expression(expr)?;
                self.evaluate_unary_op(*op, val)
            }
            Expr::Array(elements) => {
                let mut values = Vec::new();
                for elem in elements {
                    values.push(self.evaluate_expression(elem)?);
                }
                Ok(JsonnetValue::Array(values))
            }
            Expr::Object(fields) => {
                let mut object = HashMap::new();
                for field in fields {
                    let key = self.evaluate_object_field_key(&field.name)?;
                    let value = self.evaluate_expression(&field.expr)?;
                    object.insert(key, value);
                }
                Ok(JsonnetValue::Object(object))
            }
            Expr::Call { func, args } => {
                let func_val = self.evaluate_expression(func)?;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.evaluate_expression(arg)?);
                }
                self.call_function(func_val, arg_vals)
            }
            Expr::Index { target, index } => {
                let target_val = self.evaluate_expression(target)?;
                let index_val = self.evaluate_expression(index)?;
                self.index_access(target_val, index_val)
            }
            Expr::Local { bindings, body } => {
                // Create a new scope for local variables
                let mut local_scope = HashMap::new();

                // Evaluate bindings
                for (name, expr) in bindings {
                    let value = self.evaluate_expression(expr)?;
                    local_scope.insert(name.clone(), value);
                }

                // Temporarily add to globals (simple implementation)
                let old_globals = self.globals.clone();
                for (name, value) in &local_scope {
                    self.globals.insert(name.clone(), value.clone());
                }

                let result = self.evaluate_expression(body);

                // Restore globals
                self.globals = old_globals;

                result
            }
            Expr::Function { parameters, body } => {
                Ok(JsonnetValue::Function(JsonnetFunction::new(parameters.clone(), Box::new((**body).clone()), HashMap::new())))
            }
            Expr::If { cond, then_branch, else_branch } => {
                let cond_val = self.evaluate_expression(cond)?;
                if cond_val.is_truthy() {
                    self.evaluate_expression(then_branch)
                } else if let Some(else_expr) = else_branch {
                    self.evaluate_expression(else_expr)
                } else {
                    Ok(JsonnetValue::Null)
                }
            }
            _ => Err(JsonnetError::runtime_error("Expression type not implemented yet")),
        }
    }

    /// Evaluate binary operation
    fn evaluate_binary_op(&self, op: BinaryOp, left: JsonnetValue, right: JsonnetValue) -> Result<JsonnetValue> {
        match op {
            BinaryOp::Add => left.add(&right),
            BinaryOp::Sub => left.sub(&right),
            BinaryOp::Mul => left.mul(&right),
            BinaryOp::Div => left.div(&right),
            BinaryOp::Mod => left.modulo(&right),
            BinaryOp::Lt => left.lt(&right),
            BinaryOp::Le => left.le(&right),
            BinaryOp::Gt => left.gt(&right),
            BinaryOp::Ge => left.ge(&right),
            BinaryOp::Eq => left.eq(&right),
            BinaryOp::Ne => left.ne(&right),
            BinaryOp::And => left.and(&right),
            BinaryOp::Or => left.or(&right),
            _ => Err(JsonnetError::runtime_error("Binary operator not implemented")),
        }
    }

    /// Evaluate unary operation
    fn evaluate_unary_op(&self, op: UnaryOp, value: JsonnetValue) -> Result<JsonnetValue> {
        match op {
            UnaryOp::Not => value.not(),
            UnaryOp::Neg => value.neg(),
            UnaryOp::Pos => Ok(value),
            UnaryOp::BitNot => Err(JsonnetError::runtime_error("Bitwise NOT not implemented")),
        }
    }

    /// Evaluate object field key
    fn evaluate_object_field_key(&self, key_expr: &Expr) -> Result<String> {
        match key_expr {
            Expr::Literal(JsonnetValue::String(s)) => Ok(s.clone()),
            Expr::Var(name) => Ok(name.clone()),
            _ => Err(JsonnetError::runtime_error("Object field key must be a string literal or identifier")),
        }
    }

    /// Call a function
    fn call_function(&mut self, func: JsonnetValue, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match func {
            JsonnetValue::Function(f) => {
                if args.len() != f.parameters.len() {
                    return Err(JsonnetError::runtime_error(
                        &format!("Expected {} arguments, got {}", f.parameters.len(), args.len())
                    ));
                }

                // Create parameter bindings
                let mut old_globals = self.globals.clone();
                for (param, arg) in f.parameters.iter().zip(args) {
                    self.globals.insert(param.clone(), arg);
                }

                let result = self.evaluate_expression(&f.body);

                // Restore globals
                self.globals = old_globals;

                result
            }
            _ => Err(JsonnetError::runtime_error("Cannot call non-function value")),
        }
    }

    /// Index access
    fn index_access(&self, target: JsonnetValue, index: JsonnetValue) -> Result<JsonnetValue> {
        match (&target, &index) {
            (JsonnetValue::Array(arr), JsonnetValue::Number(idx)) => {
                let idx = *idx as usize;
                if idx >= arr.len() {
                    return Err(JsonnetError::runtime_error("Array index out of bounds"));
                }
                Ok(arr[idx].clone())
            }
            (JsonnetValue::Object(obj), JsonnetValue::String(key)) => {
                if let Some(value) = obj.get(key) {
                    Ok(value.clone())
                } else {
                    Ok(JsonnetValue::Null) // Jsonnet returns null for missing keys
                }
            }
            _ => Err(JsonnetError::runtime_error("Invalid index operation")),
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

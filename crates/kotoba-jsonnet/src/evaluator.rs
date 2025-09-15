//! Jsonnet expression evaluator

use crate::ast::{self, BinaryOp, Expr, Program, Stmt, UnaryOp};
use crate::error::{JsonnetError, Result};
use crate::value::{JsonnetBuiltin, JsonnetFunction, JsonnetValue};
use std::collections::HashMap;

/// Jsonnet evaluator
pub struct Evaluator {
    /// Global environment
    globals: HashMap<String, JsonnetValue>,
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        let mut evaluator = Evaluator {
            globals: HashMap::new(),
        };
        evaluator.init_stdlib();
        evaluator
    }

    /// Initialize the standard library
    fn init_stdlib(&mut self) {

        // Add std object with functions
        let mut std_object = HashMap::new();

        // std.length function
        std_object.insert("length".to_string(), JsonnetValue::Builtin(JsonnetBuiltin::Length));

        // For now, add other std functions as built-ins that will delegate to StdLib
        // TODO: Create a proper std function wrapper
        let std_functions = vec![
            "type", "makeArray", "filter", "map", "foldl", "foldr", "range", "join", "split",
            "contains", "startsWith", "endsWith", "substr", "char", "codepoint", "toString",
            "parseInt", "parseJson", "encodeUTF8", "decodeUTF8", "md5", "base64", "base64Decode",
            "manifestJson", "manifestJsonEx", "manifestYaml", "escapeStringJson", "escapeStringYaml",
            "escapeStringPython", "escapeStringBash", "escapeStringDollars", "stringChars", "stringBytes",
            "format", "isArray", "isBoolean", "isFunction", "isNumber", "isObject", "isString",
            "count", "find", "member", "modulo", "pow", "exp", "log", "sqrt", "sin", "cos", "tan",
            "asin", "acos", "atan", "floor", "ceil", "round", "abs", "max", "min", "clamp",
            "assertEqual", "sort", "uniq", "reverse", "mergePatch", "get", "objectFields",
            "objectFieldsAll", "objectHas", "objectHasAll", "objectValues", "objectValuesAll",
            "prune", "mapWithKey", "toLower", "toUpper", "trim", "trace", "all", "any",
            "id", "equals", "lines", "strReplace", "sha1", "sha256", "sha3", "sha512",
            "asciiLower", "asciiUpper", "set", "setMember", "setUnion", "setInter", "setDiff",
            "flatMap", "mapWithIndex", "lstripChars", "rstripChars", "stripChars", "findSubstr", "repeat",
            "manifestIni", "manifestPython", "manifestCpp", "manifestXmlJsonml",
            "log2", "log10", "log1p", "expm1"
        ];

        for func_name in std_functions {
            std_object.insert(func_name.to_string(), JsonnetValue::Builtin(JsonnetBuiltin::StdLibFunction(func_name.to_string())));
        }

        self.globals.insert("std".to_string(), JsonnetValue::Object(std_object));
    }

    /// Evaluate a Jsonnet file
    pub fn evaluate_file(&mut self, source: &str, _filename: &str) -> Result<JsonnetValue> {
        use crate::parser::Parser;

        // Try to parse as a single expression first
        let mut parser = Parser::new();
        match parser.parse_expression(source) {
            Ok(expr) => self.evaluate_expression(&expr),
            Err(_) => {
                // If that fails, try parsing as a program
                let mut program_parser = Parser::new();
                let program = program_parser.parse(source)?;
                self.evaluate_program(program)
            }
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
            Stmt::Assert { cond, message } => {
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
                Ok(JsonnetValue::Null) // Assert statements don't return a value
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
                    Err(JsonnetError::runtime_error(format!("Undefined variable: {}", name)))
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
            Expr::ArrayComp { expr, var, array, cond } => {
                let array_val = self.evaluate_expression(array)?;
                let mut result = Vec::new();

                if let JsonnetValue::Array(elements) = array_val {
                    for element in elements {
                        // Bind the variable in a local scope
                        let old_globals = self.globals.clone();
                        self.globals.insert(var.clone(), element.clone());

                        // Check condition if present
                        let condition_met = if let Some(cond_expr) = cond {
                            let cond_val = self.evaluate_expression(cond_expr)?;
                            cond_val.is_truthy()
                        } else {
                            true
                        };

                        if condition_met {
                            let value = self.evaluate_expression(expr)?;
                            result.push(value);
                        }

                        // Restore globals
                        self.globals = old_globals;
                    }
                } else {
                    return Err(JsonnetError::RuntimeError {
                        message: "Array comprehension requires an array".to_string(),
                    });
                }

                Ok(JsonnetValue::Array(result))
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
    fn evaluate_object_field_key(&mut self, field_name: &ast::FieldName) -> Result<String> {
        match field_name {
            ast::FieldName::Fixed(name) => Ok(name.clone()),
            ast::FieldName::Computed(expr) => {
                match self.evaluate_expression(expr)? {
                    JsonnetValue::String(s) => Ok(s),
                    _ => Err(JsonnetError::runtime_error("Computed field name must evaluate to a string")),
                }
            }
        }
    }

    /// Call a function
    fn call_function(&mut self, func: JsonnetValue, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match func {
            JsonnetValue::Function(f) => {
                if args.len() != f.parameters.len() {
                    return Err(JsonnetError::runtime_error(
                        format!("Expected {} arguments, got {}", f.parameters.len(), args.len())
                    ));
                }

                // Create parameter bindings
                let old_globals = self.globals.clone();
                for (param, arg) in f.parameters.iter().zip(args) {
                    self.globals.insert(param.clone(), arg);
                }

                let result = self.evaluate_expression(&f.body);

                // Restore globals
                self.globals = old_globals;

                result
            }
            JsonnetValue::Builtin(builtin) => {
                builtin.call(args)
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evaluate_null() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file("null", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::Null);
    }

    #[test]
    fn test_evaluate_boolean() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file("true", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::boolean(true));
    }

    #[test]
    fn test_evaluate_number() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file("42", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(42.0));
    }

    #[test]
    fn test_evaluate_string() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file(r#""hello""#, "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::string("hello"));
    }

    #[test]
    fn test_evaluate_arithmetic() {
        let mut evaluator = Evaluator::new();

        let result = evaluator.evaluate_file("2 + 3", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(5.0));

        let result = evaluator.evaluate_file("10 - 4", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(6.0));

        let result = evaluator.evaluate_file("3 * 4", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(12.0));

        let result = evaluator.evaluate_file("8 / 2", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(4.0));
    }

    #[test]
    fn test_evaluate_string_concatenation() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file(r#""hello" + " " + "world""#, "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::string("hello world"));
    }

    #[test]
    fn test_evaluate_std_length() {
        let mut evaluator = Evaluator::new();

        let result = evaluator.evaluate_file("std.length([1, 2, 3])", "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(3.0));

        let result = evaluator.evaluate_file(r#"std.length("hello")"#, "");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::number(5.0));
    }

    #[test]
    fn test_evaluate_division_by_zero() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file("1 / 0", "");
        assert!(result.is_err());
        match result.err().unwrap() {
            JsonnetError::RuntimeError { message: _ } => {}, // Division by zero is now a runtime error
            _ => panic!("Expected runtime error"),
        }
    }

    #[test]
    fn test_evaluate_undefined_variable() {
        let mut evaluator = Evaluator::new();
        let result = evaluator.evaluate_file("undefined_var", "");
        assert!(result.is_err());
        match result.err().unwrap() {
            JsonnetError::RuntimeError { message: _ } => {}, // Undefined variable is now a runtime error
            _ => panic!("Expected runtime error"),
        }
    }
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

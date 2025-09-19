//! Jsonnet expression evaluator

use crate::ast::{self, BinaryOp, Expr, FieldName, ObjectField, Program, Stmt, UnaryOp, Visibility};
use crate::error::{JsonnetError, Result};
use crate::value::{JsonnetBuiltin, JsonnetFunction, JsonnetValue};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Jsonnet evaluator
pub struct Evaluator {
    /// Global environment
    globals: HashMap<String, JsonnetValue>,
    /// Stack for tracking recursion depth
    stack_depth: usize,
    /// Maximum allowed stack depth
    max_stack_depth: usize,
    /// Import paths for resolving imports
    import_paths: Vec<PathBuf>,
    /// Top-level arguments
    tla_args: HashMap<String, String>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        let mut evaluator = Evaluator {
            globals: HashMap::new(),
            stack_depth: 0,
            max_stack_depth: 1000,
            import_paths: Vec::new(),
            tla_args: HashMap::new(),
        };
        evaluator.init_stdlib();
        evaluator
    }

    /// Create a new evaluator with custom import paths
    pub fn with_import_paths(import_paths: Vec<PathBuf>) -> Self {
        let mut evaluator = Self::new();
        evaluator.import_paths = import_paths;
        evaluator
    }

    /// Add an import path
    pub fn add_import_path(&mut self, path: PathBuf) {
        self.import_paths.push(path);
    }

    /// Add a top-level argument (as code)
    pub fn add_tla_code(&mut self, key: &str, value: &str) {
        self.tla_args.insert(key.to_string(), value.to_string());
    }

    /// Initialize the standard library
    fn init_stdlib(&mut self) {
        use crate::stdlib::*;

        // Add std object with functions
        let mut std_object = HashMap::new();

        // std.length function
        std_object.insert("length".to_string(), JsonnetValue::Builtin(JsonnetBuiltin::Length));

        // Add more stdlib functions as needed
        self.globals.insert("std".to_string(), JsonnetValue::Object(std_object));
    }

    /// Evaluate a Jsonnet file with proper import resolution
    pub fn evaluate_file(&mut self, source: &str, filename: &str) -> Result<JsonnetValue> {
        use crate::parser::Parser;

        // Set up import paths based on file location
        if let Some(parent) = Path::new(filename).parent() {
            self.import_paths.insert(0, parent.to_path_buf());
        }

        let mut parser = Parser::new();
        let program = parser.parse(source)?;

        // Handle imports first
        let program = self.resolve_imports(program)?;

        self.evaluate_program(program)
    }

    /// Evaluate a Jsonnet expression string
    pub fn evaluate(&mut self, source: &str) -> Result<JsonnetValue> {
        use crate::parser::Parser;
        let mut parser = Parser::new();
        let program = parser.parse(source)?;
        let program = self.resolve_imports(program)?;
        self.evaluate_program(program)
    }

    /// Resolve imports in a program
    fn resolve_imports(&mut self, mut program: Program) -> Result<Program> {
        let mut resolved_statements = Vec::new();

        for stmt in program.statements {
            match stmt {
                Stmt::Expr(Expr::Import(path)) => {
                    // Import the file and add it to the global environment
                    let imported_value = self.import_file(&path)?;
                    // For imports, we don't add to statements, just make available globally
                    // In Jsonnet, imports create bindings
                    let var_name = self.import_path_to_var_name(&path);
                    resolved_statements.push(Stmt::Local(vec![(var_name, Expr::Literal(imported_value))], Box::new(Expr::Literal(JsonnetValue::Null))));
                }
                Stmt::Expr(Expr::ImportStr(path)) => {
                    // Import as string
                    let content = self.import_file_as_string(&path)?;
                    let var_name = self.import_path_to_var_name(&path);
                    resolved_statements.push(Stmt::Local(vec![(var_name, Expr::Literal(JsonnetValue::String(content)))], Box::new(Expr::Literal(JsonnetValue::Null))));
                }
                other => resolved_statements.push(other),
            }
        }

        program.statements = resolved_statements;
        Ok(program)
    }

    /// Import a file and evaluate it
    fn import_file(&mut self, path: &str) -> Result<JsonnetValue> {
        let full_path = self.resolve_import_path(path)?;
        let content = std::fs::read_to_string(&full_path)
            .map_err(|e| JsonnetError::runtime_error(&format!("Failed to read import '{}': {}", path, e)))?;

        // Evaluate the imported file
        let filename = full_path.to_string_lossy().to_string();
        self.evaluate_file(&content, &filename)
    }

    /// Import a file as string
    fn import_file_as_string(&mut self, path: &str) -> Result<String> {
        let full_path = self.resolve_import_path(path)?;
        std::fs::read_to_string(&full_path)
            .map_err(|e| JsonnetError::runtime_error(&format!("Failed to read importstr '{}': {}", path, e)))
    }

    /// Resolve an import path to an absolute path
    fn resolve_import_path(&self, path: &str) -> Result<PathBuf> {
        // If it's an absolute path, use it directly
        if Path::new(path).is_absolute() {
            return Ok(PathBuf::from(path));
        }

        // Try each import path
        for base_path in &self.import_paths {
            let candidate = base_path.join(path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try current directory as fallback
        let candidate = PathBuf::from(path);
        if candidate.exists() {
            return Ok(candidate);
        }

        Err(JsonnetError::runtime_error(&format!("Import '{}' not found", path)))
    }

    /// Convert import path to variable name
    fn import_path_to_var_name(&self, path: &str) -> String {
        // Simple conversion: remove extension and replace path separators
        let path = Path::new(path);
        let stem = path.file_stem().unwrap_or_default().to_string_lossy();
        stem.replace(['/', '\\', '.'], "_")
    }

    /// Check stack depth to prevent infinite recursion
    fn check_stack_depth(&mut self) -> Result<()> {
        self.stack_depth += 1;
        if self.stack_depth > self.max_stack_depth {
            return Err(JsonnetError::runtime_error(
                format!("Maximum evaluation depth ({}) exceeded", self.max_stack_depth)
            ));
        }
        Ok(())
    }

    /// Reset stack depth
    fn reset_stack_depth(&mut self) {
        self.stack_depth = 0;
    }

    /// Evaluate a parsed program
    fn evaluate_program(&mut self, program: Program) -> Result<JsonnetValue> {
        self.reset_stack_depth();
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
        self.check_stack_depth()?;

        let result = match expr {
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
            Expr::Import(_) | Expr::ImportStr(_) => {
                // These should have been resolved during import resolution
                Err(JsonnetError::runtime_error("Import statement found during evaluation"))
            }
            Expr::Error(expr) => {
                let value = self.evaluate_expression(expr)?;
                match value {
                    JsonnetValue::String(s) => Err(JsonnetError::runtime_error(&s)),
                    _ => Err(JsonnetError::runtime_error("Error expression must evaluate to string")),
                }
            }
            Expr::Assert { cond, message, expr: assert_expr } => {
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
                self.evaluate_expression(assert_expr)
            }
        };

        // Decrement stack depth
        if self.stack_depth > 0 {
            self.stack_depth -= 1;
        }

        result
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
            BinaryOp::Concat => {
                // String concatenation
                match (&left, &right) {
                    (JsonnetValue::String(a), JsonnetValue::String(b)) => Ok(JsonnetValue::string(a.clone() + b)),
                    _ => Err(JsonnetError::runtime_error("Concat operator requires strings")),
                }
            }
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
    fn evaluate_object_field_key(&self, key_expr: &FieldName) -> Result<String> {
        match key_expr {
            FieldName::Fixed(s) => Ok(s.clone()),
            FieldName::Computed(expr) => {
                let value = self.evaluate_expression(expr)?;
                match value {
                    JsonnetValue::String(s) => Ok(s),
                    _ => Err(JsonnetError::runtime_error("Object field key must be a string")),
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
                        &format!("Expected {} arguments, got {}", f.parameters.len(), args.len())
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
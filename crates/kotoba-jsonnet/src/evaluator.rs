//! Jsonnet expression evaluator

use crate::ast::*;
use crate::error::{JsonnetError, Result};
use crate::eval::{Context, handlers::*};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::stdlib::StdLibWithCallback;
use crate::value::JsonnetValue;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Jsonnet evaluator that can handle imports, TLAs, and full evaluation
pub struct Evaluator {
    /// Top-level code arguments
    tla_code_args: HashMap<String, String>,
    /// Top-level string arguments
    tla_str_args: HashMap<String, String>,
    /// Import path resolution
    import_paths: Vec<PathBuf>,
    /// Already loaded files (to prevent circular imports)
    loaded_files: HashMap<PathBuf, JsonnetValue>,
    /// Operation handler
    op_handler: Box<dyn OpHandler>,
    /// Function call handler
    funcall_handler: Box<dyn FuncallHandler>,
    /// Comprehension handler
    comp_handler: Box<dyn ComprehensionHandler>,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    /// Create a new evaluator with default handlers
    pub fn new() -> Self {
        Evaluator {
            tla_code_args: HashMap::new(),
            tla_str_args: HashMap::new(),
            import_paths: vec![PathBuf::from(".")],
            loaded_files: HashMap::new(),
            op_handler: Box::new(DefaultOpHandler),
            funcall_handler: Box::new(DefaultFuncallHandler),
            comp_handler: Box::new(DefaultComprehensionHandler),
        }
    }

    /// Add a top-level code argument
    pub fn add_tla_code(&mut self, key: &str, value: &str) {
        self.tla_code_args.insert(key.to_string(), value.to_string());
    }

    /// Add a top-level string argument
    pub fn add_tla_str(&mut self, key: &str, value: &str) {
        self.tla_str_args.insert(key.to_string(), value.to_string());
    }

    /// Add an import path
    pub fn add_import_path<P: AsRef<Path>>(&mut self, path: P) {
        self.import_paths.push(path.as_ref().to_path_buf());
    }

    /// Set import paths (replaces existing ones)
    pub fn set_import_paths(&mut self, paths: Vec<PathBuf>) {
        self.import_paths = paths;
    }

    /// Evaluate a Jsonnet expression from source code
    pub fn evaluate(&mut self, source: &str) -> Result<JsonnetValue> {
        self.evaluate_with_filename(source, "<string>")
    }

    /// Evaluate a Jsonnet file
    pub fn evaluate_file(&mut self, source: &str, filename: &str) -> Result<JsonnetValue> {
        // Clear loaded files for each new evaluation
        self.loaded_files.clear();

        // Parse the source into AST
        let mut parser = Parser::new();
        let program = parser.parse(source)?;

        // Create evaluation context
        let mut context = Context::new();

        // Set up stdlib
        let mut stdlib = StdLibWithCallback::new(self);
        context.set_global("std".to_string(), JsonnetValue::Function(stdlib.clone()));

        // Evaluate TLA code arguments
        for (key, code) in &self.tla_code_args {
            let value = self.evaluate(code)?;
            context.set_global(key.clone(), value);
        }

        // Evaluate TLA string arguments
        for (key, str_val) in &self.tla_str_args {
            context.set_global(key.clone(), JsonnetValue::String(str_val.clone()));
        }

        // Evaluate the program
        self.eval_program(&program, &mut context)
    }

    /// Evaluate a file from the filesystem
    pub fn evaluate_file_path<P: AsRef<Path>>(&mut self, path: P) -> Result<JsonnetValue> {
        let path = path.as_ref();
        let source = fs::read_to_string(path)?;
        let filename = path.to_string_lossy().to_string();
        self.evaluate_file(&source, &filename)
    }

    /// Evaluate an AST program
    fn eval_program(&mut self, program: &Program, context: &mut Context) -> Result<JsonnetValue> {
        let mut result = JsonnetValue::Null;

        for stmt in program.statements() {
            result = self.eval_stmt(stmt, context)?;
        }

        Ok(result)
    }

    /// Evaluate a statement
    fn eval_stmt(&mut self, stmt: &Stmt, context: &mut Context) -> Result<JsonnetValue> {
        match stmt {
            Stmt::Expr(expr) => self.eval_expr(expr, context),
            Stmt::Local(bindings) => {
                context.push_scope();
                for (name, expr) in bindings {
                    let value = self.eval_expr(expr, context)?;
                    context.set_variable(name.clone(), value);
                }
                Ok(JsonnetValue::Null)
            }
            Stmt::Assert(expr, message) => {
                let condition = self.eval_expr(expr, context)?;
                if !condition.is_truthy() {
                    let msg = if let Some(msg_expr) = message {
                        self.eval_expr(msg_expr, context)?.as_string().unwrap_or_default()
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(JsonnetError::runtime_error(msg));
                }
                Ok(JsonnetValue::Null)
            }
        }
    }

    /// Evaluate an expression
    fn eval_expr(&mut self, expr: &Expr, context: &mut Context) -> Result<JsonnetValue> {
        context.push_depth()?;

        let result = match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::StringInterpolation(parts) => self.eval_string_interpolation(parts, context),
            Expr::Var(name) => self.eval_variable(name, context),
            Expr::BinaryOp { left, op, right } => {
                let left_val = self.eval_expr(left, context)?;
                let right_val = self.eval_expr(right, context)?;
                self.op_handler.eval_binary_op(context, left_val, *op, right_val)
            }
            Expr::UnaryOp { op, expr } => {
                let val = self.eval_expr(expr, context)?;
                self.op_handler.eval_unary_op(context, *op, val)
            }
            Expr::Array(elements) => {
                let mut result = Vec::new();
                for elem in elements {
                    result.push(self.eval_expr(elem, context)?);
                }
                Ok(JsonnetValue::Array(result))
            }
            Expr::Object(fields) => self.eval_object(fields, context),
            Expr::ArrayComp { expr: comp_expr, var, array, cond } => {
                self.eval_list_comprehension(context, comp_expr, var, array, cond.as_ref().map(|e| e.as_ref()))
            }
            Expr::ObjectComp { field, var, array } => {
                // For simplicity, we'll handle object comprehensions as field: value pairs
                match field.as_ref() {
                    ObjectField::Field { key, value, .. } => {
                        self.eval_dict_comprehension(context, key, value, var, array, None)
                    }
                    _ => Err(JsonnetError::runtime_error("Unsupported object comprehension field type")),
                }
            }
            Expr::Call { func, args } => {
                let func_val = self.eval_expr(func, context)?;
                let mut arg_vals = Vec::new();
                for arg in args {
                    arg_vals.push(self.eval_expr(arg, context)?);
                }
                self.funcall_handler.call_function(context, func_val, arg_vals)
            }
            Expr::Index { target, index } => {
                let target_val = self.eval_expr(target, context)?;
                let index_val = self.eval_expr(index, context)?;
                self.eval_index(target_val, index_val)
            }
            Expr::Slice { target, start, end, step } => {
                let target_val = self.eval_expr(target, context)?;
                let start_val = start.as_ref().map(|e| self.eval_expr(e, context)).transpose()?;
                let end_val = end.as_ref().map(|e| self.eval_expr(e, context)).transpose()?;
                let step_val = step.as_ref().map(|e| self.eval_expr(e, context)).transpose()?;
                self.eval_slice(target_val, start_val, end_val, step_val)
            }
            Expr::Local { bindings, body } => {
                context.push_scope();
                for (name, expr) in bindings {
                    let value = self.eval_expr(expr, context)?;
                    context.set_variable(name.clone(), value);
                }
                let result = self.eval_expr(body, context);
                context.pop_scope();
                result
            }
            Expr::Function { parameters, body } => {
                Ok(JsonnetValue::Function(JsonnetFunction {
                    parameters: parameters.clone(),
                    body: body.clone(),
                    captured_context: context.fork(),
                }))
            }
            Expr::If { cond, then_branch, else_branch } => {
                let cond_val = self.eval_expr(cond, context)?;
                if cond_val.is_truthy() {
                    self.eval_expr(then_branch, context)
                } else if let Some(else_expr) = else_branch {
                    self.eval_expr(else_expr, context)
                } else {
                    Ok(JsonnetValue::Null)
                }
            }
            Expr::Assert { cond, message } => {
                let cond_val = self.eval_expr(cond, context)?;
                if !cond_val.is_truthy() {
                    let msg = if let Some(msg_expr) = message {
                        self.eval_expr(msg_expr, context)?.as_string().unwrap_or_default()
                    } else {
                        "Assertion failed".to_string()
                    };
                    return Err(JsonnetError::runtime_error(msg));
                }
                Ok(JsonnetValue::Null)
            }
        };

        context.pop_depth();
        result
    }

    /// Evaluate string interpolation
    fn eval_string_interpolation(&mut self, parts: &[StringInterpolationPart], context: &mut Context) -> Result<JsonnetValue> {
        let mut result = String::new();

        for part in parts {
            match part {
                StringInterpolationPart::Literal(s) => result.push_str(s),
                StringInterpolationPart::Interpolation(expr) => {
                    let value = self.eval_expr(expr, context)?;
                    result.push_str(&value.as_string().unwrap_or_default());
                }
            }
        }

        Ok(JsonnetValue::String(result))
    }

    /// Evaluate a variable reference
    fn eval_variable(&mut self, name: &str, context: &mut Context) -> Result<JsonnetValue> {
        if let Some(value) = context.get_variable(name) {
            Ok(value.clone())
        } else {
            Err(JsonnetError::runtime_error(format!("Undefined variable: {}", name)))
        }
    }

    /// Evaluate an object
    fn eval_object(&mut self, fields: &[ObjectField], context: &mut Context) -> Result<JsonnetValue> {
        let mut object = HashMap::new();

        for field in fields {
            match field {
                ObjectField::Field { key, value, .. } => {
                    let key_str = self.eval_expr(key, context)?.as_string()
                        .ok_or_else(|| JsonnetError::runtime_error("Object key must be a string"))?;
                    let value_val = self.eval_expr(value, context)?;
                    object.insert(key_str, value_val);
                }
                ObjectField::FieldStr { name, value } => {
                    let value_val = self.eval_expr(value, context)?;
                    object.insert(name.clone(), value_val);
                }
                ObjectField::Assert { expr, message } => {
                    let cond_val = self.eval_expr(expr, context)?;
                    if !cond_val.is_truthy() {
                        let msg = if let Some(msg_expr) = message {
                            self.eval_expr(msg_expr, context)?.as_string().unwrap_or_default()
                        } else {
                            "Object assertion failed".to_string()
                        };
                        return Err(JsonnetError::runtime_error(msg));
                    }
                }
                ObjectField::Local(bindings) => {
                    context.push_scope();
                    for (name, expr) in bindings {
                        let value = self.eval_expr(expr, context)?;
                        context.set_variable(name.clone(), value);
                    }
                }
            }
        }

        Ok(JsonnetValue::Object(object))
    }

    /// Evaluate array/string indexing
    fn eval_index(&self, target: JsonnetValue, index: JsonnetValue) -> Result<JsonnetValue> {
        match (&target, &index) {
            (JsonnetValue::Array(arr), JsonnetValue::Number(n)) => {
                let idx = *n as usize;
                arr.get(idx)
                    .cloned()
                    .ok_or_else(|| JsonnetError::runtime_error(format!("Array index {} out of bounds", idx)))
            }
            (JsonnetValue::String(s), JsonnetValue::Number(n)) => {
                let idx = *n as usize;
                s.chars().nth(idx)
                    .map(|c| JsonnetValue::String(c.to_string()))
                    .ok_or_else(|| JsonnetError::runtime_error(format!("String index {} out of bounds", idx)))
            }
            (JsonnetValue::Object(obj), JsonnetValue::String(key)) => {
                obj.get(key)
                    .cloned()
                    .ok_or_else(|| JsonnetError::runtime_error(format!("Object has no field '{}'", key)))
            }
            _ => Err(JsonnetError::runtime_error("Invalid index operation")),
        }
    }

    /// Evaluate slicing
    fn eval_slice(&self, target: JsonnetValue, start: Option<JsonnetValue>, end: Option<JsonnetValue>, step: Option<JsonnetValue>) -> Result<JsonnetValue> {
        let step_val = step.as_ref()
            .and_then(|v| v.as_number())
            .unwrap_or(1.0);

        if step_val == 0.0 {
            return Err(JsonnetError::runtime_error("Slice step cannot be zero"));
        }

        match target {
            JsonnetValue::Array(arr) => {
                let len = arr.len();
                let start_idx = start.as_ref()
                    .and_then(|v| v.as_number())
                    .map(|n| if n < 0.0 { len as isize + n as isize } else { n as isize } as usize)
                    .unwrap_or(0)
                    .min(len);

                let end_idx = end.as_ref()
                    .and_then(|v| v.as_number())
                    .map(|n| if n < 0.0 { len as isize + n as isize } else { n as isize } as usize)
                    .unwrap_or(len)
                    .min(len);

                if step_val > 0.0 {
                    let mut result = Vec::new();
                    let mut i = start_idx;
                    while i < end_idx {
                        result.push(arr[i].clone());
                        i += step_val as usize;
                    }
                    Ok(JsonnetValue::Array(result))
                } else {
                    let mut result = Vec::new();
                    let mut i = if start_idx >= end_idx { start_idx } else { end_idx - 1 };
                    while (step_val < 0.0 && i >= end_idx) || (step_val > 0.0 && i <= start_idx) {
                        result.push(arr[i].clone());
                        if step_val < 0.0 {
                            if i == 0 { break; }
                            i -= (-step_val) as usize;
                        } else {
                            i += step_val as usize;
                        }
                    }
                    Ok(JsonnetValue::Array(result))
                }
            }
            JsonnetValue::String(s) => {
                let chars: Vec<char> = s.chars().collect();
                let len = chars.len();
                let start_idx = start.as_ref()
                    .and_then(|v| v.as_number())
                    .map(|n| if n < 0.0 { len as isize + n as isize } else { n as isize } as usize)
                    .unwrap_or(0)
                    .min(len);

                let end_idx = end.as_ref()
                    .and_then(|v| v.as_number())
                    .map(|n| if n < 0.0 { len as isize + n as isize } else { n as isize } as usize)
                    .unwrap_or(len)
                    .min(len);

                if step_val > 0.0 {
                    let mut result = String::new();
                    let mut i = start_idx;
                    while i < end_idx {
                        result.push(chars[i]);
                        i += step_val as usize;
                    }
                    Ok(JsonnetValue::String(result))
                } else {
                    let mut result = String::new();
                    let mut i = if start_idx >= end_idx { start_idx } else { end_idx - 1 };
                    while (step_val < 0.0 && i >= end_idx) || (step_val > 0.0 && i <= start_idx) {
                        result.push(chars[i]);
                        if step_val < 0.0 {
                            if i == 0 { break; }
                            i -= (-step_val) as usize;
                        } else {
                            i += step_val as usize;
                        }
                    }
                    Ok(JsonnetValue::String(result))
                }
            }
            _ => Err(JsonnetError::runtime_error("Slice operation only supported on arrays and strings")),
        }
    }

    /// Resolve an import path
    fn resolve_import(&self, import_path: &str, current_file: Option<&Path>) -> Result<PathBuf> {
        let import_path = Path::new(import_path);

        // If it's an absolute path or starts with ./ or ../, try relative to current file first
        if import_path.is_absolute() || import_path.starts_with(".") {
            if let Some(current) = current_file {
                let resolved = current.parent().unwrap_or(current).join(import_path);
                if resolved.exists() {
                    return Ok(resolved.canonicalize()?);
                }
            }
        }

        // Try import paths
        for base_path in &self.import_paths {
            let resolved = base_path.join(import_path);
            if resolved.exists() {
                return Ok(resolved.canonicalize()?);
            }
        }

        Err(JsonnetError::runtime_error(format!("Import '{}' not found", import_path)))
    }

    /// Load and evaluate an imported file
    fn load_import(&mut self, import_path: &str, current_file: Option<&Path>) -> Result<JsonnetValue> {
        let resolved_path = self.resolve_import(import_path, current_file)?;

        // Check for circular imports
        if self.loaded_files.contains_key(&resolved_path) {
            return Ok(self.loaded_files[&resolved_path].clone());
        }

        // Load and evaluate the file
        let source = fs::read_to_string(&resolved_path)?;
        let filename = resolved_path.to_string_lossy().to_string();

        // Temporarily update current file context for nested imports
        let result = self.evaluate_file(&source, &filename)?;
        self.loaded_files.insert(resolved_path, result.clone());

        Ok(result)
    }

    /// Evaluate a list comprehension
    fn eval_list_comprehension(&mut self, context: &mut Context, expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue> {
        let collection_val = self.eval_expr(collection, context)?;
        let mut result = Vec::new();

        match collection_val {
            JsonnetValue::Array(arr) => {
                for item in arr {
                    context.push_scope();
                    context.set_variable(var.to_string(), item);

                    let include = if let Some(cond) = condition {
                        self.eval_expr(cond, context)?.is_truthy()
                    } else {
                        true
                    };

                    if include {
                        let value = self.eval_expr(expr, context)?;
                        result.push(value);
                    }

                    context.pop_scope();
                }
            }
            _ => return Err(JsonnetError::runtime_error("List comprehension requires array")),
        }

        Ok(JsonnetValue::Array(result))
    }

    /// Evaluate a dict comprehension
    fn eval_dict_comprehension(&mut self, context: &mut Context, key_expr: &Expr, value_expr: &Expr, var: &str, collection: &Expr, condition: Option<&Expr>) -> Result<JsonnetValue> {
        let collection_val = self.eval_expr(collection, context)?;
        let mut result = HashMap::new();

        match collection_val {
            JsonnetValue::Array(arr) => {
                for item in arr {
                    context.push_scope();
                    context.set_variable(var.to_string(), item.clone());

                    let include = if let Some(cond) = condition {
                        self.eval_expr(cond, context)?.is_truthy()
                    } else {
                        true
                    };

                    if include {
                        let key_val = self.eval_expr(key_expr, context)?;
                        let value_val = self.eval_expr(value_expr, context)?;

                        if let JsonnetValue::String(key_str) = key_val {
                            result.insert(key_str, value_val);
                        } else {
                            return Err(JsonnetError::runtime_error("Dict comprehension key must be a string"));
                        }
                    }

                    context.pop_scope();
                }
            }
            _ => return Err(JsonnetError::runtime_error("Dict comprehension requires array")),
        }

        Ok(JsonnetValue::Object(result))
    }
}

impl FunctionCallback for Evaluator {
    fn call_function(&mut self, func: JsonnetValue, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        match func {
            JsonnetValue::Function(jsonnet_func) => {
                // Create a new context with captured variables
                let mut call_context = jsonnet_func.captured_context.clone();

                // Check parameter count
                if args.len() != jsonnet_func.parameters.len() {
                    return Err(JsonnetError::runtime_error(format!(
                        "Expected {} arguments, got {}",
                        jsonnet_func.parameters.len(),
                        args.len()
                    )));
                }

                // Bind parameters
                for (param, arg) in jsonnet_func.parameters.iter().zip(args) {
                    call_context.set_variable(param.clone(), arg);
                }

                // Evaluate function body
                self.eval_expr(&jsonnet_func.body, &mut call_context)
            }
            _ => Err(JsonnetError::runtime_error("Cannot call non-function value")),
        }
    }

    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        // Handle import functions
        if name == "import" {
            if args.len() != 1 {
                return Err(JsonnetError::runtime_error("import() expects exactly 1 argument"));
            }
            match &args[0] {
                JsonnetValue::String(path) => self.load_import(path, None),
                _ => Err(JsonnetError::runtime_error("import() argument must be a string")),
            }
        } else if name == "importstr" {
            if args.len() != 1 {
                return Err(JsonnetError::runtime_error("importstr() expects exactly 1 argument"));
            }
            match &args[0] {
                JsonnetValue::String(path) => {
                    let resolved_path = self.resolve_import(path, None)?;
                    let content = fs::read_to_string(resolved_path)?;
                    Ok(JsonnetValue::String(content))
                }
                _ => Err(JsonnetError::runtime_error("importstr() argument must be a string")),
            }
        } else {
            Err(JsonnetError::runtime_error(format!("Unknown external function: {}", name)))
        }
    }
}

/// Jsonnet function representation
#[derive(Clone)]
pub struct JsonnetFunction {
    pub parameters: Vec<String>,
    pub body: Box<Expr>,
    pub captured_context: Context,
}
//! The Kotoba Pure Semantic Analyzer
//!
//! This crate provides PURE semantic analysis functionality for Kotoba language.
//! It takes Abstract Syntax Tree (AST) and performs semantic analysis without
//! any side effects.
//!
//! ## Pure Kernel Component
//!
//! This analyzer is part of the Pure Kernel - it performs only deterministic,
//! side-effect-free computations. All I/O operations are handled by the
//! Effects Shell components.

use std::collections::HashMap;
use kotoba_syntax::{Expr, Program, Stmt};

/// Pure semantic analysis result
#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisResult {
    /// Symbol table mapping names to their definitions
    pub symbol_table: HashMap<String, SymbolInfo>,
    /// Type information for expressions
    pub type_info: HashMap<String, TypeInfo>,
    /// Semantic errors found during analysis
    pub errors: Vec<SemanticError>,
    /// Warnings about potential issues
    pub warnings: Vec<SemanticWarning>,
}

/// Information about a symbol (variable, function, etc.)
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolInfo {
    pub name: String,
    pub kind: SymbolKind,
    pub scope: Scope,
    pub definition_location: Option<Location>,
}

/// Type information for expressions
#[derive(Debug, Clone, PartialEq)]
pub struct TypeInfo {
    pub inferred_type: KotobaType,
    pub location: Location,
}

/// Different kinds of symbols
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Variable,
    Function,
    Type,
    Module,
}

/// Scope information
#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Global,
    Local(String), // scope name
    Function(String), // function name
}

/// Location in source code
#[derive(Debug, Clone, PartialEq)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

/// Semantic errors that can occur during analysis
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticError {
    UndefinedVariable(String, Location),
    TypeMismatch {
        expected: KotobaType,
        found: KotobaType,
        location: Location,
    },
    DuplicateDefinition(String, Location),
    InvalidOperation(String, Location),
    // --- New Errors for Pure Functional Validation ---
    ForbiddenConstruct(String, Location),
    TopLevelNotFunction(Location),
}

/// Semantic warnings
#[derive(Debug, Clone, PartialEq)]
pub enum SemanticWarning {
    UnusedVariable(String, Location),
    ShadowedVariable(String, Location),
}

/// Kotoba type system
#[derive(Debug, Clone, PartialEq)]
pub enum KotobaType {
    String,
    Number,
    Boolean,
    Object,
    Array(Box<KotobaType>),
    Function(Vec<KotobaType>, Box<KotobaType>), // params, return type
    Any,
}

/// Pure semantic analyzer - no side effects, fully deterministic
pub struct PureAnalyzer {
    // Configuration for analysis rules
    config: AnalyzerConfig,
}

/// Configuration for the analyzer
#[derive(Debug, Clone)]
pub struct AnalyzerConfig {
    pub strict_mode: bool,
    pub allow_shadowing: bool,
    pub check_unused_variables: bool,
    /// Enforce the pure functional subset of Kotoba/Jsonnet
    pub enforce_pure_functional_rules: bool,
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            allow_shadowing: false,
            check_unused_variables: true,
            enforce_pure_functional_rules: true, // Default to strict rules
        }
    }
}

impl PureAnalyzer {
    /// Create a new pure analyzer with default configuration
    pub fn new() -> Self {
        Self {
            config: AnalyzerConfig::default(),
        }
    }

    /// Create a new analyzer with custom configuration
    pub fn with_config(config: AnalyzerConfig) -> Self {
        Self { config }
    }

    /// Perform pure semantic analysis on an AST
    ///
    /// This function is PURE: same input always produces same output,
    /// no side effects, no external dependencies.
    pub fn analyze(&self, ast: &Program) -> AnalysisResult {
        let mut symbol_table = HashMap::new();
        let mut type_info = HashMap::new();
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // If enabled, run the pure functional validator first.
        if self.config.enforce_pure_functional_rules {
            let mut validator = PureFunctionalValidator::new();
            validator.validate_program(ast);
            errors.extend(validator.errors);
        }

        // The existing (or future) analysis can run here.
        self.analyze_program(
            ast,
            &mut symbol_table,
            &mut type_info,
            &mut errors,
            &mut warnings,
        );

        AnalysisResult {
            symbol_table,
            type_info,
            errors,
            warnings,
        }
    }

    /// Pure analysis of a program (simplified)
    fn analyze_program(
        &self,
        program: &Program,
        symbol_table: &mut HashMap<String, SymbolInfo>,
        _type_info: &mut HashMap<String, TypeInfo>,
        errors: &mut Vec<SemanticError>,
        warnings: &mut Vec<SemanticWarning>,
    ) {
        // Simplified analysis - in real implementation this would traverse the AST
        // and build symbol tables, check types, etc.

        // For now, just demonstrate the structure
        // In a real implementation, this would analyze:
        // - Variable declarations and usage
        // - Function definitions and calls
        // - Type checking
        // - Scope analysis
        // - Import resolution (pure part only)

        // Example: check for undefined variables (simplified)
        for node in &program.statements {
            match node {
                // This would be actual AST node matching in real implementation
                _ => {
                    // Placeholder - real implementation would analyze each node type
                }
            }
        }

        // If strict mode is enabled, warn about unused variables
        if self.config.check_unused_variables {
            // This would check the symbol table for unused variables
            // and add warnings to the warnings vector
        }
    }
}

// --- Visitor Trait for AST Traversal ---
trait AstVisitor {
    fn visit_program(&mut self, program: &Program);
    fn visit_stmt(&mut self, stmt: &Stmt);
    fn visit_expr(&mut self, expr: &Expr);
}

// --- Pure Functional Validator ---
struct PureFunctionalValidator {
    errors: Vec<SemanticError>,
}

impl PureFunctionalValidator {
    fn new() -> Self {
        Self { errors: Vec::new() }
    }

    fn validate_program(&mut self, program: &Program) {
        // Rule: The entry point must be a single function.
        if program.statements.len() != 1 {
            self.add_error("Expected a single top-level function.", Location { line: 1, column: 1 });
            return;
        }

        if let Some(stmt) = program.statements.get(0) {
            if let Stmt::Expr(expr) = stmt {
                if !matches!(expr, Expr::Function { .. }) {
                    self.add_error("Top-level statement must be a function.", Location { line: 1, column: 1 });
                }
                self.visit_expr(expr);
            } else {
                self.add_error("Top-level statement must be a function expression.", Location { line: 1, column: 1 });
            }
        }
    }

    fn add_error(&mut self, message: &str, location: Location) {
        self.errors.push(SemanticError::ForbiddenConstruct(message.to_string(), location));
    }
}

impl AstVisitor for PureFunctionalValidator {
    fn visit_program(&mut self, program: &Program) {
        for stmt in &program.statements {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Expr(expr) => self.visit_expr(expr),
            Stmt::Local(bindings) => {
                for (_, expr) in bindings {
                    self.visit_expr(expr);
                }
            }
            Stmt::Assert { cond, message, .. } => {
                self.visit_expr(cond);
                if let Some(msg) = message {
                    self.visit_expr(msg);
                }
            }
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Var(name) if name == "self" || name == "super" => {
                self.add_error(&format!("Use of '{}' is forbidden.", name), Location { line: 0, column: 0 }); // Placeholder location
            }
            Expr::Index { target, index } => {
                 if let Expr::Var(target_name) = &**target {
                    if target_name == "std" {
                        if let Expr::Literal(kotoba_syntax::KotobaValue::String(s)) = &**index {
                            match s.as_str() {
                                "extVar" | "native" | "trace" => {
                                    self.add_error(&format!("Use of 'std.{}' is forbidden.", s), Location { line: 0, column: 0 });
                                }
                                _ => {}
                            }
                        }
                    }
                }
                self.visit_expr(target);
                self.visit_expr(index);
            }
            Expr::Object(fields) => {
                for field in fields {
                    if field.visibility != kotoba_syntax::Visibility::Normal {
                        self.add_error("Field visibility modifiers (::, :::) are forbidden.", Location { line: 0, column: 0 });
                    }
                    self.visit_expr(&field.expr);
                }
            }
            // --- Recursive traversal for other expression types ---
            Expr::BinaryOp { left, right, .. } => {
                self.visit_expr(left);
                self.visit_expr(right);
            }
            Expr::UnaryOp { expr, .. } => self.visit_expr(expr),
            Expr::Array(elements) => {
                for el in elements {
                    self.visit_expr(el);
                }
            }
            Expr::ArrayComp { expr, array, cond, .. } => {
                self.visit_expr(expr);
                self.visit_expr(array);
                if let Some(c) = cond {
                    self.visit_expr(c);
                }
            }
            Expr::ObjectComp{ field, array, .. } => {
                self.visit_expr(&field.expr);
                self.visit_expr(array);
            }
            Expr::Call { func, args } => {
                self.visit_expr(func);
                for arg in args {
                    self.visit_expr(arg);
                }
            }
            Expr::Slice { target, start, end, step } => {
                self.visit_expr(target);
                start.as_ref().map(|e| self.visit_expr(e));
                end.as_ref().map(|e| self.visit_expr(e));
                step.as_ref().map(|e| self.visit_expr(e));
            }
            Expr::Local { bindings, body } => {
                for (_, val) in bindings {
                    self.visit_expr(val);
                }
                self.visit_expr(body);
            }
            Expr::Function { body, .. } => self.visit_expr(body),
            Expr::If { cond, then_branch, else_branch } => {
                self.visit_expr(cond);
                self.visit_expr(then_branch);
                if let Some(e) = else_branch {
                    self.visit_expr(e);
                }
            }
             Expr::Assert { cond, message, expr } => {
                self.visit_expr(cond);
                message.as_ref().map(|e| self.visit_expr(e));
                self.visit_expr(expr);
            }
            Expr::Error(_) => {
                self.add_error("'error' expressions are forbidden.", Location { line: 0, column: 0 });
            }
            // No action needed for these
            Expr::Literal(_) | Expr::StringInterpolation(_) | Expr::Var(_) | Expr::Import(_) | Expr::ImportStr(_) => {}
        }
    }
}


/// Effects Shell wrapper for the pure analyzer
/// This handles I/O operations and external dependencies
pub mod effects_analyzer {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Effects-based analyzer that wraps the pure analyzer
    pub struct Analyzer {
        pure_analyzer: PureAnalyzer,
    }

    impl Analyzer {
        /// Create a new analyzer with default configuration
        pub fn new() -> Self {
            Self {
                pure_analyzer: PureAnalyzer::new(),
            }
        }

        /// Analyze a file from disk (effects: file I/O)
        pub fn analyze_file<P: AsRef<Path>>(&self, path: P) -> Result<AnalysisResult, AnalyzerError> {
            // Read file (side effect)
            let source = fs::read_to_string(path)
                .map_err(|e| AnalyzerError::IoError(e.to_string()))?;

            // Parse (could be pure or effects depending on implementation)
            // For now assume we have parsed AST
            let ast = self.parse_source(&source)?;

            // Pure analysis (no side effects)
            Ok(self.pure_analyzer.analyze(&ast))
        }

        /// Analyze source code string (effects: parsing may involve external libraries)
        pub fn analyze_source(&self, source: &str) -> Result<AnalysisResult, AnalyzerError> {
            let ast = self.parse_source(source)?;
            Ok(self.pure_analyzer.analyze(&ast))
        }

        /// Parse source code (may involve external libraries, so effects)
        fn parse_source(&self, _source: &str) -> Result<Program, AnalyzerError> {
            // In real implementation, this would use the parser crate
            // For now, return a dummy program
            Ok(Program {
                statements: vec![],
            })
        }
    }

    /// Errors that can occur during analysis (including I/O errors)
    #[derive(Debug, Clone)]
    pub enum AnalyzerError {
        IoError(String),
        ParseError(String),
        SemanticError(SemanticError),
    }

    impl From<SemanticError> for AnalyzerError {
        fn from(error: SemanticError) -> Self {
            AnalyzerError::SemanticError(error)
        }
    }
}

// Re-export the effects-based analyzer as the main interface
// This maintains backward compatibility while providing pure analysis internally
pub use effects_analyzer::Analyzer;

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_parser::Parser;
    use kotoba_syntax::{Location, SemanticError};

    fn analyze_source(source: &str) -> AnalysisResult {
        let mut parser = Parser::new();
        let program = parser.parse(source).expect("Failed to parse for test");
        
        let config = AnalyzerConfig {
            enforce_pure_functional_rules: true,
            ..Default::default()
        };
        let analyzer = PureAnalyzer::with_config(config);
        analyzer.analyze(&program)
    }

    #[test]
    fn test_valid_pure_functional_code() {
        let source = r#"
        function(params)
            local a = params.a;
            {
                field: a + 1
            }
        "#;
        let result = analyze_source(source);
        assert!(result.errors.is_empty(), "Expected no errors for valid code, but found: {:?}", result.errors);
    }

    #[test]
    fn test_forbidden_std_extvar() {
        let source = "function(params) std.extVar('foo')";
        let result = analyze_source(source);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(result.errors[0], SemanticError::ForbiddenConstruct(_, _)));
        if let SemanticError::ForbiddenConstruct(msg, _) = &result.errors[0] {
            assert!(msg.contains("std.extVar"));
        }
    }

    #[test]
    fn test_forbidden_self() {
        let source = "function(params) { val: self.other }";
        let result = analyze_source(source);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(result.errors[0], SemanticError::ForbiddenConstruct(_, _)));
        if let SemanticError::ForbiddenConstruct(msg, _) = &result.errors[0] {
            assert!(msg.contains("'self'"));
        }
    }

    #[test]
    fn test_forbidden_field_visibility() {
        let source = "function(params) { field:: 1 }";
        let result = analyze_source(source);
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(result.errors[0], SemanticError::ForbiddenConstruct(_, _)));
        if let SemanticError::ForbiddenConstruct(msg, _) = &result.errors[0] {
            assert!(msg.contains("Field visibility modifiers"));
        }
    }

    #[test]
    fn test_top_level_is_not_a_function() {
        let source = "{ a: 1 }"; // Not a function
        let result = analyze_source(source);
        assert_eq!(result.errors.len(), 1);
        if let SemanticError::ForbiddenConstruct(msg, _) = &result.errors[0] {
            assert!(msg.contains("Top-level statement must be a function"));
        }
    }

    #[test]
    fn test_multiple_statements_are_forbidden() {
        let source = "local a = 1; { b: a }"; // Not a single function
        let result = analyze_source(source);
        assert_eq!(result.errors.len(), 1);
        if let SemanticError::ForbiddenConstruct(msg, _) = &result.errors[0] {
            assert!(msg.contains("Expected a single top-level function"));
        }
    }
}

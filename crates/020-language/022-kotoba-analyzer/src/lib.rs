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
use kotoba_syntax::Program;

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
}

impl Default for AnalyzerConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            allow_shadowing: false,
            check_unused_variables: true,
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

        // Analyze the AST (simplified implementation)
        self.analyze_program(ast, &mut symbol_table, &mut type_info, &mut errors, &mut warnings);

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

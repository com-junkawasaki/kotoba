//! Pure Code Linter - no side effects, fully deterministic
//!
//! This module provides pure code linting functionality.
//! All linting operations are deterministic and have no side effects.

use crate::{DiagnosticLevel, SemanticWarning, SemanticError};
use std::collections::HashMap;

/// Pure code linter - performs only deterministic linting operations
#[derive(Debug, Clone)]
pub struct PureLinter {
    config: LinterConfig,
}

#[derive(Debug, Clone)]
pub struct LinterConfig {
    pub enabled_rules: Vec<String>,
    pub strict_mode: bool,
    pub check_unused_variables: bool,
    pub check_shadowing: bool,
}

impl Default for LinterConfig {
    fn default() -> Self {
        Self {
            enabled_rules: vec![
                "no-unused-vars".to_string(),
                "no-shadowing".to_string(),
                "no-undefined-vars".to_string(),
            ],
            strict_mode: false,
            check_unused_variables: true,
            check_shadowing: true,
        }
    }
}

impl PureLinter {
    /// Create a new pure linter with default configuration
    pub fn new() -> Self {
        Self {
            config: LinterConfig::default(),
        }
    }

    /// Create a pure linter with custom configuration
    pub fn with_config(config: LinterConfig) -> Self {
        Self { config }
    }

    /// Pure linting of source code content
    ///
    /// This function is PURE: same input always produces same output,
    /// no side effects, no external dependencies.
    pub fn lint_content(&self, content: &str, filename: Option<&str>) -> LintResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // Parse the content into basic tokens/lines for analysis
        let lines: Vec<&str> = content.lines().collect();

        // Run enabled linting rules
        for rule in &self.config.enabled_rules {
            match rule.as_str() {
                "no-unused-vars" if self.config.check_unused_variables => {
                    self.check_unused_variables(&lines, &mut warnings);
                }
                "no-shadowing" if self.config.check_shadowing => {
                    self.check_shadowing(&lines, &mut warnings);
                }
                "no-undefined-vars" => {
                    self.check_undefined_variables(&lines, &mut errors);
                }
                _ => {} // Unknown rule, skip
            }
        }

        LintResult {
            filename: filename.map(|s| s.to_string()),
            errors,
            warnings,
        }
    }

    /// Check for unused variables (simplified implementation)
    fn check_unused_variables(&self, lines: &[&str], warnings: &mut Vec<SemanticWarning>) {
        // Simplified: look for variable declarations that aren't used
        for (i, line) in lines.iter().enumerate() {
            if line.contains("let ") || line.contains("const ") {
                // Extract variable name (very simplified)
                if let Some(var_name) = self.extract_variable_name(line) {
                    // Check if it's used later (simplified check)
                    let mut used = false;
                    for later_line in &lines[i + 1..] {
                        if later_line.contains(&var_name) {
                            used = true;
                            break;
                        }
                    }

                    if !used {
                        warnings.push(SemanticWarning::UnusedVariable(
                            var_name,
                            super::Location { line: i + 1, column: 0 },
                        ));
                    }
                }
            }
        }
    }

    /// Check for variable shadowing (simplified implementation)
    fn check_shadowing(&self, lines: &[&str], warnings: &mut Vec<SemanticWarning>) {
        let mut defined_vars = std::collections::HashSet::new();

        for (i, line) in lines.iter().enumerate() {
            if line.contains("let ") || line.contains("const ") {
                if let Some(var_name) = self.extract_variable_name(line) {
                    if defined_vars.contains(&var_name) {
                        warnings.push(SemanticWarning::ShadowedVariable(
                            var_name,
                            super::Location { line: i + 1, column: 0 },
                        ));
                    } else {
                        defined_vars.insert(var_name);
                    }
                }
            }
        }
    }

    /// Check for undefined variables (simplified implementation)
    fn check_undefined_variables(&self, lines: &[&str], errors: &mut Vec<SemanticError>) {
        let mut defined_vars = std::collections::HashSet::new();

        for (i, line) in lines.iter().enumerate() {
            // Check for variable declarations
            if line.contains("let ") || line.contains("const ") {
                if let Some(var_name) = self.extract_variable_name(line) {
                    defined_vars.insert(var_name);
                }
            }

            // Check for variable usage
            for word in line.split_whitespace() {
                if word.chars().all(|c| c.is_alphanumeric() || c == '_') && !word.is_empty() {
                    if !defined_vars.contains(word) &&
                       !self.is_builtin(word) &&
                       !line.contains(&format!("{} ", word)) && // Declaration
                       !line.contains(&format!("{}:", word)) {   // Object key
                        errors.push(SemanticError::UndefinedVariable(
                            word.to_string(),
                            super::Location { line: i + 1, column: 0 },
                        ));
                    }
                }
            }
        }
    }

    /// Extract variable name from a declaration line (very simplified)
    fn extract_variable_name(&self, line: &str) -> Option<String> {
        if let Some(let_pos) = line.find("let ") {
            let after_let = &line[let_pos + 4..];
            if let Some(space_pos) = after_let.find(' ') {
                return Some(after_let[..space_pos].trim().to_string());
            } else if let Some(eq_pos) = after_let.find('=') {
                return Some(after_let[..eq_pos].trim().to_string());
            }
        }
        None
    }

    /// Check if a word is a built-in identifier
    fn is_builtin(&self, word: &str) -> bool {
        matches!(word, "true" | "false" | "null" | "undefined" | "console" | "window" | "document")
    }
}

/// Result of linting operations
#[derive(Debug, Clone, PartialEq)]
pub struct LintResult {
    pub filename: Option<String>,
    pub errors: Vec<SemanticError>,
    pub warnings: Vec<SemanticWarning>,
}

impl LintResult {
    /// Check if the result has any issues
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    /// Get total number of issues
    pub fn issue_count(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_linting_is_deterministic() {
        let linter = PureLinter::new();
        let content = r#"let x = 1; console.log(y);"#;

        // Same input should always produce same output
        let result1 = linter.lint_content(content, Some("test.js"));
        let result2 = linter.lint_content(content, Some("test.js"));
        let result3 = linter.lint_content(content, Some("test.js"));

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_pure_linter_clone() {
        let linter1 = PureLinter::new();
        let linter2 = linter1.clone();

        let content = r#"let x = 1;"#;
        let result1 = linter1.lint_content(content, None);
        let result2 = linter2.lint_content(content, None);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_lint_result_has_issues() {
        let result_no_issues = LintResult {
            filename: None,
            errors: vec![],
            warnings: vec![],
        };

        let result_with_issues = LintResult {
            filename: None,
            errors: vec![SemanticError::UndefinedVariable("test".to_string(), super::super::Location { line: 1, column: 0 })],
            warnings: vec![],
        };

        assert!(!result_no_issues.has_issues());
        assert!(result_with_issues.has_issues());
        assert_eq!(result_with_issues.issue_count(), 1);
    }

    #[test]
    fn test_unused_variable_detection() {
        let config = LinterConfig {
            enabled_rules: vec!["no-unused-vars".to_string()],
            check_unused_variables: true,
            ..Default::default()
        };

        let linter = PureLinter::with_config(config);
        let content = "let unused_var = 1;\nlet used_var = 2;\nconsole.log(used_var);";

        let result = linter.lint_content(content, None);

        // Should detect unused variable
        assert!(result.warnings.iter().any(|w| matches!(w, SemanticWarning::UnusedVariable(var, _) if var == "unused_var")));
    }
}

//! Pure Code Formatter - no side effects, fully deterministic
//!
//! This module provides pure code formatting functionality.
//! All formatting operations are deterministic and have no side effects.

use crate::FormatterConfig;

/// Pure code formatter - performs only deterministic formatting operations
#[derive(Debug, Clone)]
pub struct PureFormatter {
    config: FormatterConfig,
}

impl PureFormatter {
    /// Create a new pure formatter with default configuration
    pub fn new() -> Self {
        Self {
            config: FormatterConfig::default(),
        }
    }

    /// Create a pure formatter with custom configuration
    pub fn with_config(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Pure formatting of source code content
    ///
    /// This function is PURE: same input always produces same output,
    /// no side effects, no external dependencies.
    pub fn format_content(&self, content: &str) -> String {
        if content.is_empty() {
            return String::new();
        }

        // Simplified formatting implementation
        // In real implementation, this would:
        // 1. Parse the source code into AST
        // 2. Apply formatting rules based on config
        // 3. Generate formatted code from AST

        let mut formatted = String::with_capacity(content.len() * 2);

        // Basic formatting: add indentation and line breaks
        let mut indent_level = 0;
        let indent_str = self.get_indent_string();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                formatted.push('\n');
                continue;
            }

            // Handle braces for indentation
            if trimmed.contains('{') {
                formatted.push_str(&indent_str.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
                indent_level += 1;
            } else if trimmed.contains('}') {
                indent_level = indent_level.saturating_sub(1);
                formatted.push_str(&indent_str.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
            } else {
                formatted.push_str(&indent_str.repeat(indent_level));
                formatted.push_str(trimmed);
                formatted.push('\n');
            }
        }

        // Remove trailing newline if original didn't have one
        if !content.ends_with('\n') && formatted.ends_with('\n') {
            formatted.pop();
        }

        formatted
    }

    /// Get the indent string based on configuration
    fn get_indent_string(&self) -> String {
        match self.config.indent_style {
            crate::IndentStyle::Space => " ".repeat(self.config.indent_width),
            crate::IndentStyle::Tab => "\t".repeat(self.config.indent_width),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FormatterConfig;

    #[test]
    fn test_pure_formatting_is_deterministic() {
        let formatter = PureFormatter::new();
        let content = r#"function test() { return "hello"; }"#;

        // Same input should always produce same output
        let result1 = formatter.format_content(content);
        let result2 = formatter.format_content(content);
        let result3 = formatter.format_content(content);

        assert_eq!(result1, result2);
        assert_eq!(result2, result3);
    }

    #[test]
    fn test_pure_formatter_clone() {
        let formatter1 = PureFormatter::new();
        let formatter2 = formatter1.clone();

        let content = r#"let x = 1;"#;
        let result1 = formatter1.format_content(content);
        let result2 = formatter2.format_content(content);

        assert_eq!(result1, result2);
    }

    #[test]
    fn test_formatting_with_different_configs() {
        let space_config = FormatterConfig {
            indent_style: crate::IndentStyle::Space,
            indent_width: 4,
            ..Default::default()
        };

        let tab_config = FormatterConfig {
            indent_style: crate::IndentStyle::Tab,
            indent_width: 2,
            ..Default::default()
        };

        let space_formatter = PureFormatter::with_config(space_config);
        let tab_formatter = PureFormatter::with_config(tab_config);

        let content = "{\n  \"key\": \"value\"\n}";

        let space_result = space_formatter.format_content(content);
        let tab_result = tab_formatter.format_content(content);

        // Results should be different due to different indentation
        assert_ne!(space_result, tab_result);

        // But each should be deterministic
        assert_eq!(space_result, space_formatter.format_content(content));
        assert_eq!(tab_result, tab_formatter.format_content(content));
    }

    #[test]
    fn test_empty_content() {
        let formatter = PureFormatter::new();

        assert_eq!(formatter.format_content(""), "");
        assert_eq!(formatter.format_content("   "), "   ");
    }
}

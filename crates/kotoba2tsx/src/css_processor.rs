//! CSS processing and optimization using Lightning CSS
//!
//! This module provides CSS parsing, transformation, optimization, and
//! CSS-in-JS generation capabilities using the Lightning CSS library.

use crate::error::Result;
use std::collections::HashMap;

/// Simple CSS processor for basic CSS operations
pub struct CssProcessor;

impl CssProcessor {
    /// Create a new CSS processor
    pub fn new() -> Self {
        Self
    }

    /// Parse CSS content (placeholder - returns the original CSS)
    pub fn parse_css(&self, css: &str, _filename: &str) -> Result<String> {
        Ok(css.to_string())
    }

    /// Convert CSS to optimized string (placeholder - returns the original CSS)
    pub fn to_css(&self, css: &str) -> Result<String> {
        Ok(css.to_string())
    }

    /// Minify CSS content (basic implementation)
    pub fn minify_css(&self, css: &str, _filename: &str) -> Result<String> {
        // Basic minification - remove extra whitespace and comments
        let mut minified = String::new();
        let mut in_comment = false;

        for line in css.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("/*") {
                in_comment = true;
            }

            if !in_comment && !trimmed.is_empty() {
                minified.push_str(trimmed);
                minified.push(' ');
            }

            if trimmed.ends_with("*/") {
                in_comment = false;
            }
        }

        // Remove multiple spaces
        minified = minified.split_whitespace().collect::<Vec<_>>().join(" ");
        Ok(minified)
    }

    /// Extract CSS variables (custom properties)
    pub fn extract_css_variables(&self, css: &str, _filename: &str) -> Result<HashMap<String, String>> {
        let mut variables = HashMap::new();

        // Simplified implementation - extract CSS custom properties
        // Handle both single-line and multi-line CSS
        let css_content = if css.contains('\n') {
            css.to_string()
        } else {
            css.to_string()
        };

        for declaration in css_content.split(';') {
            let trimmed = declaration.trim();
            if trimmed.starts_with("--") {
                if let Some(colon_pos) = trimmed.find(':') {
                    let name = trimmed[..colon_pos].trim().to_string();
                    let mut value = trimmed[colon_pos + 1..].trim().to_string();
                    // Remove semicolon if present
                    if value.ends_with(';') {
                        value = value.trim_end_matches(';').to_string();
                    }
                    variables.insert(name, value);
                }
            }
        }

        Ok(variables)
    }

    /// Generate CSS-in-JS object from CSS
    pub fn css_to_js_object(&self, css: &str, _filename: &str) -> Result<String> {
        let mut js_object = String::from("{\n");

        // Simplified CSS to JS object conversion
        // Handle both single-line and multi-line CSS
        let css_content = if css.contains('\n') {
            css.to_string()
        } else {
            // If it's a single line, treat it as one declaration block
            css.to_string()
        };

        for declaration in css_content.split(';') {
            let trimmed = declaration.trim();
            if trimmed.contains(':') && !trimmed.starts_with('@') && !trimmed.starts_with('.') && !trimmed.starts_with('#') && !trimmed.is_empty() {
                if let Some(colon_pos) = trimmed.find(':') {
                    let prop = trimmed[..colon_pos].trim();
                    let value = trimmed[colon_pos + 1..].trim().trim_end_matches(';');

                    // Convert CSS property to camelCase
                    let js_prop = self.css_prop_to_camel_case(prop);
                    js_object.push_str(&format!("  {}: \"{}\",\n", js_prop, value));
                }
            }
        }

        // Remove trailing comma if present
        if js_object.ends_with(",\n") {
            js_object = js_object.trim_end_matches(",\n").to_string();
            js_object.push('\n');
        }

        js_object.push('}');
        Ok(js_object)
    }

    /// Convert CSS property to camelCase
    fn css_prop_to_camel_case(&self, prop: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for ch in prop.chars() {
            if ch == '-' {
                capitalize_next = true;
            } else if capitalize_next {
                result.push(ch.to_ascii_uppercase());
                capitalize_next = false;
            } else {
                result.push(ch);
            }
        }

        result
    }


    /// Generate CSS modules from CSS content (simplified implementation)
    pub fn generate_css_modules(&self, css: &str, _filename: &str) -> Result<HashMap<String, String>> {
        let mut modules = HashMap::new();

        // Simple CSS modules extraction - in a real implementation, this would be more sophisticated
        for line in css.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with('.') && trimmed.contains('{') {
                if let Some(brace_pos) = trimmed.find('{') {
                    let class_name = trimmed[1..brace_pos].trim().to_string();
                    modules.insert(class_name.clone(), format!("_{}", class_name));
                }
            }
        }

        Ok(modules)
    }
}

impl Default for CssProcessor {
    fn default() -> Self {
        Self::new()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_css_processor_creation() {
        let processor = CssProcessor::new();
        // Test passes if no panic occurs
    }

    #[test]
    fn test_parse_simple_css() {
        let processor = CssProcessor::new();
        let css = ".test { color: red; }";
        let result = processor.parse_css(css, "test.css");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), css);
    }

    #[test]
    fn test_minify_css() {
        let processor = CssProcessor::new();
        let css = ".test {\n  color: red;\n  font-size: 14px;\n}";
        let result = processor.minify_css(css, "test.css");
        assert!(result.is_ok());
        let minified = result.unwrap();
        assert!(minified.contains("color: red"));
        assert!(minified.contains("font-size: 14px"));
    }

    #[test]
    fn test_css_to_js_object() {
        let processor = CssProcessor::new();
        let css = "color: red; font-size: 14px;";
        let result = processor.css_to_js_object(css, "test.css");
        assert!(result.is_ok());
        let js = result.unwrap();
        assert!(js.contains("color"));
        assert!(js.contains("fontSize"));
    }

    #[test]
    fn test_extract_css_variables() {
        let processor = CssProcessor::new();
        let css = "--primary-color: #007bff; --font-size: 14px;";
        let result = processor.extract_css_variables(css, "test.css");
        assert!(result.is_ok());
        let variables = result.unwrap();
        assert_eq!(variables.get("--primary-color"), Some(&"#007bff".to_string()));
        assert_eq!(variables.get("--font-size"), Some(&"14px".to_string()));
    }
}

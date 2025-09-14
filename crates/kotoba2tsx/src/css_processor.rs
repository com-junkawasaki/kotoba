//! CSS processing and optimization using Lightning CSS
//!
//! This module provides CSS parsing, transformation, optimization, and
//! CSS-in-JS generation capabilities using the Lightning CSS library.

use crate::error::{Kotoba2TSError, Result};
use lightningcss::{
    bundler::{Bundler, FileProvider},
    css_modules::CssModuleExports,
    printer::PrinterOptions,
    rules::{CssRule, CssRuleList},
    stylesheet::{MinifyOptions, ParserOptions, StyleSheet},
    targets::{Browsers, Targets},
    traits::IntoOwned,
    values::color::CssColor,
};
use std::collections::HashMap;
use std::path::Path;

/// CSS processor using Lightning CSS
pub struct CssProcessor {
    targets: Targets,
    minify_options: MinifyOptions,
}

impl CssProcessor {
    /// Create a new CSS processor with default configuration
    pub fn new() -> Self {
        Self {
            targets: Targets {
                browsers: Some(Browsers {
                    android: Some(80 << 16),
                    chrome: Some(80 << 16),
                    edge: Some(80 << 16),
                    firefox: Some(75 << 16),
                    ios_saf: Some(13 << 16),
                    safari: Some(13 << 16),
                    ..Browsers::default()
                }),
                ..Targets::default()
            },
            minify_options: MinifyOptions {
                targets: None,
                unused_symbols: Default::default(),
            },
        }
    }

    /// Create a new CSS processor with custom browser targets
    pub fn with_targets(targets: Targets) -> Self {
        Self {
            targets,
            minify_options: MinifyOptions {
                targets: Some(targets),
                unused_symbols: Default::default(),
            },
        }
    }

    /// Parse CSS content into a stylesheet
    pub fn parse_css(&self, css: &str, filename: &str) -> Result<StyleSheet> {
        let mut stylesheet = StyleSheet::parse(css, ParserOptions {
            filename: filename.to_string(),
            css_modules: None,
            source_index: 0,
            error_recovery: true,
            warnings: None,
        })
        .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS parse error: {}", err)))?;

        // Apply browser-specific transformations
        stylesheet
            .minify(self.minify_options.clone())
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS minify error: {}", err)))?;

        Ok(stylesheet)
    }

    /// Convert CSS to optimized string
    pub fn to_css(&self, stylesheet: &StyleSheet) -> Result<String> {
        let mut css = String::new();
        let printer_options = PrinterOptions {
            minify: false,
            targets: Some(self.targets),
            ..PrinterOptions::default()
        };

        stylesheet
            .to_css(printer_options)
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS generation error: {}", err)))?
            .write(&mut css)
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS write error: {}", err)))?;

        Ok(css)
    }

    /// Minify CSS content
    pub fn minify_css(&self, css: &str, filename: &str) -> Result<String> {
        let stylesheet = self.parse_css(css, filename)?;

        let mut minified = String::new();
        let printer_options = PrinterOptions {
            minify: true,
            targets: Some(self.targets),
            ..PrinterOptions::default()
        };

        stylesheet
            .to_css(printer_options)
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS minification error: {}", err)))?
            .write(&mut minified)
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS write error: {}", err)))?;

        Ok(minified)
    }

    /// Extract CSS variables (custom properties)
    pub fn extract_css_variables(&self, css: &str, filename: &str) -> Result<HashMap<String, String>> {
        let stylesheet = self.parse_css(css, filename)?;
        let mut variables = HashMap::new();

        for rule in &stylesheet.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                for declaration in &style_rule.declarations.declarations {
                    if let lightningcss::properties::Property::Custom(custom) = &declaration.0 {
                        if let Some(value) = self.css_value_to_string(&custom.value) {
                            variables.insert(custom.name.clone(), value);
                        }
                    }
                }
            }
        }

        Ok(variables)
    }

    /// Convert CSS value to string representation
    fn css_value_to_string(&self, value: &lightningcss::values::CustomValue) -> Option<String> {
        // This is a simplified implementation
        // In practice, you'd need to handle all CSS value types
        match value {
            lightningcss::values::CustomValue::CssFunction(_) => None,
            lightningcss::values::CustomValue::TokenList(tokens) => {
                Some(tokens.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" "))
            }
        }
    }

    /// Generate CSS-in-JS object from CSS
    pub fn css_to_js_object(&self, css: &str, filename: &str) -> Result<String> {
        let stylesheet = self.parse_css(css, filename)?;
        let mut js_object = String::from("{\n");

        for rule in &stylesheet.rules.0 {
            if let CssRule::Style(style_rule) = rule {
                // Convert selectors to camelCase for JS object keys
                for selector in &style_rule.selectors.0 {
                    let selector_str = selector.to_string();
                    let js_key = self.selector_to_js_key(&selector_str);

                    js_object.push_str(&format!("  \"{}\": {{\n", js_key));

                    for declaration in &style_rule.declarations.declarations {
                        let (prop, value) = self.declaration_to_js(declaration);
                        js_object.push_str(&format!("    {}: \"{}\",\n", prop, value));
                    }

                    js_object.push_str("  },\n");
                }
            }
        }

        js_object.push('}');
        Ok(js_object)
    }

    /// Convert CSS selector to JavaScript object key
    fn selector_to_js_key(&self, selector: &str) -> String {
        // Simple conversion - in practice, you'd want more sophisticated selector parsing
        selector
            .replace(".", "")
            .replace("#", "")
            .replace(" ", "_")
            .replace("-", "_")
            .replace(":", "_")
    }

    /// Convert CSS declaration to JavaScript property-value pair
    fn declaration_to_js(&self, declaration: &lightningcss::stylesheet::Declaration) -> (String, String) {
        match &declaration.0 {
            lightningcss::properties::Property::Display(d) => ("display".to_string(), format!("{:?}", d).to_lowercase()),
            lightningcss::properties::Property::Width(w) => ("width".to_string(), self.length_to_string(w)),
            lightningcss::properties::Property::Height(h) => ("height".to_string(), self.length_to_string(h)),
            lightningcss::properties::Property::Color(c) => ("color".to_string(), self.color_to_string(c)),
            lightningcss::properties::Property::BackgroundColor(c) => ("backgroundColor".to_string(), self.color_to_string(c)),
            lightningcss::properties::Property::FontSize(fs) => ("fontSize".to_string(), self.length_to_string(fs)),
            lightningcss::properties::Property::Margin(m) => ("margin".to_string(), self.rect_to_string(m)),
            lightningcss::properties::Property::Padding(p) => ("padding".to_string(), self.rect_to_string(p)),
            _ => ("unknown".to_string(), "".to_string()),
        }
    }

    /// Convert CSS length to string
    fn length_to_string(&self, length: &lightningcss::values::length::Length) -> String {
        match length {
            lightningcss::values::length::Length::Value { value, unit } => {
                format!("{}{}", value, unit)
            }
            lightningcss::values::length::Length::Auto => "auto".to_string(),
            _ => "".to_string(),
        }
    }

    /// Convert CSS color to string
    fn color_to_string(&self, color: &CssColor) -> String {
        match color {
            CssColor::CurrentColor => "currentColor".to_string(),
            CssColor::Transparent => "transparent".to_string(),
            CssColor::RGBA(rgba) => format!("rgba({}, {}, {}, {})", rgba.red, rgba.green, rgba.blue, rgba.alpha),
            CssColor::LAB(_) => "lab(...)".to_string(), // Simplified
            CssColor::LCH(_) => "lch(...)".to_string(), // Simplified
            CssColor::OKLAB(_) => "oklab(...)".to_string(), // Simplified
            CssColor::OKLCH(_) => "oklch(...)".to_string(), // Simplified
            CssColor::SRGB(_) => "color(srgb ...)".to_string(), // Simplified
            CssColor::DisplayP3(_) => "color(display-p3 ...)".to_string(), // Simplified
            CssColor::XYZ(_) => "color(xyz ...)".to_string(), // Simplified
        }
    }

    /// Convert CSS rect (margin/padding) to string
    fn rect_to_string(&self, rect: &lightningcss::values::rect::Rect<lightningcss::values::length::Length>) -> String {
        match (rect.0.as_ref(), rect.1.as_ref(), rect.2.as_ref(), rect.3.as_ref()) {
            (top, right, bottom, left) => {
                if top == right && right == bottom && bottom == left {
                    self.length_to_string(top)
                } else {
                    format!("{} {} {} {}", self.length_to_string(top), self.length_to_string(right), self.length_to_string(bottom), self.length_to_string(left))
                }
            }
        }
    }

    /// Generate CSS modules from CSS content
    pub fn generate_css_modules(&self, css: &str, filename: &str) -> Result<CssModuleExports> {
        let mut stylesheet = StyleSheet::parse(css, ParserOptions {
            filename: filename.to_string(),
            css_modules: Some(lightningcss::css_modules::Config {
                pattern: lightningcss::css_modules::Pattern::Local,
            }),
            source_index: 0,
            error_recovery: true,
            warnings: None,
        })
        .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS modules parse error: {}", err)))?;

        stylesheet
            .minify(self.minify_options.clone())
            .map_err(|err| Kotoba2TSError::CssProcessing(format!("CSS modules minify error: {}", err)))?;

        let exports = stylesheet.to_css_modules().map_err(|err| {
            Kotoba2TSError::CssProcessing(format!("CSS modules generation error: {}", err))
        })?;

        Ok(exports)
    }
}

impl Default for CssProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS bundler for combining multiple CSS files
pub struct CssBundler {
    processor: CssProcessor,
}

impl CssBundler {
    /// Create a new CSS bundler
    pub fn new() -> Self {
        Self {
            processor: CssProcessor::new(),
        }
    }

    /// Bundle multiple CSS files into one
    pub fn bundle_files(&self, files: Vec<(String, String)>) -> Result<String> {
        let mut bundled_css = String::new();

        for (filename, content) in files {
            let processed = self.processor.to_css(&self.processor.parse_css(&content, &filename)?)?;
            bundled_css.push_str(&processed);
            bundled_css.push('\n');
        }

        Ok(bundled_css)
    }

    /// Bundle CSS with dependency resolution
    pub fn bundle_with_deps(&self, entry_file: &str, file_provider: impl FileProvider) -> Result<String> {
        let mut bundler = Bundler::new(file_provider, ParserOptions {
            filename: entry_file.to_string(),
            css_modules: None,
            source_index: 0,
            error_recovery: true,
            warnings: None,
        });

        let stylesheet = bundler.bundle(entry_file).map_err(|err| {
            Kotoba2TSError::CssProcessing(format!("CSS bundling error: {}", err))
        })?;

        self.processor.to_css(&stylesheet)
    }
}

impl Default for CssBundler {
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
        assert!(processor.targets.browsers.is_some());
    }

    #[test]
    fn test_parse_simple_css() {
        let processor = CssProcessor::new();
        let css = ".test { color: red; }";
        let result = processor.parse_css(css, "test.css");
        assert!(result.is_ok());
    }

    #[test]
    fn test_minify_css() {
        let processor = CssProcessor::new();
        let css = ".test {\n  color: red;\n  font-size: 14px;\n}";
        let result = processor.minify_css(css, "test.css");
        assert!(result.is_ok());
        let minified = result.unwrap();
        assert!(!minified.contains('\n'));
    }

    #[test]
    fn test_css_to_js_object() {
        let processor = CssProcessor::new();
        let css = ".button { color: red; font-size: 14px; }";
        let result = processor.css_to_js_object(css, "test.css");
        assert!(result.is_ok());
        let js = result.unwrap();
        assert!(js.contains("button"));
        assert!(js.contains("color"));
    }

    #[test]
    fn test_selector_to_js_key() {
        let processor = CssProcessor::new();
        assert_eq!(processor.selector_to_js_key(".my-class"), "my-class");
        assert_eq!(processor.selector_to_js_key("#my-id"), "my-id");
        assert_eq!(processor.selector_to_js_key(".btn:hover"), "btn_hover");
    }
}

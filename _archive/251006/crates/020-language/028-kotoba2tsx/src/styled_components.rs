//! Styled-components and Emotion CSS-in-JS integration
//!
//! This module provides support for generating styled-components and Emotion
//! CSS-in-JS code from Kotoba configurations and CSS styles.

use crate::error::Result;
use crate::css_processor::CssProcessor;
use crate::swc_integration::SwcCodeGenerator;
use std::collections::HashMap;

/// Styled-components generator
pub struct StyledComponentsGenerator {
    css_processor: CssProcessor,
}

impl StyledComponentsGenerator {
    /// Create a new styled-components generator
    pub fn new() -> Self {
        Self {
            css_processor: CssProcessor::new(),
        }
    }

    /// Generate styled-components code from CSS
    pub fn generate_styled_component(&self, component_name: &str, css: &str, tag_name: &str) -> Result<String> {
        let processed_css = self.css_processor.minify_css(css, &format!("{}.css", component_name))?;

        let styled_code = format!(
            "const {} = styled.{}`\n{}\n`;\n",
            component_name,
            tag_name,
            processed_css
        );

        Ok(styled_code)
    }

    /// Generate styled-components with props interpolation
    pub fn generate_styled_component_with_props(
        &self,
        component_name: &str,
        css: &str,
        tag_name: &str,
        props: Vec<String>,
    ) -> Result<String> {
        let mut processed_css = css.to_string();

        // Replace prop placeholders with styled-components interpolation
        for prop in &props {
            let placeholder = format!("${{{}}}", prop);
            let interpolation = format!("${{({{{}}}) => {}.{} || 'inherit'}}", prop, "props", prop);
            processed_css = processed_css.replace(&placeholder, &interpolation);
        }

        let styled_code = format!(
            "const {} = styled.{}`\n{}\n`;\n",
            component_name,
            tag_name,
            processed_css
        );

        Ok(styled_code)
    }

    /// Generate styled-components with theme support
    pub fn generate_styled_component_with_theme(
        &self,
        component_name: &str,
        css: &str,
        tag_name: &str,
        theme_props: Vec<String>,
    ) -> Result<String> {
        let mut processed_css = css.to_string();

        // Replace theme placeholders with styled-components theme interpolation
        for prop in &theme_props {
            let placeholder = format!("${{theme.{}}}", prop);
            let interpolation = format!("${{({{theme}}) => theme.{}}}", prop);
            processed_css = processed_css.replace(&placeholder, &interpolation);
        }

        let styled_code = format!(
            "const {} = styled.{}`\n{}\n`;\n",
            component_name,
            tag_name,
            processed_css
        );

        Ok(styled_code)
    }

    /// Generate styled-components import statement
    pub fn generate_styled_import(&self) -> String {
        "import styled from 'styled-components';\n".to_string()
    }

    /// Generate styled-components with TypeScript types
    pub fn generate_typed_styled_component(
        &self,
        component_name: &str,
        css: &str,
        tag_name: &str,
        _props_interface: &str,
    ) -> Result<String> {
        let processed_css = self.css_processor.minify_css(css, &format!("{}.css", component_name))?;

        let styled_code = format!(
            "const {} = styled.{}<{}Props>`\n{}\n`;\n",
            component_name,
            tag_name,
            component_name,
            processed_css
        );

        Ok(styled_code)
    }

    /// Generate CSS helper functions for styled-components
    pub fn generate_css_helpers(&self) -> String {
        let mut helpers = String::new();

        helpers.push_str("// CSS Helper Functions\n");
        helpers.push_str("const css = styled.css;\n");
        helpers.push_str("const keyframes = styled.keyframes;\n");
        helpers.push_str("const createGlobalStyle = styled.createGlobalStyle;\n\n");

        helpers
    }

    /// Generate keyframes for animations
    pub fn generate_keyframes(&self, name: &str, keyframes_css: &str) -> Result<String> {
        let processed_css = self.css_processor.minify_css(keyframes_css, &format!("{}.css", name))?;

        let keyframes_code = format!(
            "const {} = keyframes`\n{}\n`;\n",
            name,
            processed_css
        );

        Ok(keyframes_code)
    }

    /// Generate global styles
    pub fn generate_global_styles(&self, css: &str) -> Result<String> {
        let processed_css = self.css_processor.minify_css(css, "global.css")?;

        let global_code = format!(
            "const GlobalStyles = createGlobalStyle`\n{}\n`;\n",
            processed_css
        );

        Ok(global_code)
    }
}

impl Default for StyledComponentsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Emotion CSS-in-JS generator
pub struct EmotionGenerator {
    css_processor: CssProcessor,
}

impl EmotionGenerator {
    /// Create a new Emotion generator
    pub fn new() -> Self {
        Self {
            css_processor: CssProcessor::new(),
        }
    }

    /// Generate Emotion css function call
    pub fn generate_emotion_css(&self, component_name: &str, css: &str) -> Result<String> {
        let processed_css = self.css_processor.minify_css(css, &format!("{}.css", component_name))?;

        let emotion_code = format!(
            "const {}Styles = css`\n{}\n`;\n",
            component_name,
            processed_css
        );

        Ok(emotion_code)
    }

    /// Generate Emotion styled function
    pub fn generate_emotion_styled(&self, component_name: &str, tag_name: &str, css: &str) -> Result<String> {
        let processed_css = self.css_processor.minify_css(css, &format!("{}.css", component_name))?;

        let emotion_code = format!(
            "const {} = styled('{}')`\n{}\n`;\n",
            component_name,
            tag_name,
            processed_css
        );

        Ok(emotion_code)
    }

    /// Generate Emotion import statement
    pub fn generate_emotion_import(&self) -> String {
        "import { css, styled } from '@emotion/react';\n".to_string()
    }

    /// Generate Emotion with props interpolation
    pub fn generate_emotion_with_props(
        &self,
        component_name: &str,
        css: &str,
        props: Vec<String>,
    ) -> Result<String> {
        let mut processed_css = css.to_string();

        // Replace prop placeholders with Emotion interpolation
        for prop in &props {
            let placeholder = format!("${{{}}}", prop);
            let interpolation = format!("${{({{{}}}) => {}.{} || 'inherit'}}", prop, "props", prop);
            processed_css = processed_css.replace(&placeholder, &interpolation);
        }

        let emotion_code = format!(
            "const {}Styles = css`\n{}\n`;\n",
            component_name,
            processed_css
        );

        Ok(emotion_code)
    }

    /// Generate Emotion component with cx function for conditional styles
    pub fn generate_emotion_component_with_cx(
        &self,
        component_name: &str,
        base_css: &str,
        conditional_styles: HashMap<String, String>,
    ) -> Result<String> {
        let mut code = String::new();

        // Base styles
        let base_processed = self.css_processor.minify_css(base_css, &format!("{}.css", component_name))?;
        code.push_str(&format!(
            "const {}BaseStyles = css`\n{}\n`;\n",
            component_name,
            base_processed
        ));

        // Collect keys before moving conditional_styles
        let keys: Vec<String> = conditional_styles.keys().cloned().collect();

        // Conditional styles
        for (condition, css) in conditional_styles {
            let processed = self.css_processor.minify_css(&css, &format!("{}_{}.css", component_name, condition))?;
            code.push_str(&format!(
                "const {}{}Styles = css`\n{}\n`;\n",
                component_name,
                condition,
                processed
            ));
        }

        // Component function
        let props_list = keys.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
        code.push_str(&format!(
            "const {} = ({{ {}, className, ...props }}) => {{\n",
            component_name,
            props_list
        ));
        code.push_str("  const conditionalClasses = [\n");

        for condition in &keys {
            code.push_str(&format!("    {} && {}{}Styles,\n", condition, component_name, condition));
        }

        code.push_str("  ].filter(Boolean);\n\n");
        let base_styles = format!("{}BaseStyles", component_name);
        code.push_str(&format!("  return (\n    <div\n      className={{cx({}, ...conditionalClasses, className)}} \n      {{...props}}\n    />\n  );\n}};\n", base_styles));

        Ok(code)
    }

    /// Generate Emotion cx import
    pub fn generate_emotion_cx_import(&self) -> String {
        "import { cx } from '@emotion/css';\n".to_string()
    }
}

impl Default for EmotionGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// CSS-in-JS theme provider generator
pub struct ThemeProviderGenerator {}

impl ThemeProviderGenerator {
    /// Create a new theme provider generator
    pub fn new() -> Self {
        Self {}
    }

    /// Generate theme type definitions
    pub fn generate_theme_types(&self, theme_structure: HashMap<String, String>) -> String {
        let mut types = String::new();
        types.push_str("export interface Theme {\n");

        for (key, value_type) in theme_structure {
            types.push_str(&format!("  {}: {};\n", key, value_type));
        }

        types.push_str("}\n\n");
        types
    }

    /// Generate theme object
    pub fn generate_theme_object(&self, theme_data: HashMap<String, serde_json::Value>) -> String {
        let mut theme = String::from("export const theme: Theme = {\n");

        for (key, value) in theme_data {
            match value {
                serde_json::Value::String(s) => theme.push_str(&format!("  {}: '{}',\n", key, s)),
                serde_json::Value::Number(n) => theme.push_str(&format!("  {}: {},\n", key, n)),
                serde_json::Value::Bool(b) => theme.push_str(&format!("  {}: {},\n", key, b)),
                serde_json::Value::Object(obj) => {
                    theme.push_str(&format!("  {}: {{\n", key));
                    for (sub_key, sub_value) in obj {
                        match sub_value {
                            serde_json::Value::String(s) => theme.push_str(&format!("    {}: '{}',\n", sub_key, s)),
                            serde_json::Value::Number(n) => theme.push_str(&format!("    {}: {},\n", sub_key, n)),
                            serde_json::Value::Bool(b) => theme.push_str(&format!("    {}: {},\n", sub_key, b)),
                            _ => theme.push_str(&format!("    {}: null,\n", sub_key)),
                        }
                    }
                    theme.push_str("  },\n");
                }
                _ => theme.push_str(&format!("  {}: null,\n", key)),
            }
        }

        theme.push_str("};\n\n");
        theme
    }

    /// Generate theme provider component
    pub fn generate_theme_provider(&self, library: &str) -> String {
        match library {
            "styled-components" => {
                "import { ThemeProvider } from 'styled-components';\n\n".to_string() +
                "const App = () => (\n" +
                "  <ThemeProvider theme={theme}>\n" +
                "    {/* Your app components */}\n" +
                "  </ThemeProvider>\n" +
                ");\n\n"
            }
            "emotion" => {
                "import { ThemeProvider } from '@emotion/react';\n\n".to_string() +
                "const App = () => (\n" +
                "  <ThemeProvider theme={theme}>\n" +
                "    {/* Your app components */}\n" +
                "  </ThemeProvider>\n" +
                ");\n\n"
            }
            _ => "".to_string(),
        }
    }
}

impl Default for ThemeProviderGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_styled_components_generator() {
        let generator = StyledComponentsGenerator::new();
        let css = "color: red; font-size: 14px;";
        let result = generator.generate_styled_component("Button", css, "button");
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("const Button = styled.button"));
        assert!(code.contains("color: red"));
    }

    #[test]
    fn test_emotion_generator() {
        let generator = EmotionGenerator::new();
        let css = "color: blue; font-size: 16px;";
        let result = generator.generate_emotion_css("Button", css);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("const ButtonStyles = css"));
        assert!(code.contains("color: blue"));
    }

    #[test]
    fn test_styled_components_with_props() {
        let generator = StyledComponentsGenerator::new();
        let css = "color: ${color}; font-size: ${size}px;";
        let props = vec!["color".to_string(), "size".to_string()];
        let result = generator.generate_styled_component_with_props("Button", css, "button", props);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("${({color}) => props.color"));
    }

    #[test]
    fn test_theme_provider_generator() {
        let generator = ThemeProviderGenerator::new();
        let mut theme_structure = HashMap::new();
        theme_structure.insert("primary".to_string(), "string".to_string());
        theme_structure.insert("fontSize".to_string(), "number".to_string());

        let types = generator.generate_theme_types(theme_structure);
        assert!(types.contains("interface Theme"));
        assert!(types.contains("primary: string;"));
    }

    #[test]
    fn test_theme_object_generation() {
        let generator = ThemeProviderGenerator::new();
        let mut theme_data = HashMap::new();
        theme_data.insert("primary".to_string(), json!("#007bff"));
        theme_data.insert("fontSize".to_string(), json!(14));

        let theme_obj = generator.generate_theme_object(theme_data);
        assert!(theme_obj.contains("primary: '#007bff'"));
        assert!(theme_obj.contains("fontSize: 14"));
    }
}

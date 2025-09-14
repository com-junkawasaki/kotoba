//! Parser for .kotoba configuration files

use crate::error::{Kotoba2TSError, Result};
use crate::types::{ComponentType, KotobaComponent, KotobaConfig};
use serde_json;
use std::collections::HashMap;
use tokio::fs as async_fs;
use jsonnet::JsonnetVm;

/// Parser for .kotoba files
pub struct KotobaParser {
    jsonnet_evaluator: Option<Box<dyn Fn(&str) -> Result<String> + Send + Sync>>,
}

impl KotobaParser {
    /// Create a new KotobaParser
    pub fn new() -> Self {
        Self {
            jsonnet_evaluator: None,
        }
    }

    /// Set a custom Jsonnet evaluator
    pub fn with_jsonnet_evaluator<F>(mut self, evaluator: F) -> Self
    where
        F: Fn(&str) -> Result<String> + Send + Sync + 'static,
    {
        self.jsonnet_evaluator = Some(Box::new(evaluator));
        self
    }

    /// Parse a .kotoba file from disk
    pub async fn parse_file(&self, file_path: &str) -> Result<KotobaConfig> {
        let content = async_fs::read_to_string(file_path).await
            .map_err(|_| Kotoba2TSError::FileNotFound(file_path.to_string()))?;
        self.parse_content(&content)
    }

    /// Parse .kotoba content from a string
    pub fn parse_content(&self, content: &str) -> Result<KotobaConfig> {
        // First, try to evaluate as Jsonnet
        let json_content = self.evaluate_jsonnet(content)?;

        // Parse the JSON content
        let parsed: serde_json::Value = serde_json::from_str(&json_content)?;

        // Convert to KotobaConfig
        self.parse_json_value(parsed)
    }

    /// Evaluate Jsonnet content to JSON
    fn evaluate_jsonnet(&self, content: &str) -> Result<String> {
        // If custom evaluator is provided, use it
        if let Some(ref evaluator) = self.jsonnet_evaluator {
            return evaluator(content);
        }

        // Default simple Jsonnet evaluation
        self.default_jsonnet_evaluation(content)
    }

    /// Default Jsonnet evaluation using jsonnet-rs
    fn default_jsonnet_evaluation(&self, content: &str) -> Result<String> {
        // Try to evaluate with jsonnet-rs first
        let mut vm = JsonnetVm::new();
        let result = vm.evaluate_snippet("input", content);
        match result {
            Ok(result_str) => {
                // Convert to owned string immediately to avoid lifetime issues
                Ok(result_str.to_string())
            }
            Err(e) => {
                // If jsonnet evaluation fails, try to parse as plain JSON
                match serde_json::from_str::<serde_json::Value>(content) {
                    Ok(value) => Ok(serde_json::to_string_pretty(&value)?),
                    Err(_) => {
                        Err(Kotoba2TSError::Jsonnet(format!("Failed to evaluate as Jsonnet or JSON: {}", e)))
                    }
                }
            }
        }
    }


    /// Remove comments from Jsonnet content
    fn remove_comments(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_multiline_comment = false;

        for line in content.lines() {
            let trimmed = line.trim();

            if in_multiline_comment {
                if trimmed.ends_with("*/") {
                    in_multiline_comment = false;
                }
                continue;
            }

            if trimmed.starts_with("/*") {
                if !trimmed.ends_with("*/") {
                    in_multiline_comment = true;
                }
                continue;
            }

            // Remove single-line comments
            if let Some(comment_start) = trimmed.find("//") {
                let before_comment = &trimmed[..comment_start];
                if !before_comment.trim().is_empty() {
                    result.push_str(before_comment);
                    result.push('\n');
                }
                continue;
            }

            if !trimmed.is_empty() {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }


    /// Parse JSON value into KotobaConfig
    fn parse_json_value(&self, value: serde_json::Value) -> Result<KotobaConfig> {
        let obj = value.as_object()
            .ok_or_else(|| Kotoba2TSError::InvalidFileFormat("Root must be an object".to_string()))?;

        // Extract config
        let config = obj.get("config")
            .and_then(|c| c.as_object())
            .cloned()
            .unwrap_or_default();

        let name = config.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("KotobaApp")
            .to_string();

        let version = config.get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("0.1.0")
            .to_string();

        let theme = config.get("theme")
            .and_then(|t| t.as_str())
            .unwrap_or("light")
            .to_string();

        // Parse components
        let mut components = HashMap::new();
        if let Some(comps) = obj.get("components").and_then(|c| c.as_object()) {
            for (name, comp) in comps {
                let component = self.parse_component(comp, ComponentType::Component)?;
                components.insert(name.clone(), component);
            }
        }

        // Parse handlers
        let mut handlers = HashMap::new();
        if let Some(hdls) = obj.get("handlers").and_then(|h| h.as_object()) {
            for (name, hdl) in hdls {
                let handler = self.parse_component(hdl, ComponentType::Handler)?;
                handlers.insert(name.clone(), handler);
            }
        }

        // Parse states
        let mut states = HashMap::new();
        if let Some(sts) = obj.get("states").and_then(|s| s.as_object()) {
            for (name, state) in sts {
                // Check if state is defined as object with "initial" field or direct value
                let initial = if let Some(obj) = state.as_object() {
                    obj.get("initial").cloned().unwrap_or(serde_json::Value::Null)
                } else {
                    // Direct value (boolean, string, number, etc.)
                    state.clone()
                };
                states.insert(name.clone(), initial);
            }
        }

        Ok(KotobaConfig {
            name,
            version,
            theme,
            components,
            handlers,
            states,
            config: config.into_iter().collect(),
        })
    }

    /// Parse a component from JSON value
    fn parse_component(&self, value: &serde_json::Value, default_type: ComponentType) -> Result<KotobaComponent> {
        let obj = value.as_object()
            .ok_or_else(|| Kotoba2TSError::InvalidComponent("Component must be an object".to_string()))?;

        let r#type = obj.get("type")
            .and_then(|t| t.as_str())
            .map(|t| match t {
                "component" => ComponentType::Component,
                "config" => ComponentType::Config,
                "handler" => ComponentType::Handler,
                "state" => ComponentType::State,
                _ => default_type.clone(),
            })
            .unwrap_or(default_type);

        let name = obj.get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| Kotoba2TSError::MissingField {
                field: "name".to_string(),
                component: "component".to_string(),
            })?
            .to_string();

        let component_type = obj.get("component_type")
            .and_then(|ct| ct.as_str())
            .map(|s| s.to_string());

        let props = obj.get("props")
            .and_then(|p| p.as_object())
            .map(|p| p.clone().into_iter().collect())
            .unwrap_or_default();

        let children = obj.get("children")
            .and_then(|c| c.as_array())
            .map(|arr| arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect())
            .unwrap_or_default();

        let function = obj.get("function")
            .and_then(|f| f.as_str())
            .map(|s| s.to_string());

        let initial = obj.get("initial").cloned();

        let metadata = obj.get("metadata")
            .and_then(|m| m.as_object())
            .map(|m| m.clone().into_iter().collect())
            .unwrap_or_default();

        Ok(KotobaComponent {
            r#type,
            name,
            component_type,
            props,
            children,
            function,
            initial,
            metadata,
        })
    }
}

impl Default for KotobaParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parser_new() {
        let parser = KotobaParser::new();
        assert!(parser.jsonnet_evaluator.is_none());
    }

    #[test]
    fn test_parser_with_jsonnet_evaluator() {
        let evaluator = |content: &str| Ok(content.to_string());
        let parser = KotobaParser::new().with_jsonnet_evaluator(evaluator);
        assert!(parser.jsonnet_evaluator.is_some());
    }

    #[test]
    fn test_parse_content_simple_json() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {
                "name": "TestApp",
                "version": "1.0.0",
                "theme": "dark"
            },
            "components": {},
            "handlers": {},
            "states": {}
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.name, "TestApp");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_parse_content_with_component() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {
                "name": "TestApp",
                "version": "1.0.0"
            },
            "components": {
                "Button": {
                    "type": "component",
                    "name": "Button",
                    "component_type": "button",
                    "props": {
                        "className": "btn",
                        "disabled": false
                    },
                    "children": []
                }
            },
            "handlers": {},
            "states": {}
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.contains_key("Button"));

        let button = &config.components["Button"];
        assert_eq!(button.name, "Button");
        assert_eq!(button.component_type, Some("button".to_string()));
        assert_eq!(button.props["className"], json!("btn"));
        assert_eq!(button.props["disabled"], json!(false));
    }

    #[test]
    fn test_parse_content_with_handler() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {
                "name": "TestApp",
                "version": "1.0.0"
            },
            "components": {},
            "handlers": {
                "onClick": {
                    "type": "handler",
                    "name": "onClick",
                    "function": "console.log('clicked');"
                }
            },
            "states": {}
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.handlers.contains_key("onClick"));

        let handler = &config.handlers["onClick"];
        assert_eq!(handler.name, "onClick");
        assert_eq!(handler.function, Some("console.log('clicked');".to_string()));
    }

    #[test]
    fn test_parse_content_with_state() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {
                "name": "TestApp",
                "version": "1.0.0"
            },
            "components": {},
            "handlers": {},
            "states": {
                "count": 0,
                "isVisible": true,
                "userName": "John"
            }
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.states["count"], json!(0));
        assert_eq!(config.states["isVisible"], json!(true));
        assert_eq!(config.states["userName"], json!("John"));
    }

    #[test]
    fn test_parse_invalid_json() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {
                "name": "TestApp",
                invalid json here
            }
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_missing_required_fields() {
        let parser = KotobaParser::new();
        let content = r#"{
            "config": {},
            "components": {
                "Test": {
                    "type": "component"
                }
            }
        }"#;

        let result = parser.parse_content(content);
        assert!(result.is_err());
        // Should fail due to missing "name" field
    }

    #[test]
    fn test_remove_comments() {
        let parser = KotobaParser::new();

        let content = r#"// This is a comment
{
    "config": {
        "name": "Test" // inline comment
    }
}
/* Multi-line
   comment */"#;

        let result = parser.remove_comments(content);
        // Should remove comments but keep valid JSON
        assert!(result.contains("Test"));
        assert!(!result.contains("//"));
        assert!(!result.contains("/*"));
    }

    #[test]
    fn test_parse_jsonnet_basic() {
        let parser = KotobaParser::new();
        let jsonnet_content = r#"{
            "config": {
                "name": "JsonnetApp",
                "version": "1.0.0",
                "theme": "dark"
            },
            "components": {},
            "handlers": {},
            "states": {}
        }"#;

        let result = parser.parse_content(jsonnet_content);
        if let Err(ref e) = result {
            println!("Jsonnet parse error: {:?}", e);
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.name, "JsonnetApp");
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_parse_jsonnet_with_variables() {
        let parser = KotobaParser::new();
        // Test Jsonnet local variables
        let jsonnet_content = r#"local appName = "VariableApp";
local version = "2.0.0";

{
    config: {
        name: appName,
        version: version,
        theme: "light"
    },
    components: {},
    handlers: {},
    states: {}
}"#;

        let result = parser.parse_content(jsonnet_content);
        if let Err(ref e) = result {
            println!("Jsonnet parse error: {:?}", e);
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.name, "VariableApp");
        assert_eq!(config.version, "2.0.0");
    }

    #[test]
    fn test_parse_jsonnet_with_functions() {
        let parser = KotobaParser::new();
        // Test Jsonnet functions
        let jsonnet_content = r#"local makeButton = function(name, className) {
    type: "component",
    name: name,
    component_type: "button",
    props: {
        className: className,
        disabled: false
    },
    children: []
};

{
    config: {
        name: "FunctionApp",
        version: "1.0.0",
        theme: "light"
    },
    components: {
        PrimaryButton: makeButton("PrimaryButton", "btn-primary"),
        SecondaryButton: makeButton("SecondaryButton", "btn-secondary")
    },
    handlers: {},
    states: {}
}"#;

        let result = parser.parse_content(jsonnet_content);
        if let Err(ref e) = result {
            println!("Jsonnet parse error: {:?}", e);
        }
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.contains_key("PrimaryButton"));
        assert!(config.components.contains_key("SecondaryButton"));

        let primary_btn = &config.components["PrimaryButton"];
        assert_eq!(primary_btn.props["className"], serde_json::json!("btn-primary"));
    }

    #[test]
    fn test_parse_jsonnet_invalid() {
        let parser = KotobaParser::new();
        let invalid_jsonnet = r#"local invalid = ;
{
    config: {
        name: "InvalidApp",
    },
}"#;

        let result = parser.parse_content(invalid_jsonnet);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_parse_file_nonexistent() {
        let parser = KotobaParser::new();
        let result = parser.parse_file("nonexistent_file.kotoba").await;
        assert!(result.is_err());
        match result.err().unwrap() {
            Kotoba2TSError::FileNotFound(_) => {},
            _ => panic!("Expected FileNotFound error"),
        }
    }
}

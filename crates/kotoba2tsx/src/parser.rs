//! Parser for .kotoba configuration files

use crate::error::{Kotoba2TSError, Result};
use crate::types::{ComponentType, KotobaComponent, KotobaConfig};
use serde_json;
use std::collections::HashMap;
use tokio::fs as async_fs;

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

    /// Default simple Jsonnet evaluation (basic implementation)
    fn default_jsonnet_evaluation(&self, content: &str) -> Result<String> {
        // Remove comments (basic implementation)
        let cleaned = self.remove_comments(content);

        // Try to parse as JSON directly first
        match serde_json::from_str::<serde_json::Value>(&cleaned) {
            Ok(value) => {
                // If it's already valid JSON, return it
                Ok(serde_json::to_string_pretty(&value)?)
            }
            Err(_) => {
                // Try basic Jsonnet evaluation
                self.evaluate_basic_jsonnet(&cleaned)
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

    /// Basic Jsonnet evaluation (simplified implementation)
    fn evaluate_basic_jsonnet(&self, content: &str) -> Result<String> {
        // This is a very basic implementation that handles some Jsonnet features
        // For production use, you should use a proper Jsonnet library

        let mut processed = content.to_string();

        // Handle local variables (very basic)
        processed = self.process_local_variables(&processed)?;

        // Handle object comprehensions (basic)
        processed = self.process_object_comprehensions(&processed)?;

        // Try to parse as JSON
        match serde_json::from_str::<serde_json::Value>(&processed) {
            Ok(value) => Ok(serde_json::to_string_pretty(&value)?),
            Err(e) => Err(Kotoba2TSError::Jsonnet(format!("Failed to evaluate Jsonnet: {}", e))),
        }
    }

    /// Process local variables (basic implementation)
    fn process_local_variables(&self, content: &str) -> Result<String> {
        // This is a very simplified implementation
        // A real Jsonnet implementation would be much more complex
        Ok(content.to_string())
    }

    /// Process object comprehensions (basic implementation)
    fn process_object_comprehensions(&self, content: &str) -> Result<String> {
        // This is a very simplified implementation
        Ok(content.to_string())
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
                let initial = state.get("initial").cloned().unwrap_or(serde_json::Value::Null);
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
            config: config.into_iter().map(|(k, v)| (k, v)).collect(),
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

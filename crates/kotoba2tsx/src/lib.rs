//! # Kotoba2TSX
//!
//! Convert Kotoba configuration files (.kotoba) to React TSX components.
//!
//! This crate provides functionality to parse Jsonnet-based .kotoba files
//! and generate corresponding React TSX component code.

pub mod types;
pub mod parser;
pub mod generator;
pub mod error;
pub mod swc_integration;
pub mod css_processor;
pub mod styled_components;

#[cfg(feature = "cli")]
pub mod cli;

pub use types::*;
pub use parser::*;
pub use generator::*;
pub use error::*;

/// Convert a .kotoba file to TSX code
///
/// # Arguments
/// * `input_path` - Path to the .kotoba file
/// * `output_path` - Path where the generated .tsx file will be written
///
/// # Returns
/// Result<(), Kotoba2TSError> indicating success or failure
pub async fn convert_file(input_path: &str, output_path: &str) -> crate::error::Result<()> {
    let parser = KotobaParser::new();
    let config = parser.parse_file(input_path).await?;
    let generator = TsxGenerator::new();
    generator.generate_file(&config, output_path).await?;
    Ok(())
}

/// Convert .kotoba content string to TSX code string
///
/// # Arguments
/// * `content` - The .kotoba file content as a string
///
/// # Returns
/// Result<String, Kotoba2TSError> containing the generated TSX code
pub fn convert_content(content: &str) -> crate::error::Result<String> {
    let parser = KotobaParser::new();
    let config = parser.parse_content(content)?;
    let generator = TsxGenerator::new();
    generator.generate_tsx(&config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_convert_content_simple_component() {
        let content = r#"{
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {
                "Button": {
                    "type": "component",
                    "name": "Button",
                    "component_type": "button",
                    "props": {
                        "className": "btn btn-primary",
                        "children": "Click me"
                    },
                    "children": [],
                    "metadata": {}
                }
            },
            "handlers": {},
            "states": {},
            "config": {}
        }"#;

        let result = convert_content(content);
        assert!(result.is_ok());
        let tsx = result.unwrap();
        assert!(tsx.contains("App"));
        assert!(tsx.contains("<button"));
        assert!(tsx.contains("className"));
        assert!(tsx.contains("Click me"));
    }

    #[test]
    fn test_convert_content_with_handler() {
        let content = r#"{
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {
                "Button": {
                    "type": "component",
                    "name": "Button",
                    "component_type": "button",
                    "props": {
                        "onClick": "handleClick",
                        "children": "Click me"
                    },
                    "children": [],
                    "metadata": {}
                }
            },
            "handlers": {
                "handleClick": {
                    "type": "handler",
                    "name": "handleClick",
                    "function": "console.log('Button clicked');"
                }
            },
            "state": {},
            "styles": {}
        }"#;

        let result = convert_content(content);
        assert!(result.is_ok());
        let tsx = result.unwrap();
        assert!(tsx.contains("handleClick"));
        assert!(tsx.contains("console.log"));
    }

    #[test]
    fn test_convert_content_with_state() {
        let content = r#"{
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {
                "Counter": {
                    "type": "component",
                    "name": "Counter",
                    "component_type": "div",
                    "props": {
                        "children": "{count}"
                    },
                    "children": [],
                    "metadata": {}
                }
            },
            "handlers": {},
            "states": {
                "count": {
                    "type": "state",
                    "name": "count",
                    "initial": 0
                }
            },
            "config": {}
        }"#;

        let result = convert_content(content);
        assert!(result.is_ok());
        let tsx = result.unwrap();
        assert!(tsx.contains("useState"));
        assert!(tsx.contains("count"));
        assert!(tsx.contains("0"));
    }

    #[test]
    fn test_convert_content_invalid_json() {
        let content = r#"{
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {
                "Button": {
                    "type": "component",
                    "name": "Button",
                    "component_type": "button"
                    // Missing comma here
                    "props": {}
                }
            }
        }"#;

        let result = convert_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_convert_content_empty_config() {
        let content = r#"{
            "name": "EmptyApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {},
            "handlers": {},
            "states": {},
            "config": {}
        }"#;

        let result = convert_content(content);
        assert!(result.is_ok());
        let tsx = result.unwrap();
        assert!(tsx.contains("App"));
        assert!(tsx.contains("export default"));
    }

    #[test]
    fn test_kotoba_component_serialization() {
        let component = KotobaComponent {
            r#type: ComponentType::Component,
            name: "TestButton".to_string(),
            component_type: Some("button".to_string()),
            props: {
                let mut props = HashMap::new();
                props.insert("className".to_string(), json!("btn"));
                props
            },
            children: vec![],
            function: None,
            initial: None,
            metadata: HashMap::new(),
        };

        // Test serialization
        let json = serde_json::to_string(&component).unwrap();
        assert!(json.contains("TestButton"));
        assert!(json.contains("button"));
        assert!(json.contains("btn"));

        // Test deserialization
        let deserialized: KotobaComponent = serde_json::from_str(&json).unwrap();
        assert_eq!(component, deserialized);
    }

    #[test]
    fn test_kotoba_config_serialization() {
        let mut components = HashMap::new();
        components.insert("Button".to_string(), KotobaComponent {
            r#type: ComponentType::Component,
            name: "Button".to_string(),
            component_type: Some("button".to_string()),
            props: HashMap::new(),
            children: vec![],
            function: None,
            initial: None,
            metadata: HashMap::new(),
        });

        let config = KotobaConfig {
            name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            theme: "light".to_string(),
            components,
            handlers: HashMap::new(),
            states: HashMap::new(),
            config: HashMap::new(),
        };

        // Test serialization
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("TestApp"));
        assert!(json.contains("Button"));

        // Test deserialization
        let deserialized: KotobaConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config, deserialized);
    }

    #[test]
    fn test_component_type_serialization() {
        let component_type = ComponentType::Component;
        let json = serde_json::to_string(&component_type).unwrap();
        assert_eq!(json, "\"component\"");

        let handler_type = ComponentType::Handler;
        let json = serde_json::to_string(&handler_type).unwrap();
        assert_eq!(json, "\"handler\"");

        let state_type = ComponentType::State;
        let json = serde_json::to_string(&state_type).unwrap();
        assert_eq!(json, "\"state\"");

        let config_type = ComponentType::Config;
        let json = serde_json::to_string(&config_type).unwrap();
        assert_eq!(json, "\"config\"");
    }

    #[test]
    fn test_component_with_children() {
        let content = r#"{
            "name": "TestApp",
            "version": "1.0.0",
            "theme": "light",
            "components": {
                "Container": {
                    "type": "component",
                    "name": "Container",
                    "component_type": "div",
                    "props": {
                        "className": "container"
                    },
                    "children": ["Button"],
                    "metadata": {}
                },
                "Button": {
                    "type": "component",
                    "name": "Button",
                    "component_type": "button",
                    "props": {
                        "children": "Click me"
                    },
                    "children": [],
                    "metadata": {}
                }
            },
            "handlers": {},
            "states": {},
            "config": {}
        }"#;

        let result = convert_content(content);
        assert!(result.is_ok());
        let tsx = result.unwrap();
        assert!(tsx.contains("<div"));
        assert!(tsx.contains("<button"));
        assert!(tsx.contains("container"));
        assert!(tsx.contains("Click me"));
    }
}

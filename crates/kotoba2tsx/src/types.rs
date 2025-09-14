//! Data types for Kotoba configuration and TSX generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a Kotoba component configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KotobaComponent {
    /// Component type: component, config, handler, state
    pub r#type: ComponentType,
    /// Component name
    pub name: String,
    /// Component type for React (button, div, input, etc.)
    pub component_type: Option<String>,
    /// Component properties
    pub props: HashMap<String, serde_json::Value>,
    /// Child component names
    pub children: Vec<String>,
    /// Component function body (for handlers)
    pub function: Option<String>,
    /// Initial state value (for state components)
    pub initial: Option<serde_json::Value>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Component type enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ComponentType {
    Component,
    Config,
    Handler,
    State,
}

/// Main Kotoba configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KotobaConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Theme (light/dark)
    pub theme: String,
    /// Component definitions
    pub components: HashMap<String, KotobaComponent>,
    /// Handler definitions
    pub handlers: HashMap<String, KotobaComponent>,
    /// State definitions
    pub states: HashMap<String, serde_json::Value>,
    /// Additional configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Style configuration for components
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComponentStyle {
    /// CSS class name
    pub class_name: String,
    /// Inline styles
    pub inline_styles: HashMap<String, String>,
}

/// TSX generation options
#[derive(Debug, Clone, PartialEq)]
pub struct TsxGenerationOptions {
    /// Include TypeScript types
    pub include_types: bool,
    /// Include React imports
    pub include_imports: bool,
    /// Use functional components (true) or class components (false)
    pub use_functional: bool,
    /// Include prop types
    pub include_prop_types: bool,
    /// Include default props
    pub include_default_props: bool,
    /// Format the output code
    pub format_output: bool,
}

impl Default for TsxGenerationOptions {
    fn default() -> Self {
        Self {
            include_types: true,
            include_imports: true,
            use_functional: true,
            include_prop_types: true,
            include_default_props: true,
            format_output: true,
        }
    }
}

/// Import statement for generated TSX
#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    /// Module to import from
    pub module: String,
    /// Imported items (functions, components, etc.)
    pub items: Vec<String>,
    /// Default import
    pub default_import: Option<String>,
}

/// Generated TSX component
#[derive(Debug, Clone, PartialEq)]
pub struct GeneratedComponent {
    /// Component name
    pub name: String,
    /// Component code
    pub code: String,
    /// Import statements needed
    pub imports: Vec<ImportStatement>,
    /// Component props interface
    pub props_interface: Option<String>,
    /// Default props
    pub default_props: Option<String>,
}

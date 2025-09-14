//! TSX code generator for Kotoba components

use crate::error::Result;
use crate::types::*;
use std::collections::HashMap;
use tokio::fs as async_fs;

/// TSX code generator
pub struct TsxGenerator {
    options: TsxGenerationOptions,
}

impl TsxGenerator {
    /// Create a new TsxGenerator with default options
    pub fn new() -> Self {
        Self {
            options: TsxGenerationOptions::default(),
        }
    }

    /// Create a new TsxGenerator with custom options
    pub fn with_options(options: TsxGenerationOptions) -> Self {
        Self { options }
    }

    /// Generate TSX code from KotobaConfig
    pub fn generate_tsx(&self, config: &KotobaConfig) -> Result<String> {
        let mut imports = Vec::new();
        let mut component_codes = Vec::new();
        let mut interfaces = Vec::new();
        let mut default_props = Vec::new();

        // Collect all imports
        if self.options.include_imports {
            imports.extend(self.generate_imports(config));
        }

        // Generate component code and collect interfaces/default props
        for (name, component) in &config.components {
            // Skip generating App component here if it exists - we'll generate it specially
            if name == "App" {
                // Still generate interface and default props for App
                if let Some(interface) = self.generate_props_interface_only(name, &component.props) {
                    interfaces.push(interface);
                }
                if let Some(default_prop) = self.generate_default_props_only(name, &component.props) {
                    default_props.push(default_prop);
                }
                continue;
            }

            let generated = self.generate_component(component, config)?;

            // Add component code
            component_codes.push(generated.code);

            // Collect interfaces and default props
            if let Some(interface) = generated.props_interface {
                interfaces.push(interface);
            }
            if let Some(default_prop) = generated.default_props {
                default_props.push(default_prop);
            }
        }

        // Generate handler functions
        for (name, handler) in &config.handlers {
            if let Some(function) = &handler.function {
                let handler_code = self.generate_handler_function(name, function)?;
                component_codes.push(handler_code);
            }
        }

        // Generate main app component (always generate this specially)
        let app_component = self.generate_main_app_component(config)?;
        component_codes.push(app_component);

        // Combine everything
        let mut result = String::new();

        // Add imports
        for import in imports {
            result.push_str(&self.format_import(&import));
            result.push('\n');
        }

        result.push('\n');

        // Add interfaces
        for interface in interfaces {
            result.push_str(&interface);
            result.push('\n');
        }

        // Add default props
        for default_prop in default_props {
            result.push_str(&default_prop);
            result.push('\n');
        }

        // Add component codes
        for code in component_codes {
            result.push_str(&code);
            result.push('\n');
        }

        Ok(result)
    }

    /// Generate TSX file from KotobaConfig
    pub async fn generate_file(&self, config: &KotobaConfig, output_path: &str) -> Result<()> {
        let content = self.generate_tsx(config)?;
        async_fs::write(output_path, content).await?;
        Ok(())
    }

    /// Generate import statements
    fn generate_imports(&self, config: &KotobaConfig) -> Vec<ImportStatement> {
        let mut imports = Vec::new();

        // React imports - use modern React import
        let mut react_items = vec!["FC".to_string()];
        if config.handlers.len() > 0 {
            react_items.push("useState".to_string());
            react_items.push("useEffect".to_string());
        }

        imports.push(ImportStatement {
            module: "react".to_string(),
            items: react_items,
            default_import: Some("React".to_string()),
        });

        // TypeScript types if needed (but FC is already imported from react)
        if self.options.include_types && config.handlers.is_empty() {
            // Only import additional types if we don't already have them
            imports.push(ImportStatement {
                module: "@types/react".to_string(),
                items: vec!["ReactElement".to_string()],
                default_import: None,
            });
        }

        imports
    }

    /// Generate a single component
    fn generate_component(&self, component: &KotobaComponent, config: &KotobaConfig) -> Result<GeneratedComponent> {
        let mut code = String::new();

        // Generate component props interface if needed
        let props_interface = if self.options.include_prop_types {
            self.generate_props_interface_only(&component.name, &component.props)
        } else {
            None
        };

        // Generate default props if needed
        let default_props = if self.options.include_default_props {
            self.generate_default_props_only(&component.name, &component.props)
        } else {
            None
        };

        // Generate component function
        if self.options.use_functional {
            code.push_str(&self.generate_functional_component(component, config)?);
        } else {
            code.push_str(&self.generate_class_component(component, config)?);
        }

        Ok(GeneratedComponent {
            name: component.name.clone(),
            code,
            imports: Vec::new(),
            props_interface,
            default_props,
        })
    }

    /// Generate functional component
    fn generate_functional_component(&self, component: &KotobaComponent, config: &KotobaConfig) -> Result<String> {
        let mut code = String::new();

        // Component declaration
        if self.options.include_types {
            code.push_str(&format!("const {}: FC<{}Props> = (props) => {{\n",
                component.name,
                component.name));
        } else {
            code.push_str(&format!("const {} = (props) => {{\n", component.name));
        }

        // Component body
        code.push_str("  return (\n");

        // Generate JSX
        let jsx = self.generate_jsx_element(component, config, 4)?;
        code.push_str(&jsx);

        code.push_str("  );\n");
        code.push_str("};\n\n");

        Ok(code)
    }

    /// Generate class component
    fn generate_class_component(&self, component: &KotobaComponent, config: &KotobaConfig) -> Result<String> {
        let mut code = String::new();

        code.push_str(&format!("class {} extends React.Component<{}Props> {{\n",
            component.name, component.name));
        code.push_str("  render() {\n");
        code.push_str("    return (\n");

        // Generate JSX
        let jsx = self.generate_jsx_element(component, config, 6)?;
        code.push_str(&jsx);

        code.push_str("    );\n");
        code.push_str("  }\n");
        code.push_str("}\n\n");

        Ok(code)
    }

    /// Generate JSX element
    fn generate_jsx_element(&self, component: &KotobaComponent, config: &KotobaConfig, indent: usize) -> Result<String> {
        let indent_str = " ".repeat(indent);

        if let Some(component_type) = &component.component_type {
            // Check if this is a custom component (defined in components) or HTML element
            if config.components.contains_key(component_type) {
                // This is a custom component reference
                let mut jsx = format!("{}<{}", indent_str, component_type);

                // Add props
                for (key, value) in &component.props {
                    let prop_value = self.format_prop_value(value);
                    jsx.push_str(&format!(" {}={}", self.to_camel_case(key), prop_value));
                }

                jsx.push_str(" />");
                Ok(jsx)
            } else {
                // This is an HTML element
                let mut jsx = format!("{}<{}", indent_str, component_type);

                // Add props
                for (key, value) in &component.props {
                    let prop_value = self.format_prop_value(value);
                    jsx.push_str(&format!(" {}={}", self.to_camel_case(key), prop_value));
                }

                if component.children.is_empty() {
                    jsx.push_str(" />");
                } else {
                    jsx.push_str(">\n");

                    // Add children
                    for child_name in &component.children {
                        if config.components.contains_key(child_name) {
                            // Child is a defined component - use component reference
                            let child_jsx = format!("{}<{} />\n", " ".repeat(indent + 2), child_name);
                            jsx.push_str(&child_jsx);
                        } else {
                            // Simple text child or other content
                            jsx.push_str(&format!("{}{}\n", " ".repeat(indent + 2), child_name));
                        }
                    }

                    jsx.push_str(&format!("{}</{}>", indent_str, component_type));
                }

                Ok(jsx)
            }
        } else {
            // Fragment or custom component without component_type
            let mut jsx = format!("{}<>\n", indent_str);

            for child_name in &component.children {
                if let Some(_child_component) = config.components.get(child_name) {
                    // Child is a defined component - use component reference
                    jsx.push_str(&format!("{}<{} />\n", " ".repeat(indent + 2), child_name));
                } else {
                    // Simple text child or other content
                    jsx.push_str(&format!("{}{}\n", " ".repeat(indent + 2), child_name));
                }
            }

            jsx.push_str(&format!("{}</>", indent_str));
            Ok(jsx)
        }
    }

    /// Generate props interface
    fn generate_props_interface(&self, component_name: &str, props: &HashMap<String, serde_json::Value>) -> String {
        if props.is_empty() {
            return String::new();
        }

        let mut interface = format!("interface {}Props {{\n", component_name);

        for (key, value) in props {
            let ts_type = self.infer_type(value);
            interface.push_str(&format!("  {}: {};\n", self.to_camel_case(key), ts_type));
        }

        interface.push_str("}\n\n");
        interface
    }

    /// Generate props interface only (returns Option for empty props)
    fn generate_props_interface_only(&self, component_name: &str, props: &HashMap<String, serde_json::Value>) -> Option<String> {
        if props.is_empty() {
            return None;
        }
        Some(self.generate_props_interface(component_name, props))
    }

    /// Generate default props
    fn generate_default_props(&self, component_name: &str, props: &HashMap<String, serde_json::Value>) -> String {
        if props.is_empty() {
            return String::new();
        }

        let mut default_props = format!("const {}DefaultProps: Partial<{}Props> = {{\n",
            component_name, component_name);

        for (key, value) in props {
            let prop_value = self.format_prop_value(value);
            default_props.push_str(&format!("  {}: {},\n", self.to_camel_case(key), prop_value));
        }

        default_props.push_str("};\n\n");
        default_props
    }

    /// Generate default props only (returns Option for empty props)
    fn generate_default_props_only(&self, component_name: &str, props: &HashMap<String, serde_json::Value>) -> Option<String> {
        if props.is_empty() {
            return None;
        }
        Some(self.generate_default_props(component_name, props))
    }

    /// Generate handler function
    fn generate_handler_function(&self, name: &str, function_body: &str) -> Result<String> {
        let mut code = format!("const {} = () => {{\n", name);
        code.push_str("  // Handler implementation\n");
        code.push_str(&format!("  {}\n", function_body));
        code.push_str("};\n\n");
        Ok(code)
    }

    /// Generate main app component
    fn generate_main_app_component(&self, config: &KotobaConfig) -> Result<String> {
        let mut code = String::new();

        code.push_str("const App: FC = () => {\n");

        // Add state hooks for state management
        for (name, initial_value) in &config.states {
            let initial = self.format_prop_value(initial_value);
            code.push_str(&format!("  const [{}, set{}] = useState({});\n",
                self.to_camel_case(name), self.capitalize_first(name), initial));
        }

        code.push_str("\n  return (\n");

        // Find root component - prefer "App" or use the first component
        if let Some(root_component) = config.components.get("App") {
            let jsx = self.generate_jsx_element(root_component, config, 4)?;
            code.push_str(&jsx);
        } else if let Some((_, root_component)) = config.components.iter().next() {
            let jsx = self.generate_jsx_element(root_component, config, 4)?;
            code.push_str(&jsx);
        } else {
            // No components defined, create a simple app
            code.push_str("    <div>\n");
            code.push_str("      <h1>Hello from Kotoba!</h1>\n");
            code.push_str("    </div>\n");
        }

        code.push_str("  );\n");
        code.push_str("};\n\n");

        code.push_str("export default App;\n");

        Ok(code)
    }

    /// Format import statement
    fn format_import(&self, import: &ImportStatement) -> String {
        let mut result = String::new();

        if let Some(default) = &import.default_import {
            if !import.items.is_empty() {
                // Both default and named imports
                result.push_str(&format!("import {}, {{ {} }} from '{}';",
                    default, import.items.join(", "), import.module));
            } else {
                // Only default import
                result.push_str(&format!("import {} from '{}';", default, import.module));
            }
        } else if !import.items.is_empty() {
            // Only named imports
            result.push_str(&format!("import {{ {} }} from '{}';",
                import.items.join(", "), import.module));
        }

        result
    }

    /// Format prop value for JSX
    fn format_prop_value(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(s) => format!("\"{}\"", s),
            serde_json::Value::Bool(b) => format!("{}", b),
            serde_json::Value::Number(n) => format!("{}", n),
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| self.format_prop_value(v)).collect();
                format!("[{}]", items.join(", "))
            }
            serde_json::Value::Object(obj) => {
                let mut props = Vec::new();
                for (k, v) in obj {
                    props.push(format!("{}: {}", k, self.format_prop_value(v)));
                }
                format!("{{{}}}", props.join(", "))
            }
            serde_json::Value::Null => "null".to_string(),
        }
    }

    /// Infer TypeScript type from JSON value
    fn infer_type(&self, value: &serde_json::Value) -> String {
        match value {
            serde_json::Value::String(_) => "string",
            serde_json::Value::Bool(_) => "boolean",
            serde_json::Value::Number(_) => "number",
            serde_json::Value::Array(_) => "any[]",
            serde_json::Value::Object(_) => "any",
            serde_json::Value::Null => "any",
        }.to_string()
    }

    /// Convert snake_case to camelCase
    fn to_camel_case(&self, s: &str) -> String {
        let mut result = String::new();
        let mut capitalize_next = false;

        for ch in s.chars() {
            if ch == '_' {
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

    /// Capitalize first character
    fn capitalize_first(&self, s: &str) -> String {
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        }
    }
}

impl Default for TsxGenerator {
    fn default() -> Self {
        Self::new()
    }
}

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
        Self {
            options,
        }
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

        // Return the generated code
            Ok(result)
    }

    /// Generate TSX file from KotobaConfig
    pub async fn generate_file(&self, config: &KotobaConfig, output_path: &str) -> Result<()> {
        let content = self.generate_tsx(config)?;
        async_fs::write(output_path, content).await?;
        Ok(())
    }

    /// Generate TSX code from kotoba-kotobanet FrontendConfig
    pub fn generate_tsx_from_frontend_config(&self, frontend_config: &kotoba_kotobanet::frontend::FrontendConfig) -> Result<String> {
        let mut imports = Vec::new();
        let mut component_codes = Vec::new();
        let mut interfaces = Vec::new();
        let mut default_props = Vec::new();

        // Collect all imports
        if self.options.include_imports {
            imports.extend(self.generate_imports_from_frontend_config(frontend_config));
        }

        // Generate component code and collect interfaces/default props
        for (_name, component) in &frontend_config.components {
            let generated = self.generate_component_from_frontend_def(component, frontend_config)?;

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

        // Generate API route handlers
        for api_route in &frontend_config.api_routes {
            let handler_code = self.generate_api_handler(api_route)?;
            component_codes.push(handler_code);
        }

        // Generate main app component from pages
        let app_component = self.generate_main_app_from_frontend_config(frontend_config)?;
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

        // Return the generated code
        Ok(result)
    }

    /// Generate import statements
    fn generate_imports(&self, config: &KotobaConfig) -> Vec<ImportStatement> {
        let mut imports = Vec::new();

        // React imports - use modern React import
        let mut react_items = vec!["FC".to_string()];
        if !config.handlers.is_empty() {
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

    /// Generate import statements from FrontendConfig
    fn generate_imports_from_frontend_config(&self, config: &kotoba_kotobanet::frontend::FrontendConfig) -> Vec<ImportStatement> {
        let mut imports = Vec::new();

        // React imports - use modern React import
        let mut react_items = vec!["FC".to_string()];
        if !config.api_routes.is_empty() {
            react_items.push("useState".to_string());
            react_items.push("useEffect".to_string());
        }

                imports.push(ImportStatement {
            module: "react".to_string(),
            items: react_items,
            default_import: Some("React".to_string()),
        });

        // Add React Router if we have multiple pages
        if config.pages.len() > 1 {
                    imports.push(ImportStatement {
                module: "react-router-dom".to_string(),
                items: vec!["BrowserRouter".to_string(), "Routes".to_string(), "Route".to_string()],
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

    /// Generate a single component from Frontend ComponentDef
    fn generate_component_from_frontend_def(&self, component: &kotoba_kotobanet::frontend::ComponentDef, frontend_config: &kotoba_kotobanet::frontend::FrontendConfig) -> Result<GeneratedComponent> {
        let mut code = String::new();

        // Convert ComponentDef props to HashMap for interface generation
        let props_map = self.convert_component_props_to_hashmap(&component.props);

        // Generate component props interface if needed
        let props_interface = if self.options.include_prop_types {
            self.generate_props_interface_only(&component.name, &props_map)
        } else {
            None
        };

        // Generate default props if needed
        let default_props = if self.options.include_default_props {
            self.generate_default_props_only(&component.name, &props_map)
        } else {
            None
        };

        // Generate component function
        code.push_str(&self.generate_functional_component_from_frontend_def(component, frontend_config)?);

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
        code.push('\n');

        code.push_str("  );\n");
        code.push_str("};\n\n");

        Ok(code)
    }

    /// Generate functional component from Frontend ComponentDef
    fn generate_functional_component_from_frontend_def(&self, component: &kotoba_kotobanet::frontend::ComponentDef, _frontend_config: &kotoba_kotobanet::frontend::FrontendConfig) -> Result<String> {
        let mut code = String::new();

        // Component declaration
        if self.options.include_types {
            code.push_str(&format!("const {}: FC<{}Props> = (props) => {{\n",
                component.name,
                component.name));
        } else {
            code.push_str(&format!("const {} = (props) => {{\n", component.name));
        }

        // Add state hooks if component has state
        if let Some(state) = &component.state {
            for (state_name, state_def) in state {
                let initial_value = self.format_prop_value(&state_def.initial_value);
                code.push_str(&format!("  const [{}, set{}] = useState({});\n",
                    self.to_camel_case(state_name),
                    self.capitalize_first(state_name),
                    initial_value));
            }
            code.push('\n');
        }

        // Component body
        code.push_str("  return (\n");

        // Generate JSX from render template
        let jsx = self.generate_jsx_from_render_template(&component.render, 4)?;
        code.push_str(&jsx);
        code.push('\n');

        code.push_str("  );\n");
        code.push_str("};\n\n");

        Ok(code)
    }

    /// Convert ComponentDef props to HashMap for interface generation
    fn convert_component_props_to_hashmap(&self, props: &std::collections::HashMap<String, kotoba_kotobanet::frontend::PropDef>) -> std::collections::HashMap<String, serde_json::Value> {
        let mut result = std::collections::HashMap::new();
        for (key, prop_def) in props {
            // Convert PropType to string for interface generation
            let type_value = match prop_def.type_ {
                kotoba_kotobanet::frontend::PropType::String => serde_json::Value::String("string".to_string()),
                kotoba_kotobanet::frontend::PropType::Number => serde_json::Value::String("number".to_string()),
                kotoba_kotobanet::frontend::PropType::Boolean => serde_json::Value::String("boolean".to_string()),
                kotoba_kotobanet::frontend::PropType::Array => serde_json::Value::String("array".to_string()),
                kotoba_kotobanet::frontend::PropType::Object => serde_json::Value::String("object".to_string()),
                kotoba_kotobanet::frontend::PropType::Function => serde_json::Value::String("function".to_string()),
                kotoba_kotobanet::frontend::PropType::Component => serde_json::Value::String("component".to_string()),
                kotoba_kotobanet::frontend::PropType::Custom(ref s) => serde_json::Value::String(s.clone()),
            };
            result.insert(key.clone(), type_value);
        }
        result
    }

    /// Generate JSX from render template string
    fn generate_jsx_from_render_template(&self, render_template: &str, indent: usize) -> Result<String> {
        let indent_str = " ".repeat(indent);

        // For now, treat the render template as raw JSX
        // TODO: Implement proper template parsing and variable substitution
        let jsx = format!("{}  {}", indent_str, render_template);
        Ok(jsx)
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
        code.push('\n');

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

                // Add props - pass all props as spread if any exist
                if !component.props.is_empty() {
                    jsx.push_str(" {...props}");
                }

                jsx.push_str(" />");
                Ok(jsx)
            } else {
                // This is an HTML element
                let mut jsx = format!("{}<{}", indent_str, component_type);

                // Add props
                for key in component.props.keys() {
                    let camel_key = self.to_camel_case(key);
                    jsx.push_str(&format!(" {}={{props.{}}}", camel_key, camel_key));
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

                    jsx.push_str(&format!("{}</{}>\n", indent_str, component_type));
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

            jsx.push_str(&format!("{}</>\n", indent_str));
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
            code.push('\n');
        } else if let Some((_, root_component)) = config.components.iter().next() {
            let jsx = self.generate_jsx_element(root_component, config, 4)?;
            code.push_str(&jsx);
            code.push('\n');
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

/// Enhanced TSX generator with SWC, CSS processing, and styled-components support
impl TsxGenerator {


    /// Generate API handler function
    fn generate_api_handler(&self, api_route: &kotoba_kotobanet::frontend::ApiRouteDef) -> Result<String> {
        let mut code = String::new();

        code.push_str(&format!("const {} = async (params) => {{\n", api_route.handler));
        code.push_str("  // API handler implementation\n");
        code.push_str(&format!("  const response = await fetch('{}', {{\n", api_route.path));
        code.push_str(&format!("    method: '{}',\n", api_route.method));
        code.push_str("    headers: {\n");
        code.push_str("      'Content-Type': 'application/json',\n");
        code.push_str("    },\n");
        if api_route.method != "GET" {
            code.push_str("    body: JSON.stringify(params),\n");
        }
        code.push_str("  });\n");
        code.push_str("  return response.json();\n");
        code.push_str("};\n\n");

        Ok(code)
    }

    /// Generate main app component from FrontendConfig pages
    fn generate_main_app_from_frontend_config(&self, frontend_config: &kotoba_kotobanet::frontend::FrontendConfig) -> Result<String> {
        let mut code = String::new();

        code.push_str("const App: FC = () => {\n");

        // If we have multiple pages, use React Router
        if frontend_config.pages.len() > 1 {
            code.push_str("  return (\n");
            code.push_str("    <BrowserRouter>\n");
            code.push_str("      <Routes>\n");

            for page in &frontend_config.pages {
                code.push_str(&format!("        <Route path=\"{}\" element={{<{} />}} />\n",
                    page.path, page.component));
            }

            code.push_str("      </Routes>\n");
            code.push_str("    </BrowserRouter>\n");
            code.push_str("  );\n");
        } else if let Some(page) = frontend_config.pages.first() {
            // Single page app
            code.push_str("  return (\n");
            code.push_str(&format!("    <{} />\n", page.component));
            code.push_str("  );\n");
        } else {
            // No pages defined, create a simple app
            code.push_str("  return (\n");
            code.push_str("    <div>\n");
            code.push_str("      <h1>Hello from Kotoba!</h1>\n");
            code.push_str("    </div>\n");
            code.push_str("  );\n");
        }

        code.push_str("};\n\n");
        code.push_str("export default App;\n");

        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_test_config() -> KotobaConfig {
        let mut components = HashMap::new();
        let mut props = HashMap::new();
        props.insert("className".to_string(), json!("btn"));
        props.insert("disabled".to_string(), json!(false));

        components.insert("Button".to_string(), KotobaComponent {
            r#type: ComponentType::Component,
            name: "Button".to_string(),
            component_type: Some("button".to_string()),
            props,
            children: vec![],
            function: None,
            initial: None,
            metadata: HashMap::new(),
        });

        KotobaConfig {
            name: "TestApp".to_string(),
            version: "1.0.0".to_string(),
            theme: "light".to_string(),
            components,
            handlers: HashMap::new(),
            states: HashMap::new(),
            config: HashMap::new(),
        }
    }

    fn create_test_config_with_handlers() -> KotobaConfig {
        let mut config = create_test_config();
        let mut handlers = HashMap::new();
        handlers.insert("onClick".to_string(), KotobaComponent {
            r#type: ComponentType::Handler,
            name: "onClick".to_string(),
            component_type: None,
            props: HashMap::new(),
            children: vec![],
            function: Some("console.log('clicked');".to_string()),
            initial: None,
            metadata: HashMap::new(),
        });
        config.handlers = handlers;
        config
    }

    fn create_test_config_with_states() -> KotobaConfig {
        let mut config = create_test_config();
        let mut states = HashMap::new();
        states.insert("count".to_string(), json!(0));
        states.insert("isVisible".to_string(), json!(true));
        config.states = states;
        config
    }

    #[test]
    fn test_generator_new() {
        let generator = TsxGenerator::new();
        assert_eq!(generator.options.include_types, true);
        assert_eq!(generator.options.include_imports, true);
        assert_eq!(generator.options.use_functional, true);
    }

    #[test]
    fn test_generator_with_options() {
        let options = TsxGenerationOptions {
            include_types: false,
            include_imports: false,
            use_functional: false,
            include_prop_types: false,
            include_default_props: false,
            format_output: false,
            swc_options: Default::default(),
            css_options: Default::default(),
        };
        let generator = TsxGenerator::with_options(options);
        assert_eq!(generator.options.include_types, false);
        assert_eq!(generator.options.include_imports, false);
        assert_eq!(generator.options.use_functional, false);
    }

    #[test]
    fn test_generate_imports_basic() {
        let generator = TsxGenerator::new();
        let config = create_test_config();

        let imports = generator.generate_imports(&config);
        assert!(!imports.is_empty());

        // Should include React import
        let react_import = imports.iter().find(|i| i.module == "react").unwrap();
        assert_eq!(react_import.default_import, Some("React".to_string()));
        assert!(react_import.items.contains(&"FC".to_string()));
    }

    #[test]
    fn test_generate_imports_with_handlers() {
        let generator = TsxGenerator::new();
        let config = create_test_config_with_handlers();

        let imports = generator.generate_imports(&config);
        let react_import = imports.iter().find(|i| i.module == "react").unwrap();
        assert!(react_import.items.contains(&"useState".to_string()));
        assert!(react_import.items.contains(&"useEffect".to_string()));
    }

    #[test]
    fn test_generate_props_interface() {
        let generator = TsxGenerator::new();
        let config = create_test_config();
        let component = &config.components["Button"];

        let interface = generator.generate_props_interface("Button", &component.props);
        assert!(interface.contains("interface ButtonProps"));
        assert!(interface.contains("className: string;"));
        assert!(interface.contains("disabled: boolean;"));
    }

    #[test]
    fn test_generate_props_interface_empty() {
        let generator = TsxGenerator::new();
        let props = HashMap::new();

        let interface = generator.generate_props_interface("Empty", &props);
        assert_eq!(interface, "");
    }

    #[test]
    fn test_generate_default_props() {
        let generator = TsxGenerator::new();
        let config = create_test_config();
        let component = &config.components["Button"];

        let default_props = generator.generate_default_props("Button", &component.props);
        assert!(default_props.contains("const ButtonDefaultProps"));
        assert!(default_props.contains("className: \"btn\""));
        assert!(default_props.contains("disabled: false"));
    }

    #[test]
    fn test_generate_handler_function() {
        let generator = TsxGenerator::new();
        let result = generator.generate_handler_function("onClick", "console.log('test');");

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("const onClick = () => {"));
        assert!(code.contains("console.log('test');"));
    }

    #[test]
    fn test_generate_functional_component() {
        let generator = TsxGenerator::new();
        let config = create_test_config();
        let component = &config.components["Button"];

        let result = generator.generate_functional_component(component, &config);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("const Button: FC<ButtonProps>"));
        assert!(code.contains("return ("));
    }

    #[test]
    fn test_generate_class_component() {
        let mut options = TsxGenerationOptions::default();
        options.use_functional = false;
        let generator = TsxGenerator::with_options(options);
        let config = create_test_config();
        let component = &config.components["Button"];

        let result = generator.generate_class_component(component, &config);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("class Button extends React.Component<ButtonProps>"));
        assert!(code.contains("render()"));
    }

    #[test]
    fn test_generate_jsx_element_html() {
        let generator = TsxGenerator::new();
        let config = create_test_config();
        let component = &config.components["Button"];

        let result = generator.generate_jsx_element(component, &config, 0);
        assert!(result.is_ok());
        let jsx = result.unwrap();
        println!("Generated JSX: {}", jsx); // Debug output
        assert!(jsx.contains("<button"));
        assert!(jsx.contains("className={props.className}"));
        assert!(jsx.contains("disabled={props.disabled}"));
        assert!(jsx.contains("/>"));
    }

    #[test]
    fn test_generate_jsx_element_with_children() {
        let generator = TsxGenerator::new();
        let config = create_test_config();
        let mut component = config.components["Button"].clone();
        component.children = vec!["Child".to_string()];

        let result = generator.generate_jsx_element(&component, &config, 0);
        assert!(result.is_ok());
        let jsx = result.unwrap();
        assert!(jsx.contains("<button"));
        assert!(jsx.contains("</button>"));
    }

    #[test]
    fn test_format_prop_value_string() {
        let generator = TsxGenerator::new();
        let value = json!("hello");
        let result = generator.format_prop_value(&value);
        assert_eq!(result, "\"hello\"");
    }

    #[test]
    fn test_format_prop_value_boolean() {
        let generator = TsxGenerator::new();
        let value = json!(true);
        let result = generator.format_prop_value(&value);
        assert_eq!(result, "true");
    }

    #[test]
    fn test_format_prop_value_number() {
        let generator = TsxGenerator::new();
        let value = json!(42);
        let result = generator.format_prop_value(&value);
        assert_eq!(result, "42");
    }

    #[test]
    fn test_format_prop_value_array() {
        let generator = TsxGenerator::new();
        let value = json!([1, 2, 3]);
        let result = generator.format_prop_value(&value);
        assert_eq!(result, "[1, 2, 3]");
    }

    #[test]
    fn test_format_prop_value_object() {
        let generator = TsxGenerator::new();
        let value = json!({"key": "value"});
        let result = generator.format_prop_value(&value);
        assert_eq!(result, "{key: \"value\"}");
    }

    #[test]
    fn test_to_camel_case() {
        let generator = TsxGenerator::new();
        assert_eq!(generator.to_camel_case("snake_case"), "snakeCase");
        assert_eq!(generator.to_camel_case("already_camel"), "alreadyCamel");
        assert_eq!(generator.to_camel_case("single"), "single");
        assert_eq!(generator.to_camel_case("a_b_c"), "aBC");
    }

    #[test]
    fn test_capitalize_first() {
        let generator = TsxGenerator::new();
        assert_eq!(generator.capitalize_first("hello"), "Hello");
        assert_eq!(generator.capitalize_first("Hello"), "Hello");
        assert_eq!(generator.capitalize_first(""), "");
    }

    #[test]
    fn test_infer_type() {
        let generator = TsxGenerator::new();
        assert_eq!(generator.infer_type(&json!("string")), "string");
        assert_eq!(generator.infer_type(&json!(42)), "number");
        assert_eq!(generator.infer_type(&json!(true)), "boolean");
        assert_eq!(generator.infer_type(&json!([1, 2, 3])), "any[]");
        assert_eq!(generator.infer_type(&json!({"key": "value"})), "any");
        assert_eq!(generator.infer_type(&json!(null)), "any");
    }

    #[test]
    fn test_generate_tsx_basic() {
        let generator = TsxGenerator::new();
        let config = create_test_config();

        let result = generator.generate_tsx(&config);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("import React"));
        assert!(code.contains("interface ButtonProps"));
        assert!(code.contains("const Button: FC<ButtonProps>"));
    }

    #[test]
    fn test_generate_tsx_with_states() {
        let generator = TsxGenerator::new();
        let config = create_test_config_with_states();

        let result = generator.generate_tsx(&config);
        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("const [count, setCount] = useState(0);"));
        assert!(code.contains("const [isVisible, setIsVisible] = useState(true);"));
    }

    #[test]
    fn test_format_import_statement() {
        let generator = TsxGenerator::new();

        let import = ImportStatement {
            module: "react".to_string(),
            items: vec!["FC".to_string(), "useState".to_string()],
            default_import: Some("React".to_string()),
        };

        let result = generator.format_import(&import);
        assert_eq!(result, "import React, { FC, useState } from 'react';");

        let import_no_default = ImportStatement {
            module: "utils".to_string(),
            items: vec!["helper".to_string()],
            default_import: None,
        };

        let result_no_default = generator.format_import(&import_no_default);
        assert_eq!(result_no_default, "import { helper } from 'utils';");

        let import_only_default = ImportStatement {
            module: "react".to_string(),
            items: vec![],
            default_import: Some("React".to_string()),
        };

        let result_only_default = generator.format_import(&import_only_default);
        assert_eq!(result_only_default, "import React from 'react';");
    }
}

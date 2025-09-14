//! Frontend Framework for React component definitions in Jsonnet

use crate::{KotobaNetError, Result};
use kotoba_jsonnet::JsonnetValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// React component definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    pub name: String,
    pub props: HashMap<String, PropDef>,
    pub state: Option<HashMap<String, StateDef>>,
    pub lifecycle: Option<LifecycleDef>,
    pub render: String, // JSX template
    pub styles: Option<HashMap<String, String>>,
    pub imports: Vec<String>,
}

/// Component property definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropDef {
    pub type_: PropType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: Option<String>,
}

/// Property type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropType {
    String,
    Number,
    Boolean,
    Array,
    Object,
    Function,
    Component,
    Custom(String),
}

/// Component state definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDef {
    pub initial_value: serde_json::Value,
    pub type_: PropType,
    pub description: Option<String>,
}

/// Component lifecycle methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleDef {
    pub component_did_mount: Option<String>,
    pub component_did_update: Option<String>,
    pub component_will_unmount: Option<String>,
    pub should_component_update: Option<String>,
}

/// Page/route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDef {
    pub path: String,
    pub component: String,
    pub layout: Option<String>,
    pub loading: Option<String>,
    pub error: Option<String>,
    pub meta: Option<HashMap<String, String>>,
}

/// API route definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRouteDef {
    pub path: String,
    pub method: String,
    pub handler: String,
    pub schema: Option<ApiSchema>,
    pub auth_required: bool,
}

/// API schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSchema {
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub errors: Option<Vec<ApiError>>,
}

/// API error definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub status_code: u16,
}

/// Complete frontend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendConfig {
    pub components: HashMap<String, ComponentDef>,
    pub pages: Vec<PageDef>,
    pub api_routes: Vec<ApiRouteDef>,
    pub global_styles: Option<HashMap<String, String>>,
    pub config: serde_json::Value,
}

/// Frontend Parser for React component definitions in Jsonnet
#[derive(Debug)]
pub struct FrontendParser;

impl FrontendParser {
    /// Parse frontend configuration from Jsonnet
    pub fn parse(content: &str) -> Result<FrontendConfig> {
        let evaluated = crate::evaluate_kotoba(content)?;
        Self::jsonnet_value_to_frontend_config(&evaluated)
    }

    /// Parse frontend config from file
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<FrontendConfig> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Convert JsonnetValue to FrontendConfig
    fn jsonnet_value_to_frontend_config(value: &JsonnetValue) -> Result<FrontendConfig> {
        match value {
            JsonnetValue::Object(obj) => {
                let components = Self::extract_components(obj)?;
                let pages = Self::extract_pages(obj)?;
                let api_routes = Self::extract_api_routes(obj)?;
                let global_styles = Self::extract_global_styles(obj)?;
                let config = Self::extract_config(obj)?;

                Ok(FrontendConfig {
                    components,
                    pages,
                    api_routes,
                    global_styles,
                    config,
                })
            }
            _ => Err(KotobaNetError::FrontendParse(
                "Frontend configuration must be an object".to_string(),
            )),
        }
    }

    /// Extract components from Jsonnet object
    fn extract_components(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, ComponentDef>> {
        let mut components = HashMap::new();

        if let Some(JsonnetValue::Object(comp_obj)) = obj.get("components") {
            for (name, comp_value) in comp_obj {
                if let JsonnetValue::Object(comp_def) = comp_value {
                    let component = Self::parse_component(name, comp_def)?;
                    components.insert(name.clone(), component);
                }
            }
        }

        Ok(components)
    }

    /// Parse a single component definition
    fn parse_component(name: &str, obj: &HashMap<String, JsonnetValue>) -> Result<ComponentDef> {
        let props = Self::extract_props(obj)?;
        let state = Self::extract_state(obj)?;
        let lifecycle = Self::extract_lifecycle(obj)?;
        let render = Self::extract_string(obj, "render")?;
        let styles = Self::extract_styles(obj)?;
        let imports = Self::extract_string_array(obj, "imports")?;

        Ok(ComponentDef {
            name: name.to_string(),
            props,
            state,
            lifecycle,
            render,
            styles,
            imports,
        })
    }

    /// Extract component props
    fn extract_props(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, PropDef>> {
        let mut props = HashMap::new();

        if let Some(JsonnetValue::Object(props_obj)) = obj.get("props") {
            for (prop_name, prop_value) in props_obj {
                if let JsonnetValue::Object(prop_def) = prop_value {
                    let prop = Self::parse_prop(prop_def)?;
                    props.insert(prop_name.clone(), prop);
                }
            }
        }

        Ok(props)
    }

    /// Parse a single property definition
    fn parse_prop(obj: &HashMap<String, JsonnetValue>) -> Result<PropDef> {
        let type_str = Self::extract_string(obj, "type")?;
        let type_ = Self::parse_prop_type(&type_str)?;
        let required = Self::extract_bool(obj, "required").unwrap_or(false);
        let default = obj.get("default")
            .map(|v| Self::jsonnet_value_to_json_value(v))
            .transpose()?;
        let description = Self::extract_string(obj, "description").ok();

        Ok(PropDef {
            type_,
            required,
            default,
            description,
        })
    }

    /// Parse property type
    fn parse_prop_type(type_str: &str) -> Result<PropType> {
        match type_str {
            "string" => Ok(PropType::String),
            "number" => Ok(PropType::Number),
            "boolean" => Ok(PropType::Boolean),
            "array" => Ok(PropType::Array),
            "object" => Ok(PropType::Object),
            "function" => Ok(PropType::Function),
            "component" => Ok(PropType::Component),
            custom => Ok(PropType::Custom(custom.to_string())),
        }
    }

    /// Extract component state
    fn extract_state(obj: &HashMap<String, JsonnetValue>) -> Result<Option<HashMap<String, StateDef>>> {
        if let Some(JsonnetValue::Object(state_obj)) = obj.get("state") {
            let mut state = HashMap::new();

            for (state_name, state_value) in state_obj {
                if let JsonnetValue::Object(state_def) = state_value {
                    let state_item = Self::parse_state(state_def)?;
                    state.insert(state_name.clone(), state_item);
                }
            }

            Ok(Some(state))
        } else {
            Ok(None)
        }
    }

    /// Parse state definition
    fn parse_state(obj: &HashMap<String, JsonnetValue>) -> Result<StateDef> {
        let initial_value = obj.get("initialValue")
            .ok_or_else(|| KotobaNetError::FrontendParse("State must have initialValue".to_string()))?;
        let initial_value = Self::jsonnet_value_to_json_value(initial_value)?;
        let type_str = Self::extract_string(obj, "type")?;
        let type_ = Self::parse_prop_type(&type_str)?;
        let description = Self::extract_string(obj, "description").ok();

        Ok(StateDef {
            initial_value,
            type_,
            description,
        })
    }

    /// Extract lifecycle methods
    fn extract_lifecycle(obj: &HashMap<String, JsonnetValue>) -> Result<Option<LifecycleDef>> {
        if let Some(JsonnetValue::Object(lc_obj)) = obj.get("lifecycle") {
            let component_did_mount = Self::extract_string(lc_obj, "componentDidMount").ok();
            let component_did_update = Self::extract_string(lc_obj, "componentDidUpdate").ok();
            let component_will_unmount = Self::extract_string(lc_obj, "componentWillUnmount").ok();
            let should_component_update = Self::extract_string(lc_obj, "shouldComponentUpdate").ok();

            Ok(Some(LifecycleDef {
                component_did_mount,
                component_did_update,
                component_will_unmount,
                should_component_update,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract styles
    fn extract_styles(obj: &HashMap<String, JsonnetValue>) -> Result<Option<HashMap<String, String>>> {
        if let Some(JsonnetValue::Object(styles_obj)) = obj.get("styles") {
            let mut styles = HashMap::new();

            for (class_name, style_value) in styles_obj {
                if let JsonnetValue::String(style_str) = style_value {
                    styles.insert(class_name.clone(), style_str.clone());
                }
            }

            Ok(Some(styles))
        } else {
            Ok(None)
        }
    }

    /// Extract pages from Jsonnet object
    fn extract_pages(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<PageDef>> {
        let mut pages = Vec::new();

        if let Some(JsonnetValue::Array(page_array)) = obj.get("pages") {
            for page_value in page_array {
                if let JsonnetValue::Object(page_obj) = page_value {
                    let page = Self::parse_page(page_obj)?;
                    pages.push(page);
                }
            }
        }

        Ok(pages)
    }

    /// Parse page definition
    fn parse_page(obj: &HashMap<String, JsonnetValue>) -> Result<PageDef> {
        let path = Self::extract_string(obj, "path")?;
        let component = Self::extract_string(obj, "component")?;
        let layout = Self::extract_string(obj, "layout").ok();
        let loading = Self::extract_string(obj, "loading").ok();
        let error = Self::extract_string(obj, "error").ok();
        let meta = Self::extract_string_map(obj, "meta");

        Ok(PageDef {
            path,
            component,
            layout,
            loading,
            error,
            meta,
        })
    }

    /// Extract API routes
    fn extract_api_routes(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<ApiRouteDef>> {
        let mut routes = Vec::new();

        if let Some(JsonnetValue::Array(route_array)) = obj.get("apiRoutes") {
            for route_value in route_array {
                if let JsonnetValue::Object(route_obj) = route_value {
                    let route = Self::parse_api_route(route_obj)?;
                    routes.push(route);
                }
            }
        }

        Ok(routes)
    }

    /// Parse API route definition
    fn parse_api_route(obj: &HashMap<String, JsonnetValue>) -> Result<ApiRouteDef> {
        let path = Self::extract_string(obj, "path")?;
        let method = Self::extract_string(obj, "method")?;
        let handler = Self::extract_string(obj, "handler")?;
        let schema = Self::extract_api_schema(obj)?;
        let auth_required = Self::extract_bool(obj, "authRequired").unwrap_or(false);

        Ok(ApiRouteDef {
            path,
            method,
            handler,
            schema,
            auth_required,
        })
    }

    /// Extract API schema
    fn extract_api_schema(obj: &HashMap<String, JsonnetValue>) -> Result<Option<ApiSchema>> {
        if let Some(JsonnetValue::Object(schema_obj)) = obj.get("schema") {
            let input = Self::extract_value_map(schema_obj, "input")?;
            let output = Self::extract_value_map(schema_obj, "output")?;
            let errors = Self::extract_api_errors(schema_obj)?;

            Ok(Some(ApiSchema {
                input: input.map(|v| Self::jsonnet_object_to_json_value(&v)).transpose()?,
                output: output.map(|v| Self::jsonnet_object_to_json_value(&v)).transpose()?,
                errors,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract API errors
    fn extract_api_errors(obj: &HashMap<String, JsonnetValue>) -> Result<Option<Vec<ApiError>>> {
        if let Some(JsonnetValue::Array(error_array)) = obj.get("errors") {
            let mut errors = Vec::new();

            for error_value in error_array {
                if let JsonnetValue::Object(error_obj) = error_value {
                    let code = Self::extract_string(error_obj, "code")?;
                    let message = Self::extract_string(error_obj, "message")?;
                    let status_code = Self::extract_number(error_obj, "statusCode")? as u16;

                    errors.push(ApiError {
                        code,
                        message,
                        status_code,
                    });
                }
            }

            Ok(Some(errors))
        } else {
            Ok(None)
        }
    }

    /// Extract global styles
    fn extract_global_styles(obj: &HashMap<String, JsonnetValue>) -> Result<Option<HashMap<String, String>>> {
        Ok(Self::extract_string_map(obj, "globalStyles"))
    }

    /// Extract config
    fn extract_config(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        if let Some(JsonnetValue::Object(config_obj)) = obj.get("config") {
            Self::jsonnet_object_to_json_value(config_obj)
        } else {
            Ok(serde_json::Value::Object(serde_json::Map::new()))
        }
    }

    fn jsonnet_object_to_json_value(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        let mut map = serde_json::Map::new();
        for (key, value) in obj {
            let json_value = Self::jsonnet_value_to_json_value(value)?;
            map.insert(key.clone(), json_value);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn jsonnet_value_to_json_value(value: &JsonnetValue) -> Result<serde_json::Value> {
        match value {
            JsonnetValue::Null => Ok(serde_json::Value::Null),
            JsonnetValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            JsonnetValue::Number(n) => Ok(serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap())),
            JsonnetValue::String(s) => Ok(serde_json::Value::String(s.clone())),
            JsonnetValue::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(Self::jsonnet_value_to_json_value(item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            }
            JsonnetValue::Object(obj) => Self::jsonnet_object_to_json_value(obj),
            JsonnetValue::Function(_) => Err(KotobaNetError::FrontendParse("Functions cannot be converted to JSON".to_string())),
        }
    }

    // Helper methods

    fn extract_string(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<String> {
        match obj.get(key) {
            Some(JsonnetValue::String(s)) => Ok(s.clone()),
            _ => Err(KotobaNetError::FrontendParse(format!("Expected string for key '{}'", key))),
        }
    }

    fn extract_bool(obj: &HashMap<String, JsonnetValue>, key: &str) -> Option<bool> {
        match obj.get(key) {
            Some(JsonnetValue::Boolean(b)) => Some(*b),
            _ => None,
        }
    }

    fn extract_number(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<f64> {
        match obj.get(key) {
            Some(JsonnetValue::Number(n)) => Ok(*n),
            _ => Err(KotobaNetError::FrontendParse(format!("Expected number for key '{}'", key))),
        }
    }

    fn extract_string_array(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<Vec<String>> {
        match obj.get(key) {
            Some(JsonnetValue::Array(arr)) => {
                let mut strings = Vec::new();
                for item in arr {
                    if let JsonnetValue::String(s) = item {
                        strings.push(s.clone());
                    }
                }
                Ok(strings)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn extract_string_map(obj: &HashMap<String, JsonnetValue>, key: &str) -> Option<HashMap<String, String>> {
        match obj.get(key) {
            Some(JsonnetValue::Object(map_obj)) => {
                let mut result = HashMap::new();
                for (k, v) in map_obj {
                    if let JsonnetValue::String(s) = v {
                        result.insert(k.clone(), s.clone());
                    }
                }
                Some(result)
            }
            _ => None,
        }
    }

    fn extract_value_map(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<Option<HashMap<String, JsonnetValue>>> {
        match obj.get(key) {
            Some(JsonnetValue::Object(map_obj)) => Ok(Some(map_obj.clone())),
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_component() {
        let component_def = r#"
        {
            components: {
                Button: {
                    props: {
                        text: {
                            type: "string",
                            required: true,
                        },
                        onClick: {
                            type: "function",
                            required: false,
                        }
                    },
                    render: "<button onClick={props.onClick}>{props.text}</button>",
                    imports: ["React"],
                }
            }
        }
        "#;

        let result = FrontendParser::parse(component_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.contains_key("Button"));

        let button = &config.components["Button"];
        assert_eq!(button.name, "Button");
        assert!(button.props.contains_key("text"));
        assert!(button.props.contains_key("onClick"));
    }
}

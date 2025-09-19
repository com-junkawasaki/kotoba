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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
            JsonnetValue::Builtin(_) => Err(KotobaNetError::FrontendParse("Builtins cannot be converted to JSON".to_string())),
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
    use std::io::Write;
    use tempfile::NamedTempFile;

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

    #[test]
    fn test_parse_all_prop_types() {
        let prop_types = vec![
            ("string", PropType::String),
            ("number", PropType::Number),
            ("boolean", PropType::Boolean),
            ("array", PropType::Array),
            ("object", PropType::Object),
            ("function", PropType::Function),
            ("component", PropType::Component),
        ];

        for (type_str, expected_type) in prop_types {
            let component_def = format!(r#"
            {{
                components: {{
                    TestComponent: {{
                        props: {{
                            testProp: {{
                                type: "{}",
                                required: true,
                            }}
                        }},
                        render: "<div>{{props.testProp}}</div>",
                        imports: ["React"],
                    }}
                }}
            }}
            "#, type_str);

            let result = FrontendParser::parse(&component_def);
            assert!(result.is_ok(), "Failed to parse prop type: {}", type_str);

            let config = result.unwrap();
            let component = &config.components["TestComponent"];
            let prop = &component.props["testProp"];
            assert_eq!(prop.type_, expected_type);
        }
    }

    #[test]
    fn test_parse_custom_prop_type() {
        let component_def = r#"
        {
            components: {
                CustomComponent: {
                    props: {
                        customProp: {
                            type: "CustomType",
                            required: true,
                        }
                    },
                    render: "<div>{props.customProp}</div>",
                    imports: ["React"],
                }
            }
        }
        "#;

        let result = FrontendParser::parse(component_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        let component = &config.components["CustomComponent"];
        let prop = &component.props["customProp"];

        match &prop.type_ {
            PropType::Custom(custom_type) => assert_eq!(custom_type, "CustomType"),
            _ => panic!("Expected custom prop type"),
        }
    }

    #[test]
    fn test_parse_component_with_state() {
        let component_def = r#"
        {
            components: {
                Counter: {
                    props: {
                        initialValue: {
                            type: "number",
                            required: false,
                            default: 0,
                        }
                    },
                    state: {
                        count: {
                            initialValue: 0,
                            type: "number",
                            description: "Current counter value",
                        },
                        isActive: {
                            initialValue: true,
                            type: "boolean",
                            description: "Whether counter is active",
                        }
                    },
                    render: "<div>{state.count}</div>",
                    imports: ["React", "useState"],
                }
            }
        }
        "#;

        let result = FrontendParser::parse(component_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        let component = &config.components["Counter"];

        assert!(component.state.is_some());
        let state = component.state.as_ref().unwrap();
        assert_eq!(state.len(), 2);
        assert!(state.contains_key("count"));
        assert!(state.contains_key("isActive"));

        let count_state = &state["count"];
        assert_eq!(count_state.type_, PropType::Number);
        assert_eq!(count_state.description, Some("Current counter value".to_string()));

        let active_state = &state["isActive"];
        assert_eq!(active_state.type_, PropType::Boolean);
        assert_eq!(active_state.description, Some("Whether counter is active".to_string()));
    }

    #[test]
    fn test_parse_component_with_lifecycle() {
        let component_def = r#"
        {
            components: {
                LifecycleComponent: {
                    props: {
                        id: {
                            type: "string",
                            required: true,
                        }
                    },
                    lifecycle: {
                        componentDidMount: "console.log('Component mounted');",
                        componentDidUpdate: "console.log('Component updated');",
                        componentWillUnmount: "console.log('Component will unmount');",
                        shouldComponentUpdate: "return true;",
                    },
                    render: "<div id={props.id}>Lifecycle Component</div>",
                    imports: ["React"],
                }
            }
        }
        "#;

        let result = FrontendParser::parse(component_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        let component = &config.components["LifecycleComponent"];

        assert!(component.lifecycle.is_some());
        let lifecycle = component.lifecycle.as_ref().unwrap();
        assert_eq!(lifecycle.component_did_mount, Some("console.log('Component mounted');".to_string()));
        assert_eq!(lifecycle.component_did_update, Some("console.log('Component updated');".to_string()));
        assert_eq!(lifecycle.component_will_unmount, Some("console.log('Component will unmount');".to_string()));
        assert_eq!(lifecycle.should_component_update, Some("return true;".to_string()));
    }

    #[test]
    fn test_parse_component_with_styles() {
        let component_def = r#"
        {
            components: {
                StyledComponent: {
                    props: {
                        variant: {
                            type: "string",
                            required: false,
                            default: "primary",
                        }
                    },
                    styles: {
                        container: "display: flex; align-items: center;",
                        button: "padding: 10px 20px; border-radius: 5px;",
                    },
                    render: "<div className='container'><button className='button'>{props.variant}</button></div>",
                    imports: ["React"],
                }
            }
        }
        "#;

        let result = FrontendParser::parse(component_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        let component = &config.components["StyledComponent"];

        assert!(component.styles.is_some());
        let styles = component.styles.as_ref().unwrap();
        assert_eq!(styles.len(), 2);
        assert_eq!(styles["container"], "display: flex; align-items: center;");
        assert_eq!(styles["button"], "padding: 10px 20px; border-radius: 5px;");
    }

    #[test]
    fn test_parse_pages() {
        let config_def = r#"
        {
            pages: [
                {
                    path: "/",
                    component: "HomePage",
                    layout: "MainLayout",
                    loading: "LoadingSpinner",
                    error: "ErrorPage",
                    meta: {
                        title: "Home Page",
                        description: "Welcome to our application",
                    }
                },
                {
                    path: "/about",
                    component: "AboutPage",
                    meta: {
                        title: "About Us",
                    }
                }
            ]
        }
        "#;

        let result = FrontendParser::parse(config_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.pages.len(), 2);

        let home_page = &config.pages[0];
        assert_eq!(home_page.path, "/");
        assert_eq!(home_page.component, "HomePage");
        assert_eq!(home_page.layout, Some("MainLayout".to_string()));
        assert_eq!(home_page.loading, Some("LoadingSpinner".to_string()));
        assert_eq!(home_page.error, Some("ErrorPage".to_string()));

        let meta = home_page.meta.as_ref().unwrap();
        assert_eq!(meta["title"], "Home Page");
        assert_eq!(meta["description"], "Welcome to our application");

        let about_page = &config.pages[1];
        assert_eq!(about_page.path, "/about");
        assert_eq!(about_page.component, "AboutPage");
        assert!(about_page.layout.is_none());
        assert!(about_page.loading.is_none());
        assert!(about_page.error.is_none());
    }

    #[test]
    fn test_parse_api_routes() {
        let config_def = r#"
        {
            apiRoutes: [
                {
                    path: "/api/users",
                    method: "GET",
                    handler: "getUsers",
                    authRequired: false,
                    schema: {
                        input: {
                            query: {
                                page: "number",
                                limit: "number",
                            }
                        },
                        output: {
                            users: "array",
                            total: "number",
                        },
                        errors: [
                            {
                                code: "VALIDATION_ERROR",
                                message: "Invalid input parameters",
                                statusCode: 400,
                            }
                        ]
                    }
                },
                {
                    path: "/api/users",
                    method: "POST",
                    handler: "createUser",
                    authRequired: true,
                }
            ]
        }
        "#;

        let result = FrontendParser::parse(config_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.api_routes.len(), 2);

        let get_users_route = &config.api_routes[0];
        assert_eq!(get_users_route.path, "/api/users");
        assert_eq!(get_users_route.method, "GET");
        assert_eq!(get_users_route.handler, "getUsers");
        assert!(!get_users_route.auth_required);

        assert!(get_users_route.schema.is_some());
        let schema = get_users_route.schema.as_ref().unwrap();
        assert!(schema.input.is_some());
        assert!(schema.output.is_some());
        assert!(schema.errors.is_some());
        assert_eq!(schema.errors.as_ref().unwrap().len(), 1);

        let create_user_route = &config.api_routes[1];
        assert_eq!(create_user_route.path, "/api/users");
        assert_eq!(create_user_route.method, "POST");
        assert_eq!(create_user_route.handler, "createUser");
        assert!(create_user_route.auth_required);
        assert!(create_user_route.schema.is_none());
    }

    #[test]
    fn test_parse_global_styles() {
        let config_def = r#"
        {
            globalStyles: {
                "body": "margin: 0; font-family: Arial, sans-serif;",
                ".container": "max-width: 1200px; margin: 0 auto;",
                ".button-primary": "background: blue; color: white; padding: 10px 20px;",
            }
        }
        "#;

        let result = FrontendParser::parse(config_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.global_styles.is_some());
        let global_styles = config.global_styles.as_ref().unwrap();
        assert_eq!(global_styles.len(), 3);
        assert!(global_styles.contains_key("body"));
        assert!(global_styles.contains_key(".container"));
        assert!(global_styles.contains_key(".button-primary"));
    }

    #[test]
    fn test_parse_complex_frontend_config() {
        let config_def = r#"
        {
            components: {
                Button: {
                    props: {
                        text: { type: "string", required: true },
                        variant: { type: "string", required: false, default: "primary" },
                        onClick: { type: "function", required: false }
                    },
                    state: {
                        isHovered: { initialValue: false, type: "boolean" }
                    },
                    lifecycle: {
                        componentDidMount: "console.log('Button mounted');"
                    },
                    styles: {
                        button: "padding: 10px 20px; border: none; border-radius: 5px;"
                    },
                    render: "<button className='button' onClick={props.onClick}>{props.text}</button>",
                    imports: ["React"]
                }
            },
            pages: [
                {
                    path: "/",
                    component: "HomePage",
                    layout: "AppLayout",
                    meta: { title: "Home" }
                }
            ],
            apiRoutes: [
                {
                    path: "/api/users",
                    method: "GET",
                    handler: "getUsers",
                    authRequired: true
                }
            ],
            globalStyles: {
                body: "font-family: sans-serif;"
            },
            config: {
                theme: "light",
                apiUrl: "https://api.example.com"
            }
        }
        "#;

        let result = FrontendParser::parse(config_def);
        assert!(result.is_ok());

        let config = result.unwrap();

        // Test components
        assert_eq!(config.components.len(), 1);
        assert!(config.components.contains_key("Button"));
        let button = &config.components["Button"];
        assert_eq!(button.props.len(), 3);
        assert!(button.state.is_some());
        assert!(button.lifecycle.is_some());
        assert!(button.styles.is_some());

        // Test pages
        assert_eq!(config.pages.len(), 1);

        // Test API routes
        assert_eq!(config.api_routes.len(), 1);

        // Test global styles
        assert!(config.global_styles.is_some());

        // Test config
        assert!(config.config.get("theme").is_some());
        assert!(config.config.get("apiUrl").is_some());
    }

    #[test]
    fn test_parse_minimal_config() {
        let config_def = r#"
        {
            components: {
                SimpleDiv: {
                    render: "<div>Hello World</div>",
                    imports: ["React"]
                }
            }
        }
        "#;

        let result = FrontendParser::parse(config_def);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.components.len(), 1);
        let component = &config.components["SimpleDiv"];
        assert_eq!(component.name, "SimpleDiv");
        assert!(component.props.is_empty());
        assert!(component.state.is_none());
        assert!(component.lifecycle.is_none());
        assert!(component.styles.is_none());
        assert!(component.imports.contains(&"React".to_string()));
    }

    #[test]
    fn test_parse_file_success() {
        let config_content = r#"
        {
            components: {
                TestComponent: {
                    render: "<div>Test</div>",
                    imports: ["React"]
                }
            }
        }
        "#;

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(config_content.as_bytes()).unwrap();
        let file_path = temp_file.path();

        let result = FrontendParser::parse_file(file_path);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert_eq!(config.components.len(), 1);
        assert!(config.components.contains_key("TestComponent"));
    }

    #[test]
    fn test_parse_file_not_found() {
        let result = FrontendParser::parse_file("/nonexistent/file.jsonnet");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), KotobaNetError::Io(_)));
    }

    #[test]
    fn test_parse_missing_required_fields() {
        // Missing render
        let config1 = r#"
        {
            components: {
                BadComponent: {
                    imports: ["React"]
                }
            }
        }
        "#;
        let result1 = FrontendParser::parse(config1);
        assert!(result1.is_err());

        // Missing component name in props
        let config2 = r#"
        {
            components: {
                TestComponent: {
                    props: {
                        "": { type: "string" }
                    },
                    render: "<div></div>",
                    imports: ["React"]
                }
            }
        }
        "#;
        let result2 = FrontendParser::parse(config2);
        assert!(result2.is_err());
    }

    #[test]
    fn test_parse_invalid_prop_type() {
        let config = r#"
        {
            components: {
                BadComponent: {
                    props: {
                        badProp: { type: 123 }
                    },
                    render: "<div></div>",
                    imports: ["React"]
                }
            }
        }
        "#;

        let result = FrontendParser::parse(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_non_object_root() {
        let config = r#"
        "this should be an object"
        "#;

        let result = FrontendParser::parse(config);
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, KotobaNetError::FrontendParse(_)));
        assert!(error.to_string().contains("Frontend configuration must be an object"));
    }

    #[test]
    fn test_parse_empty_components() {
        let config = r#"
        {
            components: {}
        }
        "#;

        let result = FrontendParser::parse(config);
        assert!(result.is_ok());

        let config = result.unwrap();
        assert!(config.components.is_empty());
    }

    #[test]
    fn test_parse_invalid_state_definition() {
        let config = r#"
        {
            components: {
                BadComponent: {
                    state: {
                        badState: {}
                    },
                    render: "<div></div>",
                    imports: ["React"]
                }
            }
        }
        "#;

        let result = FrontendParser::parse(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_component_with_default_values() {
        let config = r#"
        {
            components: {
                ComponentWithDefaults: {
                    props: {
                        optionalProp: {
                            type: "string",
                            required: false,
                            default: "default value",
                            description: "An optional property"
                        }
                    },
                    render: "<div>{props.optionalProp}</div>",
                    imports: ["React"]
                }
            }
        }
        "#;

        let result = FrontendParser::parse(config);
        assert!(result.is_ok());

        let config = result.unwrap();
        let component = &config.components["ComponentWithDefaults"];
        let prop = &component.props["optionalProp"];

        assert!(!prop.required);
        assert!(prop.default.is_some());
        assert_eq!(prop.description, Some("An optional property".to_string()));
    }

    #[test]
    fn test_serialization() {
        let config = FrontendConfig {
            components: HashMap::new(),
            pages: vec![PageDef {
                path: "/test".to_string(),
                component: "TestPage".to_string(),
                layout: Some("TestLayout".to_string()),
                loading: None,
                error: None,
                meta: Some(HashMap::from([
                    ("title".to_string(), "Test Page".to_string()),
                ])),
            }],
            api_routes: vec![ApiRouteDef {
                path: "/api/test".to_string(),
                method: "GET".to_string(),
                handler: "testHandler".to_string(),
                schema: None,
                auth_required: true,
            }],
            global_styles: Some(HashMap::from([
                ("body".to_string(), "margin: 0;".to_string()),
            ])),
            config: serde_json::json!({"test": "value"}),
        };

        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("/test"));
        assert!(json.contains("TestPage"));
        assert!(json.contains("/api/test"));
        assert!(json.contains("testHandler"));
        assert!(json.contains("margin: 0"));
        assert!(json.contains("value"));
    }
}

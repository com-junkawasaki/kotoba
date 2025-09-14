//! HTTP Parser for .kotoba.json configuration files

use crate::{KotobaNetError, Result};
use kotoba_jsonnet::JsonnetValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP route configuration parsed from Jsonnet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpRouteConfig {
    pub path: String,
    pub method: HttpMethod,
    pub handler: String,
    pub middleware: Vec<String>,
    pub auth_required: bool,
    pub cors_enabled: bool,
    pub rate_limit: Option<RateLimitConfig>,
}

/// HTTP method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

/// Rate limiting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub burst_limit: u32,
}

/// Complete HTTP server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpConfig {
    pub routes: Vec<HttpRouteConfig>,
    pub middleware: HashMap<String, MiddlewareConfig>,
    pub auth: Option<AuthConfig>,
    pub cors: Option<CorsConfig>,
    pub static_files: Option<StaticFilesConfig>,
}

/// Middleware configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    pub name: String,
    pub config: serde_json::Value,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub provider: String,
    pub config: serde_json::Value,
}

/// CORS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
    pub max_age: Option<u32>,
}

/// Static files configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaticFilesConfig {
    pub root: String,
    pub index_file: Option<String>,
    pub cache_control: Option<String>,
}

/// HTTP Parser for .kotoba.json files
#[derive(Debug)]
pub struct HttpParser;

impl HttpParser {
    /// Parse a .kotoba.json file containing HTTP configuration
    pub fn parse(content: &str) -> Result<HttpConfig> {
        // First evaluate the Jsonnet code
        let evaluated = crate::evaluate_kotoba(content)?;

        // Convert to HTTP config
        Self::jsonnet_value_to_http_config(&evaluated)
    }

    /// Parse HTTP config from file path
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<HttpConfig> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Convert JsonnetValue to HttpConfig
    fn jsonnet_value_to_http_config(value: &JsonnetValue) -> Result<HttpConfig> {
        match value {
            JsonnetValue::Object(obj) => {
                let routes = Self::extract_routes(obj)?;
                let middleware = Self::extract_middleware(obj)?;
                let auth = Self::extract_auth(obj)?;
                let cors = Self::extract_cors(obj)?;
                let static_files = Self::extract_static_files(obj)?;

                Ok(HttpConfig {
                    routes,
                    middleware,
                    auth,
                    cors,
                    static_files,
                })
            }
            _ => Err(KotobaNetError::HttpParse(
                "Root configuration must be an object".to_string(),
            )),
        }
    }

    /// Extract routes from Jsonnet object
    fn extract_routes(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<HttpRouteConfig>> {
        let mut routes = Vec::new();

        if let Some(JsonnetValue::Array(route_array)) = obj.get("routes") {
            for route_value in route_array {
                if let JsonnetValue::Object(route_obj) = route_value {
                    let route = Self::parse_route(route_obj)?;
                    routes.push(route);
                }
            }
        }

        Ok(routes)
    }

    /// Parse a single route configuration
    fn parse_route(obj: &HashMap<String, JsonnetValue>) -> Result<HttpRouteConfig> {
        let path = Self::extract_string(obj, "path")?;
        let method = Self::extract_method(obj)?;
        let handler = Self::extract_string(obj, "handler")?;
        let middleware = Self::extract_string_array(obj, "middleware")?;
        let auth_required = Self::extract_bool(obj, "authRequired").unwrap_or(false);
        let cors_enabled = Self::extract_bool(obj, "corsEnabled").unwrap_or(true);
        let rate_limit = Self::extract_rate_limit(obj)?;

        Ok(HttpRouteConfig {
            path,
            method,
            handler,
            middleware,
            auth_required,
            cors_enabled,
            rate_limit,
        })
    }

    /// Extract HTTP method
    fn extract_method(obj: &HashMap<String, JsonnetValue>) -> Result<HttpMethod> {
        let method_str = Self::extract_string(obj, "method")?;
        match method_str.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "PATCH" => Ok(HttpMethod::PATCH),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            "HEAD" => Ok(HttpMethod::HEAD),
            _ => Err(KotobaNetError::HttpParse(format!("Invalid HTTP method: {}", method_str))),
        }
    }

    /// Extract rate limit configuration
    fn extract_rate_limit(obj: &HashMap<String, JsonnetValue>) -> Result<Option<RateLimitConfig>> {
        if let Some(JsonnetValue::Object(rate_obj)) = obj.get("rateLimit") {
            let requests_per_minute = Self::extract_number(rate_obj, "requestsPerMinute")? as u32;
            let burst_limit = Self::extract_number(rate_obj, "burstLimit").unwrap_or((requests_per_minute * 2) as f64) as u32;

            Ok(Some(RateLimitConfig {
                requests_per_minute,
                burst_limit,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract middleware configurations
    fn extract_middleware(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, MiddlewareConfig>> {
        let mut middleware = HashMap::new();

        if let Some(JsonnetValue::Object(mw_obj)) = obj.get("middleware") {
            for (name, config) in mw_obj {
                if let JsonnetValue::Object(config_obj) = config {
                    let config_map = Self::jsonnet_object_to_hashmap(config_obj)?;
                    middleware.insert(name.clone(), MiddlewareConfig {
                        name: name.clone(),
                        config: config_map,
                    });
                }
            }
        }

        Ok(middleware)
    }

    /// Extract auth configuration
    fn extract_auth(obj: &HashMap<String, JsonnetValue>) -> Result<Option<AuthConfig>> {
        if let Some(JsonnetValue::Object(auth_obj)) = obj.get("auth") {
            let enabled = Self::extract_bool(auth_obj, "enabled").unwrap_or(true);
            let provider = Self::extract_string(auth_obj, "provider")?;
            let config = Self::jsonnet_object_to_hashmap(auth_obj)?;

            Ok(Some(AuthConfig {
                enabled,
                provider,
                config,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract CORS configuration
    fn extract_cors(obj: &HashMap<String, JsonnetValue>) -> Result<Option<CorsConfig>> {
        if let Some(JsonnetValue::Object(cors_obj)) = obj.get("cors") {
            let allowed_origins = Self::extract_string_array(cors_obj, "allowedOrigins")?;
            let allowed_methods = Self::extract_string_array(cors_obj, "allowedMethods")?;
            let allowed_headers = Self::extract_string_array(cors_obj, "allowedHeaders")?;
            let allow_credentials = Self::extract_bool(cors_obj, "allowCredentials").unwrap_or(false);
            let max_age = Self::extract_number(cors_obj, "maxAge").map(|n| n as u32).ok();

            Ok(Some(CorsConfig {
                allowed_origins,
                allowed_methods,
                allowed_headers,
                allow_credentials,
                max_age,
            }))
        } else {
            Ok(None)
        }
    }

    /// Extract static files configuration
    fn extract_static_files(obj: &HashMap<String, JsonnetValue>) -> Result<Option<StaticFilesConfig>> {
        if let Some(JsonnetValue::Object(static_obj)) = obj.get("staticFiles") {
            let root = Self::extract_string(static_obj, "root")?;
            let index_file = Self::extract_string(static_obj, "indexFile").ok();
            let cache_control = Self::extract_string(static_obj, "cacheControl").ok();

            Ok(Some(StaticFilesConfig {
                root,
                index_file,
                cache_control,
            }))
        } else {
            Ok(None)
        }
    }

    // Helper methods for extracting values from Jsonnet objects

    fn extract_string(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<String> {
        match obj.get(key) {
            Some(JsonnetValue::String(s)) => Ok(s.clone()),
            _ => Err(KotobaNetError::HttpParse(format!("Expected string for key '{}'", key))),
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
            _ => Err(KotobaNetError::HttpParse(format!("Expected number for key '{}'", key))),
        }
    }

    fn extract_string_array(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<Vec<String>> {
        match obj.get(key) {
            Some(JsonnetValue::Array(arr)) => {
                let mut strings = Vec::new();
                for item in arr {
                    if let JsonnetValue::String(s) = item {
                        strings.push(s.clone());
                    } else {
                        return Err(KotobaNetError::HttpParse(format!("Expected string array for key '{}'", key)));
                    }
                }
                Ok(strings)
            }
            _ => Ok(Vec::new()), // Default to empty array
        }
    }

    fn jsonnet_object_to_hashmap(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        // Convert JsonnetValue to serde_json::Value
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
            JsonnetValue::Object(obj) => Self::jsonnet_object_to_hashmap(obj),
            JsonnetValue::Function(_) => Err(KotobaNetError::HttpParse("Functions cannot be converted to JSON".to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_http_config() {
        let config = r#"
        {
            routes: [
                {
                    path: "/api/users",
                    method: "GET",
                    handler: "getUsers",
                    middleware: ["auth", "cors"],
                    authRequired: true,
                    corsEnabled: true,
                }
            ],
            middleware: {
                auth: {
                    type: "jwt",
                    secret: "secret-key",
                },
                cors: {
                    origins: ["*"],
                }
            }
        }
        "#;

        let result = HttpParser::parse(config);
        assert!(result.is_ok());

        let http_config = result.unwrap();
        assert_eq!(http_config.routes.len(), 1);
        assert_eq!(http_config.routes[0].path, "/api/users");
        assert!(http_config.middleware.contains_key("auth"));
    }
}

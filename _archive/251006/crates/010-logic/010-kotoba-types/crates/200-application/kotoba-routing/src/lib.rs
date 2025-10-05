//! Kotoba Routing - Pure Kernel & Effects Shell Architecture
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate provides HTTP routing with clear separation:
//!
//! - **Pure Kernel**: Route definitions, path matching, parameter extraction
//! - **Effects Shell**: HTTP request/response handling, middleware execution

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP methods
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

/// Pure route definition
#[derive(Debug, Clone, PartialEq)]
pub struct Route {
    /// HTTP method
    pub method: HttpMethod,
    /// Path pattern (e.g., "/users/:id")
    pub path: String,
    /// Route handler identifier
    pub handler: String,
    /// Middleware stack
    pub middleware: Vec<String>,
}

impl Route {
    /// Create a new route
    pub fn new(method: HttpMethod, path: impl Into<String>, handler: impl Into<String>) -> Self {
        Self {
            method,
            path: path.into(),
            handler: handler.into(),
            middleware: vec![],
        }
    }

    /// Add middleware to the route
    pub fn with_middleware(mut self, middleware: Vec<String>) -> Self {
        self.middleware = middleware;
        self
    }
}

/// Pure route match result
#[derive(Debug, Clone, PartialEq)]
pub struct RouteMatch {
    /// Matched route
    pub route: Route,
    /// Extracted path parameters
    pub params: HashMap<String, String>,
    /// Query parameters
    pub query: HashMap<String, String>,
}

/// Pure route matcher - no side effects
pub struct PureRouteMatcher {
    routes: Vec<Route>,
}

impl PureRouteMatcher {
    /// Create a new route matcher
    pub fn new(routes: Vec<Route>) -> Self {
        Self { routes }
    }

    /// Match a request path against routes (pure function)
    pub fn match_route(&self, method: &HttpMethod, path: &str) -> Option<RouteMatch> {
        // Split path and query
        let (path_part, query_part) = self.split_path_query(path);
        let query = self.parse_query(query_part);

        for route in &self.routes {
            if route.method != *method {
                continue;
            }

            if let Some(params) = self.match_path(&route.path, &path_part) {
                return Some(RouteMatch {
                    route: route.clone(),
                    params,
                    query,
                });
            }
        }

        None
    }

    /// Match path pattern against actual path (pure function)
    fn match_path(&self, pattern: &str, path: &str) -> Option<HashMap<String, String>> {
        let pattern_parts: Vec<&str> = pattern.trim_matches('/').split('/').collect();
        let path_parts: Vec<&str> = path.trim_matches('/').split('/').collect();

        if pattern_parts.len() != path_parts.len() {
            return None;
        }

        let mut params = HashMap::new();

        for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
            if pattern_part.starts_with(':') {
                // Parameter
                let param_name = &pattern_part[1..];
                params.insert(param_name.to_string(), path_part.to_string());
            } else if pattern_part != path_part {
                // Literal mismatch
                return None;
            }
        }

        Some(params)
    }

    /// Split path and query string
    fn split_path_query(&self, path: &str) -> (&str, Option<&str>) {
        if let Some(pos) = path.find('?') {
            (&path[..pos], Some(&path[pos + 1..]))
        } else {
            (path, None)
        }
    }

    /// Parse query string into parameters
    fn parse_query(&self, query: Option<&str>) -> HashMap<String, String> {
        let mut params = HashMap::new();

        if let Some(query) = query {
            for pair in query.split('&') {
                if let Some(pos) = pair.find('=') {
                    let key = &pair[..pos];
                    let value = &pair[pos + 1..];
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }

        params
    }

    /// Validate route definitions (pure function)
    pub fn validate_routes(&self) -> Result<(), RouteError> {
        for route in &self.routes {
            if route.path.is_empty() {
                return Err(RouteError::EmptyPath);
            }
            if route.handler.is_empty() {
                return Err(RouteError::EmptyHandler);
            }

            // Check for duplicate routes
            let duplicate_count = self.routes.iter()
                .filter(|r| r.method == route.method && r.path == route.path)
                .count();

            if duplicate_count > 1 {
                return Err(RouteError::DuplicateRoute(route.method.clone(), route.path.clone()));
            }
        }

        Ok(())
    }
}

/// Effects Shell route dispatcher - handles HTTP requests
pub struct RouteDispatcher {
    matcher: PureRouteMatcher,
}

impl RouteDispatcher {
    /// Create a new route dispatcher
    pub fn new(routes: Vec<Route>) -> Result<Self, RouteError> {
        let matcher = PureRouteMatcher::new(routes);
        matcher.validate_routes()?;

        Ok(Self { matcher })
    }

    /// Dispatch HTTP request (effects: depends on handler execution)
    pub async fn dispatch(&self, method: HttpMethod, path: &str) -> RouteResult {
        match self.matcher.match_route(&method, path) {
            Some(route_match) => {
                // In real implementation, this would execute the handler
                RouteResult::Match(route_match)
            }
            None => RouteResult::NotFound,
        }
    }

    /// Get all routes for inspection
    pub fn routes(&self) -> &[Route] {
        &self.matcher.routes
    }
}

/// Route dispatch result
#[derive(Debug, Clone)]
pub enum RouteResult {
    /// Route matched
    Match(RouteMatch),
    /// Route not found
    NotFound,
    /// Route error
    Error(String),
}

/// Route errors
#[derive(Debug, Clone)]
pub enum RouteError {
    EmptyPath,
    EmptyHandler,
    DuplicateRoute(HttpMethod, String),
    ValidationError(String),
}

/// Convenience functions for creating routes
pub mod routes {
    use super::*;

    pub fn get(path: impl Into<String>, handler: impl Into<String>) -> Route {
        Route::new(HttpMethod::GET, path, handler)
    }

    pub fn post(path: impl Into<String>, handler: impl Into<String>) -> Route {
        Route::new(HttpMethod::POST, path, handler)
    }

    pub fn put(path: impl Into<String>, handler: impl Into<String>) -> Route {
        Route::new(HttpMethod::PUT, path, handler)
    }

    pub fn delete(path: impl Into<String>, handler: impl Into<String>) -> Route {
        Route::new(HttpMethod::DELETE, path, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_route_matching() {
        let routes = vec![
            routes::get("/users", "list_users"),
            routes::get("/users/:id", "get_user"),
            routes::post("/users", "create_user"),
        ];

        let matcher = PureRouteMatcher::new(routes);

        // Test exact match
        let result = matcher.match_route(&HttpMethod::GET, "/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().route.handler, "list_users");

        // Test parameterized match
        let result = matcher.match_route(&HttpMethod::GET, "/users/123");
        assert!(result.is_some());
        let route_match = result.unwrap();
        assert_eq!(route_match.route.handler, "get_user");
        assert_eq!(route_match.params.get("id"), Some(&"123".to_string()));

        // Test POST match
        let result = matcher.match_route(&HttpMethod::POST, "/users");
        assert!(result.is_some());
        assert_eq!(result.unwrap().route.handler, "create_user");

        // Test no match
        let result = matcher.match_route(&HttpMethod::GET, "/posts");
        assert!(result.is_none());
    }

    #[test]
    fn test_route_validation() {
        // Valid routes
        let valid_routes = vec![
            routes::get("/users", "list_users"),
            routes::post("/users", "create_user"),
        ];
        let matcher = PureRouteMatcher::new(valid_routes);
        assert!(matcher.validate_routes().is_ok());

        // Invalid routes - duplicate
        let invalid_routes = vec![
            routes::get("/users", "list_users"),
            routes::get("/users", "another_handler"),
        ];
        let matcher = PureRouteMatcher::new(invalid_routes);
        assert!(matches!(matcher.validate_routes(), Err(RouteError::DuplicateRoute(_, _))));
    }

    #[test]
    fn test_query_parsing() {
        let routes = vec![routes::get("/search", "search_handler")];
        let matcher = PureRouteMatcher::new(routes);

        let result = matcher.match_route(&HttpMethod::GET, "/search?q=rust&page=1");
        assert!(result.is_some());
        let route_match = result.unwrap();
        assert_eq!(route_match.query.get("q"), Some(&"rust".to_string()));
        assert_eq!(route_match.query.get("page"), Some(&"1".to_string()));
    }

    #[tokio::test]
    async fn test_route_dispatcher() {
        let routes = vec![
            routes::get("/hello/:name", "hello_handler"),
        ];

        let dispatcher = RouteDispatcher::new(routes).unwrap();
        let result = dispatcher.dispatch(HttpMethod::GET, "/hello/world").await;

        match result {
            RouteResult::Match(route_match) => {
                assert_eq!(route_match.route.handler, "hello_handler");
                assert_eq!(route_match.params.get("name"), Some(&"world".to_string()));
            }
            _ => panic!("Expected route match"),
        }
    }
}

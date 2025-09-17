//! Integration with existing Kotoba crates
//!
//! このモジュールは既存のkotoba-jsonnetとkotoba-kotobasの機能を
//! 統合的に利用する方法を提供します。

use crate::error::{HandlerError, Result};
use crate::types::{HandlerContext, HandlerResult};

// Jsonnet integration removed - using kotobas only

/// Integration with kotoba-kotobas for HTTP configuration
pub mod kotobas_integration {
    use super::*;
    use kotoba_kotobas::http_parser::{HttpConfig, HttpRouteConfig, HttpMethod};

    /// Kotobas HTTP configuration handler
    pub struct KotobasHttpHandler {
        config: HttpConfig,
    }

    impl KotobasHttpHandler {
        pub fn new(config_content: &str) -> Result<Self> {
            // Parse Kotobas HTTP configuration
            let config: HttpConfig = serde_json::from_str(config_content)
                .map_err(|e| HandlerError::Parse(format!("Failed to parse HTTP config: {}", e)))?;

            Ok(Self { config })
        }

        /// Find matching route for request
        pub fn find_route(&self, method: &str, path: &str) -> Option<&HttpRouteConfig> {
            let request_method = match method {
                "GET" => HttpMethod::GET,
                "POST" => HttpMethod::POST,
                "PUT" => HttpMethod::PUT,
                "DELETE" => HttpMethod::DELETE,
                "PATCH" => HttpMethod::PATCH,
                "OPTIONS" => HttpMethod::OPTIONS,
                "HEAD" => HttpMethod::HEAD,
                _ => return None,
            };

            self.config.routes.iter()
                .find(|route| route.method == request_method && route.path == path)
        }

        /// Get middleware for route
        pub fn get_middleware(&self, route: &HttpRouteConfig) -> Vec<String> {
            route.middleware.clone()
        }
    }
}

/// Combined handler that integrates Kotobas functionality
pub struct IntegratedHandler {
    kotobas_handler: kotobas_integration::KotobasHttpHandler,
}

impl IntegratedHandler {
    /// Create new integrated handler
    pub fn new(kotobas_config: &str) -> Result<Self> {
        Ok(Self {
            kotobas_handler: kotobas_integration::KotobasHttpHandler::new(kotobas_config)?,
        })
    }

    /// Process request with integrated handlers
    pub async fn process_request(&mut self, context: HandlerContext, _content: Option<&str>) -> Result<String> {
        // Find matching route in Kotobas configuration
        if let Some(route) = self.kotobas_handler.find_route(&context.method, &context.path) {
            // Apply middleware
            let middleware = self.kotobas_handler.get_middleware(route);

            // Return route handler info
            Ok(format!(
                "Route matched: {} {} -> Handler: {}",
                route.method.as_ref(),
                route.path,
                route.handler
            ))
        } else {
            Ok("No route matched".to_string())
        }
    }
}

/// Factory function to create appropriate handler based on content type
pub fn create_handler(content: &str, _context: &HandlerContext) -> Result<Box<dyn HandlerTrait>> {
    if serde_json::from_str::<serde_json::Value>(content).is_ok() {
        // JSON content - use Kotobas handler
        let handler = kotobas_integration::KotobasHttpHandler::new(content)?;
        Ok(Box::new(KotobasWrapper(handler)))
    } else {
        Err(HandlerError::Parse("Unsupported content format - only JSON/Kotobas supported".to_string()))
    }
}

/// Handler trait for unified interface
pub trait HandlerTrait {
    fn process(&mut self, context: HandlerContext) -> Result<String>;
}

// Wrapper implementations
struct KotobasWrapper(kotobas_integration::KotobasHttpHandler);

impl HandlerTrait for KotobasWrapper {
    fn process(&mut self, context: HandlerContext) -> Result<String> {
        match self.0.find_route(&context.method, &context.path) {
            Some(route) => Ok(format!(
                "Kotobas route: {} {} -> {}",
                route.method.as_ref(),
                route.path,
                route.handler
            )),
            None => Ok("No Kotobas route matched".to_string()),
        }
    }
}

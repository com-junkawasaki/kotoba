//! Integration with existing Kotoba crates
//!
//! このモジュールは既存のkotoba-jsonnetとkotoba-kotobasの機能を
//! 統合的に利用する方法を提供します。

use crate::error::{HandlerError, Result};
use crate::types::{HandlerContext, HandlerResult};

/// Integration with kotoba-jsonnet for Jsonnet evaluation
#[cfg(feature = "jsonnet-integration")]
pub mod jsonnet_integration {
    use super::*;
    use kotoba_jsonnet::eval::{OpHandler, FuncallHandler, ComprehensionHandler, Context};
    use kotoba_jsonnet::JsonnetValue;

    /// Jsonnet evaluation handler
    pub struct JsonnetEvaluationHandler {
        op_handler: Box<dyn OpHandler>,
        funcall_handler: Box<dyn FuncallHandler>,
        comprehension_handler: Box<dyn ComprehensionHandler>,
    }

    impl JsonnetEvaluationHandler {
        pub fn new() -> Self {
            Self {
                op_handler: Box::new(kotoba_jsonnet::eval::DefaultOpHandler),
                funcall_handler: Box::new(kotoba_jsonnet::eval::DefaultFuncallHandler),
                comprehension_handler: Box::new(kotoba_jsonnet::eval::DefaultComprehensionHandler),
            }
        }

        /// Evaluate Jsonnet content
        pub fn evaluate(&mut self, content: &str, context: &HandlerContext) -> Result<String> {
            // Create Jsonnet context
            let mut jsonnet_context = Context::new();

            // Add handler context as external variables
            jsonnet_context.bind("method", JsonnetValue::String(context.method.clone()));
            jsonnet_context.bind("path", JsonnetValue::String(context.path.clone()));

            // Evaluate Jsonnet
            match kotoba_jsonnet::evaluate(content, &mut jsonnet_context) {
                Ok(value) => Ok(serde_json::to_string_pretty(&value)
                    .map_err(|e| HandlerError::Execution(format!("JSON serialization failed: {}", e)))?
                ),
                Err(e) => Err(HandlerError::Execution(format!("Jsonnet evaluation failed: {}", e))),
            }
        }
    }
}

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

/// Combined handler that integrates both Jsonnet and Kotobas
pub struct IntegratedHandler {
    #[cfg(feature = "jsonnet-integration")]
    jsonnet_handler: jsonnet_integration::JsonnetEvaluationHandler,
    kotobas_handler: kotobas_integration::KotobasHttpHandler,
}

impl IntegratedHandler {
    /// Create new integrated handler
    pub fn new(kotobas_config: &str) -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "jsonnet-integration")]
            jsonnet_handler: jsonnet_integration::JsonnetEvaluationHandler::new(),
            kotobas_handler: kotobas_integration::KotobasHttpHandler::new(kotobas_config)?,
        })
    }

    /// Process request with integrated handlers
    pub async fn process_request(&mut self, context: HandlerContext, content: Option<&str>) -> Result<String> {
        // First, find matching route in Kotobas configuration
        if let Some(route) = self.kotobas_handler.find_route(&context.method, &context.path) {
            // Apply middleware
            let middleware = self.kotobas_handler.get_middleware(route);

            // Process with Jsonnet if content is provided
            #[cfg(feature = "jsonnet-integration")]
            if let Some(content) = content {
                return self.jsonnet_handler.evaluate(content, &context);
            }

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
pub fn create_handler(content: &str, context: &HandlerContext) -> Result<Box<dyn HandlerTrait>> {
    if content.contains("jsonnet") || content.ends_with(".jsonnet") {
        #[cfg(feature = "jsonnet-integration")]
        {
            let handler = jsonnet_integration::JsonnetEvaluationHandler::new();
            Ok(Box::new(JsonnetWrapper(handler)))
        }
        #[cfg(not(feature = "jsonnet-integration"))]
        {
            Err(HandlerError::Config("Jsonnet integration not enabled".to_string()))
        }
    } else if serde_json::from_str::<serde_json::Value>(content).is_ok() {
        // JSON content - use Kotobas handler
        let handler = kotobas_integration::KotobasHttpHandler::new(content)?;
        Ok(Box::new(KotobasWrapper(handler)))
    } else {
        Err(HandlerError::Parse("Unsupported content format".to_string()))
    }
}

/// Handler trait for unified interface
pub trait HandlerTrait {
    fn process(&mut self, context: HandlerContext) -> Result<String>;
}

// Wrapper implementations
#[cfg(feature = "jsonnet-integration")]
struct JsonnetWrapper(jsonnet_integration::JsonnetEvaluationHandler);

#[cfg(feature = "jsonnet-integration")]
impl HandlerTrait for JsonnetWrapper {
    fn process(&mut self, context: HandlerContext) -> Result<String> {
        // Need content to evaluate - return placeholder for now
        Ok(format!("Jsonnet handler for {} {}", context.method, context.path))
    }
}

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

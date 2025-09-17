//! Runtime manager for coordinating external function handlers

use crate::error::Result;
use crate::value::JsonnetValue;
use super::db::{DbHandler, QueryExecutor, RewriteEngine};
use super::external::{AiModelHandler, HttpHandler, MemoryHandler, ToolHandler};
use crate::evaluator::ExternalHandler;
use std::collections::HashMap;
use std::sync::Arc;

/// Manages and dispatches calls to various external function handlers.
///
/// This struct acts as a single `ExternalHandler` that delegates calls
/// to the appropriate registered handler based on the function name prefix
/// (e.g., "ai.", "tool.", "db.").
#[derive(Debug, Clone)]
pub struct RuntimeManager {
    handlers: HashMap<String, Arc<dyn ExternalHandler>>,
}

impl ExternalHandler for RuntimeManager {
    fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        self.call_external_function(name, args)
    }
}

impl RuntimeManager {
    /// Creates a new `RuntimeManager` with a default set of handlers.
    ///
    /// # Arguments
    ///
    /// * `query_executor` - An `Arc`-wrapped instance of the `QueryExecutor`.
    /// * `rewrite_engine` - An `Arc`-wrapped instance of the `RewriteEngine`.
    pub fn new(
        query_executor: Arc<QueryExecutor>,
        rewrite_engine: Arc<RewriteEngine>,
    ) -> Self {
        let mut handlers: HashMap<String, Arc<dyn ExternalHandler>> = HashMap::new();
        handlers.insert("ai".to_string(), Arc::new(AiModelHandler {}));
        handlers.insert("http".to_string(), Arc::new(HttpHandler {}));
        handlers.insert("tool".to_string(), Arc::new(ToolHandler {}));
        handlers.insert("memory".to_string(), Arc::new(MemoryHandler {}));
        handlers.insert(
            "db".to_string(),
            Arc::new(DbHandler::new(query_executor, rewrite_engine)),
        );
        Self { handlers }
    }

    /// Registers a new external handler for a given namespace.
    pub fn register(&mut self, namespace: &str, handler: Arc<dyn ExternalHandler>) {
        self.handlers.insert(namespace.to_string(), handler);
    }

    /// Check if a function name belongs to an external handler
    pub fn is_external_function(&self, name: &str) -> bool {
        // Check for namespaced functions (e.g., "ai.httpGet", "tool.execute")
        if let Some(dot_pos) = name.find('.') {
            let namespace = &name[..dot_pos];
            self.handlers.contains_key(namespace)
        } else {
            false
        }
    }

    /// Call an external function
    pub async fn call_external_function(&mut self, name: &str, args: Vec<JsonnetValue>) -> Result<JsonnetValue> {
        if let Some(dot_pos) = name.find('.') {
            let namespace = &name[..dot_pos];
            let function = &name[dot_pos + 1..];

            if let Some(handler) = self.handlers.get_mut(namespace) {
                return handler.call(function, args).await;
            }
        }

        Err(crate::error::JsonnetError::runtime_error(format!("No handler found for external function: {}", name)))
    }

    /// Get all registered namespaces
    pub fn namespaces(&self) -> Vec<String> {
        self.handlers.keys().cloned().collect()
    }

    /// Check if a specific namespace is registered
    pub fn has_namespace(&self, namespace: &str) -> bool {
        self.handlers.contains_key(namespace)
    }

    /// Get handler for a namespace (mutable access for advanced usage)
    pub fn get_handler_mut(&mut self, namespace: &str) -> Option<&mut Arc<dyn ExternalHandler>> {
        self.handlers.get_mut(namespace)
    }

    /// Replace a handler for a namespace
    pub fn replace_handler(&mut self, handler: Arc<dyn ExternalHandler>) -> Option<Arc<dyn ExternalHandler>> {
        self.handlers.insert(handler.namespace().to_string(), handler)
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for custom runtime manager configuration
pub struct RuntimeManagerBuilder {
    handlers: Vec<Box<dyn ExternalHandler>>,
}

impl RuntimeManagerBuilder {
    pub fn new() -> Self {
        RuntimeManagerBuilder {
            handlers: Vec::new(),
        }
    }

    /// Add a custom handler
    pub fn with_handler(mut self, handler: Box<dyn ExternalHandler>) -> Self {
        self.handlers.push(handler);
        self
    }

    /// Add default handlers
    pub fn with_defaults(mut self) -> Self {
        self.handlers.push(Box::new(HttpHandler::new()));
        self.handlers.push(Box::new(AiModelHandler::new()));
        self.handlers.push(Box::new(ToolHandler::new()));
        self.handlers.push(Box::new(MemoryHandler::new()));
        self
    }

    /// Build the runtime manager
    pub fn build(self) -> RuntimeManager {
        let mut manager = RuntimeManager {
            handlers: HashMap::new(),
        };

        for handler in self.handlers {
            manager.register_handler(handler);
        }

        manager
    }
}

impl Default for RuntimeManagerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

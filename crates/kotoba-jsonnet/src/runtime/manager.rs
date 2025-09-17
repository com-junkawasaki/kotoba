//! Runtime manager for coordinating external function handlers

use crate::error::Result;
use crate::value::JsonnetValue;
use super::external::*;
use std::collections::HashMap;

/// Manager for external function handlers
pub struct RuntimeManager {
    handlers: HashMap<String, Box<dyn ExternalHandler>>,
}

impl RuntimeManager {
    /// Create a new runtime manager with default handlers
    pub fn new() -> Self {
        let mut manager = RuntimeManager {
            handlers: HashMap::new(),
        };

        // Register default handlers
        manager.register_handler(Box::new(HttpHandler::new()));
        manager.register_handler(Box::new(AiModelHandler::new()));
        manager.register_handler(Box::new(ToolHandler::new()));
        manager.register_handler(Box::new(MemoryHandler::new()));

        manager
    }

    /// Register an external handler
    pub fn register_handler(&mut self, handler: Box<dyn ExternalHandler>) {
        self.handlers.insert(handler.namespace().to_string(), handler);
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
    pub fn get_handler_mut(&mut self, namespace: &str) -> Option<&mut Box<dyn ExternalHandler>> {
        self.handlers.get_mut(namespace)
    }

    /// Replace a handler for a namespace
    pub fn replace_handler(&mut self, handler: Box<dyn ExternalHandler>) -> Option<Box<dyn ExternalHandler>> {
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

//! Runtime management for different execution environments

use crate::error::{HandlerError, Result};
use crate::types::HandlerCapabilities;
use std::sync::Arc;

/// Runtime environment types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeType {
    Native,
    Wasm,
    NodeJs,
    Deno,
    Browser,
}

/// Runtime environment
pub struct RuntimeEnvironment {
    runtime_type: RuntimeType,
    capabilities: HandlerCapabilities,
    memory_limit: u64,
    timeout_limit: u64,
}

impl RuntimeEnvironment {
    /// Create new runtime environment
    pub fn new(runtime_type: RuntimeType) -> Self {
        let capabilities = match runtime_type {
            RuntimeType::Native => HandlerCapabilities {
                supports_async: true,
                supports_streaming: true,
                supports_websocket: true,
                supports_file_upload: true,
                max_payload_size: 100 * 1024 * 1024, // 100MB
                supported_content_types: vec![
                    "application/json".to_string(),
                    "text/plain".to_string(),
                    "application/x-www-form-urlencoded".to_string(),
                    "multipart/form-data".to_string(),
                ],
            },
            RuntimeType::Wasm => HandlerCapabilities {
                supports_async: true,
                supports_streaming: false,
                supports_websocket: true,
                supports_file_upload: false,
                max_payload_size: 10 * 1024 * 1024, // 10MB
                supported_content_types: vec![
                    "application/json".to_string(),
                    "text/plain".to_string(),
                ],
            },
            RuntimeType::NodeJs | RuntimeType::Deno => HandlerCapabilities {
                supports_async: true,
                supports_streaming: true,
                supports_websocket: true,
                supports_file_upload: true,
                max_payload_size: 50 * 1024 * 1024, // 50MB
                supported_content_types: vec![
                    "application/json".to_string(),
                    "text/plain".to_string(),
                    "application/javascript".to_string(),
                ],
            },
            RuntimeType::Browser => HandlerCapabilities {
                supports_async: true,
                supports_streaming: false,
                supports_websocket: true,
                supports_file_upload: true,
                max_payload_size: 5 * 1024 * 1024, // 5MB
                supported_content_types: vec![
                    "application/json".to_string(),
                    "text/plain".to_string(),
                ],
            },
        };

        Self {
            runtime_type,
            capabilities,
            memory_limit: 100 * 1024 * 1024, // 100MB
            timeout_limit: 30_000, // 30 seconds
        }
    }

    /// Get runtime type
    pub fn runtime_type(&self) -> &RuntimeType {
        &self.runtime_type
    }

    /// Get capabilities
    pub fn capabilities(&self) -> &HandlerCapabilities {
        &self.capabilities
    }

    /// Check if runtime supports a specific feature
    pub fn supports(&self, feature: &str) -> bool {
        match feature {
            "async" => self.capabilities.supports_async,
            "streaming" => self.capabilities.supports_streaming,
            "websocket" => self.capabilities.supports_websocket,
            "file_upload" => self.capabilities.supports_file_upload,
            _ => false,
        }
    }

    /// Get memory limit
    pub fn memory_limit(&self) -> u64 {
        self.memory_limit
    }

    /// Get timeout limit
    pub fn timeout_limit(&self) -> u64 {
        self.timeout_limit
    }

    /// Set memory limit
    pub fn set_memory_limit(&mut self, limit: u64) {
        self.memory_limit = limit;
    }

    /// Set timeout limit
    pub fn set_timeout_limit(&mut self, limit: u64) {
        self.timeout_limit = limit;
    }
}

impl Default for RuntimeEnvironment {
    fn default() -> Self {
        Self::new(RuntimeType::Native)
    }
}

/// Runtime manager
pub struct RuntimeManager {
    environments: std::collections::HashMap<String, Arc<RuntimeEnvironment>>,
    current: Option<String>,
}

impl RuntimeManager {
    /// Create new runtime manager
    pub fn new() -> Self {
        let mut environments = std::collections::HashMap::new();

        // Add default environments
        environments.insert("native".to_string(), Arc::new(RuntimeEnvironment::new(RuntimeType::Native)));
        environments.insert("wasm".to_string(), Arc::new(RuntimeEnvironment::new(RuntimeType::Wasm)));
        environments.insert("nodejs".to_string(), Arc::new(RuntimeEnvironment::new(RuntimeType::NodeJs)));
        environments.insert("deno".to_string(), Arc::new(RuntimeEnvironment::new(RuntimeType::Deno)));
        environments.insert("browser".to_string(), Arc::new(RuntimeEnvironment::new(RuntimeType::Browser)));

        Self {
            environments,
            current: Some("native".to_string()),
        }
    }

    /// Register new runtime environment
    pub fn register_environment(&mut self, name: String, env: Arc<RuntimeEnvironment>) {
        self.environments.insert(name, env);
    }

    /// Get runtime environment by name
    pub fn get_environment(&self, name: &str) -> Option<Arc<RuntimeEnvironment>> {
        self.environments.get(name).cloned()
    }

    /// Set current runtime
    pub fn set_current(&mut self, name: &str) -> Result<()> {
        if self.environments.contains_key(name) {
            self.current = Some(name.to_string());
            Ok(())
        } else {
            Err(HandlerError::Config(format!("Runtime '{}' not found", name)))
        }
    }

    /// Get current runtime
    pub fn get_current(&self) -> Option<Arc<RuntimeEnvironment>> {
        self.current.as_ref()
            .and_then(|name| self.environments.get(name))
            .cloned()
    }

    /// List all available runtimes
    pub fn list_runtimes(&self) -> Vec<String> {
        self.environments.keys().cloned().collect()
    }
}

impl Default for RuntimeManager {
    fn default() -> Self {
        Self::new()
    }
}

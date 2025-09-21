//! Unified Handler implementation

use crate::error::{HandlerError, Result};
use crate::types::{HandlerContext, HandlerResult, HandlerConfig};
use kotoba_storage::KeyValueStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

// Optional dependencies - will be enabled based on features
#[cfg(feature = "kotobas")]
use kotoba_kotobas::prelude::*;

#[cfg(feature = "tsx")]
use kotoba2tsx::prelude::*;

/// Unified Handler for Kotoba ecosystem with KeyValueStore backend
#[derive(Clone)]
pub struct UnifiedHandler<T: KeyValueStore + 'static> {
    config: Arc<RwLock<HandlerConfig>>,
    storage: Arc<T>,
    #[cfg(feature = "kotobas")]
    kotobas_compiler: Arc<KotobasCompiler>,
    #[cfg(feature = "tsx")]
    tsx_converter: Arc<TsxConverter>,
    cache: Arc<RwLock<HashMap<String, HandlerResult>>>,
}

impl<T: KeyValueStore + 'static> UnifiedHandler<T> {
    /// Create new unified handler with KeyValueStore backend
    pub fn new(storage: Arc<T>) -> Self {
        Self {
            config: Arc::new(RwLock::new(HandlerConfig {
                timeout_ms: 30000,
                max_memory_mb: 100,
                enable_caching: true,
                enable_logging: true,
            })),
            storage,
            #[cfg(feature = "kotobas")]
            kotobas_compiler: Arc::new(KotobasCompiler::new()),
            #[cfg(feature = "tsx")]
            tsx_converter: Arc::new(TsxConverter::new()),
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute handler with given content and context
    pub async fn execute(&self, content: &str, context: HandlerContext) -> Result<String> {
        // Check cache first
        if self.config.read().await.enable_caching {
            let cache_key = self.generate_cache_key(content, &context);
            if let Some(cached_result) = self.cache.read().await.get(&cache_key) {
                if self.config.read().await.enable_logging {
                    println!("🔍 Cache hit for key: {}", cache_key);
                }
                return Ok(cached_result.body.clone());
            }
        }

        // Parse and validate content
        #[cfg(feature = "kotobas")]
        let parsed = self.kotobas_compiler.compile(content)
            .map_err(|e| HandlerError::Parse(format!("Failed to parse content: {}", e)))?;

        #[cfg(not(feature = "kotobas"))]
        let parsed = content.to_string(); // Fallback to raw content

        // Convert to executable format (TSX/React)
        #[cfg(feature = "tsx")]
        let executable_code = self.tsx_converter.convert(&parsed)
            .map_err(|e| HandlerError::Execution(format!("Failed to convert to TSX: {}", e)))?;

        #[cfg(not(feature = "tsx"))]
        let executable_code = parsed; // Use parsed content directly

        // Store compiled result in KeyValueStore for caching
        let compile_key = format!("compile:{}", self.generate_cache_key(content, &context));
        self.storage.put(compile_key.as_bytes(), executable_code.as_bytes()).await
            .map_err(|e| HandlerError::Storage(format!("Failed to store compiled result: {}", e)))?;

        // Execute with context
        let result = self.execute_with_context(&executable_code, context.clone()).await?;

        // Cache result
        if self.config.read().await.enable_caching {
            let cache_key = self.generate_cache_key(content, &context);
            let handler_result = HandlerResult {
                status_code: 200,
                headers: HashMap::new(),
                body: result.clone(),
                execution_time_ms: 0, // TODO: measure execution time
                memory_used_mb: 0.0,  // TODO: measure memory usage
            };
            self.cache.write().await.insert(cache_key, handler_result);
        }

        Ok(result)
    }

    /// Execute handler with file input
    pub async fn execute_file(&self, file_path: &str, context: HandlerContext) -> Result<String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| HandlerError::Io(e))?;

        self.execute(&content, context).await
    }

    /// Update handler configuration
    pub async fn update_config(&self, config: HandlerConfig) {
        *self.config.write().await = config;
    }

    /// Get current handler configuration
    pub async fn get_config(&self) -> HandlerConfig {
        self.config.read().await.clone()
    }

    /// Clear cache
    pub async fn clear_cache(&self) {
        self.cache.write().await.clear();
    }

    /// Get cache size
    pub async fn cache_size(&self) -> usize {
        self.cache.read().await.len()
    }

    // Private methods

    fn generate_cache_key(&self, content: &str, context: &HandlerContext) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        context.method.hash(&mut hasher);
        context.path.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    async fn execute_with_context(&self, tsx_code: &str, context: HandlerContext) -> Result<String> {
        // This is a simplified execution - in real implementation,
        // this would use a JavaScript runtime or WASM execution

        if self.config.read().await.enable_logging {
            println!("🚀 Executing TSX code with context: {:?}", context);
        }

        // For now, return a placeholder response
        // In real implementation, this would execute the TSX code
        // with the provided context and return the rendered result
        Ok(format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Kotoba Handler Result</title>
    <meta charset="UTF-8">
</head>
<body>
    <div id="kotoba-root">
        <h1>Kotoba Handler Executed</h1>
        <p>Method: {}</p>
        <p>Path: {}</p>
        <p>Content Length: {}</p>
        <pre>{}</pre>
    </div>
</body>
</html>"#,
            context.method,
            context.path,
            tsx_code.len(),
            tsx_code
        ))
    }
}


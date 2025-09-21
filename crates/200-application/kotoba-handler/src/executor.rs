//! Handler executor for different execution environments

use crate::error::{HandlerError, Result};
use crate::types::{HandlerContext, ExecutionMode};
use crate::handler::UnifiedHandler;
use std::sync::Arc;

/// Handler executor that manages execution across different environments
pub struct HandlerExecutor<T: crate::KeyValueStore + 'static> {
    handler: Arc<UnifiedHandler<T>>,
    execution_mode: ExecutionMode,
}

impl<T: crate::KeyValueStore + 'static> HandlerExecutor<T> {
    /// Create new executor
    pub fn new(handler: Arc<UnifiedHandler<T>>) -> Self {
        Self {
            handler,
            execution_mode: ExecutionMode::Sync,
        }
    }

    /// Set execution mode
    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    /// Execute handler synchronously
    pub async fn execute_sync(&self, content: &str, context: HandlerContext) -> Result<String> {
        match self.execution_mode {
            ExecutionMode::Sync => {
                self.handler.execute(content, context).await
            }
            ExecutionMode::Async => {
                self.execute_async_internal(content, context).await
            }
            ExecutionMode::Streaming => {
                self.execute_streaming_internal(content, context).await
            }
        }
    }

    /// Execute handler asynchronously
    pub async fn execute_async(&self, content: &str, context: HandlerContext) -> Result<String> {
        self.handler.execute(content, context).await
    }

    /// Execute handler with streaming
    pub async fn execute_streaming(&self, content: &str, context: HandlerContext) -> Result<String> {
        // Streaming implementation would return a stream
        // For now, just execute normally
        self.handler.execute(content, context).await
    }

    /// Execute from file
    pub async fn execute_file(&self, file_path: &str, context: HandlerContext) -> Result<String> {
        self.handler.execute_file(file_path, context).await
    }

    /// Batch execute multiple handlers
    pub async fn execute_batch(
        &self,
        requests: Vec<(String, HandlerContext)>
    ) -> Result<Vec<String>> {
        let mut results = Vec::new();

        for (content, context) in requests {
            let result = self.handler.execute(&content, context).await?;
            results.push(result);
        }

        Ok(results)
    }

    // Private methods

    async fn execute_async_internal(&self, content: &str, context: HandlerContext) -> Result<String> {
        // Async execution with timeout
        let handler = Arc::clone(&self.handler);
        let content = content.to_string();

        tokio::spawn(async move {
            handler.execute(&content, context).await
        })
        .await
        .map_err(|e| HandlerError::Execution(format!("Async execution failed: {}", e)))?
    }

    async fn execute_streaming_internal(&self, content: &str, context: HandlerContext) -> Result<String> {
        // Streaming execution placeholder
        // In real implementation, this would return a stream of results
        self.handler.execute(content, context).await
    }
}


/// Builder pattern for HandlerExecutor
pub struct HandlerExecutorBuilder<T: crate::KeyValueStore + 'static> {
    handler: Option<Arc<UnifiedHandler<T>>>,
    execution_mode: ExecutionMode,
}

impl<T: crate::KeyValueStore + 'static> HandlerExecutorBuilder<T> {
    pub fn new() -> Self {
        Self {
            handler: None,
            execution_mode: ExecutionMode::Sync,
        }
    }

    pub fn with_handler(mut self, handler: Arc<UnifiedHandler<T>>) -> Self {
        self.handler = Some(handler);
        self
    }

    pub fn with_mode(mut self, mode: ExecutionMode) -> Self {
        self.execution_mode = mode;
        self
    }

    pub fn build(self) -> Result<HandlerExecutor<T>> {
        let handler = self.handler.ok_or_else(|| crate::HandlerError::Config("Handler must be provided".to_string()))?;
        Ok(HandlerExecutor::new(handler).with_mode(self.execution_mode))
    }
}

impl<T: crate::KeyValueStore + 'static> Default for HandlerExecutorBuilder<T> {
    fn default() -> Self {
        Self::new()
    }
}

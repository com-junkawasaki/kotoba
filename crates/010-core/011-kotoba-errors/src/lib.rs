//! # Kotoba Errors
//!
//! Shared error types for the Kotoba ecosystem.
//! This crate provides a unified error handling system across all Kotoba components.

use thiserror::Error;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Core error types for the Kotoba system
#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KotobaError {
    /// Validation error
    #[error("Validation error: {message}")]
    Validation(String),

    /// Security-related error
    #[error("Security error: {message}")]
    Security(String),

    /// Authentication error
    #[error("Authentication error: {message}")]
    Auth(String),

    /// Authorization error
    #[error("Authorization error: {message}")]
    Authz(String),

    /// Invalid argument
    #[error("Invalid argument: {message}")]
    InvalidArgument(String),

    /// Not found
    #[error("Not found: {message}")]
    NotFound(String),

    /// Already exists
    #[error("Already exists: {message}")]
    AlreadyExists(String),

    /// IO error
    #[error("IO error: {message}")]
    Io(String),

    /// Network error
    #[error("Network error: {message}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {message}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {message}")]
    Deserialization(String),

    /// Timeout error
    #[error("Timeout error: {message}")]
    Timeout(String),

    /// Resource exhausted
    #[error("Resource exhausted: {message}")]
    ResourceExhausted(String),

    /// Unimplemented
    #[error("Unimplemented: {message}")]
    Unimplemented(String),

    /// Internal error
    #[error("Internal error: {message}")]
    Internal(String),

    /// Graph transformation error
    #[error("Graph transformation error: {message}")]
    GraphTransformation(String),

    /// Schema error
    #[error("Schema error: {message}")]
    Schema(String),

    /// Query error
    #[error("Query error: {message}")]
    Query(String),

    /// Execution error
    #[error("Execution error: {message}")]
    Execution(String),

    /// API error
    #[error("API error: {message}")]
    Api(String),

    /// Dependency resolution error
    #[error("Dependency resolution error: {message}")]
    DependencyResolution(String),

    /// Configuration error
    #[error("Configuration error: {message}")]
    Configuration(String),

    /// Database error
    #[error("Database error: {message}")]
    Database(String),

    /// Cache error
    #[error("Cache error: {message}")]
    Cache(String),

    /// Workflow error
    #[error("Workflow error: {message}")]
    Workflow(String),

    /// Plugin error
    #[error("Plugin error: {message}")]
    Plugin(String),
}

impl KotobaError {
    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation(message.into())
    }

    /// Create a security error
    pub fn security(message: impl Into<String>) -> Self {
        Self::Security(message.into())
    }

    /// Create an authentication error
    pub fn auth(message: impl Into<String>) -> Self {
        Self::Auth(message.into())
    }

    /// Create an authorization error
    pub fn authz(message: impl Into<String>) -> Self {
        Self::Authz(message.into())
    }

    /// Create an invalid argument error
    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::InvalidArgument(message.into())
    }

    /// Create a not found error
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound(message.into())
    }

    /// Create an already exists error
    pub fn already_exists(message: impl Into<String>) -> Self {
        Self::AlreadyExists(message.into())
    }

    /// Create an IO error
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network(message.into())
    }

    /// Create a serialization error
    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization(message.into())
    }

    /// Create a deserialization error
    pub fn deserialization(message: impl Into<String>) -> Self {
        Self::Deserialization(message.into())
    }

    /// Create a timeout error
    pub fn timeout(message: impl Into<String>) -> Self {
        Self::Timeout(message.into())
    }

    /// Create a resource exhausted error
    pub fn resource_exhausted(message: impl Into<String>) -> Self {
        Self::ResourceExhausted(message.into())
    }

    /// Create an unimplemented error
    pub fn unimplemented(message: impl Into<String>) -> Self {
        Self::Unimplemented(message.into())
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Create a graph transformation error
    pub fn graph_transformation(message: impl Into<String>) -> Self {
        Self::GraphTransformation(message.into())
    }

    /// Create a schema error
    pub fn schema(message: impl Into<String>) -> Self {
        Self::Schema(message.into())
    }

    /// Create a query error
    pub fn query(message: impl Into<String>) -> Self {
        Self::Query(message.into())
    }

    /// Create an execution error
    pub fn execution(message: impl Into<String>) -> Self {
        Self::Execution(message.into())
    }

    /// Create an API error
    pub fn api(message: impl Into<String>) -> Self {
        Self::Api(message.into())
    }

    /// Create a dependency resolution error
    pub fn dependency_resolution(message: impl Into<String>) -> Self {
        Self::DependencyResolution(message.into())
    }

    /// Create a configuration error
    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration(message.into())
    }

    /// Create a database error
    pub fn database(message: impl Into<String>) -> Self {
        Self::Database(message.into())
    }

    /// Create a cache error
    pub fn cache(message: impl Into<String>) -> Self {
        Self::Cache(message.into())
    }

    /// Create a workflow error
    pub fn workflow(message: impl Into<String>) -> Self {
        Self::Workflow(message.into())
    }

    /// Create a plugin error
    pub fn plugin(message: impl Into<String>) -> Self {
        Self::Plugin(message.into())
    }

    /// Check if the error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network(_) | Self::Timeout(_) | Self::ResourceExhausted(_)
        )
    }

    /// Get error category
    pub fn category(&self) -> ErrorCategory {
        match self {
            Self::Validation(_) => ErrorCategory::Validation,
            Self::Security(_) | Self::Auth(_) | Self::Authz(_) => ErrorCategory::Security,
            Self::InvalidArgument(_) | Self::NotFound(_) | Self::AlreadyExists(_) => ErrorCategory::Client,
            Self::Io(_) | Self::Network(_) | Self::Timeout(_) | Self::ResourceExhausted(_) => ErrorCategory::Infrastructure,
            Self::Serialization(_) | Self::Deserialization(_) => ErrorCategory::Data,
            Self::Unimplemented(_) | Self::Internal(_) => ErrorCategory::System,
            Self::GraphTransformation(_) | Self::Schema(_) | Self::Query(_) | Self::Execution(_) => ErrorCategory::BusinessLogic,
            Self::Api(_) | Self::DependencyResolution(_) | Self::Configuration(_) | Self::Database(_) | Self::Cache(_) | Self::Workflow(_) | Self::Plugin(_) => ErrorCategory::Service,
        }
    }

    /// Get HTTP status code for this error
    pub fn http_status_code(&self) -> u16 {
        match self {
            Self::Validation(_) => 400,
            Self::Security(_) | Self::Auth(_) | Self::Authz(_) => 401,
            Self::InvalidArgument(_) => 400,
            Self::NotFound(_) => 404,
            Self::AlreadyExists(_) => 409,
            Self::Io(_) => 500,
            Self::Network(_) => 503,
            Self::Serialization(_) | Self::Deserialization(_) => 400,
            Self::Timeout(_) => 408,
            Self::ResourceExhausted(_) => 429,
            Self::Unimplemented(_) => 501,
            Self::Internal(_) => 500,
            Self::GraphTransformation(_) => 422,
            Self::Schema(_) => 422,
            Self::Query(_) => 400,
            Self::Execution(_) => 500,
            Self::Api(_) => 500,
            Self::DependencyResolution(_) => 500,
            Self::Configuration(_) => 500,
            Self::Database(_) => 500,
            Self::Cache(_) => 500,
            Self::Workflow(_) => 500,
            Self::Plugin(_) => 500,
        }
    }
}

/// Error category for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorCategory {
    /// Validation errors
    Validation,
    /// Security errors
    Security,
    /// Client errors
    Client,
    /// Infrastructure errors
    Infrastructure,
    /// Data errors
    Data,
    /// System errors
    System,
    /// Business logic errors
    BusinessLogic,
    /// Service errors
    Service,
}

/// Result type alias
pub type KotobaResult<T> = Result<T, KotobaError>;

/// Error context for additional information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Error code
    pub code: String,
    /// Error message
    pub message: String,
    /// Additional context
    pub context: HashMap<String, String>,
    /// Timestamp
    pub timestamp: u64,
    /// Request ID if available
    pub request_id: Option<String>,
    /// Trace ID if available
    pub trace_id: Option<String>,
}

impl ErrorContext {
    /// Create a new error context
    pub fn new(code: String, message: String) -> Self {
        Self {
            code,
            message,
            context: HashMap::new(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            request_id: None,
            trace_id: None,
        }
    }

    /// Add context information
    pub fn with_context(mut self, key: String, value: String) -> Self {
        self.context.insert(key, value);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        self.request_id = Some(request_id);
        self
    }

    /// Set trace ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        self.trace_id = Some(trace_id);
        self
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// Error builder for fluent API
#[derive(Debug)]
pub struct ErrorBuilder {
    error: KotobaError,
    context: Option<ErrorContext>,
}

impl ErrorBuilder {
    /// Create a new error builder
    pub fn new(error: KotobaError) -> Self {
        Self {
            error,
            context: None,
        }
    }

    /// Add context
    pub fn with_context(mut self, key: String, value: String) -> Self {
        if let Some(ref mut ctx) = self.context {
            ctx.context.insert(key, value);
        } else {
            let mut ctx = ErrorContext::new(
                format!("{:?}", self.error.category()),
                self.error.to_string(),
            );
            ctx.context.insert(key, value);
            self.context = Some(ctx);
        }
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: String) -> Self {
        if let Some(ref mut ctx) = self.context {
            ctx.request_id = Some(request_id);
        } else {
            let mut ctx = ErrorContext::new(
                format!("{:?}", self.error.category()),
                self.error.to_string(),
            );
            ctx.request_id = Some(request_id);
            self.context = Some(ctx);
        }
        self
    }

    /// Set trace ID
    pub fn with_trace_id(mut self, trace_id: String) -> Self {
        if let Some(ref mut ctx) = self.context {
            ctx.trace_id = Some(trace_id);
        } else {
            let mut ctx = ErrorContext::new(
                format!("{:?}", self.error.category()),
                self.error.to_string(),
            );
            ctx.trace_id = Some(trace_id);
            self.context = Some(ctx);
        }
        self
    }

    /// Build the error context
    pub fn build_context(self) -> ErrorContext {
        self.context.unwrap_or_else(|| ErrorContext::new(
            format!("{:?}", self.error.category()),
            self.error.to_string(),
        ))
    }

    /// Consume the builder and return the error
    pub fn build(self) -> KotobaError {
        self.error
    }
}

/// Utility functions for error handling
pub mod utils {
    use super::*;

    /// Convert anyhow::Error to KotobaError
    pub fn anyhow_to_kotoba_error(err: anyhow::Error) -> KotobaError {
        KotobaError::Internal(err.to_string())
    }

    /// Convert std::io::Error to KotobaError
    pub fn io_to_kotoba_error(err: std::io::Error) -> KotobaError {
        KotobaError::Io(err.to_string())
    }

    /// Convert serde_json::Error to KotobaError
    pub fn json_to_kotoba_error(err: serde_json::Error) -> KotobaError {
        KotobaError::Serialization(err.to_string())
    }

    /// Create a boxed error from KotobaError
    pub fn box_error(err: KotobaError) -> Box<dyn std::error::Error + Send + Sync> {
        Box::new(err)
    }

    /// Check if an error is retryable
    pub fn is_retryable(err: &KotobaError) -> bool {
        err.is_retryable()
    }

    /// Get error category
    pub fn error_category(err: &KotobaError) -> ErrorCategory {
        err.category()
    }

    /// Get HTTP status code for error
    pub fn error_status_code(err: &KotobaError) -> u16 {
        err.http_status_code()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = KotobaError::validation("Invalid input");
        assert!(matches!(err, KotobaError::Validation(_)));
        assert_eq!(err.category(), ErrorCategory::Validation);
    }

    #[test]
    fn test_error_display() {
        let err = KotobaError::not_found("User not found");
        assert_eq!(err.to_string(), "Not found: User not found");
    }

    #[test]
    fn test_error_retryable() {
        assert!(KotobaError::Network("Connection failed".to_string()).is_retryable());
        assert!(!KotobaError::Validation("Invalid input".to_string()).is_retryable());
    }

    #[test]
    fn test_error_status_codes() {
        assert_eq!(KotobaError::Validation("Invalid".to_string()).http_status_code(), 400);
        assert_eq!(KotobaError::NotFound("Missing".to_string()).http_status_code(), 404);
        assert_eq!(KotobaError::Internal("Server error".to_string()).http_status_code(), 500);
    }

    #[test]
    fn test_error_builder() {
        let err = ErrorBuilder::new(KotobaError::validation("Invalid input"))
            .with_context("field".to_string(), "email".to_string())
            .with_request_id("req-123".to_string())
            .build();

        assert!(matches!(err, KotobaError::Validation(_)));
    }

    #[test]
    fn test_error_context() {
        let context = ErrorContext::new("VALIDATION_ERROR".to_string(), "Invalid input".to_string())
            .with_context("field".to_string(), "email".to_string())
            .with_request_id("req-123".to_string());

        assert_eq!(context.code, "VALIDATION_ERROR");
        assert_eq!(context.message, "Invalid input");
        assert_eq!(context.context.get("field"), Some(&"email".to_string()));
        assert_eq!(context.request_id, Some("req-123".to_string()));
    }
}

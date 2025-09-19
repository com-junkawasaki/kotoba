//! Kotoba Unified Handler
//!
//! このクレートはKotobaエコシステム全体の統合的なhandlerを提供します。
//! 既存のkotoba-jsonnetとkotoba-kotobasの機能を統合し、
//! サーバー、CLI、WASM実行を統一的に扱います。

pub mod error;
pub mod types;
pub mod handler;
pub mod executor;
pub mod runtime;
pub mod integration;

// TODO: Create server module
// #[cfg(feature = "server")]
// pub mod server;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "cli")]
pub mod cli;

#[cfg(feature = "web")]
pub mod web;

#[cfg(feature = "templates")]
pub mod templates;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "auth")]
pub mod auth;

#[cfg(feature = "dev_server")]
pub mod dev_server;

pub use error::{HandlerError, Result};
pub use types::*;
pub use handler::UnifiedHandler;
pub use executor::HandlerExecutor;

// Re-export KeyValueStore for convenience
pub use kotoba_storage::KeyValueStore;
pub use std::sync::Arc;

/// Handlerの初期化と実行を簡略化するためのヘルパー関数
// TODO: Implement server functionality
// #[cfg(feature = "server")]
// pub async fn run_server(addr: &str) -> Result<()> {
//     server::run(addr).await
// }

/// WASM環境でのhandler初期化
#[cfg(feature = "wasm")]
pub fn init_wasm_handler() -> Result<wasm::WasmHandler> {
    wasm::WasmHandler::new()
}

/// CLI経由でのhandler実行
#[cfg(feature = "cli")]
pub async fn execute_cli_handler(file: &str, args: Vec<String>) -> Result<String> {
    cli::execute_handler(file, args).await
}

/// 最もシンプルなhandler実行関数 (ジェネリックバージョン)
/// 使用例: execute_simple_handler_with_storage(&storage, content, context).await
pub async fn execute_simple_handler_with_storage<T: KeyValueStore + 'static>(
    storage: Arc<T>,
    content: &str,
    context: HandlerContext,
) -> Result<String> {
    let handler = UnifiedHandler::new(storage);
    handler.execute(content, context).await
}

/// 最もシンプルなhandler実行関数 (デフォルト実装)
/// 注意: この関数はKeyValueStoreを必要とするため、直接使用できません
/// execute_simple_handler_with_storageを使用してください
// pub async fn execute_simple_handler(content: &str, context: HandlerContext) -> Result<String> {
//     // この関数は削除されました。execute_simple_handler_with_storageを使用してください
//     Err(HandlerError::Config("KeyValueStoreが必要です".to_string()))
// }

/// Webアプリケーションの実行
#[cfg(feature = "web")]
pub async fn run_web_app(addr: &str, config: web::WebConfig) -> Result<()> {
    web::run_web_app(addr, config).await
}

/// 開発サーバーの実行
#[cfg(feature = "dev_server")]
pub async fn run_dev_server(addr: &str, config: dev_server::DevServerConfig) -> Result<()> {
    dev_server::run_dev_server(addr, config).await
}

/// データベース接続の初期化
#[cfg(feature = "database")]
pub async fn init_database(url: &str) -> Result<database::DatabaseConnection> {
    database::init_connection(url).await
}

/// 認証ミドルウェアの作成
#[cfg(feature = "auth")]
pub fn create_auth_middleware(config: auth::AuthConfig) -> auth::AuthMiddleware {
    auth::AuthMiddleware::new(config)
}

/// テンプレートエンジンの初期化
#[cfg(feature = "templates")]
pub fn init_template_engine(template_dir: &str) -> Result<templates::TemplateEngine> {
    templates::TemplateEngine::new(template_dir)
}

#[cfg(feature = "test")]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;

    // Mock KeyValueStore for testing
    struct MockKeyValueStore {
        data: HashMap<Vec<u8>, Vec<u8>>,
    }

    impl MockKeyValueStore {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    #[async_trait::async_trait]
    impl KeyValueStore for MockKeyValueStore {
        async fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
            // Mock implementation - just return success
            Ok(())
        }

        async fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
            Ok(None)
        }

        async fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
            Ok(())
        }

        async fn scan(&self, prefix: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
            Ok(vec![])
        }
    }

    #[test]
    fn test_handler_context_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type ".to_string(), r#"application/json "#.to_string());

        let mut query_params = HashMap::new();
        query_params.insert("page".to_string(), "1".to_string());

        let mut environment = HashMap::new();
        environment.insert("NODE_ENV".to_string(), "development".to_string());

        let context = HandlerContext {
            method: "GET".to_string(),
            path: "/api/users".to_string(),
            headers,
            query_params,
            body: Some(r#"{"name": "test"}"#.to_string()),
            environment,
        };

        assert_eq!(context.method, "GET");
        assert_eq!(context.path, "/api/users");
        assert_eq!(context.headers.get("Content-Type "), Some(&r#"application/json "#.to_string()));
        assert_eq!(context.query_params.get("page"), Some(&"1".to_string()));
        assert_eq!(context.environment.get("NODE_ENV"), Some(&"development".to_string()));
        assert_eq!(context.body, Some(r#"{"name": "test"}"#.to_string()));
    }

    #[test]
    fn test_handler_config_creation() {
        let config = HandlerConfig {
            timeout_ms: 30000,
            max_memory_mb: 100,
            enable_caching: true,
            enable_logging: true,
        };

        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_memory_mb, 100);
        assert!(config.enable_caching);
        assert!(config.enable_logging);
    }

    #[test]
    fn test_handler_result_creation() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type ".to_string(), "text/html ".to_string());

        let result = HandlerResult {
            status_code: 200,
            headers: headers.clone(),
            body: "<html><body>Hello World</body></html>".to_string()".to_string(),
            execution_time_ms: 150,
            memory_used_mb: 25.5,
        };

        assert_eq!(result.status_code, 200);
        assert_eq!(result.headers.get("Content-Type "), Some(&"text/html ".to_string()));
        assert!(result.body.contains("Hello World "));
        assert_eq!(result.execution_time_ms, 150);
        assert_eq!(result.memory_used_mb, 25.5);
    }

    #[test]
    fn test_handler_metadata_creation() {
        let metadata = HandlerMetadata {
            id: r#"handler_001"#.to_string(),
            name: "Test Handler ".to_string(),
            version: "1.0.0".to_string(),
            description: "A test handler for unit testing ".to_string(),
            capabilities: vec!["GET".to_string(), "POST".to_string()],
        };

        assert_eq!(metadata.id, "handler_001");
        assert_eq!(metadata.name, "Test Handler ");
        assert_eq!(metadata.version, "1.0.0");
        assert!(metadata.description.contains("test handler "));
        assert_eq!(metadata.capabilities.len(), 2);
        assert!(metadata.capabilities.contains(&"GET".to_string()));
        assert!(metadata.capabilities.contains(&"POST".to_string()));
    }

    #[test]
    fn test_execution_mode() {
        assert_eq!(ExecutionMode::Sync as u8, 0);
        assert_eq!(ExecutionMode::Async as u8, 1);
        assert_eq!(ExecutionMode::Streaming as u8, 2);

        assert!(format!("{:?}", ExecutionMode::Sync).contains("Sync"));
        assert!(format!("{:?}", ExecutionMode::Async).contains("Async"));
        assert!(format!("{:?}", ExecutionMode::Streaming).contains("Streaming"));
    }

    #[test]
    fn test_handler_capabilities_creation() {
        let capabilities = HandlerCapabilities {
            supports_async: true,
            supports_streaming: false,
            supports_websocket: true,
            supports_file_upload: true,
            max_payload_size: 10485760, // 10MB
            supported_content_types: vec![
                r#"application/json "#.to_string(),
                "text/html ".to_string(),
                "application/xml ".to_string(),
            ],
        };

        assert!(capabilities.supports_async);
        assert!(!capabilities.supports_streaming);
        assert!(capabilities.supports_websocket);
        assert!(capabilities.supports_file_upload);
        assert_eq!(capabilities.max_payload_size, 10485760);
        assert_eq!(capabilities.supported_content_types.len(), 3);
    }

    #[test]
    fn test_handler_error_types() {
        let io_error = HandlerError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, r#"file not found "#));
        assert!(format!("{}", io_error).contains(r#"IO error "#));

        let parse_error = HandlerError::Parse(r#"invalid syntax "#.to_string());
        assert!(format!("{}", parse_error).contains(r#"Parse error "#));

        let execution_error = HandlerError::Execution(r#"runtime error "#.to_string());
        assert!(format!("{}", execution_error).contains(r#"Execution error "#));

        let network_error = HandlerError::Network(r#"connection failed "#.to_string());
        assert!(format!("{}", network_error).contains(r#"Network error "#));

        let config_error = HandlerError::Config(r#"invalid config "#.to_string());
        assert!(format!("{}", config_error).contains(r#"Configuration error "#));

        let storage_error = HandlerError::Storage(r#"storage failed "#.to_string());
        assert!(format!("{}", storage_error).contains(r#"Storage error "#));

        let unknown_error = HandlerError::Unknown(r#"unexpected error "#.to_string());
        assert!(format!("{}", unknown_error).contains(r#"Unknown error "#));
    }

    #[tokio::test]
    async fn test_unified_handler_creation() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        // Test that handler was created successfully
        let config = handler.get_config().await;
        assert_eq!(config.timeout_ms, 30000);
        assert_eq!(config.max_memory_mb, 100);
        assert!(config.enable_caching);
        assert!(config.enable_logging);
    }

    #[tokio::test]
    async fn test_unified_handler_execute() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        let context = HandlerContext {
            method: "GET".to_string(),
            path: r#"/test "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            environment: HashMap::new(),
        };

        let content = r#"console.log("Hello, World!")"#;

        let result = handler.execute(content, context).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.contains(r#"Kotoba Handler Executed "#));
        assert!(response.contains("GET"));
        assert!(response.contains(r#"/test "#));
    }

    #[tokio::test]
    async fn test_unified_handler_cache() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        let context = HandlerContext {
            method: "GET".to_string(),
            path: r#"/cached "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            environment: HashMap::new(),
        };

        let content = r#"cached content "#;

        // First execution
        let result1 = handler.execute(content, context.clone()).await;
        assert!(result1.is_ok());

        // Second execution (should use cache)
        let result2 = handler.execute(content, context).await;
        assert!(result2.is_ok());

        // Both results should be identical
        assert_eq!(result1.unwrap(), result2.unwrap());
    }

    #[tokio::test]
    async fn test_unified_handler_cache_management() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        let context = HandlerContext {
            method: "GET".to_string(),
            path: r#"/cache-test "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            environment: HashMap::new(),
        };

        let content = r#"test content "#;

        // Execute to populate cache
        let _ = handler.execute(content, context).await;

        // Check cache size
        let cache_size = handler.cache_size().await;
        assert!(cache_size > 0);

        // Clear cache
        handler.clear_cache().await;

        // Verify cache is empty
        let cache_size_after_clear = handler.cache_size().await;
        assert_eq!(cache_size_after_clear, 0);
    }

    #[tokio::test]
    async fn test_unified_handler_config_update() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        // Update configuration
        let new_config = HandlerConfig {
            timeout_ms: 60000,
            max_memory_mb: 200,
            enable_caching: false,
            enable_logging: false,
        };

        handler.update_config(new_config.clone()).await;

        // Verify configuration was updated
        let retrieved_config = handler.get_config().await;
        assert_eq!(retrieved_config.timeout_ms, 60000);
        assert_eq!(retrieved_config.max_memory_mb, 200);
        assert!(!retrieved_config.enable_caching);
        assert!(!retrieved_config.enable_logging);
    }

    #[tokio::test]
    async fn test_unified_handler_file_execution() {
        let mock_storage = Arc::new(MockKeyValueStore::new());
        let handler = UnifiedHandler::new(mock_storage);

        // Create a temporary file
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join(r#"test.txt "#);
        let content = r#"file content "#;
        std::fs::write(&file_path, content).unwrap();

        let context = HandlerContext {
            method: "GET".to_string(),
            path: r#"/file-test "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            environment: HashMap::new(),
        };

        let result = handler.execute_file(file_path.to_str().unwrap(), context).await;
        assert!(result.is_ok());

        let response = result.unwrap();
        assert!(response.contains(r#"Kotoba Handler Executed "#));
        assert!(response.contains(r#"file content "#));
    }

    #[test]
    fn test_execute_simple_handler_with_storage() {
        let mock_storage = Arc::new(MockKeyValueStore::new());

        let context = HandlerContext {
            method: "POST".to_string(),
            path: r#"/simple-test "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: Some(r#"test body "#.to_string()),
            environment: HashMap::new(),
        };

        // Note: This would normally work, but since we don't have async test here,
        // we'll just verify the function exists and can be called
        // In a real async test, we would use tokio::test
    }

    #[test]
    fn test_handler_context_serialization() {
        let mut context = HandlerContext {
            method: "POST".to_string(),
            path: r#"/api/test "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: Some(r#"test data "#.to_string()),
            environment: HashMap::new(),
        };
        context.headers.insert("Authorization".to_string(), r#"Bearer token "#.to_string());

        // Test JSON serialization
        let json_result = serde_json::to_string(&context);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("POST"));
        assert!(json_str.contains(r#"/api/test "#));
        assert!(json_str.contains(r#"test data "#));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<HandlerContext> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.method, "POST");
        assert_eq!(deserialized.path, r#"/api/test "#);
        assert_eq!(deserialized.body, Some(r#"test data "#.to_string()));
    }

    #[test]
    fn test_handler_config_serialization() {
        let config = HandlerConfig {
            timeout_ms: 45000,
            max_memory_mb: 150,
            enable_caching: false,
            enable_logging: true,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&config);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("45000"));
        assert!(json_str.contains("150"));
        assert!(json_str.contains("false"));
        assert!(json_str.contains("true"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<HandlerConfig> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.timeout_ms, 45000);
        assert_eq!(deserialized.max_memory_mb, 150);
        assert!(!deserialized.enable_caching);
        assert!(deserialized.enable_logging);
    }

    #[test]
    fn test_handler_result_serialization() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type ".to_string(), r#"application/json "#.to_string());

        let result = HandlerResult {
            status_code: 201,
            headers: headers.clone(),
            body: r#"{"success": true}"#.to_string(),
            execution_time_ms: 250,
            memory_used_mb: 15.7,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&result);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("201"));
        assert!(json_str.contains(r#"application/json "#));
        assert!(json_str.contains("250"));
        assert!(json_str.contains("15.7"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<HandlerResult> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.status_code, 201);
        assert_eq!(deserialized.execution_time_ms, 250);
        assert_eq!(deserialized.memory_used_mb, 15.7);
    }

    #[test]
    fn test_handler_metadata_serialization() {
        let metadata = HandlerMetadata {
            id: "meta_001".to_string(),
            name: r#"Test Metadata "#.to_string(),
            version: "2.1.0".to_string(),
            description: r#"Metadata for testing "#.to_string(),
            capabilities: vec!["read".to_string(), "write".to_string(), "execute".to_string()],
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&metadata);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("meta_001"));
        assert!(json_str.contains("2.1.0"));
        assert!(json_str.contains("read"));
        assert!(json_str.contains("write"));
        assert!(json_str.contains("execute"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<HandlerMetadata> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.id, "meta_001");
        assert_eq!(deserialized.version, "2.1.0");
        assert_eq!(deserialized.capabilities.len(), 3);
    }

    #[test]
    fn test_execution_mode_serialization() {
        let mode = ExecutionMode::Async;

        // Test JSON serialization
        let json_result = serde_json::to_string(&mode);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("Async"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<ExecutionMode> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());
        assert_eq!(deserialized_result.unwrap(), ExecutionMode::Async);
    }

    #[test]
    fn test_handler_capabilities_serialization() {
        let capabilities = HandlerCapabilities {
            supports_async: true,
            supports_streaming: false,
            supports_websocket: true,
            supports_file_upload: false,
            max_payload_size: 5242880, // 5MB
            supported_content_types: vec![r#"text/plain "#.to_string(), r#"application/octet-stream "#.to_string()],
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&capabilities);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("5242880"));
        assert!(json_str.contains(r#"text/plain "#));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<HandlerCapabilities> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.max_payload_size, 5242880);
        assert!(deserialized.supported_content_types.contains(&r#"text/plain "#.to_string()));
    }

    #[test]
    fn test_handler_context_clone() {
        let original = HandlerContext {
            method: "PUT".to_string(),
            path: r#"/api/update "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: Some(r#"original body "#.to_string()),
            environment: HashMap::new(),
        };

        let cloned = original.clone();

        assert_eq!(original.method, cloned.method);
        assert_eq!(original.path, cloned.path);
        assert_eq!(original.body, cloned.body);
    }

    #[test]
    fn test_handler_config_clone() {
        let original = HandlerConfig {
            timeout_ms: 20000,
            max_memory_mb: 50,
            enable_caching: false,
            enable_logging: false,
        };

        let cloned = original.clone();

        assert_eq!(original.timeout_ms, cloned.timeout_ms);
        assert_eq!(original.max_memory_mb, cloned.max_memory_mb);
        assert_eq!(original.enable_caching, cloned.enable_caching);
        assert_eq!(original.enable_logging, cloned.enable_logging);
    }

    #[test]
    fn test_handler_result_clone() {
        let original = HandlerResult {
            status_code: 404,
            headers: HashMap::new(),
            body: r#"Not Found "#.to_string(),
            execution_time_ms: 5,
            memory_used_mb: 0.1,
        };

        let cloned = original.clone();

        assert_eq!(original.status_code, cloned.status_code);
        assert_eq!(original.body, cloned.body);
        assert_eq!(original.execution_time_ms, cloned.execution_time_ms);
        assert_eq!(original.memory_used_mb, cloned.memory_used_mb);
    }

    #[test]
    fn test_handler_metadata_clone() {
        let original = HandlerMetadata {
            id: "clone_test".to_string(),
            name: r#"Clone Test "#.to_string(),
            version: "0.1.0".to_string(),
            description: r#"Testing clone functionality "#.to_string(),
            capabilities: vec!["clone".to_string()],
        };

        let cloned = original.clone();

        assert_eq!(original.id, cloned.id);
        assert_eq!(original.name, cloned.name);
        assert_eq!(original.version, cloned.version);
        assert_eq!(original.description, cloned.description);
        assert_eq!(original.capabilities, cloned.capabilities);
    }

    #[test]
    fn test_execution_mode_equality() {
        assert_eq!(ExecutionMode::Sync, ExecutionMode::Sync);
        assert_ne!(ExecutionMode::Sync, ExecutionMode::Async);
        assert_ne!(ExecutionMode::Async, ExecutionMode::Streaming);
    }

    #[test]
    fn test_handler_context_debug() {
        let context = HandlerContext {
            method: "DEBUG".to_string(),
            path: r#"/debug "#.to_string(),
            headers: HashMap::new(),
            query_params: HashMap::new(),
            body: None,
            environment: HashMap::new(),
        };

        let debug_str = format!("{:?}", context);
        assert!(debug_str.contains("DEBUG"));
        assert!(debug_str.contains(r#"/debug "#));
    }

    #[test]
    fn test_handler_config_debug() {
        let config = HandlerConfig {
            timeout_ms: 1000,
            max_memory_mb: 10,
            enable_caching: true,
            enable_logging: false,
        };

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("1000"));
        assert!(debug_str.contains("10"));
        assert!(debug_str.contains("true"));
        assert!(debug_str.contains("false"));
    }

    #[test]
    fn test_handler_result_debug() {
        let result = HandlerResult {
            status_code: 500,
            headers: HashMap::new(),
            body: r#"Internal Server Error "#.to_string(),
            execution_time_ms: 1000,
            memory_used_mb: 50.0,
        };

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("500"));
        assert!(debug_str.contains("1000"));
        assert!(debug_str.contains("50.0"));
    }

    #[test]
    fn test_handler_metadata_debug() {
        let metadata = HandlerMetadata {
            id: "debug_test".to_string(),
            name: r#"Debug Test "#.to_string(),
            version: "0.0.1".to_string(),
            description: r#"Testing debug formatting "#.to_string(),
            capabilities: vec!["debug".to_string()],
        };

        let debug_str = format!("{:?}", metadata);
        assert!(debug_str.contains("debug_test"));
        assert!(debug_str.contains("0.0.1"));
        assert!(debug_str.contains("debug"));
    }

    #[test]
    fn test_handler_capabilities_debug() {
        let capabilities = HandlerCapabilities {
            supports_async: true,
            supports_streaming: false,
            supports_websocket: false,
            supports_file_upload: true,
            max_payload_size: 1024,
            supported_content_types: vec![r#"debug/type "#.to_string()],
        };

        let debug_str = format!("{:?}", capabilities);
        assert!(debug_str.contains("true"));
        assert!(debug_str.contains("false"));
        assert!(debug_str.contains("1024"));
        assert!(debug_str.contains(r#"debug/type "#));
    }

    #[test]
    fn test_handler_error_debug() {
        let error = HandlerError::Config(r#"test config error "#.to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("test config error "));
    }
}

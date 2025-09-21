//! Kotoba Language Server Protocol (LSP) Implementation
//!
//! Provides LSP support for Kotoba language with syntax highlighting,
//! error diagnostics, and language features.

use dashmap::DashMap;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use std::sync::Arc;
use tower_lsp::ClientSocket;

/// Backend implementation for the Kotoba LSP server
#[derive(Debug)]
pub struct Backend {
    client: Client,
    document_map: DashMap<String, String>,
}

impl Backend {
    /// Create a new LSP backend with the given client
    pub fn new(client: Client) -> Self {
        Self {
            client,
            document_map: DashMap::new(),
        }
    }

    /// Handle document changes and publish diagnostics
    pub async fn on_change(&self, params: TextDocumentItem) {
        let text = params.text.clone();
        self.document_map
            .insert(params.uri.to_string(), params.text);

        let diagnostics = match kotoba_jsonnet::parser::Parser::new().parse(&text) {
            Ok(_) => vec![],
            Err(e) => {
                let (line, column, message) = match e {
                    kotoba_jsonnet::error::JsonnetError::ParseError {
                        line,
                        column,
                        message,
                    } => (line, column, message),
                    _ => (1, 1, e.to_string()),
                };

                let range = Range {
                    start: Position {
                        line: (line - 1) as u32,
                        character: (column - 1) as u32,
                    },
                    end: Position {
                        line: (line - 1) as u32,
                        character: (column - 1) as u32,
                    },
                };

                vec![Diagnostic::new_simple(range, message)]
            }
        };

        self.client
            .publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version))
            .await;
    }

    /// Get a document from the document map
    pub fn get_document(&self, uri: &str) -> Option<String> {
        self.document_map.get(uri).map(|v| v.clone())
    }

    /// Check if a document exists in the map
    pub fn has_document(&self, uri: &str) -> bool {
        self.document_map.contains_key(uri)
    }

    /// Get the number of documents in the map
    pub fn document_count(&self) -> usize {
        self.document_map.len()
    }

    /// Clear all documents from the map
    pub fn clear_documents(&self) {
        self.document_map.clear();
    }
}

/// Text document item for internal representation
#[derive(Debug, Clone)]
pub struct TextDocumentItem {
    pub uri: Url,
    pub text: String,
    pub version: i32,
}

impl TextDocumentItem {
    pub fn new(uri: Url, text: String, version: i32) -> Self {
        Self { uri, text, version }
    }
}

/// LSP service builder for creating LSP services
pub struct LspServiceBuilder;

impl LspServiceBuilder {
    /// Create a new LSP service with the given client
    pub fn new<F>(client_fn: F) -> tower_lsp::LspService<Backend>
    where
        F: FnOnce(Client) -> Backend,
    {
        let (service, _socket) = tower_lsp::LspService::new(client_fn);
        service
    }

    /// Build the LSP server with stdin/stdout
    pub async fn build_server(
        stdin: impl tokio::io::AsyncRead + Send + Unpin + 'static,
        stdout: impl tokio::io::AsyncWrite + Send + Unpin + 'static,
        _socket: tower_lsp::LspService<Backend>,
    ) -> tower_lsp::jsonrpc::Result<()> {
        let (service, socket) = tower_lsp::LspService::new(|client| Backend::new(client));
        tower_lsp::Server::new(stdin, stdout, socket).serve(service).await;
        Ok(())
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "kotoba-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file changed!")
            .await;
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
        })
        .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
        self.document_map.remove(params.text_document.uri.as_str());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::*;
    use std::sync::Arc;

    // Mock client for testing
    struct MockClient;

    impl MockClient {
        fn new() -> Self {
            Self
        }
    }

    #[async_trait::async_trait]
    impl tower_lsp::Client for MockClient {
        async fn log_message(&self, _level: MessageType, _message: impl std::fmt::Display) {
            // Mock implementation - do nothing
        }

        async fn publish_diagnostics(&self, _uri: Url, _diagnostics: Vec<Diagnostic>, _version: Option<i32>) {
            // Mock implementation - do nothing
        }

        async fn show_message(&self, _typ: MessageType, _message: impl std::fmt::Display) {
            // Mock implementation - do nothing
        }

        async fn register_capability(&self, _registrations: Vec<Registration>) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn unregister_capability(&self, _unregistrations: Vec<UnregisteredCapability>) -> tower_lsp::jsonrpc::Result<()> {
            Ok(())
        }

        async fn workspace_configuration(&self, _items: Vec<ConfigurationItem>) -> tower_lsp::jsonrpc::Result<Vec<serde_json::Value>> {
            Ok(vec![])
        }

        async fn apply_edit(&self, _edit: WorkspaceEdit) -> tower_lsp::jsonrpc::Result<ApplyWorkspaceEditResponse> {
            Ok(ApplyWorkspaceEditResponse { applied: true, failure_reason: None, failed_change: None })
        }
    }

    #[test]
    fn test_backend_creation() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        // Should start with empty document map
        assert_eq!(backend.document_count(), 0);
        assert!(!backend.has_document("test://example.com"));
    }

    #[tokio::test]
    async fn test_backend_initialize() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let params = InitializeParams::default();
        let result = backend.initialize(params).await;

        assert!(result.is_ok());
        let init_result = result.unwrap();

        assert_eq!(init_result.server_info.as_ref().unwrap().name, "kotoba-lsp");
        assert!(init_result.server_info.as_ref().unwrap().version.is_some());

        // Check capabilities
        match init_result.capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => {
                assert_eq!(kind, TextDocumentSyncKind::FULL);
            }
            _ => panic!("Expected TextDocumentSyncKind::FULL"),
        }
    }

    #[tokio::test]
    async fn test_backend_initialized() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let params = InitializedParams {};
        // This should not panic
        backend.initialized(params).await;
    }

    #[tokio::test]
    async fn test_backend_shutdown() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let result = backend.shutdown().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_backend_did_open() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "jsonnet".to_string(),
                version: 1,
                text: "local x = 1;".to_string(),
            },
        };

        // This should not panic
        backend.did_open(params).await;

        // Document should be stored
        assert!(backend.has_document(uri.as_str()));
        assert_eq!(backend.document_count(), 1);
    }

    #[tokio::test]
    async fn test_backend_did_close() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();

        // First add a document
        backend.document_map.insert(uri.to_string(), "test content".to_string());
        assert!(backend.has_document(uri.as_str()));

        let params = DidCloseTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
        };

        // This should not panic and should remove the document
        backend.did_close(params).await;
        assert!(!backend.has_document("file:///tmp/test.jsonnet"));
    }

    #[test]
    fn test_text_document_item_creation() {
        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let text = "local x = 1;".to_string();
        let version = 1;

        let item = TextDocumentItem::new(uri.clone(), text.clone(), version);

        assert_eq!(item.uri, uri);
        assert_eq!(item.text, text);
        assert_eq!(item.version, version);
    }

    #[test]
    fn test_text_document_item_debug() {
        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let item = TextDocumentItem::new(uri, "test".to_string(), 1);

        let debug_str = format!("{:?}", item);
        assert!(debug_str.contains("TextDocumentItem"));
    }

    #[test]
    fn test_text_document_item_clone() {
        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let original = TextDocumentItem::new(uri, "test".to_string(), 1);
        let cloned = original.clone();

        assert_eq!(original.uri, cloned.uri);
        assert_eq!(original.text, cloned.text);
        assert_eq!(original.version, cloned.version);
    }

    #[test]
    fn test_backend_get_document() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = "file:///tmp/test.jsonnet";
        let content = "local x = 1;";

        // Initially should return None
        assert!(backend.get_document(uri).is_none());

        // Add document
        backend.document_map.insert(uri.to_string(), content.to_string());

        // Now should return the content
        let result = backend.get_document(uri);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), content);
    }

    #[test]
    fn test_backend_has_document() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = "file:///tmp/test.jsonnet";

        // Initially should return false
        assert!(!backend.has_document(uri));

        // Add document
        backend.document_map.insert(uri.to_string(), "content".to_string());

        // Now should return true
        assert!(backend.has_document(uri));
    }

    #[test]
    fn test_backend_document_count() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        // Initially should be 0
        assert_eq!(backend.document_count(), 0);

        // Add documents
        backend.document_map.insert("file:///tmp/test1.jsonnet".to_string(), "content1".to_string());
        backend.document_map.insert("file:///tmp/test2.jsonnet".to_string(), "content2".to_string());

        // Should be 2
        assert_eq!(backend.document_count(), 2);
    }

    #[test]
    fn test_backend_clear_documents() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        // Add documents
        backend.document_map.insert("file:///tmp/test1.jsonnet".to_string(), "content1".to_string());
        backend.document_map.insert("file:///tmp/test2.jsonnet".to_string(), "content2".to_string());
        assert_eq!(backend.document_count(), 2);

        // Clear documents
        backend.clear_documents();

        // Should be empty
        assert_eq!(backend.document_count(), 0);
    }

    #[tokio::test]
    async fn test_backend_on_change() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let text = "local x = 1;";
        let version = 1;

        let item = TextDocumentItem {
            uri: uri.clone(),
            text: text.to_string(),
            version,
        };

        // This should not panic
        backend.on_change(item).await;

        // Document should be stored
        assert!(backend.has_document(uri.as_str()));
        let stored_content = backend.get_document(uri.as_str()).unwrap();
        assert_eq!(stored_content, text);
    }

    #[test]
    fn test_backend_debug() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let debug_str = format!("{:?}", backend);
        assert!(debug_str.contains("Backend"));
    }

    #[test]
    fn test_server_info_creation() {
        let server_info = ServerInfo {
            name: "test-lsp".to_string(),
            version: Some("1.0.0".to_string()),
        };

        assert_eq!(server_info.name, "test-lsp");
        assert_eq!(server_info.version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_server_capabilities_creation() {
        let capabilities = ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::FULL,
            )),
            ..ServerCapabilities::default()
        };

        match capabilities.text_document_sync {
            Some(TextDocumentSyncCapability::Kind(kind)) => {
                assert_eq!(kind, TextDocumentSyncKind::FULL);
            }
            _ => panic!("Expected TextDocumentSyncKind::FULL"),
        }
    }

    #[test]
    fn test_diagnostic_creation() {
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 5 },
        };

        let diagnostic = Diagnostic::new_simple(range, "Test error".to_string());

        assert_eq!(diagnostic.message, "Test error");
        assert_eq!(diagnostic.range.start.line, 0);
        assert_eq!(diagnostic.range.start.character, 0);
        assert_eq!(diagnostic.range.end.line, 0);
        assert_eq!(diagnostic.range.end.character, 5);
    }

    #[test]
    fn test_position_creation() {
        let position = Position { line: 10, character: 20 };

        assert_eq!(position.line, 10);
        assert_eq!(position.character, 20);
    }

    #[test]
    fn test_range_creation() {
        let start = Position { line: 1, character: 5 };
        let end = Position { line: 1, character: 10 };

        let range = Range { start, end };

        assert_eq!(range.start.line, 1);
        assert_eq!(range.start.character, 5);
        assert_eq!(range.end.line, 1);
        assert_eq!(range.end.character, 10);
    }

    #[test]
    fn test_url_parsing() {
        let url_result = Url::parse("file:///tmp/test.jsonnet");
        assert!(url_result.is_ok());

        let url = url_result.unwrap();
        assert_eq!(url.scheme(), "file");
        assert_eq!(url.path(), "/tmp/test.jsonnet");
    }

    #[test]
    fn test_text_document_identifier_creation() {
        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let identifier = TextDocumentIdentifier { uri: uri.clone() };

        assert_eq!(identifier.uri, uri);
    }

    #[tokio::test]
    async fn test_backend_did_change() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: Some(2),
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "local y = 2;".to_string(),
            }],
        };

        // This should not panic
        backend.did_change(params).await;

        // Document should be stored
        assert!(backend.has_document(uri.as_str()));
    }

    #[test]
    fn test_versioned_text_document_identifier_creation() {
        let uri = Url::parse("file:///tmp/test.jsonnet").unwrap();
        let identifier = VersionedTextDocumentIdentifier {
            uri: uri.clone(),
            version: Some(1),
        };

        assert_eq!(identifier.uri, uri);
        assert_eq!(identifier.version, Some(1));
    }

    #[test]
    fn test_text_document_content_change_event_creation() {
        let change = TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "new content".to_string(),
        };

        assert_eq!(change.text, "new content");
        assert!(change.range.is_none());
        assert!(change.range_length.is_none());
    }

    #[tokio::test]
    async fn test_multiple_document_operations() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        // Add multiple documents
        let uri1 = Url::parse("file:///tmp/test1.jsonnet").unwrap();
        let uri2 = Url::parse("file:///tmp/test2.jsonnet").unwrap();

        // Simulate opening documents
        backend.document_map.insert(uri1.to_string(), "content1".to_string());
        backend.document_map.insert(uri2.to_string(), "content2".to_string());

        assert_eq!(backend.document_count(), 2);
        assert!(backend.has_document(uri1.as_str()));
        assert!(backend.has_document(uri2.as_str()));

        // Remove one document
        backend.document_map.remove(uri1.as_str());

        assert_eq!(backend.document_count(), 1);
        assert!(!backend.has_document(uri1.as_str()));
        assert!(backend.has_document(uri2.as_str()));

        // Clear all
        backend.clear_documents();
        assert_eq!(backend.document_count(), 0);
    }

    #[test]
    fn test_backend_concurrent_access() {
        let mock_client = MockClient::new();
        let backend = Backend::new(mock_client);

        // Test that DashMap allows concurrent access
        let backend_arc = Arc::new(backend);

        let backend_clone = Arc::clone(&backend_arc);
        std::thread::spawn(move || {
            backend_clone.document_map.insert("test".to_string(), "value".to_string());
        });

        // Wait a bit for the thread to complete
        std::thread::sleep(std::time::Duration::from_millis(10));

        assert!(backend_arc.has_document("test"));
    }

    #[test]
    fn test_lsp_service_builder_creation() {
        // Test that we can create the service builder
        let _builder = LspServiceBuilder;
        // This is just a marker struct, so we just verify it exists
    }
}

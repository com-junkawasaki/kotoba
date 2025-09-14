//! HTTPサーバー用のIR定義
//!
//! このモジュールはHTTPサーバー関連のデータ構造とIR定義を提供します。

use crate::types::{Value, Properties, ContentHash, Result, KotobaError};
use crate::ir::catalog::{LabelDef, PropertyDef};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTPメソッド
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let method_str = match self {
            HttpMethod::GET => "GET",
            HttpMethod::POST => "POST",
            HttpMethod::PUT => "PUT",
            HttpMethod::DELETE => "DELETE",
            HttpMethod::PATCH => "PATCH",
            HttpMethod::HEAD => "HEAD",
            HttpMethod::OPTIONS => "OPTIONS",
            HttpMethod::CONNECT => "CONNECT",
            HttpMethod::TRACE => "TRACE",
        };
        write!(f, "{}", method_str)
    }
}

impl HttpMethod {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(HttpMethod::GET),
            "POST" => Ok(HttpMethod::POST),
            "PUT" => Ok(HttpMethod::PUT),
            "DELETE" => Ok(HttpMethod::DELETE),
            "PATCH" => Ok(HttpMethod::PATCH),
            "HEAD" => Ok(HttpMethod::HEAD),
            "OPTIONS" => Ok(HttpMethod::OPTIONS),
            "CONNECT" => Ok(HttpMethod::CONNECT),
            "TRACE" => Ok(HttpMethod::TRACE),
            _ => Err(KotobaError::InvalidArgument(format!("Invalid HTTP method: {}", s))),
        }
    }
}

/// HTTPステータスコード
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpStatus {
    pub code: u16,
    pub reason: String,
}

impl HttpStatus {
    pub fn new(code: u16, reason: String) -> Self {
        Self { code, reason }
    }

    pub fn ok() -> Self {
        Self::new(200, "OK".to_string())
    }

    pub fn not_found() -> Self {
        Self::new(404, "Not Found".to_string())
    }

    pub fn internal_server_error() -> Self {
        Self::new(500, "Internal Server Error".to_string())
    }
}

/// HTTPヘッダー
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpHeaders {
    pub headers: HashMap<String, String>,
}

impl HttpHeaders {
    pub fn new() -> Self {
        Self {
            headers: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.headers.get(&key.to_lowercase())
    }

    pub fn set(&mut self, key: String, value: String) {
        self.headers.insert(key.to_lowercase(), value);
    }

    pub fn remove(&mut self, key: &str) {
        self.headers.remove(&key.to_lowercase());
    }
}

impl Default for HttpHeaders {
    fn default() -> Self {
        Self::new()
    }
}

/// HTTPリクエストIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRequest {
    pub id: String,
    pub method: HttpMethod,
    pub path: String,
    pub query: HashMap<String, String>,
    pub headers: HttpHeaders,
    pub body_ref: Option<ContentHash>, // ボディは外部blobとして扱う
    pub timestamp: u64,
}

impl HttpRequest {
    pub fn new(
        id: String,
        method: HttpMethod,
        path: String,
        headers: HttpHeaders,
        body_ref: Option<ContentHash>,
    ) -> Self {
        Self {
            id,
            method,
            path,
            query: HashMap::new(),
            headers,
            body_ref,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// HTTPレスポンスIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpResponse {
    pub request_id: String,
    pub status: HttpStatus,
    pub headers: HttpHeaders,
    pub body_ref: Option<ContentHash>, // ボディは外部blobとして扱う
    pub timestamp: u64,
}

impl HttpResponse {
    pub fn new(request_id: String, status: HttpStatus, headers: HttpHeaders, body_ref: Option<ContentHash>) -> Self {
        Self {
            request_id,
            status,
            headers,
            body_ref,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }
}

/// ルート定義IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpRoute {
    pub id: String,
    pub method: HttpMethod,
    pub pattern: String, // パスパターン (例: "/api/users/{id}")
    pub handler_ref: ContentHash, // ハンドラー関数のハッシュ
    pub metadata: Properties,
}

impl HttpRoute {
    pub fn new(id: String, method: HttpMethod, pattern: String, handler_ref: ContentHash) -> Self {
        Self {
            id,
            method,
            pattern,
            handler_ref,
            metadata: Properties::new(),
        }
    }
}

/// ミドルウェア定義IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpMiddleware {
    pub id: String,
    pub name: String,
    pub order: i32, // 実行順序
    pub function_ref: ContentHash, // ミドルウェア関数のハッシュ
    pub metadata: Properties,
}

impl HttpMiddleware {
    pub fn new(id: String, name: String, order: i32, function_ref: ContentHash) -> Self {
        Self {
            id,
            name,
            order,
            function_ref,
            metadata: Properties::new(),
        }
    }
}

/// サーバー設定IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpConfig {
    pub server: ServerConfig,
    pub routes: Vec<HttpRoute>,
    pub middlewares: Vec<HttpMiddleware>,
    pub static_files: Option<StaticConfig>,
}

impl HttpConfig {
    pub fn new(server: ServerConfig) -> Self {
        Self {
            server,
            routes: Vec::new(),
            middlewares: Vec::new(),
            static_files: None,
        }
    }
}

/// サーバー設定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: Some(1000),
            timeout_ms: Some(30000),
            tls: None,
        }
    }
}

/// TLS設定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
}

/// 静的ファイル設定
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticConfig {
    pub root_dir: String,
    pub url_prefix: String,
    pub cache_max_age: Option<u32>,
}

/// HTTPカタログ（スキーマ定義）
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HttpCatalog {
    pub labels: Vec<LabelDef>,
    pub properties: Vec<PropertyDef>,
    pub invariants: Vec<String>,
}

impl Default for HttpCatalog {
    fn default() -> Self {
        Self {
            labels: vec![
                LabelDef {
                    name: "Request".to_string(),
                    properties: vec![
                    PropertyDef {
                        name: "id".to_string(),
                        type_: crate::ir::catalog::ValueType::String,
                        nullable: false,
                        default: None,
                    },
                    PropertyDef {
                        name: "method".to_string(),
                        type_: crate::ir::catalog::ValueType::String,
                        nullable: false,
                        default: None,
                    },
                    PropertyDef {
                        name: "path".to_string(),
                        type_: crate::ir::catalog::ValueType::String,
                        nullable: false,
                        default: None,
                    },
                    PropertyDef {
                        name: "timestamp".to_string(),
                        type_: crate::ir::catalog::ValueType::Int,
                        nullable: false,
                        default: None,
                    },
                    ],
                    super_labels: None,
                },
                LabelDef {
                    name: "Response".to_string(),
                    properties: vec![
                    PropertyDef {
                        name: "request_id".to_string(),
                        type_: crate::ir::catalog::ValueType::String,
                        nullable: false,
                        default: None,
                    },
                    PropertyDef {
                        name: "status_code".to_string(),
                        type_: crate::ir::catalog::ValueType::Int,
                        nullable: false,
                        default: None,
                    },
                        PropertyDef {
                            name: "timestamp".to_string(),
                            type_: crate::ir::catalog::ValueType::Int,
                            nullable: false,
                            default: None,
                        },
                    ],
                    super_labels: None,
                },
                LabelDef {
                    name: "Route".to_string(),
                    properties: vec![
                        PropertyDef {
                            name: "method".to_string(),
                            type_: crate::ir::catalog::ValueType::String,
                            nullable: false,
                            default: None,
                        },
                        PropertyDef {
                            name: "pattern".to_string(),
                            type_: crate::ir::catalog::ValueType::String,
                            nullable: false,
                            default: None,
                        },
                    ],
                    super_labels: None,
                },
                LabelDef {
                    name: "Middleware".to_string(),
                    properties: vec![
                        PropertyDef {
                            name: "name".to_string(),
                            type_: crate::ir::catalog::ValueType::String,
                            nullable: false,
                            default: None,
                        },
                        PropertyDef {
                            name: "order".to_string(),
                            type_: crate::ir::catalog::ValueType::Int,
                            nullable: false,
                            default: None,
                        },
                    ],
                    super_labels: None,
                },
            ],
            properties: vec![],
            invariants: vec![
                "Request.idはユニーク".to_string(),
                "Response.request_idは存在するRequest.idを参照".to_string(),
                "Route.methodは有効なHTTPメソッド".to_string(),
                "Middleware.orderは昇順で実行".to_string(),
            ],
        }
    }
}

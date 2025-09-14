//! APIルートIR定義
//!
//! REST API、GraphQL、WebSocketなどのAPIエンドポイントを表現します。

use crate::types::{Value, Properties, ContentHash, Result, KotobaError};
use crate::frontend::component_ir::ComponentIR;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// APIメソッド
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    TRACE,
    CONNECT,
}

impl std::fmt::Display for ApiMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiMethod::GET => write!(f, "GET"),
            ApiMethod::POST => write!(f, "POST"),
            ApiMethod::PUT => write!(f, "PUT"),
            ApiMethod::DELETE => write!(f, "DELETE"),
            ApiMethod::PATCH => write!(f, "PATCH"),
            ApiMethod::HEAD => write!(f, "HEAD"),
            ApiMethod::OPTIONS => write!(f, "OPTIONS"),
            ApiMethod::TRACE => write!(f, "TRACE"),
            ApiMethod::CONNECT => write!(f, "CONNECT"),
        }
    }
}

/// APIレスポンスフォーマット
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResponseFormat {
    JSON,
    XML,
    HTML,
    Text,
    Binary,
    GraphQL,
}

/// APIルートIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiRouteIR {
    pub path: String,
    pub method: ApiMethod,
    pub handler: ApiHandlerIR,
    pub middlewares: Vec<String>, // ミドルウェア名
    pub response_format: ResponseFormat,
    pub parameters: ApiParameters,
    pub metadata: ApiMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiHandlerIR {
    pub function_name: String,
    pub component: Option<ComponentIR>, // APIコンポーネント
    pub is_async: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiParameters {
    pub path_params: Vec<ApiParameter>,
    pub query_params: Vec<ApiParameter>,
    pub body_params: Option<ApiBodySchema>,
    pub headers: Vec<ApiParameter>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default_value: Option<Value>,
    pub validation: Option<ValidationRules>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    File,
    Date,
    DateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ValidationRules {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub allowed_values: Vec<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiBodySchema {
    pub content_type: String,
    pub schema: Value, // JSON Schema または類似の構造
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiMetadata {
    pub description: Option<String>,
    pub summary: Option<String>,
    pub tags: Vec<String>,
    pub deprecated: bool,
    pub rate_limit: Option<RateLimit>,
    pub cache: Option<CacheConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests: u32,
    pub window_seconds: u64,
    pub strategy: RateLimitStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitStrategy {
    FixedWindow,
    SlidingWindow,
    TokenBucket,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CacheConfig {
    pub ttl_seconds: u64,
    pub vary_by: Vec<String>, // キャッシュキー生成のためのパラメータ
}

/// APIレスポンスIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponseIR {
    pub status_code: u16,
    pub headers: Properties,
    pub body: ApiResponseBody,
    pub metadata: ResponseMetadata,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ApiResponseBody {
    JSON(Value),
    Text(String),
    HTML(String),
    Binary(Vec<u8>),
    File { path: String, filename: String },
    // Stream(Box<dyn std::io::Read>), // ストリーミングレスポンス用 - TODO: 実装予定
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub content_type: String,
    pub content_length: Option<usize>,
    pub cache_control: Option<String>,
    pub etag: Option<String>,
}

/// データベースIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DatabaseIR {
    pub connection_string: String,
    pub db_type: DatabaseType,
    pub models: Vec<ModelIR>,
    pub migrations: Vec<MigrationIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DatabaseType {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelIR {
    pub name: String,
    pub table_name: String,
    pub fields: Vec<FieldIR>,
    pub relationships: Vec<RelationshipIR>,
    pub indexes: Vec<IndexIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldIR {
    pub name: String,
    pub field_type: FieldType,
    pub nullable: bool,
    pub default_value: Option<Value>,
    pub unique: bool,
    pub primary_key: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FieldType {
    String { max_length: Option<usize> },
    Text,
    Integer,
    BigInt,
    Float,
    Double,
    Decimal { precision: u32, scale: u32 },
    Boolean,
    Date,
    DateTime,
    Time,
    UUID,
    JSON,
    Binary,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RelationshipIR {
    pub name: String,
    pub target_model: String,
    pub relationship_type: RelationshipType,
    pub foreign_key: String,
    pub on_delete: CascadeAction,
    pub on_update: CascadeAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RelationshipType {
    OneToOne,
    OneToMany,
    ManyToMany,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CascadeAction {
    Cascade,
    Restrict,
    SetNull,
    SetDefault,
    NoAction,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndexIR {
    pub name: String,
    pub fields: Vec<String>,
    pub unique: bool,
    pub index_type: IndexType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    GIN,
    GiST,
    SPGiST,
    BRIN,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MigrationIR {
    pub version: String,
    pub description: String,
    pub up_sql: String,
    pub down_sql: String,
    pub dependencies: Vec<String>,
}

/// ミドルウェアIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiddlewareIR {
    pub name: String,
    pub middleware_type: MiddlewareType,
    pub config: Properties,
    pub order: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MiddlewareType {
    Authentication,
    Authorization,
    CORS,
    Compression,
    CSRF,
    Logging,
    RateLimiting,
    Session,
    StaticFiles,
    Custom(String),
}

/// WebSocket IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSocketIR {
    pub path: String,
    pub handler: WebSocketHandlerIR,
    pub protocols: Vec<String>,
    pub heartbeat_interval: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebSocketHandlerIR {
    pub on_connect: String,
    pub on_message: String,
    pub on_disconnect: String,
    pub on_error: String,
}

/// GraphQL IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLIR {
    pub schema: GraphQLSchemaIR,
    pub resolvers: HashMap<String, GraphQLResolverIR>,
    pub directives: Vec<GraphQLDirectiveIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLSchemaIR {
    pub query: Option<String>,
    pub mutation: Option<String>,
    pub subscription: Option<String>,
    pub types: Vec<GraphQLTypeIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLTypeIR {
    pub name: String,
    pub kind: GraphQLTypeKind,
    pub fields: Vec<GraphQLFieldIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GraphQLTypeKind {
    Object,
    Interface,
    Union,
    Enum,
    InputObject,
    Scalar,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLFieldIR {
    pub name: String,
    pub field_type: String,
    pub args: Vec<GraphQLArgumentIR>,
    pub resolver: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLArgumentIR {
    pub name: String,
    pub arg_type: String,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLResolverIR {
    pub field_name: String,
    pub function_name: String,
    pub is_async: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphQLDirectiveIR {
    pub name: String,
    pub locations: Vec<DirectiveLocation>,
    pub args: Vec<GraphQLArgumentIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
    Schema,
    Scalar,
    Object,
    FieldDefinition,
    ArgumentDefinition,
    Interface,
    Union,
    Enum,
    EnumValue,
    InputObject,
    InputFieldDefinition,
}

/// Webフレームワーク設定IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebFrameworkConfigIR {
    pub server: ServerConfig,
    pub database: Option<DatabaseIR>,
    pub api_routes: Vec<ApiRouteIR>,
    pub web_sockets: Vec<WebSocketIR>,
    pub graph_ql: Option<GraphQLIR>,
    pub middlewares: Vec<MiddlewareIR>,
    pub static_files: Vec<StaticFilesConfig>,
    pub authentication: Option<AuthConfig>,
    pub session: Option<SessionConfig>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub tls: Option<TLSConfig>,
    pub workers: usize,
    pub max_connections: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TLSConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StaticFilesConfig {
    pub route: String,
    pub directory: String,
    pub cache_control: Option<String>,
    pub gzip: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AuthConfig {
    pub provider: AuthProvider,
    pub config: Properties,
    pub jwt_secret: Option<String>,
    pub session_timeout: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AuthProvider {
    Local,
    OAuth2,
    LDAP,
    SAML,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionConfig {
    pub store: SessionStore,
    pub cookie_name: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: SameSitePolicy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SessionStore {
    Memory,
    Redis,
    Database,
    File,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_route_creation() {
        let route = ApiRouteIR {
            path: "/api/users".to_string(),
            method: ApiMethod::GET,
            handler: ApiHandlerIR {
                function_name: "getUsers".to_string(),
                component: None,
                is_async: true,
                timeout_ms: Some(5000),
            },
            middlewares: vec!["auth".to_string(), "cors".to_string()],
            response_format: ResponseFormat::JSON,
            parameters: ApiParameters {
                path_params: Vec::new(),
                query_params: vec![ApiParameter {
                    name: "limit".to_string(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default_value: Some(Value::Int(10)),
                    validation: Some(ValidationRules {
                        min_length: None,
                        max_length: None,
                        pattern: None,
                        min_value: Some(1.0),
                        max_value: Some(100.0),
                        allowed_values: Vec::new(),
                    }),
                }],
                body_params: None,
                headers: Vec::new(),
            },
            metadata: ApiMetadata {
                description: Some("Get users list".to_string()),
                summary: Some("Users API".to_string()),
                tags: vec!["users".to_string()],
                deprecated: false,
                rate_limit: Some(RateLimit {
                    requests: 100,
                    window_seconds: 60,
                    strategy: RateLimitStrategy::SlidingWindow,
                }),
                cache: Some(CacheConfig {
                    ttl_seconds: 300,
                    vary_by: vec!["user_id".to_string()],
                }),
            },
        };

        assert_eq!(route.path, "/api/users");
        assert_eq!(route.method, ApiMethod::GET);
        assert_eq!(route.handler.function_name, "getUsers");
        assert!(route.handler.is_async);
    }

    #[test]
    fn test_database_model_creation() {
        let model = ModelIR {
            name: "User".to_string(),
            table_name: "users".to_string(),
            fields: vec![
                FieldIR {
                    name: "id".to_string(),
                    field_type: FieldType::UUID,
                    nullable: false,
                    default_value: None,
                    unique: true,
                    primary_key: true,
                },
                FieldIR {
                    name: "email".to_string(),
                    field_type: FieldType::String { max_length: Some(255) },
                    nullable: false,
                    default_value: None,
                    unique: true,
                    primary_key: false,
                },
                FieldIR {
                    name: "created_at".to_string(),
                    field_type: FieldType::DateTime,
                    nullable: false,
                    default_value: Some(Value::String("NOW()".to_string())),
                    unique: false,
                    primary_key: false,
                },
            ],
            relationships: vec![
                RelationshipIR {
                    name: "posts".to_string(),
                    target_model: "Post".to_string(),
                    relationship_type: RelationshipType::OneToMany,
                    foreign_key: "user_id".to_string(),
                    on_delete: CascadeAction::Cascade,
                    on_update: CascadeAction::NoAction,
                },
            ],
            indexes: vec![
                IndexIR {
                    name: "idx_users_email".to_string(),
                    fields: vec!["email".to_string()],
                    unique: true,
                    index_type: IndexType::BTree,
                },
            ],
        };

        assert_eq!(model.name, "User");
        assert_eq!(model.table_name, "users");
        assert_eq!(model.fields.len(), 3);
        assert_eq!(model.relationships.len(), 1);
        assert_eq!(model.indexes.len(), 1);
    }

    #[test]
    fn test_middleware_creation() {
        let middleware = MiddlewareIR {
            name: "cors".to_string(),
            middleware_type: MiddlewareType::CORS,
            config: {
                let mut props = Properties::new();
                props.insert("allowed_origins".to_string(), Value::String("*".to_string()));
                props.insert("allowed_methods".to_string(), Value::String("GET,POST,PUT,DELETE".to_string()));
                props
            },
            order: 1,
        };

        assert_eq!(middleware.name, "cors");
        assert_eq!(middleware.middleware_type, MiddlewareType::CORS);
        assert_eq!(middleware.order, 1);
    }
}

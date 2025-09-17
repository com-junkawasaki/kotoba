//! HTTPハンドラーとミドルウェア処理
//!
//! このモジュールはHTTPリクエストの処理とミドルウェア実行を担当します。
//! グラフ書換えルールを使ってリクエスト処理を行います。

use kotoba_core::types::{TxId, ContentHash, Result, KotobaError, Value, Properties};
use crate::http::ir::*;
use kotoba_graph::prelude::*;
// use kotoba_storage::prelude::*; // Storage crate has issues
use kotoba_rewrite::prelude::*;
use kotoba_security::{SecurityService, AuditResult};
use kotoba_core::ir::rule::{RuleIR, Match};
use kotoba_core::ir::strategy::{StrategyIR, StrategyOp};
use kotoba_core::ir::patch::Patch;
// Re-export security types for backward compatibility
pub use kotoba_security::{JwtClaims, User, AuthResult, AuthzResult, Principal, Resource};
use std::collections::HashMap;
use std::sync::Arc;

/// HTTPリクエストプロセッサ
#[derive(Clone)]
pub struct HttpRequestProcessor {
    rewrite_engine: Arc<RewriteEngine>,
    mvcc: Arc<MVCCManager>,
    merkle: Arc<MerkleDAG>,
    security: Arc<SecurityService>,
}

impl HttpRequestProcessor {
    pub fn new(
        rewrite_engine: Arc<RewriteEngine>,
        mvcc: Arc<MVCCManager>,
        merkle: Arc<MerkleDAG>,
        security: Arc<SecurityService>,
    ) -> Self {
        Self {
            rewrite_engine,
            mvcc,
            merkle,
            security,
        }
    }

    /// HTTPリクエストを処理してレスポンスを生成
    pub async fn process_request(&self, request: HttpRequest) -> Result<HttpResponse> {
        // 簡略化された実装：直接ハンドラーを呼び出す
        self.process_request_simple(request).await
    }

    /// 簡略化されたリクエスト処理
    async fn process_request_simple(&self, request: HttpRequest) -> Result<HttpResponse> {
        // パスに基づいてレスポンスを生成
        match request.path.as_str() {
            "/ping" => {
                let mut headers = HttpHeaders::new();
                headers.set("content-type".to_string(), "application/json".to_string());
                Ok(HttpResponse::new(
                    request.id,
                    HttpStatus::ok(),
                    headers,
                    Some(ContentHash::sha256([0; 32])), // 固定のコンテンツハッシュ
                ))
            },
            "/health" => {
                let mut headers = HttpHeaders::new();
                headers.set("content-type".to_string(), "application/json".to_string());
                Ok(HttpResponse::new(
                    request.id,
                    HttpStatus::ok(),
                    headers,
                    Some(ContentHash::sha256([1; 32])), // 固定のコンテンツハッシュ
                ))
            },
            "/graphql" => {
                // GraphQL endpoint will be handled separately
                let mut headers = HttpHeaders::new();
                headers.set("content-type".to_string(), "application/json".to_string());
                Ok(HttpResponse::new(
                    request.id,
                    HttpStatus::ok(),
                    headers,
                    Some(ContentHash::sha256([2; 32])),
                ))
            },
            _ => {
                // 404 Not Found
                Ok(HttpResponse::new(
                    request.id,
                    HttpStatus::not_found(),
                    HttpHeaders::new(),
                    None,
                ))
            }
        }
    }

}

/// HTTP用リライト外部関数
pub struct HttpRewriteExterns;

impl HttpRewriteExterns {
    pub fn new() -> Self {
        Self
    }
}

impl RewriteExterns for HttpRewriteExterns {
    fn deg_ge(&self, _v: crate::types::VertexId, _k: u32) -> bool {
        // TODO: 次数チェックを実装
        true
    }

    fn edge_count_nonincreasing(&self, _g0: &GraphRef, _g1: &GraphRef) -> bool {
        // TODO: エッジ数非増加チェックを実装
        true
    }

    fn custom_measure(&self, _name: &str, _args: &[kotoba_core::types::Value]) -> f64 {
        // TODO: カスタム測定関数を実装
        0.0
    }
}

/// ミドルウェアプロセッサ
#[derive(Clone)]
pub struct MiddlewareProcessor {
    middlewares: Vec<HttpMiddleware>,
    security: Arc<SecurityService>,
}

impl MiddlewareProcessor {
    pub fn new(middlewares: Vec<HttpMiddleware>, security: Arc<SecurityService>) -> Self {
        Self { middlewares, security }
    }

    /// ミドルウェアを順序通りに実行
    pub async fn process(&self, request: &mut HttpRequest) -> Result<()> {
        // 順序でソート
        let mut sorted_middlewares = self.middlewares.clone();
        sorted_middlewares.sort_by_key(|mw| mw.order);

        for middleware in sorted_middlewares {
            self.execute_middleware(&middleware, request).await?;
        }

        Ok(())
    }

    /// 個別のミドルウェアを実行
    async fn execute_middleware(&self, middleware: &HttpMiddleware, request: &mut HttpRequest) -> Result<()> {
        match middleware.name.as_str() {
            "request_id" => {
                // X-Request-IDヘッダーを追加
                let request_id = format!("req_{}", request.id);
                request.headers.set("x-request-id".to_string(), request_id);
            },
            "logger" => {
                // ログミドルウェア（実際のログ出力はしない）
                println!("Request: {} {} {}", request.method, request.path, request.id);
            },
            "cors" => {
                // CORSヘッダー（実際のレスポンスには影響しない）
            },
            "jwt_auth" => {
                self.execute_jwt_auth_middleware(request).await?;
            },
            "authorization" => {
                self.execute_authorization_middleware(request).await?;
            },
            "rate_limit" => {
                self.execute_rate_limit_middleware(request).await?;
            },
            "csrf" => {
                self.execute_csrf_middleware(request).await?;
            },
            _ => {
                // カスタムミドルウェア（未実装）
                println!("Executing custom middleware: {}", middleware.name);
            }
        }

        Ok(())
    }

    /// JWT認証ミドルウェアを実行
    async fn execute_jwt_auth_middleware(&self, request: &mut HttpRequest) -> Result<()> {
        // AuthorizationヘッダーからJWTトークンを取得
        let auth_header = request.headers.get("authorization");

        if let Some(auth_value) = auth_header {
            if auth_value.starts_with("Bearer ") {
                let token = &auth_value[7..]; // "Bearer " を除去

                // JWTトークンを検証
                match self.security.validate_token(token) {
                    Ok(claims) => {
                        // 監査ログを記録（簡易実装）
                        println!("AUDIT: JWT authentication successful for user: {}", claims.sub);

                        // 検証成功：クレームをリクエスト属性に保存
                        request.attributes.insert(
                            "user_id".to_string(),
                            Value::String(claims.sub)
                        );
                        request.attributes.insert(
                            "roles".to_string(),
                            Value::Array(claims.roles)
                        );

                        return Ok(());
                    }
                    Err(_) => {
                        // 監査ログを記録（簡易実装）
                        println!("AUDIT: JWT authentication failed - invalid token");

                        return Err(KotobaError::Security("Invalid JWT token".to_string()));
                    }
                }
            }
        }

        // Authorizationヘッダーがない場合
        println!("AUDIT: Missing authentication token");
        Err(KotobaError::Security("Authentication required".to_string()))
    }

    /// 認可ミドルウェアを実行
    async fn execute_authorization_middleware(&self, request: &mut HttpRequest) -> Result<()> {
        // 簡易実装：認証済みユーザーのみアクセスを許可
        if request.attributes.contains_key("user_id") {
            println!("AUDIT: Authorization successful for path: {}", request.path);
            Ok(())
        } else {
            println!("AUDIT: Authorization failed - user not authenticated");
            Err(KotobaError::Security("Authorization required".to_string()))
        }
    }

    /// リクエストからリソースタイプ、アクション、リソースIDを決定
    fn determine_resource_from_request(&self, request: &HttpRequest) -> (kotoba_security::ResourceType, kotoba_security::Action, Option<String>) {
        use kotoba_security::{ResourceType, Action};

        // パスベースでリソースを決定
        match request.path.as_str() {
            // Graph operations
            path if path.starts_with("/api/graph/") => {
                let resource_id = if path.len() > "/api/graph/".len() {
                    Some(path["/api/graph/".len()..].to_string())
                } else {
                    None
                };

                match request.method {
                    HttpMethod::GET => (ResourceType::Graph, Action::Read, resource_id),
                    HttpMethod::POST => (ResourceType::Graph, Action::Create, resource_id),
                    HttpMethod::PUT => (ResourceType::Graph, Action::Update, resource_id),
                    HttpMethod::DELETE => (ResourceType::Graph, Action::Delete, resource_id),
                    _ => (ResourceType::Graph, Action::Read, resource_id),
                }
            },

            // Query operations
            path if path.starts_with("/api/query") => {
                (ResourceType::Query, Action::Execute, None)
            },

            // Admin operations
            path if path.starts_with("/api/admin") => {
                (ResourceType::Admin, Action::Admin, None)
            },

            // User operations
            path if path.starts_with("/api/user") => {
                (ResourceType::User, Action::Read, None)
            },

            // Default: filesystem access
            _ => {
                (ResourceType::FileSystem, Action::Read, Some(request.path.clone()))
            }
        }
    }

    /// レート制限ミドルウェアを実行
    async fn execute_rate_limit_middleware(&self, _request: &mut HttpRequest) -> Result<()> {
        // 簡易実装：常に許可
        Ok(())
    }

    /// CSRF保護ミドルウェアを実行
    async fn execute_csrf_middleware(&self, request: &mut HttpRequest) -> Result<()> {
        // 簡易実装：POST/PUT/DELETEの場合はCSRFトークンチェック
        let is_state_changing = matches!(request.method,
            crate::http::ir::HttpMethod::POST |
            crate::http::ir::HttpMethod::PUT |
            crate::http::ir::HttpMethod::DELETE);

        if is_state_changing {
            if request.headers.get("x-csrf-token").is_some() {
                Ok(())
            } else {
                Err(KotobaError::Security("CSRF token required".to_string()))
            }
        } else {
            Ok(())
        }
    }
}

/// ハンドラープロセッサ
#[derive(Clone)]
pub struct HandlerProcessor;

impl HandlerProcessor {
    pub fn new() -> Self {
        Self
    }

    /// 指定されたハンドラーを実行
    pub async fn process(&self, route: &HttpRoute, request: &HttpRequest) -> Result<HttpResponse> {
        // TODO: 実際のハンドラー関数の実行を実装
        // 現在はルートベースで簡単なレスポンスを返す

        match route.pattern.as_str() {
            "/ping" => {
                let mut headers = HttpHeaders::new();
                headers.set("content-type".to_string(), "application/json".to_string());
                Ok(HttpResponse::new(
                    request.id.clone(),
                    HttpStatus::ok(),
                    headers,
                    Some(ContentHash::sha256([0; 32])), // TODO: 実際のJSONコンテンツ
                ))
            },
            "/health" => {
                let mut headers = HttpHeaders::new();
                headers.set("content-type".to_string(), "application/json".to_string());
                Ok(HttpResponse::new(
                    request.id.clone(),
                    HttpStatus::ok(),
                    headers,
                    Some(ContentHash::sha256([1; 32])), // TODO: 実際のJSONコンテンツ
                ))
            },
            _ => {
                // 404 Not Found
                Ok(HttpResponse::new(
                    request.id.clone(),
                    HttpStatus::not_found(),
                    HttpHeaders::new(),
                    None,
                ))
            }
        }
    }
}

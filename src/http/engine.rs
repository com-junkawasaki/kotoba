//! HTTPサーバーエンジン
//!
//! このモジュールはHTTPサーバーのコアエンジンを提供します。
//! 設定管理、リクエスト処理、状態管理を行います。

use crate::types::{ContentHash, Result, KotobaError};
use crate::GraphRef;
use crate::http::ir::*;
use crate::http::handlers::*;
use crate::MVCCManager;
use crate::MerkleDAG;
use crate::RewriteEngine;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;

/// HTTPサーバーエンジン
#[derive(Clone)]
pub struct HttpEngine {
    config: HttpConfig,
    request_processor: HttpRequestProcessor,
    middleware_processor: MiddlewareProcessor,
    handler_processor: HandlerProcessor,
    routes: HashMap<String, HttpRoute>,
    state: Arc<RwLock<ServerState>>,
}

impl HttpEngine {
    /// 新しいHTTPエンジンを作成
    pub fn new(
        config: HttpConfig,
        mvcc: Arc<MVCCManager>,
        merkle: Arc<MerkleDAG>,
        rewrite_engine: Arc<RewriteEngine>,
    ) -> Self {
        let security_service = Arc::new(crate::http::handlers::SecurityService);
        let security_service_clone = security_service.clone();

        let request_processor = HttpRequestProcessor::new(
            rewrite_engine,
            mvcc,
            merkle,
            security_service,
        );

        let middleware_processor = MiddlewareProcessor::new(config.middlewares.clone(), security_service_clone);
        let handler_processor = HandlerProcessor::new();

        // ルートをマップにインデックス化
        let mut routes = HashMap::new();
        for route in &config.routes {
            routes.insert(route.id.clone(), route.clone());
        }

        let state = Arc::new(RwLock::new(ServerState::new()));

        Self {
            config,
            request_processor,
            middleware_processor,
            handler_processor,
            routes,
            state,
        }
    }

    /// HTTPリクエストを処理
    pub async fn handle_request(&self, raw_request: RawHttpRequest) -> Result<HttpResponse> {
        // 1. 生のリクエストをHttpRequestに変換
        let mut http_request = self.convert_raw_request(raw_request).await?;

        // 2. ミドルウェアを適用
        self.middleware_processor.process(&mut http_request).await?;

        // 3. ルーティング
        let route = self.route_request(&http_request).await?;

        // 4. ハンドラーを実行
        let response = self.handler_processor.process(&route, &http_request).await?;

        // 5. レスポンスを更新
        {
            let mut state = self.state.write().await;
            state.requests_processed += 1;
        }

        Ok(response)
    }

    /// 生のリクエストをHttpRequestに変換
    async fn convert_raw_request(&self, raw: RawHttpRequest) -> Result<HttpRequest> {
        let method = HttpMethod::from_str(&raw.method)?;
        let mut headers = HttpHeaders::new();

        for (key, value) in raw.headers {
            headers.set(key, value);
        }

        // クエリパラメータのパース
        let query = if let Some(query_str) = raw.query_string {
            self.parse_query_string(&query_str)
        } else {
            HashMap::new()
        };

        // ボディのハッシュ計算（実際の実装では外部ストレージに保存）
        let body_ref = if !raw.body.is_empty() {
            Some(self.hash_body(&raw.body))
        } else {
            None
        };

        let request_id = format!("req_{}", uuid::Uuid::new_v4());

        Ok(HttpRequest::new(
            request_id,
            method,
            raw.path,
            headers,
            body_ref,
        ))
    }

    /// クエリ文字列をパース
    fn parse_query_string(&self, query_str: &str) -> HashMap<String, String> {
        let mut query = HashMap::new();

        for pair in query_str.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                // URL decode key and value (stub implementation)
                let decoded_key = key.to_string();
                let decoded_value = value.to_string();
                if !decoded_key.is_empty() && !decoded_value.is_empty() {
                    query.insert(key.to_owned(), value.to_owned());
                }
            }
        }

        query
    }

    /// ボディのハッシュを計算
    fn hash_body(&self, body: &[u8]) -> ContentHash {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(body);
        let result = hasher.finalize();
        ContentHash(hex::encode(result))
    }

    /// リクエストをルーティング
    async fn route_request(&self, request: &HttpRequest) -> Result<HttpRoute> {
        // パスとメソッドにマッチするルートを探す
        for route in self.config.routes.iter() {
            if route.method == request.method && self.pattern_matches(&request.path, &route.pattern) {
                return Ok(route.clone());
            }
        }

        // マッチするルートが見つからない場合、デフォルトルートを返す
        // TODO: デフォルトの404ハンドラールートを実装
        Err(KotobaError::NotFound(format!("No route found for {} {}", request.method, request.path)))
    }

    /// パスパターンマッチング
    fn pattern_matches(&self, path: &str, pattern: &str) -> bool {
        if pattern == path {
            return true;
        }

        // シンプルなワイルドカードマッチング
        if pattern.ends_with("/*") {
            let prefix = &pattern[..pattern.len() - 2];
            return path.starts_with(prefix);
        }

        // パラメータマッチング（例: /users/{id}）
        if pattern.contains('{') && pattern.contains('}') {
            let pattern_parts: Vec<&str> = pattern.split('/').collect();
            let path_parts: Vec<&str> = path.split('/').collect();

            if pattern_parts.len() != path_parts.len() {
                return false;
            }

            for (pattern_part, path_part) in pattern_parts.iter().zip(path_parts.iter()) {
                if pattern_part.starts_with('{') && pattern_part.ends_with('}') {
                    // パラメータ部分は常にマッチ
                    continue;
                } else if pattern_part != path_part {
                    return false;
                }
            }

            return true;
        }

        false
    }

    /// サーバーの状態を取得
    pub async fn get_state(&self) -> ServerState {
        self.state.read().await.clone()
    }

    /// サーバーの設定を取得
    pub fn get_config(&self) -> &HttpConfig {
        &self.config
    }

    /// ルート一覧を取得
    pub fn get_routes(&self) -> &HashMap<String, HttpRoute> {
        &self.routes
    }
}

/// サーバーの状態
#[derive(Debug, Clone)]
pub struct ServerState {
    pub requests_processed: u64,
    pub uptime_seconds: u64,
    pub start_time: std::time::SystemTime,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            requests_processed: 0,
            uptime_seconds: 0,
            start_time: std::time::SystemTime::now(),
        }
    }

    pub fn update_uptime(&mut self) {
        if let Ok(duration) = self.start_time.elapsed() {
            self.uptime_seconds = duration.as_secs();
        }
    }
}

/// 生のHTTPリクエスト（HTTPライブラリからの入力）
#[derive(Debug, Clone)]
pub struct RawHttpRequest {
    pub method: String,
    pub path: String,
    pub query_string: Option<String>,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl RawHttpRequest {
    pub fn new(method: String, path: String) -> Self {
        Self {
            method,
            path,
            query_string: None,
            headers: Vec::new(),
            body: Vec::new(),
        }
    }

    pub fn with_query(mut self, query: String) -> Self {
        self.query_string = Some(query);
        self
    }

    pub fn with_header(mut self, key: String, value: String) -> Self {
        self.headers.push((key, value));
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_storage::prelude::*;
    use kotoba_rewrite::prelude::*;

    #[tokio::test]
    async fn test_pattern_matching() {
        let mvcc = Arc::new(MVCCManager::new());
        let merkle = Arc::new(MerkleDAG::new());
        let rewrite_engine = Arc::new(RewriteEngine::new());

        let config = HttpConfig::new(ServerConfig::default());
        let engine = HttpEngine::new(config, mvcc, merkle, rewrite_engine);

        // 完全一致
        assert!(engine.pattern_matches("/ping", "/ping"));
        assert!(!engine.pattern_matches("/ping", "/pong"));

        // ワイルドカード
        assert!(engine.pattern_matches("/api/users/123", "/api/*"));
        assert!(!engine.pattern_matches("/other/path", "/api/*"));

        // パラメータ
        assert!(engine.pattern_matches("/users/123", "/users/{id}"));
        assert!(engine.pattern_matches("/posts/hello-world", "/posts/{slug}"));
        assert!(!engine.pattern_matches("/users/123/profile", "/users/{id}"));
    }

    #[tokio::test]
    async fn test_convert_raw_request() {
        let mvcc = Arc::new(MVCCManager::new());
        let merkle = Arc::new(MerkleDAG::new());
        let rewrite_engine = Arc::new(RewriteEngine::new());

        let config = HttpConfig::new(ServerConfig::default());
        let engine = HttpEngine::new(config, mvcc, merkle, rewrite_engine);

        let raw_request = RawHttpRequest::new("GET".to_string(), "/ping".to_string())
            .with_query("key=value&foo=bar".to_string())
            .with_header("content-type".to_string(), "application/json".to_string())
            .with_body(b"test body".to_vec());

        let http_request = engine.convert_raw_request(raw_request).await.unwrap();

        assert_eq!(http_request.method, HttpMethod::GET);
        assert_eq!(http_request.path, "/ping");
        assert_eq!(http_request.query.get("key"), Some(&"value".to_string()));
        assert_eq!(http_request.query.get("foo"), Some(&"bar".to_string()));
        assert_eq!(http_request.headers.get("content-type"), Some(&"application/json".to_string()));
        assert!(http_request.body_ref.is_some());
    }
}

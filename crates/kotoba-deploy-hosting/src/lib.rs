//! # Kotoba Deploy Hosting
//!
//! Hosting server module for the Kotoba deployment system.
//! Provides HTTP server functionality, request routing, and application hosting.

use kotoba_core::types::Result;
use kotoba_core::prelude::KotobaError;
use kotoba_deploy_core::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;
use hyper::{Body, Request, Response, Server, StatusCode};
use hyper::service::{make_service_fn, service_fn};
use std::convert::Infallible;

/// ホスティングサーバー
#[derive(Debug)]
pub struct HostingServer {
    /// HTTPサーバー設定
    config: ServerConfig,
    /// ホストされたアプリケーション
    hosted_apps: Arc<RwLock<HashMap<String, HostedApp>>>,
    /// サーバーハンドル
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

/// ホストされたアプリケーション
#[derive(Debug, Clone)]
pub struct HostedApp {
    /// アプリケーションID
    pub id: String,
    /// デプロイメントID
    pub deployment_id: String,
    /// インスタンスID
    pub instance_id: String,
    /// ドメイン
    pub domain: String,
    /// ポート
    pub port: u16,
    /// 作成時刻
    pub created_at: SystemTime,
    /// 最終アクセス時刻
    pub last_access: SystemTime,
    /// リクエスト数
    pub request_count: u64,
}

/// サーバー設定
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// ホスト
    pub host: String,
    /// ポート
    pub port: u16,
    /// SSL有効化
    pub ssl_enabled: bool,
    /// SSL証明書パス
    pub ssl_cert_path: Option<String>,
    /// SSLキーパス
    pub ssl_key_path: Option<String>,
}

/// HTTPレスポンス
#[derive(Debug)]
pub struct HttpResponse {
    /// ステータスコード
    pub status: u16,
    /// ヘッダー
    pub headers: HashMap<String, String>,
    /// ボディ
    pub body: Vec<u8>,
}

/// HTTPリクエスト
#[derive(Debug)]
pub struct HttpRequest {
    /// メソッド
    pub method: String,
    /// パス
    pub path: String,
    /// クエリパラメータ
    pub query: HashMap<String, String>,
    /// ヘッダー
    pub headers: HashMap<String, String>,
    /// ボディ
    pub body: Vec<u8>,
}

impl HostingServer {
    /// 新しいホスティングサーバーを作成
    pub fn new(config: ServerConfig) -> Self {
        Self {
            config,
            hosted_apps: Arc::new(RwLock::new(HashMap::new())),
            server_handle: None,
        }
    }

    /// サーバーを起動
    pub async fn start(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.config.host, self.config.port).parse().unwrap();

        println!("🚀 Starting hosting server on http://{}", addr);

        let hosted_apps = Arc::clone(&self.hosted_apps);

        let make_svc = make_service_fn(move |_conn| {
            let hosted_apps = Arc::clone(&hosted_apps);
            async move {
                Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                    let hosted_apps = Arc::clone(&hosted_apps);
                    async move {
                        Self::handle_request(req, hosted_apps).await
                    }
                }))
            }
        });

        let server = Server::bind(&addr).serve(make_svc);

        self.server_handle = Some(tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("Server error: {}", e);
            }
        }));

        println!("✅ Hosting server started successfully");
        Ok(())
    }

    /// サーバーを停止
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
            println!("🛑 Hosting server stopped");
        }
        Ok(())
    }

    /// アプリケーションをホスト
    pub async fn host_application(
        &self,
        deployment_id: &str,
        instance_id: &str,
        domain: &str,
        port: u16,
    ) -> Result<String> {
        let app_id = format!("app-{}-{}", deployment_id, SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| KotobaError::Execution(format!("Failed to get system time: {}", e)))?
            .as_secs());

        let hosted_app = HostedApp {
            id: app_id.clone(),
            deployment_id: deployment_id.to_string(),
            instance_id: instance_id.to_string(),
            domain: domain.to_string(),
            port,
            created_at: SystemTime::now(),
            last_access: SystemTime::now(),
            request_count: 0,
        };

        self.hosted_apps.write().unwrap().insert(app_id.clone(), hosted_app);

        println!("✅ Application {} hosted on {}:{}", deployment_id, domain, port);
        Ok(app_id)
    }

    /// ホストされたアプリケーションを取得
    pub fn get_hosted_apps(&self) -> std::sync::RwLockReadGuard<HashMap<String, HostedApp>> {
        self.hosted_apps.read().unwrap()
    }

    /// HTTPリクエストを処理
    async fn handle_request(
        req: Request<Body>,
        hosted_apps: Arc<RwLock<HashMap<String, HostedApp>>>,
    ) -> std::result::Result<Response<Body>, Infallible> {
        let path = req.uri().path().to_string();
        let method = req.method().to_string();

        println!("📨 {} {}", method, path);

        // パスに基づいてアプリケーションを検索
        let apps = hosted_apps.read().unwrap();

        // ホストヘッダーからドメインを取得
        let host = req.headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(':').next())
            .unwrap_or("localhost");

        // ドメインに対応するアプリケーションを検索
        let app = apps.values().find(|app| app.domain == host);

        match app.cloned() {
            Some(app) => {
                // アプリケーションが見つかった場合
                let app_id = app.id.clone();

                // 読み取りロックを解放してから書き込みロックを取得
                drop(apps);
                {
                    let mut apps = hosted_apps.write().unwrap();
                    if let Some(app_mut) = apps.get_mut(&app_id) {
                        app_mut.last_access = SystemTime::now();
                        app_mut.request_count += 1;
                    }
                }

                // 簡単なレスポンスを返す
                let response = format!(
                    "🚀 Kotoba Application: {}\n📊 Deployment: {}\n🌐 Domain: {}\n⏰ Last Access: {:?}\n📈 Request Count: {}\n",
                    app.id,
                    app.deployment_id,
                    app.domain,
                    app.last_access,
                    app.request_count
                );

                Ok(Response::new(Body::from(response)))
            }
            None => {
                // アプリケーションが見つからない場合
                let not_found = format!("❌ No application found for domain: {}", host);
                let mut response = Response::new(Body::from(not_found));
                *response.status_mut() = StatusCode::NOT_FOUND;
                Ok(response)
            }
        }
    }

    /// ヘルスチェック
    pub async fn health_check(&self) -> Result<()> {
        // 基本的なヘルスチェック
        let apps = self.hosted_apps.read().unwrap();
        println!("💚 Hosting server healthy - {} applications hosted", apps.len());
        Ok(())
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            ssl_enabled: false,
            ssl_cert_path: None,
            ssl_key_path: None,
        }
    }
}

impl HttpRequest {
    /// リクエストをパース
    pub fn from_hyper_request(req: Request<Body>) -> Self {
        let method = req.method().to_string();
        let path = req.uri().path().to_string();

        // クエリパラメータをパース
        let query = req.uri().query()
            .map(|q| url::form_urlencoded::parse(q.as_bytes())
                .into_owned()
                .collect())
            .unwrap_or_default();

        // ヘッダーをパース
        let headers = req.headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // ボディを取得（簡易実装）
        let body = Vec::new(); // TODO: 実際のボディ取得を実装

        Self {
            method,
            path,
            query,
            headers,
            body,
        }
    }
}

impl HttpResponse {
    /// 成功レスポンスを作成
    pub fn ok(body: impl Into<String>) -> Self {
        Self {
            status: 200,
            headers: HashMap::new(),
            body: body.into().into_bytes(),
        }
    }

    /// エラーレスポンスを作成
    pub fn error(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: message.into().into_bytes(),
        }
    }

    /// JSONレスポンスを作成
    pub fn json(data: serde_json::Value) -> Self {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        Self {
            status: 200,
            headers,
            body: serde_json::to_string(&data).unwrap_or_default().into_bytes(),
        }
    }
}

// Re-export commonly used types
pub use HostingServer as HostingSvc;
pub use ServerConfig as HostingConfig;
pub use HostedApp as HostedApplication;

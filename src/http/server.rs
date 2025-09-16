//! HTTPサーバー
//!
//! このモジュールはHTTPサーバーのメインインターフェースを提供します。

use crate::types::Result;
use crate::http::ir::*;
use crate::http::parser::HttpConfigParser;
use crate::http::engine::{HttpEngine, RawHttpRequest};
use kotoba_storage::prelude::*;
use kotoba_rewrite::prelude::*;
use std::sync::Arc;
use std::path::Path;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::collections::HashMap;

/// HTTPサーバー
pub struct HttpServer {
    engine: HttpEngine,
    listener: Option<TcpListener>,
}

impl HttpServer {
    /// 新しいHTTPサーバーを作成
    pub async fn new(
        config: HttpConfig,
        mvcc: Arc<MVCCManager>,
        merkle: Arc<MerkleDAG>,
        rewrite_engine: Arc<RewriteEngine>,
    ) -> Result<Self> {
        let engine = HttpEngine::new(config, mvcc, merkle, rewrite_engine).await?;
        Ok(Self {
            engine,
            listener: None,
        })
    }

    /// 設定ファイルからHTTPサーバーを作成
    pub async fn from_config_file<P: AsRef<Path>>(
        config_path: P,
        mvcc: Arc<MVCCManager>,
        merkle: Arc<MerkleDAG>,
        rewrite_engine: Arc<RewriteEngine>,
    ) -> Result<Self> {
        let config = HttpConfigParser::parse(config_path)?;
        Self::new(config, mvcc, merkle, rewrite_engine).await
    }

    /// サーバーを開始
    pub async fn start(&mut self) -> Result<()> {
        let config = self.engine.get_config();
        let address = format!("{}:{}", config.server.host, config.server.port);

        println!("Starting Kotoba HTTP server on {}", address);
        let listener = TcpListener::bind(&address).await?;
        self.listener = Some(listener);

        println!("Server started successfully");
        Ok(())
    }

    /// サーバーを実行（メインループ）
    pub async fn run(&self) -> Result<()> {
        if let Some(listener) = &self.listener {
            println!("Accepting connections...");

            loop {
                let (socket, addr) = listener.accept().await?;
                println!("New connection from: {}", addr);

                // engineをクローンして渡す（ライフタイム問題を解決）
                let engine = self.engine.clone();
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_connection(socket, &engine).await {
                        eprintln!("Error handling connection: {}", e);
                    }
                });
            }
        } else {
            Err(crate::types::KotobaError::InvalidArgument(
                "Server not started. Call start() first.".to_string()
            ))
        }
    }

    /// 個別の接続を処理
    async fn handle_connection(mut socket: TcpStream, engine: &HttpEngine) -> Result<()> {
        let mut buffer = [0; 8192]; // 8KB buffer

        // HTTPリクエストを読み取り
        let n = socket.read(&mut buffer).await?;
        if n == 0 {
            return Ok(()); // Connection closed
        }

        let request_data = &buffer[..n];
        let request_str = String::from_utf8_lossy(request_data);

        // HTTPリクエストをパース
        let raw_request = Self::parse_http_request(&request_str)?;

        // リクエストを処理
        let response = engine.handle_request(raw_request).await?;

        // HTTPレスポンスを送信
        let response_bytes = Self::format_http_response(&response)?;
        socket.write_all(&response_bytes).await?;
        socket.flush().await?;

        Ok(())
    }

    /// HTTPリクエスト文字列をパース
    fn parse_http_request(request_str: &str) -> Result<RawHttpRequest> {
        let mut lines = request_str.lines();

        // リクエストラインをパース
        let request_line = lines.next().ok_or_else(|| {
            crate::types::KotobaError::InvalidArgument("Empty request".to_string())
        })?;

        let parts: Vec<&str> = request_line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(crate::types::KotobaError::InvalidArgument(
                "Invalid request line".to_string()
            ));
        }

        let method = parts[0].to_string();
        let mut path_and_query = parts[1].to_string();

        // クエリパラメータを分離
        let (path, query_string) = if let Some((p, q)) = path_and_query.split_once('?') {
            (p.to_string(), Some(q.to_string()))
        } else {
            (path_and_query, None)
        };

        let mut request = RawHttpRequest::new(method, path);
        if let Some(query) = query_string {
            request = request.with_query(query);
        }

        // ヘッダーをパース
        let mut headers = Vec::new();
        for line in lines {
            if line.is_empty() {
                break; // ヘッダーの終わり
            }

            if let Some((key, value)) = line.split_once(": ") {
                headers.push((key.to_string(), value.to_string()));
            }
        }

        for (key, value) in headers {
            request = request.with_header(key, value);
        }

        // ボディをパース（残りのデータ）
        let body_start = request_str.find("\r\n\r\n")
            .map(|pos| pos + 4)
            .unwrap_or(request_str.len());
        let body = request_str[body_start..].as_bytes().to_vec();
        request = request.with_body(body);

        Ok(request)
    }

    /// HTTPレスポンスをフォーマット
    fn format_http_response(response: &HttpResponse) -> Result<Vec<u8>> {
        let mut response_str = format!(
            "HTTP/1.1 {} {}\r\n",
            response.status.code, response.status.reason
        );

        // ヘッダーを追加
        for (key, value) in &response.headers.headers {
            response_str.push_str(&format!("{}: {}\r\n", key, value));
        }

        // Content-Lengthヘッダー
        if let Some(body_ref) = &response.body_ref {
            // TODO: 実際のボディサイズを取得
            response_str.push_str(&format!("Content-Length: 0\r\n"));
        } else {
            response_str.push_str("Content-Length: 0\r\n");
        }

        response_str.push_str("\r\n");

        // ボディを追加（実際の実装では外部ストレージから取得）
        let mut response_bytes = response_str.into_bytes();
        if let Some(_body_ref) = &response.body_ref {
            // TODO: ボディコンテンツを取得して追加
            // response_bytes.extend(body_content);
        }

        Ok(response_bytes)
    }

    /// サーバーの状態を取得
    pub async fn get_status(&self) -> ServerStatus {
        let state = self.engine.get_state().await;
        let config = self.engine.get_config();

        ServerStatus {
            host: config.server.host.clone(),
            port: config.server.port,
            requests_processed: state.requests_processed,
            uptime_seconds: state.uptime_seconds,
            routes_count: config.routes.len(),
            middlewares_count: config.middlewares.len(),
        }
    }
}

/// サーバーのステータス情報
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ServerStatus {
    pub host: String,
    pub port: u16,
    pub requests_processed: u64,
    pub uptime_seconds: u64,
    pub routes_count: usize,
    pub middlewares_count: usize,
}

/// サーバービルダー（設定ファイルからサーバーを作成するためのヘルパー）
pub struct ServerBuilder {
    config_path: Option<String>,
    config: Option<HttpConfig>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self {
            config_path: None,
            config: None,
        }
    }

    /// 設定ファイルパスを設定
    pub fn with_config_file<P: AsRef<Path>>(mut self, path: P) -> Self {
        self.config_path = Some(path.as_ref().to_string_lossy().to_string());
        self
    }

    /// 直接設定を設定
    pub fn with_config(mut self, config: HttpConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// サーバーを構築
    pub async fn build(
        self,
        mvcc: Arc<MVCCManager>,
        merkle: Arc<MerkleDAG>,
        rewrite_engine: Arc<RewriteEngine>,
    ) -> Result<HttpServer> {
        let config = if let Some(config) = self.config {
            config
        } else if let Some(path) = self.config_path {
            HttpConfigParser::parse(path)?
        } else {
            return Err(crate::types::KotobaError::InvalidArgument(
                "Either config file or config object must be provided".to_string()
            ));
        };

        HttpServer::new(config, mvcc, merkle, rewrite_engine).await
    }
}

impl Default for ServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// 簡単なHTTPサーバー起動関数
pub async fn run_server<P: AsRef<Path>>(
    config_path: P,
    mvcc: Arc<MVCCManager>,
    merkle: Arc<MerkleDAG>,
    rewrite_engine: Arc<RewriteEngine>,
) -> Result<()> {
    let mut server = HttpServer::from_config_file(
        config_path,
        mvcc,
        merkle,
        rewrite_engine,
    ).await?;

    server.start().await?;
    server.run().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_storage::prelude::*;
    use kotoba_rewrite::prelude::*;

    #[test]
    fn test_parse_http_request() {
        let request_str = "GET /ping?key=value HTTP/1.1\r\n\
                          Host: localhost:8080\r\n\
                          User-Agent: test\r\n\
                          \r\n\
                          test body";

        let request = HttpServer::parse_http_request(request_str).unwrap();

        assert_eq!(request.method, "GET");
        assert_eq!(request.path, "/ping");
        assert_eq!(request.query_string, Some("key=value".to_string()));
        assert_eq!(request.headers.len(), 2);
        assert_eq!(request.body, b"test body");
    }

    #[tokio::test]
    async fn test_server_builder() {
        let mvcc = Arc::new(MVCCManager::new());
        let merkle = Arc::new(MerkleDAG::new());
        let rewrite_engine = Arc::new(RewriteEngine::new());

        let config = HttpConfig::new(ServerConfig::default());
        let server = ServerBuilder::new()
            .with_config(config)
            .build(mvcc, merkle, rewrite_engine)
            .await
            .unwrap();

        let status = server.get_status().await;
        assert_eq!(status.host, "127.0.0.1");
        assert_eq!(status.port, 8080);
        assert_eq!(status.requests_processed, 0);
    }
}

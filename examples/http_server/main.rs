//! Web Framework HTTP Server Example
//!
//! この例はWeb Frameworkを使ってシンプルなHTTPサーバーを起動する方法を示します。

use kotoba::frontend::WebFramework;
use kotoba::frontend::api_ir::WebFrameworkConfigIR;
use kotoba::http::{HttpRequest, HttpResponse, HttpMethod, HttpHeaders};
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロガーの初期化
    env_logger::init();

    println!("🚀 Starting Kotoba Web Framework HTTP Server Example");

    // Web Frameworkの設定を作成
    let config = WebFrameworkConfigIR {
        server: kotoba::frontend::api_ir::ServerConfig {
            host: "localhost".to_string(),
            port: 3000,
            tls: None,
            workers: 4,
            max_connections: 1000,
        },
        database: None,
        api_routes: Vec::new(),
        web_sockets: Vec::new(),
        graph_ql: None,
        middlewares: Vec::new(),
        static_files: Vec::new(),
        authentication: None,
        session: None,
    };

    // Web Frameworkを作成
    let framework = Arc::new(WebFramework::new(config)?);
    println!("📄 Web Framework initialized");

    // TCPリスナーを開始
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("🌐 Server listening on http://127.0.0.1:3000");
    println!("Press Ctrl+C to stop the server");

    loop {
        let (mut socket, _) = listener.accept().await?;
        let framework = Arc::clone(&framework);

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap();

            if n == 0 {
                return;
            }

            // シンプルなHTTPリクエストのパース（本番では適切なパーサーを使用）
            let request_str = String::from_utf8_lossy(&buf[..n]);
            let path = if request_str.starts_with("GET ") {
                let line = request_str.lines().next().unwrap_or("");
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    parts[1].to_string()
                } else {
                    "/".to_string()
                }
            } else {
                "/".to_string()
            };

            // HttpRequestを作成
            let request = HttpRequest {
                id: format!("req_{}", uuid::Uuid::new_v4()),
                method: HttpMethod::GET,
                path,
                query: std::collections::HashMap::new(),
                headers: HttpHeaders::new(),
                body_ref: None,
                timestamp: 1234567890,
            };

            // Web Frameworkでリクエストを処理
            match framework.handle_request(request).await {
                Ok(response) => {
                    // HTTPレスポンスを送信
                    let response_str = format!(
                        "HTTP/1.1 {} {}\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                        if response.status.code == 200 { "200" } else { "404" },
                        response.status.reason,
                        if let Some(ref body) = response.body_ref {
                            // 実際の実装ではbody_refからコンテンツを取得
                            "<html><body><h1>Hello from Web Framework!</h1></body></html>".len()
                        } else {
                            0
                        },
                        if let Some(_) = response.body_ref {
                            "<html><body><h1>Hello from Web Framework!</h1></body></html>"
                        } else {
                            ""
                        }
                    );

                    let _ = socket.write_all(response_str.as_bytes()).await;
                }
                Err(_) => {
                    let error_response = "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\nContent-Length: 21\r\n\r\nInternal Server Error";
                    let _ = socket.write_all(error_response.as_bytes()).await;
                }
            }
        });
    }
}
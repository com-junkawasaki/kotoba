//! Kotoba Web Framework Example
//!
//! JsonnetベースのフルスタックWebフレームワークの使用例

use kotoba::frontend::*;
use kotoba::frontend::api_ir::{WebFrameworkConfigIR, ServerConfig};
use kotoba::Properties;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Kotoba Web Framework Example");

    // Web Frameworkの設定を作成
    let web_config = create_default_config();

    // WebFrameworkを作成
    let framework = Arc::new(WebFramework::new(web_config)?);

    // TCPリスナーを開始
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("🌐 Server listening on http://127.0.0.1:3000");

    loop {
        let (mut socket, _) = listener.accept().await?;
        let framework = Arc::clone(&framework);

        tokio::spawn(async move {
            let mut buf = [0; 1024];
            let n = socket.read(&mut buf).await.unwrap_or(0);

            if n > 0 {
                let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n<html><body><h1>Kotoba Web Framework Works!</h1></body></html>";
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
    }
}

/// デフォルト設定を作成
fn create_default_config() -> WebFrameworkConfigIR {
    WebFrameworkConfigIR {
        server: ServerConfig {
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
    }
}

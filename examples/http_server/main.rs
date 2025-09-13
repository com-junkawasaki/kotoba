//! HTTP Server Example
//!
//! この例は.kotoba.jsonファイルを使ってHTTPサーバーを起動する方法を示します。

use kotoba::http::server::HttpServer;
use kotoba::storage::{InMemoryMVCCManager, InMemoryMerkleDAGManager};
use kotoba::rewrite::RewriteEngine;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ロガーの初期化
    env_logger::init();

    println!("🚀 Starting Kotoba HTTP Server Example");

    // ストレージとエンジンの初期化
    let mvcc = Arc::new(InMemoryMVCCManager::new());
    let merkle = Arc::new(InMemoryMerkleDAGManager::new());
    let rewrite_engine = Arc::new(RewriteEngine::new(
        Arc::clone(&mvcc),
        Arc::clone(&merkle),
    ));

    // 設定ファイルからサーバーを作成
    let config_path = "examples/http_server/config.kotoba.json";
    println!("📄 Loading configuration from: {}", config_path);

    let mut server = HttpServer::from_config_file(
        config_path,
        mvcc,
        merkle,
        rewrite_engine,
    ).await?;

    // サーバーを起動
    server.start().await?;

    // サーバーの情報を表示
    let status = server.get_status().await;
    println!("✅ Server started successfully!");
    println!("📡 Listening on {}:{}", status.host, status.port);
    println!("📊 Routes: {}, Middlewares: {}", status.routes_count, status.middlewares_count);
    println!("🔗 Available endpoints:");
    println!("   GET  /ping");
    println!("   GET  /health");
    println!("   GET  /api/v1/status");
    println!("   POST /api/v1/echo");
    println!("   GET  /api/v1/users/{{id}}");
    println!("   GET  /api/v1/posts");
    println!();
    println!("💡 Try: curl http://{}:{}/ping", status.host, status.port);
    println!("🔄 Ready to accept connections...");

    // メインループを実行
    server.run().await?;

    Ok(())
}

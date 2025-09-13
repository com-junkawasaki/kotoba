//! Kotoba HTTP Server
//!
//! このバイナリは.kotoba.jsonまたは.kotobaファイルで設定されたHTTPサーバーを起動します。

use clap::{Arg, Command};
use kotoba::http::{HttpConfig, ServerConfig, HttpRoute, HttpMiddleware, HttpMethod};
use kotoba::http::server::{HttpServer, ServerBuilder};
use kotoba::storage::{MVCCManager, MerkleDAG};
use kotoba::rewrite::RewriteEngine;
use std::sync::Arc;
use std::path::Path;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let matches = Command::new("kotoba-server")
        .version("0.1.0")
        .author("Kotoba Team")
        .about("Kotoba GP2 Graph Rewrite HTTP Server")
        .arg(
            Arg::new("config")
                .short('c')
                .long("config")
                .value_name("FILE")
                .help("Sets the configuration file (.kotoba.json or .kotoba)")
                .required(true),
        )
        .arg(
            Arg::new("host")
                .short('h')
                .long("host")
                .value_name("HOST")
                .help("Sets the server host (default: 127.0.0.1)")
                .default_value("127.0.0.1"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .help("Sets the server port (default: 8080)")
                .default_value("8080"),
        )
        .arg(
            Arg::new("example")
                .long("example")
                .value_name("TYPE")
                .help("Creates and runs an example server (ping|api)")
                .possible_values(["ping", "api"]),
        )
        .get_matches();

    // ストレージとエンジンの初期化
    let mvcc = Arc::new(InMemoryMVCCManager::new());
    let merkle = Arc::new(InMemoryMerkleDAGManager::new());
    let rewrite_engine = Arc::new(RewriteEngine::new(Arc::clone(&mvcc), Arc::clone(&merkle)));

    let mut server = if let Some(example) = matches.get_one::<String>("example") {
        // 例のサーバーを作成
        create_example_server(example, mvcc, merkle, rewrite_engine).await?
    } else if let Some(config_path) = matches.get_one::<String>("config") {
        // 設定ファイルからサーバーを作成
        println!("Loading configuration from: {}", config_path);
        ServerBuilder::new()
            .with_config_file(config_path)
            .build(mvcc, merkle, rewrite_engine)
            .await?
    } else {
        return Err("Either config file or example must be specified".into());
    };

    // サーバーを起動
    println!("🚀 Starting Kotoba HTTP Server...");
    server.start().await?;
    println!("✅ Server started successfully!");

    // サーバーの情報を表示
    let status = server.get_status().await;
    println!("📡 Listening on {}:{}", status.host, status.port);
    println!("📊 Routes: {}, Middlewares: {}", status.routes_count, status.middlewares_count);

    // メインループを実行
    println!("🔄 Ready to accept connections...");
    server.run().await?;

    Ok(())
}

/// 例のサーバーを作成
async fn create_example_server(
    example_type: &str,
    mvcc: Arc<MVCCManager>,
    merkle: Arc<MerkleDAG>,
    rewrite_engine: Arc<RewriteEngine>,
) -> Result<HttpServer, Box<dyn std::error::Error>> {
    match example_type {
        "ping" => create_ping_server(mvcc, merkle, rewrite_engine).await,
        "api" => create_api_server(mvcc, merkle, rewrite_engine).await,
        _ => Err("Unknown example type".into()),
    }
}

/// シンプルなpingサーバーを作成
async fn create_ping_server(
    mvcc: Arc<MVCCManager>,
    merkle: Arc<MerkleDAG>,
    rewrite_engine: Arc<RewriteEngine>,
) -> Result<HttpServer, Box<dyn std::error::Error>> {
    println!("Creating ping example server...");

    let mut config = HttpConfig::new(ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        max_connections: Some(100),
        timeout_ms: Some(30000),
        tls: None,
    });

    // pingルートを追加
    let ping_route = HttpRoute::new(
        "ping_get".to_string(),
        HttpMethod::GET,
        "/ping".to_string(),
        kotoba::types::ContentHash::Sha256([1; 32]), // ダミーハッシュ
    );
    config.routes.push(ping_route);

    // ミドルウェアを追加
    let logger_middleware = HttpMiddleware::new(
        "logger".to_string(),
        "request_logger".to_string(),
        100,
        kotoba::types::ContentHash::Sha256([2; 32]), // ダミーハッシュ
    );
    config.middlewares.push(logger_middleware);

    HttpServer::new(config, mvcc, merkle, rewrite_engine).await
        .map_err(|e| e.into())
}

/// APIサーバーを作成
async fn create_api_server(
    mvcc: Arc<MVCCManager>,
    merkle: Arc<MerkleDAG>,
    rewrite_engine: Arc<RewriteEngine>,
) -> Result<HttpServer, Box<dyn std::error::Error>> {
    println!("Creating API example server...");

    let mut config = HttpConfig::new(ServerConfig {
        host: "127.0.0.1".to_string(),
        port: 8080,
        max_connections: Some(1000),
        timeout_ms: Some(30000),
        tls: None,
    });

    // 複数のルートを追加
    let routes = vec![
        (HttpMethod::GET, "/health", "health_check"),
        (HttpMethod::GET, "/api/v1/status", "api_status"),
        (HttpMethod::POST, "/api/v1/echo", "echo_handler"),
        (HttpMethod::GET, "/api/v1/users/{id}", "get_user"),
    ];

    for (method, pattern, handler) in routes {
        let route_id = format!("{}_{}", method, pattern.replace('/', "_").replace('{', "").replace('}', ""));
        let route = HttpRoute::new(
            route_id,
            method,
            pattern.to_string(),
            kotoba::types::ContentHash::Sha256([3; 32]), // ダミーハッシュ
        );
        config.routes.push(route);
    }

    // 複数のミドルウェアを追加
    let middlewares = vec![
        ("cors", "cors_middleware", 50),
        ("auth", "auth_middleware", 75),
        ("logger", "request_logger", 100),
        ("rate_limit", "rate_limiter", 25),
    ];

    for (name, function, order) in middlewares {
        let middleware = HttpMiddleware::new(
            name.to_string(),
            function.to_string(),
            order,
            kotoba::types::ContentHash::Sha256([4; 32]), // ダミーハッシュ
        );
        config.middlewares.push(middleware);
    }

    HttpServer::new(config, mvcc, merkle, rewrite_engine).await
        .map_err(|e| e.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_ping_server() {
        let mvcc = Arc::new(MVCCManager::new());
        let merkle = Arc::new(MerkleDAGManager::new());
        let rewrite_engine = Arc::new(RewriteEngine::new(Arc::clone(&mvcc), Arc::clone(&merkle)));

        let server = create_ping_server(mvcc, merkle, rewrite_engine).await.unwrap();
        let status = server.get_status().await;

        assert_eq!(status.routes_count, 1);
        assert_eq!(status.middlewares_count, 1);
        assert_eq!(status.port, 8080);
    }

    #[tokio::test]
    async fn test_create_api_server() {
        let mvcc = Arc::new(MVCCManager::new());
        let merkle = Arc::new(MerkleDAGManager::new());
        let rewrite_engine = Arc::new(RewriteEngine::new(Arc::clone(&mvcc), Arc::clone(&merkle)));

        let server = create_api_server(mvcc, merkle, rewrite_engine).await.unwrap();
        let status = server.get_status().await;

        assert_eq!(status.routes_count, 4);
        assert_eq!(status.middlewares_count, 4);
        assert_eq!(status.port, 8080);
    }
}

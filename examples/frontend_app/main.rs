//! Kotoba Web Framework Example
//!
//! JsonnetベースのフルスタックWebフレームワークの使用例

use kotoba::frontend::*;
use kotoba::frontend::api_ir::{WebFrameworkConfigIR, ServerConfig};
use kotoba::http::{HttpRequest, HttpMethod, HttpHeaders};
use kotoba::Properties;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Kotoba Web Framework Example");

    // Web Frameworkの設定を作成
    println!("📄 Creating Web Framework configuration...");
    let web_config = create_default_config();
    println!("✅ Configuration created");

    // WebFrameworkを作成
    println!("🔧 Initializing WebFramework...");
    let framework = Arc::new(WebFramework::new(web_config)?);
    println!("✅ WebFramework initialized");

    // TCPリスナーを開始
    println!("🔌 Starting TCP listener...");
    let listener = TcpListener::bind("127.0.0.1:3000").await?;
    println!("🌐 Frontend App Server listening on http://127.0.0.1:3000");
    println!("Press Ctrl+C to stop the server");

    println!("🚀 Starting main loop...");
    loop {
        println!("⏳ Waiting for connection...");
        match listener.accept().await {
            Ok((mut socket, addr)) => {
                println!("📡 New connection from: {:?}", addr);
                let framework = Arc::clone(&framework);

                tokio::spawn(async move {
            let mut buf = [0; 1024];
            match socket.read(&mut buf).await {
                Ok(n) => {
                    if n == 0 {
                        return;
                    }

                    // シンプルなHTTPリクエストのパース
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
                    let request = kotoba::http::HttpRequest {
                        id: format!("req_{}", uuid::Uuid::new_v4()),
                        method: kotoba::http::HttpMethod::GET,
                        path,
                        query: std::collections::HashMap::new(),
                        headers: kotoba::http::HttpHeaders::new(),
                        body_ref: None,
                        timestamp: 1234567890,
                    };

                    // Web Frameworkでリクエストを処理
                    match framework.handle_request(request).await {
                        Ok(response) => {
                            // HTTPレスポンスを送信
                            let response_str = format!(
                                "HTTP/1.1 {} {}\r\nContent-Type: text/html\r\n\r\n{}",
                                if response.status.code == 200 { "200" } else { "404" },
                                response.status.reason,
                                if response.status.code == 200 {
                                    "<html><head><title>Kotoba Web Framework</title></head><body><h1>Welcome to Kotoba Web Framework!</h1><p>This is a Next.js-like framework built with Rust.</p></body></html>"
                                } else {
                                    "<html><body><h1>404 Not Found</h1></body></html>"
                                }
                            );

                            let _ = socket.write_all(response_str.as_bytes()).await;
                        }
                        Err(e) => {
                            eprintln!("Request handling error: {:?}", e);
                            let error_response = "HTTP/1.1 500 Internal Server Error\r\nContent-Type: text/plain\r\n\r\nInternal Server Error";
                            let _ = socket.write_all(error_response.as_bytes()).await;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Socket read error: {:?}", e);
                }
            }
        });
    }

    Ok(())
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
        database: Some(DatabaseIR {
            connection_string: "postgresql://user:pass@localhost/kotoba_app".to_string(),
            db_type: DatabaseType::PostgreSQL,
            models: Vec::new(),
            migrations: Vec::new(),
        }),
        api_routes: Vec::new(),
        web_sockets: Vec::new(),
        graph_ql: None,
        middlewares: vec![
            MiddlewareIR {
                name: "cors".to_string(),
                middleware_type: MiddlewareType::CORS,
                config: Properties::new(),
                order: 1,
            },
            MiddlewareIR {
                name: "auth".to_string(),
                middleware_type: MiddlewareType::Authentication,
                config: Properties::new(),
                order: 2,
            },
        ],
        static_files: vec![
            StaticFilesConfig {
                route: "/static".to_string(),
                directory: "./public".to_string(),
                cache_control: Some("public, max-age=31536000".to_string()),
                gzip: true,
            },
        ],
        authentication: None,
        session: None,
    }
}

/// 開発サーバーを起動
async fn start_dev_server(framework: WebFramework) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::net::TcpListener;

    let addr = format!("{}:{}", framework.get_config().server.host, framework.get_config().server.port);
    let listener = TcpListener::bind(&addr).await?;
    println!("🚀 Development server listening on http://{}", addr);

    loop {
        let (socket, _) = listener.accept().await?;
        let framework = Arc::clone(&framework);

        tokio::spawn(async move {
            // TODO: HTTPリクエストのパースと処理を実装
            // 現在は簡易的なエコーバック
            let mut buf = [0; 1024];
            socket.readable().await.unwrap();
            match socket.try_read(&mut buf) {
                Ok(n) if n > 0 => {
                    let request = String::from_utf8_lossy(&buf[..n]);
                    println!("📨 Received request: {} bytes", n);

                    // 簡易的なHTTPレスポンス
                    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello from Kotoba Web Framework!\r\n";
                    socket.writable().await.unwrap();
                    socket.try_write(response.as_bytes()).unwrap();
                }
                _ => {}
            }
        });
    }
}

use std::io::AsyncReadExt;
use std::io::AsyncWriteExt;

/// アプリの構造を設定
async fn setup_app_structure(framework: &WebFramework) -> Result<(), Box<dyn std::error::Error>> {
    println!("📂 Setting up app structure...");

    // JsonnetファイルからルートとAPIルートを読み込み
    let route_files = vec![
        ("app/layout.libsonnet", "/"),
        ("app/page.libsonnet", "/"),
        ("app/dashboard/layout.libsonnet", "/dashboard"),
        ("app/dashboard/page.libsonnet", "/dashboard"),
        ("app/blog/[slug]/page.libsonnet", "/blog/[slug]"),
        ("app/(auth)/login/page.libsonnet", "/login"),
    ];

    let mut route_count = 0;
    let mut api_count = 0;

    // ページルートの設定
    for (file_path, base_path) in &route_files {
        let full_path = format!("examples/frontend_app/{}", file_path);
        if Path::new(&full_path).exists() {
            println!("  📄 Loading route: {}", file_path);

            // 簡略化：実際にはJsonnetファイルをパース
            let route_path = if *base_path == "/" && !file_path.contains("page.libsonnet") {
                "/".to_string()
            } else if file_path.contains("[slug]") {
                "/blog/:slug".to_string()
            } else if file_path.contains("(auth)") {
                "/login".to_string()
            } else {
                base_path.to_string()
            };

            let mut route = RouteIR::new(route_path);

            // コンポーネントタイプに基づいて適切なコンポーネントを設定
            if file_path.contains("layout.libsonnet") {
                let layout = ComponentIR::new(
                    format!("Layout_{}", base_path.replace("/", "_")),
                    ComponentType::Layout,
                );
                route.set_layout(layout);
            } else if file_path.contains("page.libsonnet") {
                let page = ComponentIR::new(
                    format!("Page_{}", base_path.replace("/", "_").replace("[", "").replace("]", "").replace("(", "").replace(")", "")),
                    ComponentType::Page,
                );
                route.set_page(page);
            }

            framework.add_route(route).await?;
            route_count += 1;
        }
    }

    // APIルートの設定
    let api_files = vec![
        "app/api/users.libsonnet",
    ];

    for file_path in &api_files {
        let full_path = format!("examples/frontend_app/{}", file_path);
        if Path::new(&full_path).exists() {
            println!("  🔌 Loading API routes: {}", file_path);

            // 簡略化：実際にはJsonnetファイルからAPIルートをパース
            // ここではサンプルAPIルートを作成
            let api_route = ApiRouteIR {
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
                    query_params: Vec::new(),
                    body_params: None,
                    headers: Vec::new(),
                },
                metadata: ApiMetadata {
                    description: Some("Get users list".to_string()),
                    summary: Some("Users API".to_string()),
                    tags: vec!["users".to_string()],
                    deprecated: false,
                    rate_limit: None,
                    cache: None,
                },
            };

            // TODO: API route registration not implemented yet
            // framework.add_api_route(api_route).await?;
            api_count += 1;
        }
    }

    println!("✅ App structure configured:");
    println!("  - Routes: {}", route_count);
    println!("  - API routes: {}", api_count);
    Ok(())
}

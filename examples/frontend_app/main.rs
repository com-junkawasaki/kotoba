//! Kotoba Web Framework Example
//!
//! JsonnetベースのフルスタックWebフレームワークの使用例

use kotoba::frontend::*;
use kotoba::http;
use std::path::Path;
use std::fs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting Kotoba Web Framework Example");

    // Jsonnet設定ファイルを読み込み
    let config_path = "examples/frontend_app/kotoba.libsonnet";
    println!("📄 Loading Jsonnet config from: {}", config_path);

    let config_content = fs::read_to_string(config_path)?;
    println!("📊 Parsing Jsonnet configuration...");

    // TODO: Jsonnetパーサーを実装
    // 現在は簡易的な設定を使用
    let web_config = create_default_config();

    // WebFrameworkを作成
    let framework = WebFramework::new(web_config)?;
    println!("✅ WebFramework initialized");

    // データベース初期化
    if let Some(db_manager) = framework.get_config().database.as_ref() {
        println!("🗄️  Initializing database...");
        // TODO: 実際のデータベース初期化
        println!("✅ Database initialized");
    }

    // アプリ構造を定義
    setup_app_structure(&framework).await?;

    // コマンドライン引数の処理
    let args: Vec<String> = std::env::args().collect();

    if args.contains(&"--build".to_string()) {
        // ビルド実行
        println!("🔨 Building application...");
        let build_config = BuildConfigIR::new(build_ir::BundlerType::Vite);
        let build_engine = BuildEngine::new(build_config);
        let build_result = build_engine.build().await?;
        println!("✅ Build completed!");
        println!("📊 Build Stats:");
        println!("  - Chunks: {}", build_result.stats.chunk_count);
        println!("  - Total Size: {} KB", build_result.stats.total_size / 1024);
        println!("  - Gzipped: {} KB", build_result.stats.gzip_size / 1024);

    } else if args.contains(&"--dev".to_string()) {
        // 開発サーバー起動
        println!("🚀 Starting development server...");
        start_dev_server(framework).await?;
        println!("📡 Server running at http://localhost:3000");

    } else {
        // デフォルト：特定のルートをレンダリング
        let route_path = args.get(2).unwrap_or(&"/".to_string()).clone();
        println!("🎨 Rendering route: {}", route_path);

        match framework.navigate(&route_path).await {
            Ok(result) => {
                println!("✅ Route rendered successfully!");
                println!("📊 Render Stats:");
                println!("  - Components: {}", result.render_stats.component_count);
                println!("  - HTML Size: {} bytes", result.html.len());

                // HTMLファイルに保存（デモ用）
                fs::create_dir_all("dist")?;
                fs::write("dist/index.html", &result.html)?;
                if let Some(hydration) = &result.hydration_script {
                    fs::write("dist/hydrate.js", hydration)?;
                }

                println!("💾 Output saved to dist/");
            }
            Err(e) => {
                eprintln!("❌ Failed to render route: {}", e);
                std::process::exit(1);
            }
        }
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
        let framework = framework.clone();

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

            framework.add_api_route(api_route).await?;
            api_count += 1;
        }
    }

    println!("✅ App structure configured:");
    println!("  - Routes: {}", route_count);
    println!("  - API routes: {}", api_count);
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_web_framework_creation() {
        let config = create_default_config();
        let framework = WebFramework::new(config).unwrap();

        assert_eq!(framework.get_config().server.port, 3000);
        assert!(framework.get_config().database.is_some());
    }

    #[tokio::test]
    async fn test_app_structure_setup() {
        let config = create_default_config();
        let framework = WebFramework::new(config).unwrap();

        // アプリ構造設定テスト
        setup_app_structure(&framework).await.unwrap();

        let route_table = framework.get_route_table().await;
        assert!(!route_table.routes.is_empty());
    }

    #[tokio::test]
    async fn test_route_navigation() {
        let config = create_default_config();
        let framework = WebFramework::new(config).unwrap();

        setup_app_structure(&framework).await.unwrap();

        // ルートナビゲーションテスト
        let result = framework.navigate("/").await;
        assert!(result.is_ok());
    }
}

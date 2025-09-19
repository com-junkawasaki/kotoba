//! ドキュメントサーバーモジュール

use super::{DocsConfig, Result, DocsError, DocItem};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::{
    cors::CorsLayer,
    services::ServeDir,
    trace::TraceLayer,
};
use tokio::sync::RwLock;

/// サーバー状態
#[derive(Clone)]
pub struct ServerState {
    config: DocsConfig,
    items: Arc<RwLock<Vec<DocItem>>>,
    item_map: Arc<RwLock<HashMap<String, DocItem>>>,
}

/// ドキュメントサーバー
pub struct DocServer {
    state: ServerState,
    app: Router,
}

impl DocServer {
    /// 新しいサーバーを作成
    pub fn new(config: DocsConfig, items: Vec<DocItem>) -> Self {
        let mut item_map = HashMap::new();
        for item in &items {
            item_map.insert(item.id.clone(), item.clone());
        }

        let state = ServerState {
            config: config.clone(),
            items: Arc::new(RwLock::new(items)),
            item_map: Arc::new(RwLock::new(item_map)),
        };

        let app = Self::create_router(state.clone());

        Self { state, app }
    }

    /// ルーターを作成
    fn create_router(state: ServerState) -> Router {
        Router::new()
            .route("/", get(Self::index_handler))
            .route("/search", get(Self::search_handler))
            .route("/api/items", get(Self::api_items_handler))
            .route("/api/item/:id", get(Self::api_item_handler))
            .route("/api/stats", get(Self::api_stats_handler))
            .route("/api/search", post(Self::api_search_handler))
            .nest_service("/static", ServeDir::new(Self::get_static_dir(&state.config)))
            .with_state(state)
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .layer(CorsLayer::permissive())
            )
    }

    /// サーバーを起動
    pub async fn serve(self, host: &str, port: u16) -> Result<()> {
        let addr = format!("{}:{}", host, port)
            .parse::<SocketAddr>()
            .map_err(|e| DocsError::Server(format!("Invalid address: {}", e)))?;

        println!("🚀 Starting documentation server...");
        println!("📍 Server running at http://{}", addr);
        println!("📁 Serving docs from: {}", self.state.config.output_dir.display());
        println!("🔍 Search API available at http://{}/api/search", addr);
        println!("🛑 Press Ctrl+C to stop");

        let listener = tokio::net::TcpListener::bind(addr).await
            .map_err(|e| DocsError::Server(format!("Failed to bind to {}: {}", addr, e)))?;

        axum::serve(listener, self.app)
            .await
            .map_err(|e| DocsError::Server(format!("Server error: {}", e)))?;

        Ok(())
    }

    /// インデックスページハンドラー
    async fn index_handler(State(state): State<ServerState>) -> std::result::Result<Html<String>, StatusCode> {
        let items = state.items.read().await;
        let html = Self::generate_index_html(&state.config, &items).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Html(html))
    }

    /// 検索ページハンドラー
    async fn search_handler(
        State(state): State<ServerState>,
        Query(params): Query<SearchParams>,
    ) -> std::result::Result<Html<String>, StatusCode> {
        let query = params.q.unwrap_or_default();
        let items = state.items.read().await;

        let results: Vec<_> = if query.is_empty() {
            items.iter().take(20).cloned().collect()
        } else {
            items.iter()
                .filter(|item|
                    item.name.to_lowercase().contains(&query.to_lowercase()) ||
                    item.content.to_lowercase().contains(&query.to_lowercase())
                )
                .take(50)
                .cloned()
                .collect()
        };

        let html = Self::generate_search_html(&state.config, &results, &query).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Html(html))
    }

    /// API: 全項目を取得
    async fn api_items_handler(State(state): State<ServerState>) -> Json<Vec<DocItem>> {
        let items = state.items.read().await;
        Json(items.clone())
    }

    /// API: 特定の項目を取得
    async fn api_item_handler(
        State(state): State<ServerState>,
        Path(id): Path<String>,
    ) -> std::result::Result<Json<DocItem>, StatusCode> {
        let item_map = state.item_map.read().await;

        if let Some(item) = item_map.get(&id) {
            Ok(Json(item.clone()))
        } else {
            Err(StatusCode::NOT_FOUND)
        }
    }

    /// API: 統計情報を取得
    async fn api_stats_handler(State(state): State<ServerState>) -> Json<ServerStats> {
        let items = state.items.read().await;
        let stats = Self::generate_stats(&items);
        Json(stats)
    }

    /// API: 検索を実行
    async fn api_search_handler(
        State(state): State<ServerState>,
        Json(search_request): Json<SearchRequest>,
    ) -> Json<SearchResponse> {
        let items = state.items.read().await;
        let query = search_request.query.to_lowercase();

        let results: Vec<SearchResult> = items.iter()
            .filter(|item|
                item.name.to_lowercase().contains(&query) ||
                item.content.to_lowercase().contains(&query)
            )
            .take(search_request.limit.unwrap_or(50))
            .map(|item| SearchResult {
                id: item.id.clone(),
                name: item.name.clone(),
                doc_type: format!("{:?}", item.doc_type),
                content: item.content.clone(),
                url: format!("/item/{}", item.id),
            })
            .collect();

        Json(SearchResponse {
            query: search_request.query,
            total_results: results.len(),
            results,
        })
    }

    /// インデックスHTMLを生成
    async fn generate_index_html(config: &DocsConfig, items: &[DocItem]) -> Result<String> {
        let mut html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="/static/style.css">
</head>
<body>
    <div class="container">
        <header>
            <h1>{}</h1>
            <p>{}</p>
            <div class="search-box">
                <input type="text" id="search-input" placeholder="Search documentation...">
                <div id="search-results"></div>
            </div>
        </header>

        <nav class="sidebar">
            <h2>Navigation</h2>
            <ul>
                <li><a href="/">Home</a></li>
                <li><a href="/api/items">API</a></li>
                <li><a href="/search">Search</a></li>
            </ul>
        </nav>

        <main class="content">
            <h2>Documentation</h2>
            <div class="stats">
                <p>Total items: {}</p>
            </div>

            <div class="items-grid">
"#,
            config.name,
            config.name,
            config.description.as_ref().unwrap_or(&"Documentation".to_string()),
            items.len()
        );

        // 項目をグループ化して表示
        let mut grouped: HashMap<String, Vec<&DocItem>> = HashMap::new();
        for item in items {
            let type_name = format!("{:?}", item.doc_type);
            grouped.entry(type_name).or_insert_with(Vec::new).push(item);
        }

        for (type_name, type_items) in grouped {
            html.push_str(&format!("<h3>{}</h3>\n<ul>\n", type_name));
            for item in type_items.iter().take(10) {
                html.push_str(&format!(
                    r#"<li><a href="/item/{}">{}</a></li>"#,
                    item.id, item.name
                ));
            }
            if type_items.len() > 10 {
                html.push_str(&format!("<li>... and {} more</li>\n", type_items.len() - 10));
            }
            html.push_str("</ul>\n");
        }

        html.push_str(
            r#"
            </div>
        </main>
    </div>

    <script src="/static/script.js"></script>
</body>
</html>"#
        );

        Ok(html)
    }

    /// 検索HTMLを生成
    async fn generate_search_html(
        config: &DocsConfig,
        results: &[DocItem],
        query: &str
    ) -> Result<String> {
        let mut html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Search - {}</title>
    <link rel="stylesheet" href="/static/style.css">
</head>
<body>
    <div class="container">
        <header>
            <h1>Search Results</h1>
            <p>Query: "{}" - Found {} results</p>
            <a href="/">← Back to home</a>
        </header>

        <main class="content">
            <div class="search-results">
"#,
            config.name, query, results.len()
        );

        for item in results {
            html.push_str(&format!(
                r#"
                <div class="search-result">
                    <h3><a href="/item/{}">{}</a></h3>
                    <p class="doc-type">{:?}</p>
                    <p>{}</p>
                </div>
                "#,
                item.id,
                item.name,
                item.doc_type,
                item.content.chars().take(200).collect::<String>()
            ));
        }

        html.push_str(
            r#"
            </div>
        </main>
    </div>
</body>
</html>"#
        );

        Ok(html)
    }

    /// 統計情報を生成
    fn generate_stats(items: &[DocItem]) -> ServerStats {
        let mut stats = ServerStats::default();

        for item in items {
            match item.doc_type {
                super::DocType::Module => stats.modules += 1,
                super::DocType::Function => stats.functions += 1,
                super::DocType::Struct => stats.structs += 1,
                super::DocType::Enum => stats.enums += 1,
                super::DocType::Trait => stats.traits += 1,
                super::DocType::Constant => stats.constants += 1,
                super::DocType::Macro => stats.macros += 1,
                super::DocType::TypeAlias => stats.type_aliases += 1,
                super::DocType::Method => stats.methods += 1,
                super::DocType::Field => stats.fields += 1,
                super::DocType::Variant => stats.variants += 1,
                super::DocType::AssociatedType => stats.associated_types += 1,
                super::DocType::AssociatedConstant => stats.associated_constants += 1,
            }
        }

        stats.total = items.len();
        stats
    }

    /// 静的ファイルディレクトリを取得
    fn get_static_dir(config: &DocsConfig) -> PathBuf {
        config.output_dir.join("html")
    }
}

/// 検索パラメータ
#[derive(Deserialize)]
struct SearchParams {
    q: Option<String>,
}

/// 検索リクエスト
#[derive(Deserialize)]
struct SearchRequest {
    query: String,
    limit: Option<usize>,
}

/// 検索レスポンス
#[derive(Serialize)]
struct SearchResponse {
    query: String,
    total_results: usize,
    results: Vec<SearchResult>,
}

/// 検索結果
#[derive(Serialize)]
struct SearchResult {
    id: String,
    name: String,
    doc_type: String,
    content: String,
    url: String,
}

/// サーバー統計
#[derive(Serialize, Default)]
struct ServerStats {
    total: usize,
    modules: usize,
    functions: usize,
    structs: usize,
    enums: usize,
    traits: usize,
    constants: usize,
    macros: usize,
    type_aliases: usize,
    methods: usize,
    fields: usize,
    variants: usize,
    associated_types: usize,
    associated_constants: usize,
}

/// 開発サーバーを起動するユーティリティ関数
pub async fn serve_docs(config: DocsConfig, items: Vec<DocItem>) -> Result<()> {
    let server = DocServer::new(config.clone(), items);
    server.serve(&config.server.host, config.server.port).await
}

/// デフォルトの静的ファイルを提供するサーバーを起動
pub async fn serve_static(dir: PathBuf, host: &str, port: u16) -> Result<()> {
    let addr = format!("{}:{}", host, port)
        .parse::<SocketAddr>()
        .map_err(|e| DocsError::Server(format!("Invalid address: {}", e)))?;

    let app = Router::new()
        .nest_service("/", ServeDir::new(dir))
        .layer(CorsLayer::permissive());

    println!("🚀 Starting static file server...");
    println!("📍 Server running at http://{}", addr);
    println!("🛑 Press Ctrl+C to stop");

    let listener = tokio::net::TcpListener::bind(addr).await
        .map_err(|e| DocsError::Server(format!("Failed to bind to {}: {}", addr, e)))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| DocsError::Server(format!("Server error: {}", e)))?;

    Ok(())
}

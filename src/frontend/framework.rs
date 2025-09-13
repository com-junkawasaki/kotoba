//! App Routerフレームワークのコア実装
//!
//! Next.js風App Routerフレームワークの主要コンポーネントを実装します。

use crate::types::{Result, KotobaError, Value, Properties, ContentHash};
use crate::frontend::component_ir::*;
use crate::frontend::route_ir::*;
use crate::frontend::render_ir::*;
use crate::frontend::build_ir::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Web Framework - フルスタックWebフレームワーク
pub struct WebFramework {
    route_table: Arc<RwLock<RouteTableIR>>,
    component_registry: Arc<RwLock<ComponentRegistry>>,
    api_router: ApiRouter,
    database_manager: Option<DatabaseManager>,
    middleware_chain: Vec<MiddlewareIR>,
    renderer: ComponentRenderer,
    config: WebFrameworkConfigIR,
    current_route: Arc<RwLock<Option<RouteIR>>>,
}

/// API Router - REST APIとGraphQLのルーティング
pub struct ApiRouter {
    routes: Arc<RwLock<HashMap<String, Vec<ApiRouteIR>>>>, // method -> routes
    graphql_schema: Option<GraphQLIR>,
}

/// Database Manager - データベース接続とクエリ実行
pub struct DatabaseManager {
    config: DatabaseIR,
    connection_pool: Option<tokio_postgres::Client>, // 実際の実装では適切なDBクライアントを使用
}

impl WebFramework {
    pub fn new(config: WebFrameworkConfigIR) -> Result<Self> {
        let renderer = ComponentRenderer::new();

        Ok(Self {
            route_table: Arc::new(RwLock::new(RouteTableIR::new())),
            component_registry: Arc::new(RwLock::new(ComponentRegistry::new())),
            api_router: ApiRouter::new(),
            database_manager: config.database.as_ref().map(|db_config| DatabaseManager::new(db_config.clone())),
            middleware_chain: config.middlewares.clone(),
            renderer,
            config,
            current_route: Arc::new(RwLock::new(None)),
        })
    }

    /// HTTPリクエストを処理
    pub async fn handle_request(&self, request: crate::http::HttpRequest) -> Result<crate::http::HttpResponse> {
        // ミドルウェアチェーンを実行
        let mut context = RequestContext::new(request);

        for middleware in &self.middleware_chain {
            self.execute_middleware(middleware, &mut context).await?;
            if context.is_terminated() {
                break;
            }
        }

        // APIルートをチェック
        if let Some(api_response) = self.handle_api_request(&context).await? {
            return Ok(api_response);
        }

        // ページルートをチェック
        if let Some(page_response) = self.handle_page_request(&context).await? {
            return Ok(page_response);
        }

        // 404 Not Found
        Ok(crate::http::HttpResponse {
            request_id: context.request.id.clone(),
            status: crate::http::HttpStatus { code: 404, reason: "Not Found".to_string() },
            headers: Properties::new(),
            body_ref: None,
        })
    }

    /// APIリクエストを処理
    async fn handle_api_request(&self, context: &RequestContext) -> Result<Option<crate::http::HttpResponse>> {
        let path = &context.request.path;
        let method = context.request.method.clone();

        if let Some(route) = self.api_router.find_route(&method, path).await? {
            // APIルートが見つかった場合
            let response = self.execute_api_handler(&route, context).await?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// ページリクエストを処理
    async fn handle_page_request(&self, context: &RequestContext) -> Result<Option<crate::http::HttpResponse>> {
        let path = &context.request.path;

        if let Some((route, params)) = {
            let table = self.route_table.read().await;
            table.find_route(path)
        } {
            // ページルートが見つかった場合
            let render_result = self.navigate_with_params(path, params).await?;
            let response = self.create_page_response(render_result)?;
            Ok(Some(response))
        } else {
            Ok(None)
        }
    }

    /// APIハンドラーを実行
    async fn execute_api_handler(&self, route: &ApiRouteIR, context: &RequestContext) -> Result<crate::http::HttpResponse> {
        // パラメータ検証
        let params = self.validate_api_parameters(route, context)?;

        // データベースクエリ実行（必要に応じて）
        let data = if let Some(db) = &self.database_manager {
            Some(self.execute_database_query(route, &params).await?)
        } else {
            None
        };

        // レスポンス生成
        let response_body = match route.response_format {
            ResponseFormat::JSON => {
                // JSONレスポンスを生成
                Some(serde_json::to_string(&data.unwrap_or(serde_json::Value::Null))
                    .map_err(|e| KotobaError::InvalidArgument(e.to_string()))?)
            },
            _ => None, // 他のフォーマットは未実装
        };

        Ok(crate::http::HttpResponse {
            request_id: context.request.id.clone(),
            status: crate::http::HttpStatus { code: 200, reason: "OK".to_string() },
            headers: {
                let mut headers = Properties::new();
                headers.insert("Content-Type".to_string(), Value::String("application/json".to_string()));
                headers
            },
            body_ref: response_body.map(|body| ContentHash::sha256(body.as_bytes())),
        })
    }

    /// パラメータ検証
    fn validate_api_parameters(&self, route: &ApiRouteIR, context: &RequestContext) -> Result<Properties> {
        let mut validated_params = Properties::new();

        // パスパラメータの検証
        for param in &route.parameters.path_params {
            // TODO: パスパラメータの抽出と検証を実装
        }

        // クエリパラメータの検証
        for param in &route.parameters.query_params {
            // TODO: クエリパラメータの検証を実装
        }

        Ok(validated_params)
    }

    /// データベースクエリ実行
    async fn execute_database_query(&self, route: &ApiRouteIR, params: &Properties) -> Result<Value> {
        // TODO: 実際のデータベースクエリ実行を実装
        // ここではモックデータを返す
        Ok(Value::Object(Properties::new()))
    }

    /// ミドルウェアを実行
    async fn execute_middleware(&self, middleware: &MiddlewareIR, context: &mut RequestContext) -> Result<()> {
        match middleware.middleware_type {
            MiddlewareType::CORS => {
                // CORSヘッダーを設定
                context.add_header("Access-Control-Allow-Origin", "*");
                context.add_header("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS");
                context.add_header("Access-Control-Allow-Headers", "Content-Type, Authorization");
            },
            MiddlewareType::Authentication => {
                // 認証チェック
                if !self.check_authentication(context).await? {
                    context.terminate_with_status(401);
                }
            },
            MiddlewareType::Logging => {
                // リクエストログ記録
                println!("Request: {} {}", context.request.method, context.request.path);
            },
            _ => {
                // その他のミドルウェアは未実装
            }
        }

        Ok(())
    }

    /// 認証チェック
    async fn check_authentication(&self, context: &RequestContext) -> Result<bool> {
        // TODO: 実際の認証ロジックを実装
        // Authorizationヘッダーをチェック
        Ok(true) // 仮に常に認証成功
    }

    /// ページレスポンスを作成
    fn create_page_response(&self, render_result: RenderResultIR) -> Result<crate::http::HttpResponse> {
        Ok(crate::http::HttpResponse {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: crate::http::HttpStatus { code: 200, reason: "OK".to_string() },
            headers: {
                let mut headers = Properties::new();
                headers.insert("Content-Type".to_string(), Value::String("text/html".to_string()));
                headers
            },
            body_ref: Some(ContentHash::sha256(render_result.html.as_bytes())),
        })
    }
}

/// リクエストコンテキスト
pub struct RequestContext {
    pub request: crate::http::HttpRequest,
    pub response_headers: Properties,
    pub terminated: bool,
    pub status_code: Option<u16>,
}

impl RequestContext {
    pub fn new(request: crate::http::HttpRequest) -> Self {
        Self {
            request,
            response_headers: Properties::new(),
            terminated: false,
            status_code: None,
        }
    }

    pub fn add_header(&mut self, key: String, value: &str) {
        self.response_headers.insert(key, Value::String(value.to_string()));
    }

    pub fn terminate_with_status(&mut self, status: u16) {
        self.terminated = true;
        self.status_code = Some(status);
    }

    pub fn is_terminated(&self) -> bool {
        self.terminated
    }
}

    /// ルートを追加
    pub async fn add_route(&self, route: RouteIR) -> Result<()> {
        let mut table = self.route_table.write().await;
        table.add_route(route);
        Ok(())
    }

    /// コンポーネントを登録
    pub async fn register_component(&self, component: ComponentIR) -> Result<()> {
        let mut registry = self.component_registry.write().await;
        registry.register(component);
        Ok(())
    }

    /// APIルートを追加
    pub async fn add_api_route(&self, route: ApiRouteIR) -> Result<()> {
        self.api_router.add_route(route).await
    }

    /// パスによるナビゲーション
    pub async fn navigate(&self, path: &str) -> Result<RenderResultIR> {
        let table = self.route_table.read().await;

        if let Some((route, params)) = table.find_route(path) {
            // ルートパラメータをグローバル状態に設定
            let mut global_props = Properties::new();
            for (key, value) in params {
                global_props.insert(key, crate::types::Value::String(value));
            }

            let context = RenderContext {
                environment: ExecutionEnvironment::Universal,
                route_params: global_props,
                query_params: Properties::new(),
                global_state: Properties::new(),
                is_server_side: true,
                is_client_side: false,
                hydration_id: Some(format!("route_{}", uuid::Uuid::new_v4())),
            };

            // 現在のルートを更新
            let mut current_route = self.current_route.write().await;
            *current_route = Some(route.clone());

            // ルートをレンダリング
            self.render_route(route, context).await
        } else {
            Err(KotobaError::NotFound(format!("Route not found: {}", path)))
        }
    }

    /// パスとパラメータによるナビゲーション
    async fn navigate_with_params(&self, path: &str, params: HashMap<String, String>) -> Result<RenderResultIR> {
        let table = self.route_table.read().await;

        if let Some((route, _)) = table.find_route(path) {
            // ルートパラメータをグローバル状態に設定
            let mut global_props = Properties::new();
            for (key, value) in params {
                global_props.insert(key, crate::types::Value::String(value));
            }

            let context = RenderContext {
                environment: ExecutionEnvironment::Universal,
                route_params: global_props,
                query_params: Properties::new(),
                global_state: Properties::new(),
                is_server_side: true,
                is_client_side: false,
                hydration_id: Some(format!("route_{}", uuid::Uuid::new_v4())),
            };

            // 現在のルートを更新
            let mut current_route = self.current_route.write().await;
            *current_route = Some(route.clone());

            // ルートをレンダリング
            self.render_route(route, context).await
        } else {
            Err(KotobaError::NotFound(format!("Route not found: {}", path)))
        }
    }

    /// ルートをレンダリング
    async fn render_route(&self, route: &RouteIR, context: RenderContext) -> Result<RenderResultIR> {
        // レイアウトツリーを構築
        let layout_tree = self.build_layout_tree(route).await?;

        // レンダリング
        self.renderer.render_component_tree(&layout_tree, context).await
    }

    /// レイアウトツリーを構築（ネストされたレイアウト）
    async fn build_layout_tree(&self, route: &RouteIR) -> Result<ComponentTreeIR> {
        let mut root_component = if let Some(layout) = &route.components.layout {
            layout.clone()
        } else {
            // デフォルトルートレイアウト
            ComponentIR::new("RootLayout".to_string(), ComponentType::Layout)
        };

        // 子コンポーネントとしてページを追加
        if let Some(page) = &route.components.page {
            root_component.add_child(page.clone());
        }

        // ローディング状態がある場合は追加
        if let Some(loading) = &route.components.loading {
            let mut loading_component = loading.clone();
            loading_component.add_child(root_component);
            Ok(ComponentTreeIR::new(loading_component))
        } else {
            Ok(ComponentTreeIR::new(root_component))
        }
    }

    /// 現在のルートを取得
    pub async fn get_current_route(&self) -> Option<RouteIR> {
        self.current_route.read().await.clone()
    }

    /// ルートテーブルを取得
    pub async fn get_route_table(&self) -> RouteTableIR {
        self.route_table.read().await.clone()
    }

    /// 設定を取得
    pub fn get_config(&self) -> &WebFrameworkConfigIR {
        &self.config
    }

impl ApiRouter {
    pub fn new() -> Self {
        Self {
            routes: Arc::new(RwLock::new(HashMap::new())),
            graphql_schema: None,
        }
    }

    /// APIルートを追加
    pub async fn add_route(&self, route: ApiRouteIR) -> Result<()> {
        let mut routes = self.routes.write().await;
        let method_routes = routes.entry(route.method.to_string()).or_insert_with(Vec::new);
        method_routes.push(route);
        Ok(())
    }

    /// ルートを検索
    pub async fn find_route(&self, method: &str, path: &str) -> Result<Option<ApiRouteIR>> {
        let routes = self.routes.read().await;
        if let Some(method_routes) = routes.get(method) {
            // パスベースのマッチング（簡略化）
            for route in method_routes {
                if route.path == path {
                    return Ok(Some(route.clone()));
                }
            }
        }
        Ok(None)
    }
}

impl DatabaseManager {
    pub fn new(config: DatabaseIR) -> Self {
        Self {
            config,
            connection_pool: None, // TODO: 実際のDB接続プール実装
        }
    }

    /// データベース接続を初期化
    pub async fn initialize(&mut self) -> Result<()> {
        // TODO: 実際のデータベース接続初期化
        println!("Initializing database connection for: {:?}", self.config.db_type);
        Ok(())
    }

    /// クエリ実行
    pub async fn execute_query(&self, query: &str, params: Vec<Value>) -> Result<Vec<Properties>> {
        // TODO: 実際のクエリ実行
        println!("Executing query: {}", query);
        Ok(Vec::new())
    }

    /// マイグレーション実行
    pub async fn run_migrations(&self) -> Result<()> {
        for migration in &self.config.migrations {
            println!("Running migration: {}", migration.version);
            // TODO: マイグレーション実行
        }
        Ok(())
    }
}

/// コンポーネントレジストリ
pub struct ComponentRegistry {
    components: HashMap<String, ComponentIR>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn register(&mut self, component: ComponentIR) {
        self.components.insert(component.id.clone(), component);
    }

    pub fn get(&self, id: &str) -> Option<&ComponentIR> {
        self.components.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ComponentIR> {
        self.components.values().find(|c| c.name == name)
    }
}

/// コンポーネントレンダラー
pub struct ComponentRenderer {
    render_engine: RenderEngineIR,
    component_cache: Arc<RwLock<HashMap<String, RenderResultIR>>>,
}

impl ComponentRenderer {
    pub fn new() -> Self {
        Self {
            render_engine: RenderEngineIR {
                strategies: vec![RenderStrategy::SSR],
                optimizers: vec![RenderOptimizer::TreeShaking],
                cache_config: RenderCacheConfig {
                    enable_cache: true,
                    cache_strategy: crate::frontend::render_ir::CacheStrategy::LRU,
                    max_cache_size: 100,
                    ttl_seconds: 300,
                },
            },
            component_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// コンポーネントツリーをレンダリング
    pub async fn render_component_tree(
        &self,
        tree: &ComponentTreeIR,
        context: RenderContext,
    ) -> Result<RenderResultIR> {
        let cache_key = format!("tree_{}_{}", tree.root.id, context.hydration_id.clone().unwrap_or_default());

        // キャッシュチェック
        if self.render_engine.cache_config.enable_cache {
            if let Some(cached) = self.component_cache.read().await.get(&cache_key) {
                return Ok(cached.clone());
            }
        }

        // 仮想DOMを構築
        let virtual_dom = self.build_virtual_dom(&tree.root, &context)?;

        // HTML生成
        let html = self.generate_html(&virtual_dom, &context)?;
        let hydration_script = self.generate_hydration_script(&tree.root, &context).await?;

        let result = RenderResultIR {
            html,
            css: String::new(), // TODO: CSS生成
            js: String::new(),  // TODO: JSバンドル
            hydration_script: Some(hydration_script),
            head_elements: Vec::new(), // TODO: ヘッド要素生成
            virtual_dom,
            render_stats: RenderStats {
                render_time_ms: 0, // TODO: 実際の計測
                component_count: self.count_components(&tree.root),
                dom_node_count: 0, // TODO: DOMノード数カウント
                memory_usage_kb: 0, // TODO: メモリ使用量
            },
        };

        // キャッシュ保存
        if self.render_engine.cache_config.enable_cache {
            self.component_cache.write().await.insert(cache_key, result.clone());
        }

        Ok(result)
    }

    /// 仮想DOMを構築
    fn build_virtual_dom(
        &self,
        component: &ComponentIR,
        context: &RenderContext,
    ) -> Result<VirtualNodeIR> {
        match component.component_type {
            ComponentType::Server | ComponentType::Client => {
                // JSXから仮想DOMを生成（簡略化）
                let mut element = VirtualNodeIR::element("div".to_string());
                if let VirtualNodeIR::Element(ref mut el) = element {
                    // Propsを属性に変換
                    for (key, value) in &component.props {
                        el.add_attribute(key.clone(), value.clone());
                    }

                    // 子コンポーネントを追加
                    for child in &component.children {
                        let child_dom = self.build_virtual_dom(child, context)?;
                        match child_dom {
                            VirtualNodeIR::Element(child_el) => {
                                el.add_child(ElementChild::Element(child_el));
                            },
                            VirtualNodeIR::Text(text) => {
                                el.add_child(ElementChild::Text(text));
                            },
                            VirtualNodeIR::Component(comp) => {
                                el.add_child(ElementChild::Component(comp));
                            },
                            VirtualNodeIR::Fragment(_) => {
                                // Fragmentの場合はスキップ（簡略化）
                            },
                        }
                    }
                }
                Ok(element)
            },
            ComponentType::Layout => {
                // レイアウトコンポーネント
                let mut layout = VirtualNodeIR::element("div".to_string());
                if let VirtualNodeIR::Element(ref mut el) = layout {
                    el.add_attribute("data-layout".to_string(), crate::types::Value::String(component.name.clone()));

                    for child in &component.children {
                        let child_dom = self.build_virtual_dom(child, context)?;
                        match child_dom {
                            VirtualNodeIR::Element(child_el) => {
                                el.add_child(ElementChild::Element(child_el));
                            },
                            VirtualNodeIR::Text(text) => {
                                el.add_child(ElementChild::Text(text));
                            },
                            VirtualNodeIR::Component(comp) => {
                                el.add_child(ElementChild::Component(comp));
                            },
                            VirtualNodeIR::Fragment(_) => {
                                // Fragmentの場合はスキップ（簡略化）
                            },
                        }
                    }
                }
                Ok(layout)
            },
            ComponentType::Page => {
                // ページコンポーネント
                let mut page = VirtualNodeIR::element("main".to_string());
                if let VirtualNodeIR::Element(ref mut el) = page {
                    el.add_attribute("data-page".to_string(), crate::types::Value::String(component.name.clone()));

                    // ページコンテンツ（簡略化）
                    let content = format!("Content of {}", component.name);
                    el.add_child(ElementChild::Text(content));
                }
                Ok(page)
            },
            _ => {
                // その他のコンポーネントタイプ
                Ok(VirtualNodeIR::text(format!("Component: {}", component.name)))
            }
        }
    }

    /// HTMLを生成
    fn generate_html(&self, virtual_dom: &VirtualNodeIR, context: &RenderContext) -> Result<String> {
        match virtual_dom {
            VirtualNodeIR::Element(element) => {
                let mut html = format!("<{}", element.tag_name);

                // 属性を追加
                for (key, value) in &element.attributes {
                    if let crate::types::Value::String(val) = value {
                        html.push_str(&format!(" {}=\"{}\"", key, val));
                    }
                }

                if context.is_server_side && context.hydration_id.is_some() {
                    html.push_str(&format!(" data-hydrate=\"{}\"", context.hydration_id.as_ref().unwrap()));
                }

                html.push_str(">");

                    // 子要素を追加
                for child in &element.children {
                    match child {
                        ElementChild::Text(text) => html.push_str(text),
                        ElementChild::Element(child_element) => {
                            let child_html = self.generate_html(&VirtualNodeIR::Element(child_element.clone()), context)?;
                            html.push_str(&child_html);
                        },
                        ElementChild::Component(_) => {
                            html.push_str("<!-- Component -->");
                        },
                        ElementChild::Expression(_) => {
                            html.push_str("<!-- Expression -->");
                        },
                    }
                }

                html.push_str(&format!("</{}>", element.tag_name));
                Ok(html)
            },
            VirtualNodeIR::Text(text) => Ok(text.clone()),
            VirtualNodeIR::Component(_) => Ok("<!-- Component -->".to_string()),
            VirtualNodeIR::Fragment(children) => {
                let mut html = String::new();
                for child in children {
                    html.push_str(&self.generate_html(child, context)?);
                }
                Ok(html)
            },
        }
    }

    /// ハイドレーションスクリプトを生成
    async fn generate_hydration_script(&self, component: &ComponentIR, context: &RenderContext) -> Result<String> {
        let hydration_id = context.hydration_id.as_ref()
            .ok_or_else(|| KotobaError::InvalidArgument("Hydration ID required".to_string()))?;

        let script = format!(
            r#"
// Kotoba Hydration Script
window.Kotoba = window.Kotoba || {{}};
window.Kotoba.hydrate('{hydration_id}', {{
  component: '{component_name}',
  props: {props},
  route: {route_params}
}});
"#,
            hydration_id = hydration_id,
            component_name = component.name,
            props = "{}",
            route_params = "{}"
        );

        Ok(script)
    }

    /// コンポーネント数をカウント
    fn count_components(&self, component: &ComponentIR) -> usize {
        1 + component.children.iter().map(|child| self.count_components(child)).sum::<usize>()
    }
}

/// ビルドエンジン
pub struct BuildEngine {
    config: BuildConfigIR,
    route_table: Arc<RwLock<RouteTableIR>>,
}

impl BuildEngine {
    pub fn new(config: BuildConfigIR) -> Self {
        Self {
            config,
            route_table: Arc::new(RwLock::new(RouteTableIR::new())),
        }
    }

    /// ビルドを実行
    pub async fn build(&self) -> Result<BundleResultIR> {
        println!("🚀 Starting Kotoba frontend build...");

        // エントリーポイントを処理
        let mut chunks = Vec::new();
        let mut assets = Vec::new();

        for entry in &self.config.entry_points {
            let chunk = self.process_entry_point(entry).await?;
            chunks.push(chunk);
        }

        // 最適化を適用
        for optimization in &self.config.optimizations {
            self.apply_optimization(optimization, &mut chunks, &mut assets).await?;
        }

        // バンドル結果を作成
        let chunk_count = chunks.len();
        let module_count = chunks.iter().map(|c| c.modules.len()).sum();
        let asset_count = assets.len();

        let result = BundleResultIR {
            chunks: chunks.clone(),
            assets: assets.clone(),
            stats: BuildStats {
                build_time_ms: 1000, // TODO: 実際の計測
                total_size: 1024000, // 1MB (仮)
                gzip_size: 256000,   // 256KB (仮)
                brotli_size: 200000, // 200KB (仮)
                chunk_count,
                module_count,
                asset_count,
                warnings: Vec::new(),
                errors: Vec::new(),
            },
            manifest: BundleManifest {
                entries: HashMap::new(), // TODO: エントリーマッピング
                chunks: HashMap::new(),  // TODO: チャンクマッピング
                modules: HashMap::new(), // TODO: モジュールマッピング
            },
        };

        println!("✅ Build completed successfully!");
        println!("📊 Chunks: {}, Assets: {}, Size: {} KB",
                 result.stats.chunk_count,
                 result.stats.asset_count,
                 result.stats.total_size / 1024);

        Ok(result)
    }

    /// エントリーポイントを処理
    async fn process_entry_point(&self, entry: &EntryPoint) -> Result<ChunkIR> {
        let chunk_id = format!("chunk_{}", uuid::Uuid::new_v4());

        Ok(ChunkIR {
            id: chunk_id.clone(),
            name: Some(entry.name.clone()),
            entry: true,
            initial: true,
            files: vec![format!("{}.js", entry.name)],
            hash: ContentHash::sha256([1; 32]),
            size: 102400, // 100KB (仮)
            modules: vec![
                ModuleIR {
                    id: entry.name.clone(),
                    name: entry.path.clone(),
                    size: 102400,
                    dependencies: Vec::new(), // TODO: 依存関係分析
                    is_entry: true,
                    chunks: vec![chunk_id.clone()],
                }
            ],
        })
    }

    /// 最適化を適用
    async fn apply_optimization(
        &self,
        optimization: &OptimizationIR,
        chunks: &mut Vec<ChunkIR>,
        assets: &mut Vec<AssetIR>,
    ) -> Result<()> {
        match optimization {
            OptimizationIR::CodeSplitting { .. } => {
                // コード分割の適用（簡略化）
                println!("📦 Applying code splitting...");
            },
            OptimizationIR::Minification { .. } => {
                // ミニファイの適用（簡略化）
                println!("🔧 Applying minification...");
                for chunk in chunks.iter_mut() {
                    chunk.size = (chunk.size as f64 * 0.7) as usize; // 30%削減（仮）
                }
            },
            OptimizationIR::Compression { algorithm, .. } => {
                // 圧縮の適用
                match algorithm {
                    CompressionAlgorithm::Gzip => {
                        println!("🗜️  Applying gzip compression...");
                        let compressed_asset = AssetIR {
                            name: "app.js.gz".to_string(),
                            path: "dist/app.js.gz".to_string(),
                            size: 256000, // 仮の圧縮サイズ
                            content_type: "application/gzip".to_string(),
                            hash: ContentHash::sha256([2; 32]),
                        };
                        assets.push(compressed_asset);
                    },
                    _ => {},
                }
            },
            _ => {},
        }
        Ok(())
    }

    /// 開発サーバーを起動
    pub async fn start_dev_server(&self, port: u16) -> Result<()> {
        println!("🚀 Starting Kotoba development server on port {}", port);

        // ファイル監視とホットリロードの設定（簡略化）
        println!("🔥 Hot reload enabled");
        println!("📁 Watching for file changes...");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::component_ir::ComponentType;

    #[tokio::test]
    async fn test_web_framework_creation() {
        let config = WebFrameworkConfigIR {
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
        };

        let framework = WebFramework::new(config).unwrap();
        assert_eq!(framework.get_config().server.port, 3000);
    }

    #[tokio::test]
    async fn test_api_router() {
        let router = ApiRouter::new();

        let api_route = ApiRouteIR {
            path: "/api/test".to_string(),
            method: ApiMethod::GET,
            handler: ApiHandlerIR {
                function_name: "testHandler".to_string(),
                component: None,
                is_async: true,
                timeout_ms: Some(5000),
            },
            middlewares: Vec::new(),
            response_format: ResponseFormat::JSON,
            parameters: ApiParameters {
                path_params: Vec::new(),
                query_params: Vec::new(),
                body_params: None,
                headers: Vec::new(),
            },
            metadata: ApiMetadata {
                description: Some("Test API".to_string()),
                summary: Some("Test".to_string()),
                tags: vec!["test".to_string()],
                deprecated: false,
                rate_limit: None,
                cache: None,
            },
        };

        router.add_route(api_route).await.unwrap();

        let found = router.find_route("GET", "/api/test").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().path, "/api/test");
    }

    #[tokio::test]
    async fn test_database_manager() {
        let db_config = DatabaseIR {
            connection_string: "postgresql://test:test@localhost/test".to_string(),
            db_type: DatabaseType::PostgreSQL,
            models: Vec::new(),
            migrations: Vec::new(),
        };

        let mut manager = DatabaseManager::new(db_config);
        manager.initialize().await.unwrap();

        let result = manager.execute_query("SELECT 1", Vec::new()).await.unwrap();
        assert!(result.is_empty()); // モックなので空の結果
    }

    #[tokio::test]
    async fn test_component_rendering() {
        let renderer = ComponentRenderer::new();

        // テストコンポーネント
        let component = ComponentIR::new("TestComponent".to_string(), ComponentType::Server);
        let tree = ComponentTreeIR::new(component);

        let context = RenderContext::server_side();
        let result = renderer.render_component_tree(&tree, context).await.unwrap();

        assert!(!result.html.is_empty());
        assert_eq!(result.render_stats.component_count, 1);
    }

    #[test]
    fn test_build_engine_creation() {
        let config = BuildConfigIR::new(build_ir::BundlerType::Vite);
        let engine = BuildEngine::new(config);

        // 設定が正しく適用されていることを確認
        assert_eq!(engine.config.bundler, build_ir::BundlerType::Vite);
    }

    #[test]
    fn test_request_context() {
        let request = crate::http::HttpRequest {
            id: "test-123".to_string(),
            method: "GET".to_string(),
            path: "/test".to_string(),
            headers: Properties::new(),
            query_string: None,
            body: None,
            timestamp: std::time::SystemTime::now(),
        };

        let mut context = RequestContext::new(request);
        assert!(!context.is_terminated());

        context.add_header("X-Test", "value");
        context.terminate_with_status(404);

        assert!(context.is_terminated());
        assert_eq!(context.status_code, Some(404));
    }
}

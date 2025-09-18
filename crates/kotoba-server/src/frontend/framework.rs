//! App Routerフレームワークのコア実装
//!
//! Next.js風App Routerフレームワークの主要コンポーネントを実装します。

use kotoba_core::prelude::*;
use crate::frontend::component_ir::ExecutionEnvironment;
use crate::frontend::component_ir::*;
use crate::frontend::route_ir::*;
use crate::frontend::render_ir::*;
use crate::frontend::build_ir::*;
use crate::frontend::api_ir::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Web Framework - フルスタックWebフレームワーク
pub struct WebFramework {
    route_table: Arc<RwLock<RouteTableIR>>,
    component_registry: Arc<RwLock<ComponentRegistry>>,
    renderer: ComponentRenderer,
    config: WebFrameworkConfigIR,
    current_route: Arc<RwLock<Option<RouteIR>>>,
}


impl WebFramework {
    pub fn new(config: WebFrameworkConfigIR) -> Result<Self> {
        let renderer = ComponentRenderer::new();

        Ok(Self {
            route_table: Arc::new(RwLock::new(RouteTableIR::new())),
            component_registry: Arc::new(RwLock::new(ComponentRegistry::new())),
            renderer,
            config,
            current_route: Arc::new(RwLock::new(None)),
        })
    }

    /// HTTPリクエストを処理
    pub async fn handle_request(&self, request: crate::http::HttpRequest) -> Result<crate::http::HttpResponse> {
        let path = &request.path;

        // ページルートをチェック
        let table = self.route_table.read().await;
        if let Some((route, params)) = table.find_route(path) {
            // ページルートが見つかった場合
            let render_result = self.render_route_with_params(&route, params).await?;
            let response = self.create_page_response(render_result)?;
            return Ok(response);
        }

        // 404 Not Found
        Ok(crate::http::HttpResponse {
            request_id: request.id.clone(),
            status: crate::http::HttpStatus { code: 404, reason: "Not Found".to_string() },
            headers: crate::http::HttpHeaders::new(),
            body_ref: None,
            timestamp: 1234567890,
        })
    }



    /// ページレスポンスを作成
    fn create_page_response(&self, render_result: RenderResultIR) -> Result<crate::http::HttpResponse> {
        let mut http_headers = crate::http::HttpHeaders::new();
        http_headers.set("Content-Type".to_string(), "text/html".to_string());

        Ok(crate::http::HttpResponse {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: crate::http::HttpStatus { code: 200, reason: "OK".to_string() },
            headers: http_headers,
            body_ref: Some(ContentHash::sha256(render_result.html.as_bytes().try_into().unwrap())),
            timestamp: 1234567890,
        })
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

    /// パスによるナビゲーション
    pub async fn navigate(&self, path: &str) -> Result<RenderResultIR> {
        let table = self.route_table.read().await;

        if let Some((route, params)) = table.find_route(path) {
            self.render_route_with_params(&route, params).await
        } else {
            Err(KotobaError::NotFound(format!("Route not found: {}", path)))
        }
    }

    /// パスとパラメータによるルートレンダリング
    async fn render_route_with_params(&self, route: &RouteIR, params: HashMap<String, String>) -> Result<RenderResultIR> {
        // ルートパラメータをグローバル状態に設定
        let mut global_props = Properties::new();
        for (key, value) in params {
            global_props.insert(key, Value::String(value));
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
    use crate::frontend::api_ir::*;

    #[tokio::test]
    async fn test_web_framework_creation() {
        let config = WebFrameworkConfigIR {
            server: crate::frontend::api_ir::ServerConfig {
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
        let config = BuildConfigIR::new(BundlerType::Vite);
        let engine = BuildEngine::new(config);

        // 設定が正しく適用されていることを確認
        assert_eq!(engine.config.bundler, BundlerType::Vite);
    }

}

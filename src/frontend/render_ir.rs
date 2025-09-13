//! レンダリングエンジンIR定義
//!
//! 仮想DOM、コンポーネントツリー、レンダリングパイプラインを表現します。

use crate::types::{Value, Properties, ContentHash, Result, KotobaError};
use crate::frontend::component_ir::{ComponentIR, ElementIR, ElementChild, ComponentType, ExecutionEnvironment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 仮想DOMノードIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VirtualNodeIR {
    Element(ElementIR),
    Component(ComponentIR),
    Text(String),
    Fragment(Vec<VirtualNodeIR>),
}

impl VirtualNodeIR {
    pub fn element(tag_name: String) -> Self {
        VirtualNodeIR::Element(ElementIR::new(tag_name))
    }

    pub fn component(component: ComponentIR) -> Self {
        VirtualNodeIR::Component(component)
    }

    pub fn text(content: String) -> Self {
        VirtualNodeIR::Text(content)
    }

    pub fn fragment(children: Vec<VirtualNodeIR>) -> Self {
        VirtualNodeIR::Fragment(children)
    }
}

/// レンダリングコンテキスト
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderContext {
    pub environment: ExecutionEnvironment,
    pub route_params: Properties,
    pub query_params: Properties,
    pub global_state: Properties,
    pub is_server_side: bool,
    pub is_client_side: bool,
    pub hydration_id: Option<String>,
}

impl RenderContext {
    pub fn new() -> Self {
        Self {
            environment: ExecutionEnvironment::Universal,
            route_params: Properties::new(),
            query_params: Properties::new(),
            global_state: Properties::new(),
            is_server_side: false,
            is_client_side: false,
            hydration_id: None,
        }
    }

    pub fn server_side() -> Self {
        Self {
            is_server_side: true,
            ..Self::new()
        }
    }

    pub fn client_side() -> Self {
        Self {
            is_client_side: true,
            ..Self::new()
        }
    }

    pub fn with_route_params(mut self, params: Properties) -> Self {
        self.route_params = params;
        self
    }

    pub fn with_query_params(mut self, params: Properties) -> Self {
        self.query_params = params;
        self
    }
}

/// レンダリング結果IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderResultIR {
    pub html: String,
    pub css: String,
    pub js: String,
    pub hydration_script: Option<String>,
    pub head_elements: Vec<HeadElementIR>,
    pub virtual_dom: VirtualNodeIR,
    pub render_stats: RenderStats,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HeadElementIR {
    pub element_type: HeadElementType,
    pub attributes: Properties,
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HeadElementType {
    Title,
    Meta,
    Link,
    Script,
    Style,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderStats {
    pub render_time_ms: u64,
    pub component_count: usize,
    pub dom_node_count: usize,
    pub memory_usage_kb: usize,
}

/// レンダリングエンジンIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderEngineIR {
    pub strategies: Vec<RenderStrategy>,
    pub optimizers: Vec<RenderOptimizer>,
    pub cache_config: RenderCacheConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderStrategy {
    /// サーバーサイドレンダリング
    SSR,
    /// 静的サイト生成
    SSG,
    /// クライアントサイドレンダリング
    CSR,
    /// ストリーミングSSR
    StreamingSSR,
    /// プログレッシブハイドレーション
    ProgressiveHydration,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderOptimizer {
    /// コード分割
    CodeSplitting,
    /// ツリーシェイキング
    TreeShaking,
    /// 遅延読み込み
    LazyLoading,
    /// プリロード
    Preload,
    /// プリフェッチ
    Prefetch,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderCacheConfig {
    pub enable_cache: bool,
    pub cache_strategy: CacheStrategy,
    pub max_cache_size: usize,
    pub ttl_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CacheStrategy {
    LRU,
    LFU,
    TimeBased,
    SizeBased,
}

/// 差分更新IR (仮想DOM差分)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DiffIR {
    pub patches: Vec<PatchIR>,
    pub old_tree: VirtualNodeIR,
    pub new_tree: VirtualNodeIR,
    pub affected_nodes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PatchIR {
    /// ノード追加
    Insert {
        parent_id: String,
        node: VirtualNodeIR,
        index: usize,
    },
    /// ノード削除
    Remove {
        node_id: String,
    },
    /// ノード更新
    Update {
        node_id: String,
        attributes: Properties,
        text_content: Option<String>,
    },
    /// ノード移動
    Move {
        node_id: String,
        new_parent_id: String,
        new_index: usize,
    },
    /// 属性更新
    UpdateAttribute {
        node_id: String,
        attribute_name: String,
        new_value: Option<Value>,
    },
}

/// コンポーネントライフサイクルIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LifecycleEventIR {
    Mount {
        component_id: String,
        props: Properties,
    },
    Update {
        component_id: String,
        old_props: Properties,
        new_props: Properties,
    },
    Unmount {
        component_id: String,
    },
    Error {
        component_id: String,
        error: String,
        error_boundary_id: Option<String>,
    },
    Suspend {
        component_id: String,
        fallback: VirtualNodeIR,
    },
    Resume {
        component_id: String,
    },
}

/// レンダリングパイプラインIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderPipelineIR {
    pub stages: Vec<RenderStage>,
    pub error_handling: ErrorHandlingStrategy,
    pub performance_monitoring: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RenderStage {
    /// コンポーネント解決
    ResolveComponents,
    /// Propsマッピング
    MapProps,
    /// 状態初期化
    InitializeState,
    /// 仮想DOM構築
    BuildVirtualDOM,
    /// 最適化適用
    ApplyOptimizations,
    /// HTML生成
    GenerateHTML,
    /// ハイドレーションスクリプト生成
    GenerateHydrationScript,
    /// バンドル処理
    BundleAssets,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorHandlingStrategy {
    /// 即時失敗
    FailFast,
    /// エラーバウンダリ使用
    UseErrorBoundaries,
    /// フォールバックコンテンツ
    FallbackContent,
    /// ログ記録のみ
    LogOnly,
}

/// Suspense境界IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuspenseBoundaryIR {
    pub id: String,
    pub children: Vec<VirtualNodeIR>,
    pub fallback: VirtualNodeIR,
    pub pending_promises: Vec<String>,
    pub resolved: bool,
}

impl SuspenseBoundaryIR {
    pub fn new(id: String, fallback: VirtualNodeIR) -> Self {
        Self {
            id,
            children: Vec::new(),
            fallback,
            pending_promises: Vec::new(),
            resolved: false,
        }
    }

    pub fn add_child(&mut self, child: VirtualNodeIR) {
        self.children.push(child);
    }

    pub fn add_promise(&mut self, promise_id: String) {
        self.pending_promises.push(promise_id);
    }

    pub fn resolve_promise(&mut self, promise_id: &str) {
        self.pending_promises.retain(|id| id != promise_id);
        if self.pending_promises.is_empty() {
            self.resolved = true;
        }
    }
}

/// ハイドレーションIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydrationIR {
    pub server_html: String,
    pub client_script: String,
    pub hydration_map: HashMap<String, HydrationNode>,
    pub event_listeners: Vec<EventListenerIR>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HydrationNode {
    pub id: String,
    pub component_type: ComponentType,
    pub props: Properties,
    pub state: Properties,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventListenerIR {
    pub element_id: String,
    pub event_type: String,
    pub handler_function: String,
    pub options: EventOptions,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventOptions {
    pub capture: bool,
    pub once: bool,
    pub passive: bool,
}

/// メモ化IR (React.memo相当)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemoizationIR {
    pub component_id: String,
    pub comparison_function: Option<String>,
    pub cached_props: Properties,
    pub cache_hit: bool,
}

impl MemoizationIR {
    pub fn new(component_id: String) -> Self {
        Self {
            component_id,
            comparison_function: None,
            cached_props: Properties::new(),
            cache_hit: false,
        }
    }

    pub fn should_update(&self, new_props: &Properties) -> bool {
        // シンプルな比較（実際の実装ではcomparison_functionを使用）
        &self.cached_props != new_props
    }

    pub fn update_cache(&mut self, new_props: Properties) {
        self.cached_props = new_props;
        self.cache_hit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::component_ir::ComponentIR;

    #[test]
    fn test_virtual_node_creation() {
        let element = VirtualNodeIR::element("div".to_string());
        match element {
            VirtualNodeIR::Element(el) => assert_eq!(el.tag_name, "div"),
            _ => panic!("Expected Element"),
        }

        let text = VirtualNodeIR::text("Hello".to_string());
        match text {
            VirtualNodeIR::Text(content) => assert_eq!(content, "Hello"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_render_context() {
        let context = RenderContext::server_side()
            .with_route_params({
                let mut props = Properties::new();
                props.insert("id".to_string(), Value::String("123".to_string()));
                props
            });

        assert!(context.is_server_side);
        assert_eq!(context.route_params.get("id"), Some(&Value::String("123".to_string())));
    }

    #[test]
    fn test_suspense_boundary() {
        let fallback = VirtualNodeIR::text("Loading...".to_string());
        let mut boundary = SuspenseBoundaryIR::new("suspense-1".to_string(), fallback);

        boundary.add_promise("promise-1".to_string());
        boundary.add_promise("promise-2".to_string());

        assert_eq!(boundary.pending_promises.len(), 2);
        assert!(!boundary.resolved);

        boundary.resolve_promise("promise-1");
        assert_eq!(boundary.pending_promises.len(), 1);
        assert!(!boundary.resolved);

        boundary.resolve_promise("promise-2");
        assert_eq!(boundary.pending_promises.len(), 0);
        assert!(boundary.resolved);
    }

    #[test]
    fn test_memoization() {
        let mut memo = MemoizationIR::new("MyComponent".to_string());

        let mut new_props = Properties::new();
        new_props.insert("count".to_string(), Value::Int(1));

        assert!(memo.should_update(&new_props));

        memo.update_cache(new_props.clone());
        assert!(!memo.should_update(&new_props)); // 同じpropsなので更新不要

        let mut different_props = Properties::new();
        different_props.insert("count".to_string(), Value::Int(2));
        assert!(memo.should_update(&different_props)); // 異なるpropsなので更新必要
    }
}

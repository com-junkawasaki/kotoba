//! App RouterルーティングIR定義
//!
//! Next.js App RouterのファイルベースルーティングをKotoba IRで表現します。

use crate::types::{Value, Properties, ContentHash, Result, KotobaError};
use crate::frontend::component_ir::{ComponentIR, ComponentType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ルートタイプ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RouteType {
    /// 静的ルート (/about)
    Static,
    /// 動的ルート (/users/[id])
    Dynamic,
    /// キャッチオールルート (/docs/[...slug])
    CatchAll,
    /// オプションキャッチオール (/docs/[[...slug]])
    OptionalCatchAll,
    /// ルートグループ ((group))
    Group,
    /// 並列ルート (@parallel)
    Parallel,
    /// インターセプトされたルート (..)(..)
    Intercept,
}

/// ルートセグメント
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteSegment {
    pub name: String,
    pub segment_type: RouteType,
    pub params: Vec<String>, // 動的パラメータ名
    pub is_optional: bool,
}

impl RouteSegment {
    pub fn static_segment(name: String) -> Self {
        Self {
            name,
            segment_type: RouteType::Static,
            params: Vec::new(),
            is_optional: false,
        }
    }

    pub fn dynamic_segment(param: String) -> Self {
        Self {
            name: format!("[{}]", param),
            segment_type: RouteType::Dynamic,
            params: vec![param],
            is_optional: false,
        }
    }

    pub fn catch_all_segment(param: String) -> Self {
        Self {
            name: format!("[...{}]", param),
            segment_type: RouteType::CatchAll,
            params: vec![param],
            is_optional: false,
        }
    }

    pub fn optional_catch_all_segment(param: String) -> Self {
        Self {
            name: format!("[[...{}]]", param),
            segment_type: RouteType::OptionalCatchAll,
            params: vec![param],
            is_optional: true,
        }
    }
}

/// ルートIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteIR {
    pub path: String,
    pub segments: Vec<RouteSegment>,
    pub components: RouteComponents,
    pub metadata: RouteMetadata,
    pub children: Vec<RouteIR>, // ネストされたルート
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteComponents {
    pub page: Option<ComponentIR>,
    pub layout: Option<ComponentIR>,
    pub loading: Option<ComponentIR>,
    pub error: Option<ComponentIR>,
    pub template: Option<ComponentIR>,
    pub not_found: Option<ComponentIR>,
}

impl RouteComponents {
    pub fn new() -> Self {
        Self {
            page: None,
            layout: None,
            loading: None,
            error: None,
            template: None,
            not_found: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub keywords: Vec<String>,
    pub open_graph: Option<OpenGraphMetadata>,
    pub twitter_card: Option<TwitterCardMetadata>,
    pub canonical_url: Option<String>,
    pub robots: Option<RobotsMetadata>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenGraphMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub url: Option<String>,
    pub type_: Option<String>, // "website", "article", etc.
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwitterCardMetadata {
    pub card: Option<String>, // "summary", "summary_large_image", etc.
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RobotsMetadata {
    pub index: bool,
    pub follow: bool,
    pub noarchive: bool,
    pub nosnippet: bool,
    pub noimageindex: bool,
    pub nocache: bool,
}

impl RouteIR {
    pub fn new(path: String) -> Self {
        Self {
            path: path.clone(),
            segments: Self::parse_path_segments(&path),
            components: RouteComponents::new(),
            metadata: RouteMetadata {
                title: None,
                description: None,
                keywords: Vec::new(),
                open_graph: None,
                twitter_card: None,
                canonical_url: None,
                robots: None,
            },
            children: Vec::new(),
        }
    }

    /// パスをセグメントにパース
    fn parse_path_segments(path: &str) -> Vec<RouteSegment> {
        path.trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|segment| {
                if segment.starts_with("[[") && segment.ends_with("]]") {
                    // オプションキャッチオール [[...slug]]
                    let param = segment.strip_prefix("[[").unwrap()
                        .strip_suffix("]]").unwrap()
                        .strip_prefix("...").unwrap();
                    RouteSegment::optional_catch_all_segment(param.to_string())
                } else if segment.starts_with("[...") && segment.ends_with("]") {
                    // キャッチオール [...slug]
                    let param = segment.strip_prefix("[...").unwrap()
                        .strip_suffix("]").unwrap();
                    RouteSegment::catch_all_segment(param.to_string())
                } else if segment.starts_with('[') && segment.ends_with(']') {
                    // 動的 [id]
                    let param = segment.strip_prefix('[').unwrap()
                        .strip_suffix(']').unwrap();
                    RouteSegment::dynamic_segment(param.to_string())
                } else if segment.starts_with('(') && segment.ends_with(')') {
                    // ルートグループ (group) - 実際のルーティングには影響しない
                    RouteSegment {
                        name: segment.to_string(),
                        segment_type: RouteType::Group,
                        params: Vec::new(),
                        is_optional: false,
                    }
                } else if segment.starts_with('@') {
                    // 並列ルート @parallel
                    RouteSegment {
                        name: segment.to_string(),
                        segment_type: RouteType::Parallel,
                        params: Vec::new(),
                        is_optional: false,
                    }
                } else {
                    // 静的セグメント
                    RouteSegment::static_segment(segment.to_string())
                }
            })
            .collect()
    }

    /// ページコンポーネントを設定
    pub fn set_page(&mut self, component: ComponentIR) {
        self.components.page = Some(component);
    }

    /// レイアウトコンポーネントを設定
    pub fn set_layout(&mut self, component: ComponentIR) {
        self.components.layout = Some(component);
    }

    /// ローディングコンポーネントを設定
    pub fn set_loading(&mut self, component: ComponentIR) {
        self.components.loading = Some(component);
    }

    /// エラーコンポーネントを設定
    pub fn set_error(&mut self, component: ComponentIR) {
        self.components.error = Some(component);
    }

    /// 子ルートを追加
    pub fn add_child(&mut self, child: RouteIR) {
        self.children.push(child);
    }

    /// パスがこのルートにマッチするかチェック
    pub fn matches_path(&self, request_path: &str) -> Option<HashMap<String, String>> {
        let request_segments: Vec<&str> = request_path.trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if self.segments.len() != request_segments.len() {
            // 長さが違う場合はマッチしない（ただしオプションキャッチオールの場合を除く）
            if let Some(last_segment) = self.segments.last() {
                if matches!(last_segment.segment_type, RouteType::OptionalCatchAll) && request_segments.len() < self.segments.len() {
                    // オプションキャッチオールの場合、長さが短くてもOK
                } else {
                    return None;
                }
            } else {
                return None;
            }
        }

        let mut params = HashMap::new();

        for (i, segment) in self.segments.iter().enumerate() {
            let request_segment = request_segments.get(i);

            match segment.segment_type {
                RouteType::Static => {
                    if request_segment != Some(&segment.name.as_str()) {
                        return None;
                    }
                }
                RouteType::Dynamic => {
                    if let Some(param_name) = segment.params.first() {
                        if let Some(value) = request_segment {
                            params.insert(param_name.clone(), value.to_string());
                        } else {
                            return None;
                        }
                    }
                }
                RouteType::CatchAll => {
                    if let Some(param_name) = segment.params.first() {
                        let remaining_segments: Vec<String> = request_segments[i..].iter()
                            .map(|s| s.to_string())
                            .collect();
                        params.insert(param_name.clone(), remaining_segments.join("/"));
                        break; // キャッチオールは残りを全て消費
                    }
                }
                RouteType::OptionalCatchAll => {
                    if let Some(param_name) = segment.params.first() {
                        if i < request_segments.len() {
                            let remaining_segments: Vec<String> = request_segments[i..].iter()
                                .map(|s| s.to_string())
                                .collect();
                            params.insert(param_name.clone(), remaining_segments.join("/"));
                        } else {
                            params.insert(param_name.clone(), "".to_string());
                        }
                        break;
                    }
                }
                RouteType::Group | RouteType::Parallel | RouteType::Intercept => {
                    // これらのセグメントはルーティングに影響しない
                    continue;
                }
            }
        }

        Some(params)
    }
}

/// ルーティングテーブルIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RouteTableIR {
    pub routes: Vec<RouteIR>,
    pub base_path: String,
    pub middleware: Vec<ComponentIR>, // グローバルミドルウェア
}

impl RouteTableIR {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            base_path: "/".to_string(),
            middleware: Vec::new(),
        }
    }

    /// ルートを追加
    pub fn add_route(&mut self, route: RouteIR) {
        self.routes.push(route);
    }

    /// ミドルウェアを追加
    pub fn add_middleware(&mut self, middleware: ComponentIR) {
        self.middleware.push(middleware);
    }

    /// パスにマッチするルートを検索
    pub fn find_route(&self, path: &str) -> Option<(&RouteIR, HashMap<String, String>)> {
        for route in &self.routes {
            if let Some(params) = route.matches_path(path) {
                return Some((route, params));
            }
        }
        None
    }

    /// ネストされたルート構造を構築
    pub fn build_nested_routes(&mut self) {
        // TODO: ファイルシステム構造からネストされたルートを構築
        // app/ ディレクトリの構造を解析してRouteIRツリーを構築
    }
}

/// ナビゲーションIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavigationIR {
    pub from_path: String,
    pub to_path: String,
    pub navigation_type: NavigationType,
    pub state: Properties,
    pub replace: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NavigationType {
    Push,
    Replace,
    Back,
    Forward,
    Reload,
}

impl NavigationIR {
    pub fn new(from_path: String, to_path: String) -> Self {
        Self {
            from_path,
            to_path,
            navigation_type: NavigationType::Push,
            state: Properties::new(),
            replace: false,
        }
    }

    pub fn replace(mut self, replace: bool) -> Self {
        self.replace = replace;
        self
    }

    pub fn with_state(mut self, state: Properties) -> Self {
        self.state = state;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_parsing() {
        // 静的ルート
        let route = RouteIR::new("/about".to_string());
        assert_eq!(route.segments.len(), 1);
        assert_eq!(route.segments[0].name, "about");
        assert_eq!(route.segments[0].segment_type, RouteType::Static);

        // 動的ルート
        let route = RouteIR::new("/users/[id]".to_string());
        assert_eq!(route.segments.len(), 2);
        assert_eq!(route.segments[1].segment_type, RouteType::Dynamic);
        assert_eq!(route.segments[1].params, vec!["id"]);

        // キャッチオールルート
        let route = RouteIR::new("/docs/[...slug]".to_string());
        assert_eq!(route.segments[1].segment_type, RouteType::CatchAll);
        assert_eq!(route.segments[1].params, vec!["slug"]);
    }

    #[test]
    fn test_route_matching() {
        let route = RouteIR::new("/users/[id]".to_string());

        // マッチするパス
        let params = route.matches_path("/users/123").unwrap();
        assert_eq!(params.get("id"), Some(&"123".to_string()));

        // マッチしないパス
        assert!(route.matches_path("/users").is_none());
        assert!(route.matches_path("/users/123/profile").is_none());

        // キャッチオールルート
        let catch_all_route = RouteIR::new("/docs/[...slug]".to_string());
        let params = catch_all_route.matches_path("/docs/getting-started/installation").unwrap();
        assert_eq!(params.get("slug"), Some(&"getting-started/installation".to_string()));
    }

    #[test]
    fn test_route_table() {
        let mut table = RouteTableIR::new();

        let mut route1 = RouteIR::new("/".to_string());
        let page1 = ComponentIR::new("HomePage".to_string(), ComponentType::Page);
        route1.set_page(page1);
        table.add_route(route1);

        let mut route2 = RouteIR::new("/about".to_string());
        let page2 = ComponentIR::new("AboutPage".to_string(), ComponentType::Page);
        route2.set_page(page2);
        table.add_route(route2);

        // ルート検索
        let (route, params) = table.find_route("/about").unwrap();
        assert_eq!(route.path, "/about");
        assert!(params.is_empty());

        assert!(table.find_route("/nonexistent").is_none());
    }
}

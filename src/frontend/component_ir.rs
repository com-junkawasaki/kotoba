//! コンポーネントIR定義
//!
//! ReactコンポーネントをKotobaのIRで表現します。

use crate::types::{Properties, ContentHash, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// コンポーネントの種類
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentType {
    /// Server Component (デフォルト)
    Server,
    /// Client Component ('use client' ディレクティブ付き)
    Client,
    /// Layout Component (layout.js)
    Layout,
    /// Page Component (page.js)
    Page,
    /// Loading Component (loading.js)
    Loading,
    /// Error Component (error.js)
    Error,
    /// Template Component (template.js)
    Template,
    /// Not Found Component (not-found.js)
    NotFound,
}

/// コンポーネントの実行環境
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionEnvironment {
    /// サーバーサイドでのみ実行
    ServerOnly,
    /// クライアントサイドでのみ実行
    ClientOnly,
    /// サーバー/クライアント両方で実行可能
    Universal,
}

/// コンポーネントIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentIR {
    pub id: String,
    pub name: String,
    pub component_type: ComponentType,
    pub environment: ExecutionEnvironment,
    pub props: Properties,
    pub state: Properties,
    pub children: Vec<ComponentIR>,
    pub imports: Vec<ImportIR>,
    pub source_hash: ContentHash,
    pub metadata: Properties,
}

impl ComponentIR {
    pub fn new(name: String, component_type: ComponentType) -> Self {
        let environment = match component_type {
            ComponentType::Server => ExecutionEnvironment::ServerOnly,
            ComponentType::Client => ExecutionEnvironment::ClientOnly,
            _ => ExecutionEnvironment::Universal,
        };

        Self {
            id: format!("component_{}", uuid::Uuid::new_v4()),
            name,
            component_type,
            environment,
            props: Properties::new(),
            state: Properties::new(),
            children: Vec::new(),
            imports: Vec::new(),
            source_hash: ContentHash::sha256([0; 32]), // TODO: 実際のソースから計算
            metadata: Properties::new(),
        }
    }

    /// 子コンポーネントを追加
    pub fn add_child(&mut self, child: ComponentIR) {
        self.children.push(child);
    }

    /// Propsを設定
    pub fn set_prop(&mut self, key: String, value: Value) {
        self.props.insert(key, value);
    }

    /// Stateを設定
    pub fn set_state(&mut self, key: String, value: Value) {
        self.state.insert(key, value);
    }

    /// Importを追加
    pub fn add_import(&mut self, import: ImportIR) {
        self.imports.push(import);
    }
}

/// Import IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImportIR {
    pub module: String,
    pub specifiers: Vec<String>,
    pub is_default: bool,
    pub alias: Option<String>,
}

impl ImportIR {
    pub fn new(module: String) -> Self {
        Self {
            module,
            specifiers: Vec::new(),
            is_default: false,
            alias: None,
        }
    }

    pub fn add_specifier(&mut self, specifier: String) {
        self.specifiers.push(specifier);
    }

    pub fn set_default(&mut self, is_default: bool) {
        self.is_default = is_default;
    }

    pub fn set_alias(&mut self, alias: String) {
        self.alias = Some(alias);
    }
}

/// 仮想DOM要素IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ElementIR {
    pub tag_name: String,
    pub attributes: Properties,
    pub children: Vec<ElementChild>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementChild {
    Element(ElementIR),
    Text(String),
    Component(ComponentIR),
    Expression(String), // JSX式 {variable}
}

impl ElementIR {
    pub fn new(tag_name: String) -> Self {
        Self {
            tag_name,
            attributes: Properties::new(),
            children: Vec::new(),
            key: None,
        }
    }

    pub fn add_attribute(&mut self, key: String, value: Value) {
        self.attributes.insert(key, value);
    }

    pub fn add_child(&mut self, child: ElementChild) {
        self.children.push(child);
    }

    pub fn set_key(&mut self, key: String) {
        self.key = Some(key);
    }
}

/// フックIR (React Hooksの表現)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HookIR {
    UseState {
        variable: String,
        initial_value: Value,
    },
    UseEffect {
        dependencies: Vec<String>,
        effect_code: String,
    },
    UseContext {
        context_name: String,
        variable: String,
    },
    UseReducer {
        reducer_name: String,
        initial_state: Value,
        variable: String,
    },
    CustomHook {
        name: String,
        args: Vec<Value>,
        return_variable: String,
    },
}

/// イベントハンドラーIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventHandlerIR {
    pub event_type: String, // 'onClick', 'onChange', etc.
    pub handler_function: String, // 関数名
    pub prevent_default: bool,
    pub stop_propagation: bool,
}

/// コンポーネントメタデータ
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentMetadata {
    pub is_async: bool,
    pub uses_suspense: bool,
    pub has_error_boundary: bool,
    pub cache_strategy: Option<ComponentCacheStrategy>,
    pub revalidation_strategy: Option<RevalidationStrategy>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ComponentCacheStrategy {
    NoCache,
    Cache { duration: u64 },
    Revalidate { tags: Vec<String> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RevalidationStrategy {
    Never,
    OnDemand,
    TimeBased { interval: u64 },
    TagBased { tags: Vec<String> },
}

/// JSX/TSXパーサー結果のIR表現
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct JSXIR {
    pub elements: Vec<ElementIR>,
    pub components: Vec<ComponentIR>,
    pub hooks: Vec<HookIR>,
    pub event_handlers: Vec<EventHandlerIR>,
    pub metadata: ComponentMetadata,
}

impl JSXIR {
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            components: Vec::new(),
            hooks: Vec::new(),
            event_handlers: Vec::new(),
            metadata: ComponentMetadata {
                is_async: false,
                uses_suspense: false,
                has_error_boundary: false,
                cache_strategy: None,
                revalidation_strategy: None,
            },
        }
    }
}

/// コンポーネントツリーIR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComponentTreeIR {
    pub root: ComponentIR,
    pub route_context: Properties, // ルーティングコンテキスト
    pub global_state: Properties, // グローバル状態
}

impl ComponentTreeIR {
    pub fn new(root: ComponentIR) -> Self {
        Self {
            root,
            route_context: Properties::new(),
            global_state: Properties::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_ir_creation() {
        let mut component = ComponentIR::new("MyComponent".to_string(), ComponentType::Server);
        component.set_prop("title".to_string(), Value::String("Hello".to_string()));

        assert_eq!(component.name, "MyComponent");
        assert_eq!(component.component_type, ComponentType::Server);
        assert_eq!(component.environment, ExecutionEnvironment::ServerOnly);
    }

    #[test]
    fn test_element_ir_creation() {
        let mut element = ElementIR::new("div".to_string());
        element.add_attribute("className".to_string(), Value::String("container".to_string()));
        element.add_child(ElementChild::Text("Hello World".to_string()));

        assert_eq!(element.tag_name, "div");
        assert_eq!(element.children.len(), 1);
    }

    #[test]
    fn test_import_ir_creation() {
        let mut import = ImportIR::new("react".to_string());
        import.add_specifier("useState".to_string());
        import.add_specifier("useEffect".to_string());

        assert_eq!(import.module, "react");
        assert_eq!(import.specifiers.len(), 2);
    }
}

//! Template Engine Module
//!
//! このモジュールは様々なテンプレートエンジンの統合を提供します。
//! Tera、Handlebars、Liquidなどのテンプレートエンジンをサポートします。

use crate::{HandlerError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// サポートされるテンプレートエンジン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TemplateEngineType {
    Tera,
    Handlebars,
    Liquid,
}

/// テンプレート設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateConfig {
    pub engine: TemplateEngineType,
    pub template_dir: String,
    pub cache_enabled: bool,
    pub autoescape: bool,
    pub custom_filters: Vec<String>,
}

/// テンプレートエンジン
pub struct TemplateEngine {
    config: TemplateConfig,
    #[cfg(feature = "tera")]
    tera: Option<tera::Tera>,
    #[cfg(feature = "handlebars")]
    handlebars: Option<handlebars::Handlebars<'static>>,
}

impl TemplateEngine {
    /// 新しいテンプレートエンジンを作成
    pub fn new(template_dir: &str) -> Result<Self> {
        let config = TemplateConfig {
            engine: TemplateEngineType::Tera, // デフォルトはTera
            template_dir: template_dir.to_string(),
            cache_enabled: true,
            autoescape: true,
            custom_filters: vec![],
        };

        Self::with_config(config)
    }

    /// 設定付きでテンプレートエンジンを作成
    pub fn with_config(config: TemplateConfig) -> Result<Self> {
        let mut engine = Self {
            config: config.clone(),
            #[cfg(feature = "tera")]
            tera: None,
            #[cfg(feature = "handlebars")]
            handlebars: None,
        };

        engine.initialize()?;
        Ok(engine)
    }

    /// テンプレートエンジンを初期化
    fn initialize(&mut self) -> Result<()> {
        match self.config.engine {
            #[cfg(feature = "tera")]
            TemplateEngineType::Tera => {
                let mut tera = tera::Tera::new(&format!("{}/**/*.html", self.config.template_dir))
                    .map_err(|e| HandlerError::Jsonnet(format!("Tera initialization error: {}", e)))?;

                if self.config.autoescape {
                    tera.autoescape_on(vec![".html", ".htm"]);
                }

                // カスタムフィルターの登録
                for filter in &self.config.custom_filters {
                    // カスタムフィルターの実装は必要に応じて追加
                }

                self.tera = Some(tera);
            }

            #[cfg(feature = "handlebars")]
            TemplateEngineType::Handlebars => {
                let mut handlebars = handlebars::Handlebars::new();

                handlebars
                    .register_templates_directory(".html", &self.config.template_dir)
                    .map_err(|e| HandlerError::Jsonnet(format!("Handlebars initialization error: {}", e)))?;

                // カスタムヘルパーの登録
                for helper in &self.config.custom_filters {
                    // カスタムヘルパーの実装は必要に応じて追加
                }

                self.handlebars = Some(handlebars);
            }

            #[cfg(not(any(feature = "tera", feature = "handlebars")))]
            _ => {
                return Err(HandlerError::Jsonnet(
                    "No template engine features enabled. Enable 'tera' or 'handlebars' feature.".to_string()
                ));
            }
        }

        Ok(())
    }

    /// テンプレートをレンダリング
    pub fn render(&self, template_name: &str, context: &serde_json::Value) -> Result<String> {
        match self.config.engine {
            #[cfg(feature = "tera")]
            TemplateEngineType::Tera => {
                if let Some(tera) = &self.tera {
                    let context = tera::Context::from_serialize(context)
                        .map_err(|e| HandlerError::Jsonnet(format!("Context serialization error: {}", e)))?;

                    tera.render(template_name, &context)
                        .map_err(|e| HandlerError::Jsonnet(format!("Template rendering error: {}", e)))
                } else {
                    Err(HandlerError::Jsonnet("Tera engine not initialized".to_string()))
                }
            }

            #[cfg(feature = "handlebars")]
            TemplateEngineType::Handlebars => {
                if let Some(handlebars) = &self.handlebars {
                    handlebars.render(template_name, context)
                        .map_err(|e| HandlerError::Jsonnet(format!("Template rendering error: {}", e)))
                } else {
                    Err(HandlerError::Jsonnet("Handlebars engine not initialized".to_string()))
                }
            }

            #[cfg(not(any(feature = "tera", feature = "handlebars")))]
            _ => {
                Err(HandlerError::Jsonnet(
                    "No template engine features enabled".to_string()
                ))
            }
        }
    }

    /// テンプレートをレンダリング（HashMapコンテキスト）
    pub fn render_with_hashmap(&self, template_name: &str, context: &HashMap<String, serde_json::Value>) -> Result<String> {
        let json_context: serde_json::Value = serde_json::to_value(context)
            .map_err(|e| HandlerError::Jsonnet(format!("Context conversion error: {}", e)))?;

        self.render(template_name, &json_context)
    }

    /// レイアウト付きでテンプレートをレンダリング
    pub fn render_with_layout(&self, template_name: &str, layout_name: &str, context: &serde_json::Value) -> Result<String> {
        let content = self.render(template_name, context)?;

        let mut layout_context = context.clone();
        if let Some(obj) = layout_context.as_object_mut() {
            obj.insert("content".to_string(), serde_json::Value::String(content));
        }

        self.render(layout_name, &layout_context)
    }

    /// パーシャルテンプレートをレンダリング
    pub fn render_partial(&self, template_name: &str, context: &serde_json::Value) -> Result<String> {
        self.render(template_name, context)
    }

    /// テンプレートが存在するかチェック
    pub fn has_template(&self, template_name: &str) -> bool {
        match self.config.engine {
            #[cfg(feature = "tera")]
            TemplateEngineType::Tera => {
                self.tera.as_ref().map_or(false, |tera| tera.get_template_names().any(|name| name == template_name))
            }

            #[cfg(feature = "handlebars")]
            TemplateEngineType::Handlebars => {
                self.handlebars.as_ref().map_or(false, |hb| hb.has_template(template_name))
            }

            #[cfg(not(any(feature = "tera", feature = "handlebars")))]
            _ => false,
        }
    }

    /// 利用可能なテンプレート一覧を取得
    pub fn get_template_names(&self) -> Vec<String> {
        match self.config.engine {
            #[cfg(feature = "tera")]
            TemplateEngineType::Tera => {
                self.tera.as_ref()
                    .map(|tera| tera.get_template_names().map(|s| s.to_string()).collect())
                    .unwrap_or_default()
            }

            #[cfg(feature = "handlebars")]
            TemplateEngineType::Handlebars => {
                // Handlebars doesn't provide a direct way to get template names
                vec![]
            }

            #[cfg(not(any(feature = "tera", feature = "handlebars")))]
            _ => vec![],
        }
    }

    /// カスタムフィルターを登録（Tera用）
    #[cfg(feature = "tera")]
    pub fn register_tera_filter<F>(&mut self, name: &str, filter: F) -> Result<()>
    where
        F: tera::Filter + Send + Sync + 'static,
    {
        if let Some(tera) = &mut self.tera {
            tera.register_filter(name, filter);
            Ok(())
        } else {
            Err(HandlerError::Jsonnet("Tera engine not initialized".to_string()))
        }
    }

    /// カスタム関数を登録（Tera用）
    #[cfg(feature = "tera")]
    pub fn register_tera_function<F>(&mut self, name: &str, function: F) -> Result<()>
    where
        F: tera::Function + Send + Sync + 'static,
    {
        if let Some(tera) = &mut self.tera {
            tera.register_function(name, function);
            Ok(())
        } else {
            Err(HandlerError::Jsonnet("Tera engine not initialized".to_string()))
        }
    }

    /// カスタムヘルパーを登録（Handlebars用）
    #[cfg(feature = "handlebars")]
    pub fn register_handlebars_helper<F>(&mut self, name: &str, helper: F) -> Result<()>
    where
        F: handlebars::HelperDef + Send + Sync + 'static,
    {
        if let Some(handlebars) = &mut self.handlebars {
            handlebars.register_helper(name, Box::new(helper));
            Ok(())
        } else {
            Err(HandlerError::Jsonnet("Handlebars engine not initialized".to_string()))
        }
    }

    /// テンプレートをリロード（開発時用）
    pub fn reload_templates(&mut self) -> Result<()> {
        self.initialize()
    }

    /// 設定を取得
    pub fn config(&self) -> &TemplateConfig {
        &self.config
    }

    /// エンジンタイプを取得
    pub fn engine_type(&self) -> &TemplateEngineType {
        &self.config.engine
    }
}

/// ユーティリティ関数

/// シンプルなテンプレートレンダリング（設定不要）
pub fn render_simple_template(template: &str, context: &HashMap<String, String>) -> Result<String> {
    let mut result = template.to_string();

    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    Ok(result)
}

/// JSONコンテキストをHashMapに変換
pub fn json_to_hashmap(json: &serde_json::Value) -> Result<HashMap<String, String>> {
    let mut result = HashMap::new();

    if let Some(obj) = json.as_object() {
        for (key, value) in obj {
            if let Some(str_value) = value.as_str() {
                result.insert(key.clone(), str_value.to_string());
            } else {
                // 他の型も文字列に変換
                result.insert(key.clone(), value.to_string());
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_simple_template_rendering() {
        let template = "<h1>{{title}}</h1><p>{{content}}</p>";
        let mut context = HashMap::new();
        context.insert("title".to_string(), "Hello".to_string());
        context.insert("content".to_string(), "World".to_string());

        let result = render_simple_template(template, &context).unwrap();
        assert_eq!(result, "<h1>Hello</h1><p>World</p>");
    }

    #[test]
    fn test_json_to_hashmap() {
        let json = json!({
            "name": "Alice",
            "age": 30,
            "city": "Tokyo"
        });

        let result = json_to_hashmap(&json).unwrap();
        assert_eq!(result.get("name").unwrap(), "Alice");
        assert_eq!(result.get("age").unwrap(), "30");
        assert_eq!(result.get("city").unwrap(), "Tokyo");
    }

    #[cfg(feature = "tera")]
    #[test]
    fn test_template_config_creation() {
        let config = TemplateConfig {
            engine: TemplateEngineType::Tera,
            template_dir: "templates".to_string(),
            cache_enabled: true,
            autoescape: true,
            custom_filters: vec!["markdown".to_string()],
        };

        assert_eq!(config.template_dir, "templates");
        assert!(config.cache_enabled);
        assert!(config.autoescape);
    }

    #[cfg(feature = "handlebars")]
    #[test]
    fn test_handlebars_config_creation() {
        let config = TemplateConfig {
            engine: TemplateEngineType::Handlebars,
            template_dir: "templates".to_string(),
            cache_enabled: false,
            autoescape: false,
            custom_filters: vec![],
        };

        assert_eq!(config.template_dir, "templates");
        assert!(!config.cache_enabled);
        assert!(!config.autoescape);
    }
}

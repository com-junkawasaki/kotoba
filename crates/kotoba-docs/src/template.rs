//! テンプレート処理モジュール

use super::{DocsConfig, DocItem, Result, DocsError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tera::{Context, Tera, Value};
use serde_json;

/// テンプレートエンジン
pub struct TemplateEngine {
    /// Teraインスタンス
    tera: Tera,

    /// テンプレートディレクトリ
    template_dir: Option<PathBuf>,

    /// カスタムフィルター
    custom_filters: HashMap<String, Box<dyn TemplateFilter>>,
}

/// テンプレートフィルター
pub trait TemplateFilter: Send + Sync {
    fn filter(&self, value: &Value, args: &HashMap<String, Value>) -> tera::Result<Value>;
}

/// ドキュメントテンプレート
#[derive(Debug, Clone)]
pub struct DocTemplate {
    /// テンプレート名
    pub name: String,

    /// テンプレートの内容
    pub content: String,

    /// テンプレートの種類
    pub template_type: TemplateType,
}

/// テンプレートの種類
#[derive(Debug, Clone, PartialEq)]
pub enum TemplateType {
    /// HTMLページ
    Html,

    /// Markdownファイル
    Markdown,

    /// 部分テンプレート
    Partial,

    /// レイアウト
    Layout,
}

/// テンプレートコンテキスト
#[derive(Debug)]
pub struct TemplateContext {
    /// 設定
    pub config: DocsConfig,

    /// 現在のドキュメント項目
    pub item: Option<DocItem>,

    /// 全てのドキュメント項目
    pub items: Vec<DocItem>,

    /// 追加の変数
    pub variables: HashMap<String, Value>,

    /// 生成日時
    pub generated_at: chrono::DateTime<chrono::Utc>,
}

impl TemplateEngine {
    /// 新しいテンプレートエンジンを作成
    pub fn new(template_dir: Option<PathBuf>) -> Self {
        let mut tera = Tera::default();

        // デフォルトテンプレートを登録
        Self::register_default_templates(&mut tera);

        // カスタムテンプレートを読み込み
        if let Some(dir) = &template_dir {
            if dir.exists() {
                let pattern = format!("{}/**/*.html", dir.display());
                if let Err(e) = tera.add_template_files(&[pattern]) {
                    println!("Warning: Failed to load custom templates: {}", e);
                }
            }
        }

        Self {
            tera,
            template_dir,
            custom_filters: HashMap::new(),
        }
    }

    /// テンプレートをレンダリング
    pub fn render(&self, template_name: &str, context: &TemplateContext) -> Result<String> {
        let mut tera_context = self.create_tera_context(context);

        self.tera.render(template_name, &tera_context)
            .map_err(|e| DocsError::Template(format!("Failed to render template '{}': {}", template_name, e)))
    }

    /// テンプレートが存在するかチェック
    pub fn has_template(&self, name: &str) -> bool {
        self.tera.templates.contains_key(name)
    }

    /// カスタムフィルターを登録
    pub fn register_filter<F>(&mut self, name: String, filter: F)
    where
        F: TemplateFilter + 'static,
    {
        self.custom_filters.insert(name, Box::new(filter));
    }

    /// テンプレートを追加
    pub fn add_template(&mut self, name: String, content: String) -> Result<()> {
        self.tera.add_raw_template(&name, &content)
            .map_err(|e| DocsError::Template(format!("Failed to add template '{}': {}", name, e)))
    }

    /// テンプレートファイルを再読み込み
    pub fn reload_templates(&mut self) -> Result<()> {
        if let Some(dir) = &self.template_dir {
            if dir.exists() {
                let pattern = format!("{}/**/*.html", dir.display());
                self.tera.full_reload()
                    .map_err(|e| DocsError::Template(format!("Failed to reload templates: {}", e)))?;
            }
        }
        Ok(())
    }

    /// Teraコンテキストを作成
    fn create_tera_context(&self, context: &TemplateContext) -> Context {
        let mut tera_context = Context::new();

        // 設定を追加
        tera_context.insert("config", &context.config);
        tera_context.insert("generated_at", &context.generated_at.to_rfc3339());

        // ドキュメント項目を追加
        if let Some(item) = &context.item {
            tera_context.insert("item", item);
            tera_context.insert("item_id", &item.id);
            tera_context.insert("item_name", &item.name);
            tera_context.insert("item_content", &item.content);
            tera_context.insert("item_signature", &item.signature);
            tera_context.insert("item_doc_type", &format!("{:?}", item.doc_type));
        }

        // 全項目を追加
        tera_context.insert("items", &context.items);
        tera_context.insert("total_items", &context.items.len());

        // グループ化された項目を追加
        let grouped = self.group_items_by_type(&context.items);
        tera_context.insert("grouped_items", &grouped);

        // 統計情報を追加
        let stats = self.generate_stats(&context.items);
        tera_context.insert("stats", &stats);

        // カスタム変数を追加
        for (key, value) in &context.variables {
            tera_context.insert(key, value);
        }

        // ユーティリティ関数を追加
        tera_context.insert("utils", &TemplateUtils);

        tera_context
    }

    /// デフォルトテンプレートを登録
    fn register_default_templates(tera: &mut Tera) {
        // インデックスページ
        let index_template = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ config.name }}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="sidebar">
        <h2>{{ config.name }}</h2>
        {% if config.description %}
        <p>{{ config.description }}</p>
        {% endif %}

        <div class="search-box">
            <input type="text" class="search-input" placeholder="Search...">
            <div class="search-results"></div>
        </div>

        <nav>
            <ul class="nav-list">
                {% for type_name, type_items in grouped_items %}
                <li>
                    <strong>{{ type_name }}</strong>
                    <ul>
                        {% for item in type_items | slice(end=10) %}
                        <li><a href="{{ item.slug }}.html">{{ item.name }}</a></li>
                        {% endfor %}
                        {% if type_items | length > 10 %}
                        <li><a href="{{ type_name | lower }}.html">... and {{ type_items | length - 10 }} more</a></li>
                        {% endfor %}
                    </ul>
                </li>
                {% endfor %}
            </ul>
        </nav>
    </div>

    <div class="main-content">
        <header>
            <h1>{{ config.name }}</h1>
            {% if config.description %}
            <p>{{ config.description }}</p>
            {% endif %}

            <div class="stats">
                <p>{{ total_items }} items documented</p>
                <p>Generated on {{ generated_at | date:format="%Y-%m-%d %H:%M" }}</p>
            </div>
        </header>

        <main>
            {% for type_name, type_items in grouped_items %}
            <section class="doc-section">
                <h2>{{ type_name }}</h2>
                <div class="items-grid">
                    {% for item in type_items %}
                    <article class="item-card">
                        <h3><a href="{{ item.slug }}.html">{{ item.name }}</a></h3>
                        {% if item.signature %}
                        <pre><code>{{ item.signature }}</code></pre>
                        {% endif %}
                        <p>{{ item.content | truncate(100) | markdown }}</p>
                    </article>
                    {% endfor %}
                </div>
            </section>
            {% endfor %}
        </main>

        <footer class="footer">
            <p>Generated by Kotoba Docs</p>
        </footer>
    </div>

    <script src="script.js"></script>
</body>
</html>
"#;

        // 個別項目ページ
        let item_template = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ item.name }} - {{ config.name }}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="sidebar">
        <h2>{{ config.name }}</h2>
        <nav>
            <ul class="nav-list">
                <li><a href="index.html">← Home</a></li>
                {% for type_name, type_items in grouped_items %}
                <li><a href="{{ type_name | lower }}.html">{{ type_name }}</a></li>
                {% endfor %}
            </ul>
        </nav>
    </div>

    <div class="main-content">
        <header>
            <nav class="breadcrumbs">
                <a href="index.html">Home</a> >
                <a href="{{ item_doc_type | lower }}.html">{{ item_doc_type }}</a> >
                <span>{{ item.name }}</span>
            </nav>

            <h1>{{ item.name }}</h1>
            {% if item_signature %}
            <div class="signature">
                <pre><code>{{ item_signature }}</code></pre>
            </div>
            {% endif %}
        </header>

        <main>
            <div class="content">
                {{ item_content | markdown }}
            </div>

            {% if item.tags %}
            <section class="tags">
                <h3>Tags</h3>
                <div class="tag-list">
                    {% for tag in item.tags %}
                    <span class="tag">{{ tag }}</span>
                    {% endfor %}
                </div>
            </section>
            {% endif %}
        </main>

        <footer class="footer">
            <p>Generated by Kotoba Docs on {{ generated_at | date:format="%Y-%m-%d %H:%M" }}</p>
        </footer>
    </div>

    <script src="script.js"></script>
</body>
</html>
"#;

        // モジュールページ
        let module_template = r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{{ doc_type }} - {{ config.name }}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <div class="sidebar">
        <h2>{{ config.name }}</h2>
        <nav>
            <ul class="nav-list">
                <li><a href="index.html">Home</a></li>
                {% for type_name, _ in grouped_items %}
                <li><a href="{{ type_name | lower }}.html">{{ type_name }}</a></li>
                {% endfor %}
            </ul>
        </nav>
    </div>

    <div class="main-content">
        <header>
            <h1>{{ doc_type }}</h1>
            <p>{{ items | length }} items</p>
        </header>

        <main>
            <div class="items-list">
                {% for item in items %}
                <article class="item-summary">
                    <h3><a href="{{ item.slug }}.html">{{ item.name }}</a></h3>
                    {% if item.signature %}
                    <code>{{ item.signature }}</code>
                    {% endif %}
                    <p>{{ item.content | truncate(150) }}</p>
                </article>
                {% endfor %}
            </div>
        </main>

        <footer class="footer">
            <p>Generated by Kotoba Docs</p>
        </footer>
    </div>

    <script src="script.js"></script>
</body>
</html>
"#;

        // テンプレートを登録
        tera.add_raw_template("index.html", index_template).unwrap();
        tera.add_raw_template("item.html", item_template).unwrap();
        tera.add_raw_template("module.html", module_template).unwrap();
    }

    /// 項目をタイプごとにグループ化
    fn group_items_by_type(&self, items: &[DocItem]) -> HashMap<String, Vec<DocItem>> {
        let mut grouped = HashMap::new();

        for item in items {
            let type_name = format!("{:?}", item.doc_type);
            grouped.entry(type_name).or_insert_with(Vec::new).push(item.clone());
        }

        grouped
    }

    /// 統計情報を生成
    fn generate_stats(&self, items: &[DocItem]) -> TemplateStats {
        let mut stats = TemplateStats::default();

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
}

/// テンプレート統計
#[derive(serde::Serialize)]
pub struct TemplateStats {
    pub total: usize,
    pub modules: usize,
    pub functions: usize,
    pub structs: usize,
    pub enums: usize,
    pub traits: usize,
    pub constants: usize,
    pub macros: usize,
    pub type_aliases: usize,
    pub methods: usize,
    pub fields: usize,
    pub variants: usize,
    pub associated_types: usize,
    pub associated_constants: usize,
}

impl Default for TemplateStats {
    fn default() -> Self {
        Self {
            total: 0,
            modules: 0,
            functions: 0,
            structs: 0,
            enums: 0,
            traits: 0,
            constants: 0,
            macros: 0,
            type_aliases: 0,
            methods: 0,
            fields: 0,
            variants: 0,
            associated_types: 0,
            associated_constants: 0,
        }
    }
}

/// テンプレートユーティリティ
#[derive(serde::Serialize)]
pub struct TemplateUtils;

/// Teraフィルターの実装

/// Markdownフィルター
pub struct MarkdownFilter;

impl TemplateFilter for MarkdownFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        if let Some(text) = value.as_str() {
            // 簡易的なMarkdown変換
            let html = text
                .replace("**", "<strong>")
                .replace("**", "</strong>")
                .replace("*", "<em>")
                .replace("*", "</em>")
                .replace("`", "<code>")
                .replace("`", "</code>")
                .replace("\n", "<br>");

            Ok(Value::String(html))
        } else {
            Ok(value.clone())
        }
    }
}

/// Slugフィルター
pub struct SlugFilter;

impl TemplateFilter for SlugFilter {
    fn filter(&self, value: &Value, _args: &HashMap<String, Value>) -> tera::Result<Value> {
        if let Some(text) = value.as_str() {
            let slug = text
                .to_lowercase()
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == '-')
                .collect::<String>()
                .replace(" ", "-");

            Ok(Value::String(slug))
        } else {
            Ok(value.clone())
        }
    }
}

/// テンプレートヘルパー関数

/// テンプレートコンテキストを作成
pub fn create_context(config: DocsConfig, items: Vec<DocItem>) -> TemplateContext {
    TemplateContext {
        config,
        item: None,
        items,
        variables: HashMap::new(),
        generated_at: chrono::Utc::now(),
    }
}

/// 項目用のコンテキストを作成
pub fn create_item_context(config: DocsConfig, item: DocItem, all_items: Vec<DocItem>) -> TemplateContext {
    TemplateContext {
        config,
        item: Some(item),
        items: all_items,
        variables: HashMap::new(),
        generated_at: chrono::Utc::now(),
    }
}

/// モジュール用のコンテキストを作成
pub fn create_module_context(
    config: DocsConfig,
    module_type: String,
    module_items: Vec<DocItem>,
    all_items: Vec<DocItem>,
) -> TemplateContext {
    let mut variables = HashMap::new();
    variables.insert("doc_type".to_string(), Value::String(module_type));

    TemplateContext {
        config,
        item: None,
        items: all_items,
        variables,
        generated_at: chrono::Utc::now(),
    }
}

/// デフォルトのテンプレートエンジンを作成
pub fn create_default_engine(template_dir: Option<PathBuf>) -> TemplateEngine {
    let mut engine = TemplateEngine::new(template_dir);

    // カスタムフィルターを登録
    engine.register_filter("markdown".to_string(), MarkdownFilter);
    engine.register_filter("slug".to_string(), SlugFilter);

    engine
}

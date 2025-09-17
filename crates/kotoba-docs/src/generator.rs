//! ドキュメントジェネレーションモジュール

use super::{DocItem, DocsConfig, GenerateResult, OutputFormat, Result, DocsError};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};
use pulldown_cmark::{Options, Parser};
use indicatif::{ProgressBar, ProgressStyle};
use chrono::Utc;

/// ドキュメントジェネレータ
pub struct DocGenerator {
    config: DocsConfig,
    tera: Tera,
    items: Vec<DocItem>,
    item_map: HashMap<String, DocItem>,
}

impl DocGenerator {
    /// 新しいジェネレータを作成
    pub fn new(config: DocsConfig, items: Vec<DocItem>) -> Self {
        let mut tera = Tera::default();

        // テンプレートディレクトリを設定
        if let Some(template_dir) = &config.template_dir {
            let template_pattern = format!("{}/**/*.html", template_dir.display());
            if let Err(e) = tera.add_template_files(vec![(template_pattern, None::<&str>)]) {
                println!("Warning: Failed to load templates from {}: {}", template_dir.display(), e);
            }
        } else {
            // デフォルトテンプレートを使用
            Self::load_default_templates(&mut tera);
        }

        let mut item_map = HashMap::new();
        for item in &items {
            item_map.insert(item.id.clone(), item.clone());
        }

        Self {
            config,
            tera,
            items,
            item_map,
        }
    }

    /// ドキュメントを生成
    pub async fn generate(&self) -> Result<GenerateResult> {
        let start_time = std::time::Instant::now();

        println!("🚀 Starting documentation generation...");
        println!("📁 Output directory: {}", self.config.output_dir.display());
        println!("📄 Formats: {:?}", self.config.formats);

        // 出力ディレクトリを作成
        fs::create_dir_all(&self.config.output_dir)
            .map_err(|e| DocsError::Io(e))?;

        let mut total_docs = 0;
        let mut errors = 0;

        // プログレスバーを作成
        let pb = ProgressBar::new(self.config.formats.len() as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );

        // 各出力形式でドキュメントを生成
        for format in &self.config.formats {
            pb.set_message(format!("Generating {}", format_name(format)));

            match format {
                OutputFormat::Html => {
                    let count = self.generate_html().await?;
                    total_docs += count;
                }
                OutputFormat::Markdown => {
                    let count = self.generate_markdown().await?;
                    total_docs += count;
                }
                OutputFormat::Json => {
                    let count = self.generate_json().await?;
                    total_docs += count;
                }
                OutputFormat::Pdf => {
                    // PDF生成は将来の拡張
                    println!("⚠️  PDF generation not yet implemented");
                }
            }

            pb.inc(1);
        }

        pb.finish_with_message("Documentation generation completed");

        // 検索インデックスを生成
        if self.config.search.enabled {
            self.generate_search_index().await?;
        }

        // 静的アセットをコピー
        self.copy_static_assets().await?;

        let generation_time = start_time.elapsed();

        Ok(GenerateResult::new()
            .success(total_docs, self.items.len(), self.config.output_dir.clone(), generation_time))
    }

    /// HTMLドキュメントを生成
    async fn generate_html(&self) -> Result<usize> {
        let html_dir = self.config.output_dir.join("html");
        fs::create_dir_all(&html_dir)
            .map_err(|e| DocsError::Io(e))?;

        let mut count = 0;

        // インデックスページを生成
        self.generate_html_index(&html_dir).await?;
        count += 1;

        // 各ドキュメント項目のページを生成
        for item in &self.items {
            self.generate_html_page(item, &html_dir).await?;
            count += 1;
        }

        // モジュールページを生成
        let modules = self.group_items_by_type();
        for (doc_type, items) in modules {
            self.generate_html_module_page(&doc_type, &items, &html_dir).await?;
            count += 1;
        }

        Ok(count)
    }

    /// HTMLインデックスページを生成
    async fn generate_html_index(&self, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();

        // 設定情報を追加
        context.insert("config", &self.config);
        context.insert("items", &self.items);
        context.insert("modules", &self.group_items_by_type());
        context.insert("generated_at", &Utc::now().to_rfc3339());

        // 統計情報を追加
        let stats = self.generate_stats();
        context.insert("stats", &stats);

        let html = self.tera.render("index.html", &context)
            .map_err(|e| DocsError::Template(format!("Failed to render index.html: {}", e)))?;

        let index_path = output_dir.join("index.html");
        fs::write(&index_path, html)
            .map_err(|e| DocsError::Io(e))?;

        println!("✅ Generated index.html");
        Ok(())
    }

    /// HTMLページを生成
    async fn generate_html_page(&self, item: &DocItem, output_dir: &Path) -> Result<()> {
        let mut context = Context::new();

        context.insert("config", &self.config);
        context.insert("item", item);
        context.insert("related_items", &self.get_related_items(item));
        context.insert("breadcrumbs", &self.generate_breadcrumbs(item));
        context.insert("generated_at", &Utc::now().to_rfc3339());

        // マークダウンをHTMLに変換
        let html_content = self.markdown_to_html(&item.content);
        context.insert("html_content", &html_content);

        let html = self.tera.render("item.html", &context)
            .map_err(|e| DocsError::Template(format!("Failed to render item.html: {}", e)))?;

        // ファイルパスを生成
        let file_name = format!("{}.html", item.slug());
        let file_path = output_dir.join(file_name);

        fs::write(&file_path, html)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// HTMLモジュールページを生成
    async fn generate_html_module_page(&self, doc_type: &str, items: &[DocItem], output_dir: &Path) -> Result<()> {
        let mut context = Context::new();

        context.insert("config", &self.config);
        context.insert("doc_type", doc_type);
        context.insert("items", items);
        context.insert("generated_at", &Utc::now().to_rfc3339());

        let html = self.tera.render("module.html", &context)
            .map_err(|e| DocsError::Template(format!("Failed to render module.html: {}", e)))?;

        let file_name = format!("{}.html", doc_type.to_lowercase());
        let file_path = output_dir.join(file_name);

        fs::write(&file_path, html)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// マークダウンドキュメントを生成
    async fn generate_markdown(&self) -> Result<usize> {
        let md_dir = self.config.output_dir.join("markdown");
        fs::create_dir_all(&md_dir)
            .map_err(|e| DocsError::Io(e))?;

        let mut count = 0;

        // READMEファイルを生成
        self.generate_markdown_readme(&md_dir).await?;
        count += 1;

        // 各ドキュメント項目のMarkdownファイルを生成
        for item in &self.items {
            self.generate_markdown_file(item, &md_dir).await?;
            count += 1;
        }

        Ok(count)
    }

    /// Markdown READMEを生成
    async fn generate_markdown_readme(&self, output_dir: &Path) -> Result<()> {
        let mut content = format!("# {}\n\n", self.config.name);

        if let Some(description) = &self.config.description {
            content.push_str(&format!("{}\n\n", description));
        }

        content.push_str("## Modules\n\n");

        let modules = self.group_items_by_type();
        for (doc_type, items) in modules {
            content.push_str(&format!("### {}\n\n", doc_type));
            for item in items {
                content.push_str(&format!("- [{}]({}.md) - {}\n",
                    item.name,
                    item.slug(),
                    item.content.lines().next().unwrap_or("")));
            }
            content.push_str("\n");
        }

        let readme_path = output_dir.join("README.md");
        fs::write(&readme_path, content)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// Markdownファイルを生成
    async fn generate_markdown_file(&self, item: &DocItem, output_dir: &Path) -> Result<()> {
        let mut content = format!("# {}\n\n", item.name);

        if let Some(signature) = &item.signature {
            content.push_str(&format!("```rust\n{}\n```\n\n", signature));
        }

        content.push_str(&item.content);
        content.push_str("\n\n");

        // 関連項目を追加
        if !item.related_items.is_empty() {
            content.push_str("## Related\n\n");
            for related_id in &item.related_items {
                if let Some(related_item) = self.item_map.get(related_id) {
                    content.push_str(&format!("- [{}]({}.md)\n",
                        related_item.name,
                        related_item.slug()));
                }
            }
            content.push_str("\n");
        }

        let file_name = format!("{}.md", item.slug());
        let file_path = output_dir.join(file_name);

        fs::write(&file_path, content)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// JSONドキュメントを生成
    async fn generate_json(&self) -> Result<usize> {
        let json_dir = self.config.output_dir.join("json");
        fs::create_dir_all(&json_dir)
            .map_err(|e| DocsError::Io(e))?;

        // 全項目のJSONを生成
        let json_data = serde_json::to_string_pretty(&self.items)
            .map_err(|e| DocsError::Json(e))?;

        let items_path = json_dir.join("items.json");
        fs::write(&items_path, json_data)
            .map_err(|e| DocsError::Io(e))?;

        // 設定のJSONを生成
        let config_data = serde_json::to_string_pretty(&self.config)
            .map_err(|e| DocsError::Json(e))?;

        let config_path = json_dir.join("config.json");
        fs::write(&config_path, config_data)
            .map_err(|e| DocsError::Io(e))?;

        // 統計情報のJSONを生成
        let stats = self.generate_stats();
        let stats_data = serde_json::to_string_pretty(&stats)
            .map_err(|e| DocsError::Json(e))?;

        let stats_path = json_dir.join("stats.json");
        fs::write(&stats_path, stats_data)
            .map_err(|e| DocsError::Io(e))?;

        Ok(3) // 3つのJSONファイルを生成
    }

    /// 検索インデックスを生成
    async fn generate_search_index(&self) -> Result<()> {
        let index_dir = self.config.output_dir.join("html");
        fs::create_dir_all(&index_dir)
            .map_err(|e| DocsError::Io(e))?;

        #[derive(serde::Serialize)]
        struct SearchEntry {
            id: String,
            title: String,
            content: String,
            url: String,
            doc_type: String,
        }

        let mut entries = vec![];

        for item in &self.items {
            entries.push(SearchEntry {
                id: item.id.clone(),
                title: item.name.clone(),
                content: item.content.clone(),
                url: format!("{}.html", item.slug()),
                doc_type: format!("{:?}", item.doc_type),
            });
        }

        let index_data = serde_json::to_string_pretty(&entries)
            .map_err(|e| DocsError::Json(e))?;

        let index_path = index_dir.join(&self.config.search.index_file);
        fs::write(&index_path, index_data)
            .map_err(|e| DocsError::Io(e))?;

        println!("✅ Generated search index: {}", self.config.search.index_file);
        Ok(())
    }

    /// 静的アセットをコピー
    async fn copy_static_assets(&self) -> Result<()> {
        // デフォルトのCSSとJSを生成
        self.generate_default_css().await?;
        self.generate_default_js().await?;

        Ok(())
    }

    /// デフォルトCSSを生成
    async fn generate_default_css(&self) -> Result<()> {
        let css_content = r#"
:root {
    --primary-color: #2563eb;
    --secondary-color: #64748b;
    --background-color: #ffffff;
    --text-color: #1f2937;
    --border-color: #e5e7eb;
    --code-background: #f8fafc;
}

* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    line-height: 1.6;
    color: var(--text-color);
    background-color: var(--background-color);
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
}

header {
    border-bottom: 1px solid var(--border-color);
    padding-bottom: 2rem;
    margin-bottom: 2rem;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--primary-color);
    margin-bottom: 1rem;
}

h1 { font-size: 2.5rem; }
h2 { font-size: 2rem; }
h3 { font-size: 1.5rem; }

code {
    background-color: var(--code-background);
    padding: 0.25rem 0.5rem;
    border-radius: 0.25rem;
    font-family: 'Monaco', 'Menlo', 'Ubuntu Mono', monospace;
    font-size: 0.9em;
}

pre code {
    display: block;
    padding: 1rem;
    overflow-x: auto;
}

.sidebar {
    position: fixed;
    left: 2rem;
    top: 2rem;
    width: 250px;
    height: calc(100vh - 4rem);
    overflow-y: auto;
    background-color: #f9fafb;
    padding: 1rem;
    border-radius: 0.5rem;
}

.main-content {
    margin-left: 280px;
    min-height: calc(100vh - 4rem);
}

.search-box {
    margin-bottom: 1rem;
}

.search-input {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border-color);
    border-radius: 0.25rem;
}

.nav-list {
    list-style: none;
}

.nav-list li {
    margin-bottom: 0.5rem;
}

.nav-list a {
    color: var(--text-color);
    text-decoration: none;
    display: block;
    padding: 0.25rem;
    border-radius: 0.25rem;
}

.nav-list a:hover {
    background-color: var(--primary-color);
    color: white;
}

.footer {
    margin-top: 4rem;
    padding-top: 2rem;
    border-top: 1px solid var(--border-color);
    text-align: center;
    color: var(--secondary-color);
}
"#;

        let css_dir = self.config.output_dir.join("html");
        fs::create_dir_all(&css_dir)
            .map_err(|e| DocsError::Io(e))?;

        let css_path = css_dir.join("style.css");
        fs::write(&css_path, css_content)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// デフォルトJavaScriptを生成
    async fn generate_default_js(&self) -> Result<()> {
        let js_content = r#"
// Search functionality
class DocSearch {
    constructor() {
        this.index = null;
        this.searchInput = document.querySelector('.search-input');
        this.resultsContainer = document.querySelector('.search-results');

        this.init();
    }

    async init() {
        if (this.searchInput) {
            this.searchInput.addEventListener('input', (e) => this.search(e.target.value));
            await this.loadIndex();
        }
    }

    async loadIndex() {
        try {
            const response = await fetch('search-index.json');
            this.index = await response.json();
        } catch (error) {
            console.warn('Failed to load search index:', error);
        }
    }

    search(query) {
        if (!this.index || !query.trim()) {
            this.displayResults([]);
            return;
        }

        const results = this.index
            .filter(item =>
                item.title.toLowerCase().includes(query.toLowerCase()) ||
                item.content.toLowerCase().includes(query.toLowerCase())
            )
            .slice(0, 10);

        this.displayResults(results);
    }

    displayResults(results) {
        if (!this.resultsContainer) return;

        if (results.length === 0) {
            this.resultsContainer.innerHTML = '<p>No results found</p>';
            return;
        }

        const html = results.map(result => `
            <div class="search-result">
                <a href="${result.url}">
                    <h4>${result.title}</h4>
                    <p>${result.content.substring(0, 100)}...</p>
                </a>
            </div>
        `).join('');

        this.resultsContainer.innerHTML = html;
    }
}

// Initialize search when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    new DocSearch();
});

// Syntax highlighting (basic implementation)
document.addEventListener('DOMContentLoaded', () => {
    const codeBlocks = document.querySelectorAll('pre code');

    codeBlocks.forEach(block => {
        // Basic syntax highlighting for Rust
        if (block.classList.contains('language-rust') || block.textContent.includes('fn ')) {
            block.innerHTML = block.innerHTML
                .replace(/\bfn\b/g, '<span style="color: #2563eb;">fn</span>')
                .replace(/\blet\b/g, '<span style="color: #2563eb;">let</span>')
                .replace(/\bstruct\b/g, '<span style="color: #2563eb;">struct</span>')
                .replace(/\bimpl\b/g, '<span style="color: #2563eb;">impl</span>')
                .replace(/\bpub\b/g, '<span style="color: #dc2626;">pub</span>')
                .replace(/\buse\b/g, '<span style="color: #dc2626;">use</span>')
                .replace(/(&[a-zA-Z_][a-zA-Z0-9_]*)/g, '<span style="color: #059669;">$1</span>')
                .replace(/(\/\/.*$)/gm, '<span style="color: #6b7280;">$1</span>');
        }
    });
});
"#;

        let js_dir = self.config.output_dir.join("html");
        fs::create_dir_all(&js_dir)
            .map_err(|e| DocsError::Io(e))?;

        let js_path = js_dir.join("script.js");
        fs::write(&js_path, js_content)
            .map_err(|e| DocsError::Io(e))?;

        Ok(())
    }

    /// デフォルトテンプレートを読み込む
    fn load_default_templates(tera: &mut Tera) {
        // 基本的なHTMLテンプレートを定義
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
                {% for module_name, module_items in modules %}
                <li>
                    <a href="#\{\{ module_name \}\}">\{\{ module_name \}\}</a>
                    <ul>
                        {% for item in module_items %}
                        <li><a href="\{\{ item.slug \}\}.html">\{\{ item.name \}\}</a></li>
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
        </header>

        <main>
            {% for module_name, module_items in modules %}
            <section id="{{ module_name | lower }}">
                <h2>{{ module_name }}</h2>
                <div class="module-grid">
                    {% for item in module_items %}
                    <div class="item-card">
                        <h3><a href="{{ item.slug }}.html">{{ item.name }}</a></h3>
                        {% if item.signature %}
                        <code>{{ item.signature }}</code>
                        {% endif %}
                        <p>{{ item.content | truncate(100) }}</p>
                    </div>
                    {% endfor %}
                </div>
            </section>
            {% endfor %}
        </main>

        <footer class="footer">
            <p>Generated by Kotoba Docs on {{ generated_at }}</p>
        </footer>
    </div>

    <script src="script.js"></script>
</body>
</html>
"#;

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
                <li><a href="index.html">Home</a></li>
                {% for module_name, module_items in modules %}
                <li><a href="{{ module_name | lower }}.html">{{ module_name }}</a></li>
                {% endfor %}
            </ul>
        </nav>
    </div>

    <div class="main-content">
        <header>
            <nav class="breadcrumbs">
                {% for crumb in breadcrumbs %}
                <a href="{{ crumb.url }}">{{ crumb.name }}</a> >
                {% endfor %}
                <span>{{ item.name }}</span>
            </nav>
            <h1>{{ item.name }}</h1>
            {% if item.signature %}
            <pre><code>{{ item.signature }}</code></pre>
            {% endif %}
        </header>

        <main>
            <div class="content">
                {{ html_content | safe }}
            </div>

            {% if related_items %}
            <section class="related">
                <h2>Related Items</h2>
                <ul>
                    {% for related in related_items %}
                    <li><a href="{{ related.slug }}.html">{{ related.name }}</a></li>
                    {% endfor %}
                </ul>
            </section>
            {% endif %}
        </main>

        <footer class="footer">
            <p>Generated by Kotoba Docs on {{ generated_at }}</p>
        </footer>
    </div>

    <script src="script.js"></script>
</body>
</html>
"#;

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
                {% for module_name, _ in modules %}
                <li><a href="{{ module_name | lower }}.html">{{ module_name }}</a></li>
                {% endfor %}
            </ul>
        </nav>
    </div>

    <div class="main-content">
        <header>
            <h1>{{ doc_type }}</h1>
        </header>

        <main>
            <div class="items-grid">
                {% for item in items %}
                <div class="item-card">
                    <h3><a href="{{ item.slug }}.html">{{ item.name }}</a></h3>
                    {% if item.signature %}
                    <code>{{ item.signature }}</code>
                    {% endif %}
                    <p>{{ item.content | truncate(150) }}</p>
                </div>
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

        // テンプレートを追加
        tera.add_raw_template("index.html", index_template).unwrap();
        tera.add_raw_template("item.html", item_template).unwrap();
        tera.add_raw_template("module.html", module_template).unwrap();
    }

    /// ユーティリティ関数

    /// マークダウンをHTMLに変換
    fn markdown_to_html(&self, markdown: &str) -> String {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);

        let parser = Parser::new_ext(markdown, options);
        let mut html_output = String::new();

        // Convert events to HTML
        for event in parser {
            match event {
                pulldown_cmark::Event::Text(text) => {
                    html_output.push_str(&html_escape::encode_text(&text));
                }
                pulldown_cmark::Event::Code(code) => {
                    html_output.push_str(&format!("<code>{}</code>", html_escape::encode_text(&code)));
                }
                pulldown_cmark::Event::Html(html) => {
                    html_output.push_str(&html);
                }
                pulldown_cmark::Event::SoftBreak => {
                    html_output.push('\n');
                }
                pulldown_cmark::Event::HardBreak => {
                    html_output.push_str("<br>");
                }
                pulldown_cmark::Event::Start(tag) => {
                    match tag {
                        pulldown_cmark::Tag::Paragraph => html_output.push_str("<p>"),
                        pulldown_cmark::Tag::Heading(level, _, _) => html_output.push_str(&format!("<h{}>", level)),
                        pulldown_cmark::Tag::BlockQuote => html_output.push_str("<blockquote>"),
                        pulldown_cmark::Tag::CodeBlock(_) => html_output.push_str("<pre><code>"),
                        pulldown_cmark::Tag::List(_) => html_output.push_str("<ul>"),
                        pulldown_cmark::Tag::Item => html_output.push_str("<li>"),
                        pulldown_cmark::Tag::Emphasis => html_output.push_str("<em>"),
                        pulldown_cmark::Tag::Strong => html_output.push_str("<strong>"),
                        _ => {}
                    }
                }
                pulldown_cmark::Event::End(tag) => {
                    match tag {
                        pulldown_cmark::Tag::Paragraph => html_output.push_str("</p>"),
                        pulldown_cmark::Tag::Heading(level, _, _) => html_output.push_str(&format!("</h{}>", level)),
                        pulldown_cmark::Tag::BlockQuote => html_output.push_str("</blockquote>"),
                        pulldown_cmark::Tag::CodeBlock(_) => html_output.push_str("</code></pre>"),
                        pulldown_cmark::Tag::List(_) => html_output.push_str("</ul>"),
                        pulldown_cmark::Tag::Item => html_output.push_str("</li>"),
                        pulldown_cmark::Tag::Emphasis => html_output.push_str("</em>"),
                        pulldown_cmark::Tag::Strong => html_output.push_str("</strong>"),
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        html_output
    }

    /// 項目をタイプごとにグループ化
    fn group_items_by_type(&self) -> HashMap<String, Vec<DocItem>> {
        let mut groups = HashMap::new();

        for item in &self.items {
            let type_name = format!("{:?}", item.doc_type);
            groups.entry(type_name).or_insert_with(Vec::new).push(item.clone());
        }

        groups
    }

    /// 関連項目を取得
    fn get_related_items(&self, item: &DocItem) -> Vec<DocItem> {
        item.related_items.iter()
            .filter_map(|id| self.item_map.get(id))
            .cloned()
            .collect()
    }

    /// パンくずリストを生成
    fn generate_breadcrumbs(&self, item: &DocItem) -> Vec<BreadcrumbItem> {
        let mut breadcrumbs = vec![];

        // ルートを追加
        breadcrumbs.push(BreadcrumbItem {
            name: "Home".to_string(),
            url: "index.html".to_string(),
        });

        // タイプを追加
        let type_name = format!("{:?}", item.doc_type);
        breadcrumbs.push(BreadcrumbItem {
            name: type_name.clone(),
            url: format!("{}.html", type_name.to_lowercase()),
        });

        breadcrumbs
    }

    /// 統計情報を生成
    fn generate_stats(&self) -> DocStats {
        let mut stats = DocStats::default();

        for item in &self.items {
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

        stats.total = self.items.len();
        stats
    }
}

/// パンくずリスト項目
#[derive(serde::Serialize)]
struct BreadcrumbItem {
    name: String,
    url: String,
}

/// ドキュメント統計
#[derive(serde::Serialize, Default)]
struct DocStats {
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

/// 出力形式の名前を取得
fn format_name(format: &OutputFormat) -> &'static str {
    match format {
        OutputFormat::Html => "HTML",
        OutputFormat::Markdown => "Markdown",
        OutputFormat::Json => "JSON",
        OutputFormat::Pdf => "PDF",
    }
}

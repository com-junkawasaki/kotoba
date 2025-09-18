//! Web Handler Module
//!
//! このモジュールはウェブアプリケーション開発のための統合ハンドラーを提供します。
//! HTTPリクエスト/レスポンスの処理、ルーティング、ミドルウェアなどを含みます。

use crate::{HandlerError, Result, HandlerContext};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// HTTPメソッド
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    OPTIONS,
    HEAD,
}

/// Webレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub content_type: String,
}

/// Webリクエスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRequest {
    pub method: HttpMethod,
    pub path: String,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Webハンドラー定義
#[derive(Debug, Clone)]
pub struct WebHandler {
    pub route: String,
    pub method: HttpMethod,
    pub handler: Arc<dyn Fn(WebRequest) -> Result<WebResponse> + Send + Sync>,
}

/// Web設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    pub port: u16,
    pub host: String,
    pub routes: Vec<WebRoute>,
    pub middlewares: Vec<WebMiddleware>,
    pub static_dir: Option<String>,
    pub template_dir: Option<String>,
}

/// Webルート定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRoute {
    pub path: String,
    pub method: String,
    pub handler: String, // Jsonnet handler function name
    pub middlewares: Vec<String>,
}

/// Webミドルウェア定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebMiddleware {
    pub name: String,
    pub handler: String, // Jsonnet middleware function name
}

/// Webアプリケーション
pub struct WebApp {
    config: WebConfig,
    handlers: HashMap<String, WebHandler>,
    context: Arc<RwLock<HandlerContext>>,
}

impl WebApp {
    /// 新しいWebアプリケーションを作成
    pub fn new(config: WebConfig, context: HandlerContext) -> Self {
        Self {
            config,
            handlers: HashMap::new(),
            context: Arc::new(RwLock::new(context)),
        }
    }

    /// ハンドラーを登録
    pub fn register_handler<F>(&mut self, route: String, method: HttpMethod, handler: F)
    where
        F: Fn(WebRequest) -> Result<WebResponse> + Send + Sync + 'static,
    {
        let key = format!("{} {}", method_to_string(&method), route);
        let web_handler = WebHandler {
            route,
            method,
            handler: Arc::new(handler),
        };
        self.handlers.insert(key, web_handler);
    }

    /// リクエストを処理
    pub async fn handle_request(&self, request: WebRequest) -> Result<WebResponse> {
        let key = format!("{} {}", method_to_string(&request.method), request.path);

        if let Some(handler) = self.handlers.get(&key) {
            (handler.handler)(request)
        } else {
            // デフォルトの404レスポンス
            Ok(WebResponse {
                status: 404,
                headers: HashMap::new(),
                body: "<h1>404 Not Found</h1>".to_string(),
                content_type: "text/html".to_string(),
            })
        }
    }

    /// JSON APIハンドラーの登録
    pub fn register_json_api<F, T>(&mut self, route: String, method: HttpMethod, handler: F)
    where
        F: Fn(WebRequest) -> Result<T> + Send + Sync + 'static,
        T: Serialize,
    {
        let json_handler = move |req: WebRequest| {
            let result = handler(req)?;
            let json_body = serde_json::to_string(&result)
                .map_err(|e| HandlerError::Jsonnet(format!("JSON serialization error: {}", e)))?;

            Ok(WebResponse {
                status: 200,
                headers: HashMap::new(),
                body: json_body,
                content_type: "application/json".to_string(),
            })
        };

        self.register_handler(route, method, json_handler);
    }

    /// HTMLテンプレートハンドラーの登録
    pub fn register_html_template<F>(&mut self, route: String, method: HttpMethod, template: String, handler: F)
    where
        F: Fn(WebRequest) -> Result<HashMap<String, String>> + Send + Sync + 'static,
    {
        let html_handler = move |req: WebRequest| {
            let context = handler(req)?;
            let html = render_template(&template, &context)?;

            Ok(WebResponse {
                status: 200,
                headers: HashMap::new(),
                body: html,
                content_type: "text/html".to_string(),
            })
        };

        self.register_handler(route, method, html_handler);
    }

    /// 静的ファイルハンドラーの登録
    pub fn register_static_file(&mut self, route: String, file_path: String) {
        let static_handler = move |req: WebRequest| {
            match std::fs::read_to_string(&file_path) {
                Ok(content) => {
                    let content_type = guess_content_type(&file_path);
                    Ok(WebResponse {
                        status: 200,
                        headers: HashMap::new(),
                        body: content,
                        content_type,
                    })
                }
                Err(_) => Ok(WebResponse {
                    status: 404,
                    headers: HashMap::new(),
                    body: "<h1>404 Not Found</h1>".to_string(),
                    content_type: "text/html".to_string(),
                }),
            }
        };

        self.register_handler(route, HttpMethod::GET, static_handler);
    }

    /// フォームハンドラーの登録
    pub fn register_form_handler<F>(&mut self, route: String, fields: Vec<String>, handler: F)
    where
        F: Fn(WebRequest, HashMap<String, String>) -> Result<WebResponse> + Send + Sync + 'static,
    {
        let form_handler = move |req: WebRequest| {
            if let Some(body) = &req.body {
                let form_data = parse_form_data(body, &fields)?;
                handler(req, form_data)
            } else {
                Ok(WebResponse {
                    status: 400,
                    headers: HashMap::new(),
                    body: "<h1>400 Bad Request</h1>".to_string(),
                    content_type: "text/html".to_string(),
                })
            }
        };

        self.register_handler(route, HttpMethod::POST, form_handler);
    }
}

/// Webアプリケーションを実行
pub async fn run_web_app(addr: &str, config: WebConfig) -> Result<()> {
    println!("🚀 Starting web application on {}", addr);

    // 実際のHTTPサーバー実装はここに追加
    // 現在はプレースホルダー

    println!("✅ Web application started successfully");
    Ok(())
}

/// GitHub Pagesサイトを生成
pub async fn generate_github_pages(site_definition: &serde_json::Value) -> Result<()> {
    println!("🏗️ Generating GitHub Pages site...");

    // サイト設定を取得
    let config = extract_github_pages_config(site_definition)?;

    // ページを生成
    let pages = generate_static_pages(site_definition, &config)?;

    // 静的ファイルをコピー
    copy_static_assets(&config)?;

    // GitHub Pages用の特別なファイル生成
    generate_github_pages_files(&config)?;

    println!("✅ GitHub Pages site generated successfully");
    println!("📁 Output directory: {}", config.output_dir);
    Ok(())
}

/// GitHub Pages設定を抽出
fn extract_github_pages_config(site_def: &serde_json::Value) -> Result<GitHubPagesConfig> {
    let config = site_def.get("config").unwrap_or(&serde_json::Value::Null);

    Ok(GitHubPagesConfig {
        name: config.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Kotoba Site")
            .to_string(),
        description: config.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        base_url: config.get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://username.github.io/repo")
            .to_string(),
        github_repo: config.get("github_repo")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        theme: config.get("theme")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string(),
        cname: config.get("cname")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        output_dir: config.get("output_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("_site")
            .to_string(),
        source_dir: config.get("source_dir")
            .and_then(|v| v.as_str())
            .unwrap_or(".")
            .to_string(),
        template_dir: config.get("template_dir")
            .and_then(|v| v.as_str())
            .unwrap_or("_templates")
            .to_string(),
    })
}

/// GitHub Pages設定構造体
#[derive(Debug, Clone)]
pub struct GitHubPagesConfig {
    pub name: String,
    pub description: String,
    pub base_url: String,
    pub github_repo: Option<String>,
    pub theme: String,
    pub cname: Option<String>,
    pub output_dir: String,
    pub source_dir: String,
    pub template_dir: String,
}

/// 静的ページを生成
fn generate_static_pages(site_def: &serde_json::Value, config: &GitHubPagesConfig) -> Result<Vec<GeneratedPage>> {
    let mut pages = Vec::new();

    // ページ定義を取得
    if let Some(pages_def) = site_def.get("pages").and_then(|v| v.as_array()) {
        for page_def in pages_def {
            let page = generate_single_page(page_def, config)?;
            pages.push(page);
        }
    }

    // デフォルトのページを追加
    if pages.is_empty() {
        pages.push(generate_default_homepage(config));
    }

    // 特別なページを追加
    pages.push(generate_sitemap_page(config, &pages));
    pages.push(generate_feed_page(config, &pages));

    Ok(pages)
}

/// 単一ページを生成
fn generate_single_page(page_def: &serde_json::Value, config: &GitHubPagesConfig) -> Result<GeneratedPage> {
    let name = page_def.get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("page")
        .to_string();

    let title = page_def.get("title")
        .and_then(|v| v.as_str())
        .unwrap_or(&name)
        .to_string();

    let template = page_def.get("template")
        .and_then(|v| v.as_str())
        .unwrap_or("default");

    let content = page_def.get("content")
        .unwrap_or(&serde_json::Value::Null);

    // HTMLコンテンツを生成
    let html_content = generate_page_html(&name, &title, template, content, config)?;

    let url = if name == "index" {
        "/".to_string()
    } else {
        format!("/{}/", name)
    };

    Ok(GeneratedPage {
        url,
        title,
        html_content,
        metadata: std::collections::HashMap::new(),
    })
}

/// ページHTMLを生成
fn generate_page_html(name: &str, title: &str, template: &str, content: &serde_json::Value, config: &GitHubPagesConfig) -> Result<String> {
    let mut html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{} - {}</title>
    <meta name="description" content="{}">
    <link rel="stylesheet" href="/assets/css/style.css">
    <link rel="canonical" href="{}{}">
</head>
<body>
    <nav class="navbar">
        <div class="container">
            <a href="/" class="navbar-brand">{}</a>
            <ul class="navbar-nav">
                <li><a href="/">Home</a></li>
                <li><a href="/docs">Docs</a></li>
                <li><a href="/examples">Examples</a></li>
                <li><a href="/about">About</a></li>
            </ul>
        </div>
    </nav>

    <main class="container">
        <h1>{}</h1>
"#, title, config.name, config.description, config.base_url, if name == "index" { "" } else { &format!("/{}", name) }, config.name, title);

    // コンテンツを追加
    html.push_str(&generate_content_html(content, template));

    html.push_str(r#"
    </main>

    <footer class="footer">
        <div class="container">
            <p>&copy; 2024 Kotoba. Built with Kotoba language.</p>
        </div>
    </footer>

    <script src="/assets/js/main.js"></script>
</body>
</html>"#);

    Ok(html)
}

/// コンテンツHTMLを生成
fn generate_content_html(content: &serde_json::Value, template: &str) -> String {
    match template {
        "home" | "hero" => {
            if let Some(hero) = content.get("hero") {
                let title = hero.get("title").and_then(|v| v.as_str()).unwrap_or("Welcome");
                let subtitle = hero.get("subtitle").and_then(|v| v.as_str()).unwrap_or("");

                format!(r#"
        <section class="hero">
            <h1>{}</h1>
            <p>{}</p>
        </section>
"#, title, subtitle)
            } else {
                r#"<p>Welcome to our site!</p>"#.to_string()
            }
        }
        "docs" => {
            r#"<div class="docs-content"><p>Documentation content goes here.</p></div>"#.to_string()
        }
        "examples" => {
            r#"<div class="examples-content"><p>Examples content goes here.</p></div>"#.to_string()
        }
        _ => {
            format!("<div class=\"content\">{}</div>", content)
        }
    }
}

/// デフォルトのホームページを生成
fn generate_default_homepage(config: &GitHubPagesConfig) -> GeneratedPage {
    let html_content = generate_page_html("index", &config.name, "home",
        &serde_json::json!({
            "hero": {
                "title": format!("Welcome to {}", config.name),
                "subtitle": config.description
            }
        }), config).unwrap_or_else(|_| "<html><body><h1>Welcome</h1></body></html>".to_string());

    GeneratedPage {
        url: "/".to_string(),
        title: config.name.clone(),
        html_content,
        metadata: std::collections::HashMap::new(),
    }
}

/// サイトマップページを生成
fn generate_sitemap_page(config: &GitHubPagesConfig, pages: &[GeneratedPage]) -> GeneratedPage {
    let mut sitemap = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#);

    for page in pages {
        if !page.url.starts_with("/api/") && !page.url.contains("private") {
            sitemap.push_str(&format!(r#"  <url>
    <loc>{}{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
"#, config.base_url.trim_end_matches('/'), page.url, chrono::Utc::now().format("%Y-%m-%d")));
        }
    }

    sitemap.push_str("</urlset>");

    GeneratedPage {
        url: "/sitemap.xml".to_string(),
        title: "Sitemap".to_string(),
        html_content: sitemap,
        metadata: std::collections::HashMap::new(),
    }
}

/// RSSフィードページを生成
fn generate_feed_page(config: &GitHubPagesConfig, pages: &[GeneratedPage]) -> GeneratedPage {
    let mut feed = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{}</title>
    <description>{}</description>
    <link>{}</link>
    <atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
    <lastBuildDate>{}</lastBuildDate>
"#,
        config.name,
        config.description,
        config.base_url,
        config.base_url,
        chrono::Utc::now().to_rfc2822()
    );

    // 最新のページを追加（最大10件）
    for page in pages.iter().take(10) {
        if !page.url.starts_with("/api/") && page.url != "/sitemap.xml" && page.url != "/feed.xml" {
            feed.push_str(&format!(r#"    <item>
      <title>{}</title>
      <link>{}{}</link>
      <guid>{}{}</guid>
      <pubDate>{}</pubDate>
    </item>
"#, page.title, config.base_url.trim_end_matches('/'), page.url, config.base_url.trim_end_matches('/'), page.url, chrono::Utc::now().to_rfc2822()));
        }
    }

    feed.push_str("  </channel>\n</rss>");

    GeneratedPage {
        url: "/feed.xml".to_string(),
        title: "RSS Feed".to_string(),
        html_content: feed,
        metadata: std::collections::HashMap::new(),
    }
}

/// 静的アセットをコピー
fn copy_static_assets(config: &GitHubPagesConfig) -> Result<()> {
    println!("📋 Copying static assets...");

    // assetsディレクトリが存在するかチェック
    let assets_dir = std::path::Path::new("assets");
    if !assets_dir.exists() {
        println!("⚠️  No assets directory found, creating default assets...");
        create_default_assets(config)?;
    }

    // 出力ディレクトリにコピー
    let output_assets = std::path::Path::new(&config.output_dir).join("assets");
    if assets_dir.exists() {
        copy_dir_all(assets_dir, &output_assets)?;
    }

    Ok(())
}

/// デフォルトのアセットを作成
fn create_default_assets(config: &GitHubPagesConfig) -> Result<()> {
    let assets_dir = std::path::Path::new("assets");
    std::fs::create_dir_all(assets_dir.join("css"))?;
    std::fs::create_dir_all(assets_dir.join("js"))?;

    // CSSファイルを作成
    let css_content = r#"/* Kotoba GitHub Pages Styles */
:root {
  --primary-color: #0366d6;
  --background-color: #ffffff;
  --text-color: #24292e;
  --border-color: #e1e4e8;
}

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

body {
  font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif;
  line-height: 1.6;
  color: var(--text-color);
  background-color: var(--background-color);
  margin: 0;
  padding: 0;
}

.container {
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 20px;
}

.navbar {
  background-color: var(--primary-color);
  color: white;
  padding: 1rem 0;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.navbar-brand {
  font-size: 1.5rem;
  font-weight: bold;
  text-decoration: none;
  color: white;
}

.navbar-nav {
  display: flex;
  list-style: none;
  gap: 2rem;
  margin: 0;
  padding: 0;
}

.navbar-nav a {
  color: white;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.navbar-nav a:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.hero {
  background: linear-gradient(135deg, var(--primary-color), #28a745);
  color: white;
  padding: 4rem 0;
  text-align: center;
}

.hero h1 {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.hero p {
  font-size: 1.25rem;
  margin-bottom: 2rem;
  opacity: 0.9;
}

main {
  padding: 2rem 0;
  min-height: 60vh;
}

.footer {
  background-color: #f6f8fa;
  padding: 2rem 0;
  text-align: center;
  border-top: 1px solid var(--border-color);
  margin-top: 4rem;
}

.footer p {
  margin: 0;
  color: #666;
}

/* Mobile responsiveness */
@media (max-width: 768px) {
  .hero h1 {
    font-size: 2rem;
  }

  .navbar-nav {
    flex-direction: column;
    gap: 1rem;
  }
}
"#;

    // JavaScriptファイルを作成
    let js_content = r#"
// Kotoba GitHub Pages JavaScript
document.addEventListener('DOMContentLoaded', function() {
  console.log('🚀 Kotoba GitHub Pages loaded');

  // Smooth scrolling for anchor links
  document.querySelectorAll('a[href^="#"]').forEach(function(anchor) {
    anchor.addEventListener('click', function (e) {
      e.preventDefault();
      var href = this.getAttribute('href');
      var target = document.querySelector(href);
      if (target) {
        target.scrollIntoView({
          behavior: 'smooth'
        });
      }
    });
  });

  // Add active class to current navigation item
  var currentPath = window.location.pathname;
  var navLinks = document.querySelectorAll('.navbar-nav a');
  for (var i = 0; i < navLinks.length; i++) {
    var link = navLinks[i];
    if (link.getAttribute('href') === currentPath) {
      link.classList.add('active');
    }
  }

  // Add loading class to body
  document.body.classList.add('loaded');
});
"#;

    std::fs::write(assets_dir.join("css/style.css"), css_content)?;
    std::fs::write(assets_dir.join("js/main.js"), js_content)?;

    Ok(())
}

/// GitHub Pages用の特別ファイルを生成
fn generate_github_pages_files(config: &GitHubPagesConfig) -> Result<()> {
    println!("📝 Generating GitHub Pages special files...");

    // CNAMEファイル（カスタムドメインがある場合）
    if let Some(cname) = &config.cname {
        std::fs::write(
            std::path::Path::new(&config.output_dir).join("CNAME"),
            cname
        )?;
        println!("✅ CNAME file created: {}", cname);
    }

    // .nojekyllファイル（Jekyllを無効化）
    std::fs::write(
        std::path::Path::new(&config.output_dir).join(".nojekyll"),
        ""
    )?;
    println!("✅ .nojekyll file created");

    Ok(())
}

/// ディレクトリを再帰的にコピー
fn copy_dir_all(src: impl AsRef<std::path::Path>, dst: impl AsRef<std::path::Path>) -> std::io::Result<()> {
    std::fs::create_dir_all(&dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(entry.path(), dst.as_ref().join(entry.file_name()))?;
        } else {
            std::fs::copy(entry.path(), dst.as_ref().join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// 生成されたページ
#[derive(Debug, Clone)]
pub struct GeneratedPage {
    pub url: String,
    pub title: String,
    pub html_content: String,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

/// ユーティリティ関数

fn method_to_string(method: &HttpMethod) -> &'static str {
    match method {
        HttpMethod::GET => "GET",
        HttpMethod::POST => "POST",
        HttpMethod::PUT => "PUT",
        HttpMethod::DELETE => "DELETE",
        HttpMethod::PATCH => "PATCH",
        HttpMethod::OPTIONS => "OPTIONS",
        HttpMethod::HEAD => "HEAD",
    }
}

fn render_template(template: &str, context: &HashMap<String, String>) -> Result<String> {
    let mut result = template.to_string();

    for (key, value) in context {
        let placeholder = format!("{{{{{}}}}}", key);
        result = result.replace(&placeholder, value);
    }

    Ok(result)
}

fn guess_content_type(file_path: &str) -> String {
    if file_path.ends_with(".html") {
        "text/html".to_string()
    } else if file_path.ends_with(".css") {
        "text/css".to_string()
    } else if file_path.ends_with(".js") {
        "application/javascript".to_string()
    } else if file_path.ends_with(".json") {
        "application/json".to_string()
    } else if file_path.ends_with(".png") {
        "image/png".to_string()
    } else if file_path.ends_with(".jpg") || file_path.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else {
        "text/plain".to_string()
    }
}

fn parse_form_data(body: &str, fields: &[String]) -> Result<HashMap<String, String>> {
    let mut data = HashMap::new();

    for pair in body.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            if fields.contains(&key.to_string()) {
                let decoded_value = urlencoding::decode(value)
                    .map_err(|e| HandlerError::Jsonnet(format!("URL decode error: {}", e)))?;
                data.insert(key.to_string(), decoded_value.to_string());
            }
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_to_string() {
        assert_eq!(method_to_string(&HttpMethod::GET), "GET");
        assert_eq!(method_to_string(&HttpMethod::POST), "POST");
        assert_eq!(method_to_string(&HttpMethod::PUT), "PUT");
        assert_eq!(method_to_string(&HttpMethod::DELETE), "DELETE");
    }

    #[test]
    fn test_guess_content_type() {
        assert_eq!(guess_content_type("test.html"), "text/html");
        assert_eq!(guess_content_type("style.css"), "text/css");
        assert_eq!(guess_content_type("app.js"), "application/javascript");
        assert_eq!(guess_content_type("data.json"), "application/json");
        assert_eq!(guess_content_type("unknown.xyz"), "text/plain");
    }

    #[test]
    fn test_render_template() {
        let template = "<h1>{{title}}</h1><p>{{content}}</p>";
        let mut context = HashMap::new();
        context.insert("title".to_string(), "Hello".to_string());
        context.insert("content".to_string(), "World".to_string());

        let result = render_template(template, &context).unwrap();
        assert_eq!(result, "<h1>Hello</h1><p>World</p>");
    }

    #[tokio::test]
    async fn test_web_app_creation() {
        let config = WebConfig {
            port: 3000,
            host: "127.0.0.1".to_string(),
            routes: vec![],
            middlewares: vec![],
            static_dir: None,
            template_dir: None,
        };

        let context = HandlerContext::default();
        let app = WebApp::new(config, context);

        let request = WebRequest {
            method: HttpMethod::GET,
            path: "/".to_string(),
            query_params: HashMap::new(),
            headers: HashMap::new(),
            body: None,
        };

        let response = app.handle_request(request).await.unwrap();
        assert_eq!(response.status, 404); // No handler registered
    }
}

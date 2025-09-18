//! # GitHub Pages Generator for Kotoba
//!
//! This module provides GitHub Pages generation capabilities implemented entirely
//! in the Kotoba language. It allows users to define entire websites using
//! Jsonnet syntax and automatically generate static sites.
//!
//! ## Features
//!
//! - **Declarative Site Definition**: Define entire sites using Jsonnet objects
//! - **Dynamic Page Generation**: Generate pages from structured data
//! - **Template System**: Built-in templating with Jsonnet functions
//! - **Asset Management**: Automatic asset processing and optimization
//! - **GitHub Pages Deployment**: Direct deployment to GitHub Pages

use crate::{Result, KotobaNetError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// GitHub Pages site configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubPagesConfig {
    /// Site metadata
    pub name: String,
    pub description: String,
    pub base_url: String,

    /// GitHub repository information
    pub github_repo: Option<String>,

    /// Theme configuration
    pub theme: String,

    /// Custom domain (optional)
    pub cname: Option<String>,

    /// Build configuration
    pub build: BuildConfig,

    /// Deployment configuration
    pub deployment: DeploymentConfig,
}

/// Build configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildConfig {
    /// Source directory
    pub source_dir: String,
    /// Output directory
    pub output_dir: String,
    /// Template directory
    pub template_dir: String,

    /// Markdown processing options
    pub markdown: MarkdownConfig,

    /// Optimization settings
    pub optimization: OptimizationConfig,
}

/// Markdown processing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkdownConfig {
    /// Enabled extensions
    pub extensions: Vec<String>,
    /// Syntax highlighting languages
    pub highlight_languages: Vec<String>,
}

/// Optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationConfig {
    /// Minify HTML output
    pub minify_html: bool,
    /// Compress assets
    pub compress_assets: bool,
    /// Generate sitemap
    pub generate_sitemap: bool,
    /// Generate RSS feed
    pub generate_feed: bool,
}

/// Deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentConfig {
    /// Deployment provider
    pub provider: String,
    /// GitHub Pages branch
    pub branch: String,
    /// Build command
    pub build_command: String,
    /// Output directory for deployment
    pub output_dir: String,
}

/// Page definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageDefinition {
    /// Page name/identifier
    pub name: String,
    /// Page title
    pub title: String,
    /// Page description
    pub description: Option<String>,
    /// Page template
    pub template: String,
    /// Page layout
    pub layout: Option<String>,
    /// Page content data
    pub content: serde_json::Value,
    /// Page metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Navigation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavItem {
    /// Navigation label
    pub label: String,
    /// Navigation URL
    pub href: String,
    /// Child navigation items
    pub children: Option<Vec<NavItem>>,
}

/// Site navigation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteNavigation {
    /// Main navigation
    pub main: Vec<NavItem>,
    /// Footer navigation
    pub footer: Vec<NavItem>,
}

/// Generated site
#[derive(Debug, Clone)]
pub struct GeneratedSite {
    /// Site configuration
    pub config: GitHubPagesConfig,
    /// Generated pages
    pub pages: Vec<GeneratedPage>,
    /// Generated assets
    pub assets: Vec<String>,
    /// Site navigation
    pub navigation: SiteNavigation,
}

/// Generated page
#[derive(Debug, Clone)]
pub struct GeneratedPage {
    /// Page URL path
    pub url: String,
    /// Page title
    pub title: String,
    /// Generated HTML content
    pub html_content: String,
    /// Page metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Built-in handler types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HandlerType {
    /// Template-based HTML response
    Template,
    /// JSON response
    Json,
    /// Static file response
    Static,
    /// Form handler
    Form,
    /// Redirect response
    Redirect,
    /// Custom handler (for advanced use cases)
    Custom,
}

/// Built-in HTTP handler definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInHandler {
    /// Handler type
    pub handler_type: HandlerType,
    /// Response content type
    pub content_type: Option<String>,
    /// Response status code
    pub status: Option<u16>,
    /// Template content or path
    pub template: Option<String>,
    /// JSON data
    pub data: Option<serde_json::Value>,
    /// Static file path
    pub file_path: Option<String>,
    /// Form fields (for form handlers)
    pub fields: Option<Vec<String>>,
    /// Redirect URL
    pub redirect_url: Option<String>,
    /// Custom handler function name
    pub custom_handler: Option<String>,
    /// Response headers
    pub headers: Option<HashMap<String, String>>,
}

/// Handler registry for built-in handlers
#[derive(Debug, Clone)]
pub struct HandlerRegistry {
    handlers: HashMap<String, BuiltInHandler>,
}

impl HandlerRegistry {
    /// Create a new handler registry
    pub fn new() -> Self {
        Self {
            handlers: HashMap::new(),
        }
    }

    /// Register a built-in handler
    pub fn register(&mut self, route: String, handler: BuiltInHandler) {
        self.handlers.insert(route, handler);
    }

    /// Get a handler for a route
    pub fn get_handler(&self, route: &str) -> Option<&BuiltInHandler> {
        self.handlers.get(route)
    }

    /// Check if a route has a handler
    pub fn has_handler(&self, route: &str) -> bool {
        self.handlers.contains_key(route)
    }

    /// Get all registered routes
    pub fn routes(&self) -> Vec<&String> {
        self.handlers.keys().collect()
    }
}

/// GitHub Pages generator
pub struct GitHubPagesGenerator {
    config: GitHubPagesConfig,
    handler_registry: HandlerRegistry,
}

impl GitHubPagesGenerator {
    /// Create a new GitHub Pages generator
    pub fn new(config: GitHubPagesConfig) -> Self {
        Self {
            config,
            handler_registry: HandlerRegistry::new(),
        }
    }

    /// Create generator from Jsonnet site definition
    pub fn from_jsonnet(jsonnet_content: &str) -> Result<Self> {
        let parsed = crate::evaluate_kotoba_to_json(jsonnet_content)?;
        let config: GitHubPagesConfig = serde_json::from_str(&parsed)
            .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "Failed to parse site config".to_string()
            }))?;

        Ok(Self::new(config))
    }

    /// Register built-in handlers from site definition
    pub fn register_handlers(&mut self, site_definition: &serde_json::Value) -> Result<()> {
        if let Some(handlers) = site_definition.get("handlers") {
            if let Some(handlers_obj) = handlers.as_object() {
                for (route, handler_def) in handlers_obj {
                    let handler: BuiltInHandler = serde_json::from_value(handler_def.clone())
                        .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                            message: "Failed to parse handler".to_string()
                        }))?;
                    self.handler_registry.register(route.clone(), handler);
                }
            }
        }
        Ok(())
    }

    /// Process a request using built-in handlers
    pub async fn process_request(&self, method: &str, path: &str) -> Result<Option<String>> {
        let route = format!("{} {}", method, path);

        if let Some(handler) = self.handler_registry.get_handler(&route) {
            match handler.handler_type {
                HandlerType::Template => {
                    if let Some(template) = &handler.template {
                        Ok(Some(template.clone()))
                    } else {
                        Ok(Some("<html><body>Template not found</body></html>".to_string()))
                    }
                }
                HandlerType::Json => {
                    if let Some(data) = &handler.data {
                        let json = serde_json::to_string_pretty(data)
                            .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                                message: "JSON serialization error".to_string()
                            }))?;
                        Ok(Some(json))
                    } else {
                        Ok(Some("{}".to_string()))
                    }
                }
                HandlerType::Static => {
                    if let Some(file_path) = &handler.file_path {
                        // For static files, return the file path for serving
                        Ok(Some(format!("static:{}", file_path)))
                    } else {
                        Ok(Some("File not found".to_string()))
                    }
                }
                HandlerType::Redirect => {
                    if let Some(url) = &handler.redirect_url {
                        Ok(Some(format!("redirect:{}", url)))
                    } else {
                        Ok(Some("Redirect URL not specified".to_string()))
                    }
                }
                HandlerType::Form => {
                    // Basic form response
                    let form_html = r#"
                    <form method="POST">
                        <input type="text" name="name" placeholder="Name" required>
                        <input type="email" name="email" placeholder="Email" required>
                        <textarea name="message" placeholder="Message" required></textarea>
                        <button type="submit">Submit</button>
                    </form>
                    "#.to_string();
                    Ok(Some(form_html))
                }
                HandlerType::Custom => {
                    // For custom handlers, return the handler function name
                    if let Some(custom_handler) = &handler.custom_handler {
                        Ok(Some(format!("custom:{}", custom_handler)))
                    } else {
                        Ok(Some("Custom handler not specified".to_string()))
                    }
                }
            }
        } else {
            Ok(None) // No handler found for this route
        }
    }

    /// Generate the complete site
    pub async fn generate_site(&mut self, site_definition: &serde_json::Value) -> Result<GeneratedSite> {
        println!("🚀 Generating GitHub Pages site: {}", self.config.name);

        // Register built-in handlers
        self.register_handlers(site_definition)?;

        // Parse site definition
        let pages = self.parse_pages(site_definition)?;
        let navigation = self.parse_navigation(site_definition)?;

        // Generate pages
        let mut generated_pages = Vec::new();
        for page_def in pages {
            let generated_page = self.generate_page(&page_def).await?;
            generated_pages.push(generated_page);
        }

        // Generate assets
        let assets = self.generate_assets().await?;

        // Generate special pages
        let sitemap_page = self.generate_sitemap(&generated_pages).await?;
        let feed_page = self.generate_feed(&generated_pages).await?;

        generated_pages.push(sitemap_page);
        generated_pages.push(feed_page);

        let site = GeneratedSite {
            config: self.config.clone(),
            pages: generated_pages,
            assets,
            navigation,
        };

        println!("✅ Site generated successfully with {} pages", site.pages.len());
        Ok(site)
    }

    /// Parse pages from site definition
    fn parse_pages(&self, site_def: &serde_json::Value) -> Result<Vec<PageDefinition>> {
        let pages = site_def.get("pages")
            .and_then(|v| v.as_array())
            .ok_or_else(|| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "No pages defined in site".to_string()
            }))?;

        let mut page_definitions = Vec::new();

        for page_value in pages {
            let page: PageDefinition = serde_json::from_value(page_value.clone())
                .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                    message: "Failed to parse page".to_string()
                }))?;
            page_definitions.push(page);
        }

        Ok(page_definitions)
    }

    /// Parse navigation from site definition
    fn parse_navigation(&self, site_def: &serde_json::Value) -> Result<SiteNavigation> {
        let navigation = site_def.get("navigation")
            .ok_or_else(|| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "No navigation defined in site".to_string()
            }))?;

        let nav: SiteNavigation = serde_json::from_value(navigation.clone())
            .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "Failed to parse navigation".to_string()
            }))?;

        Ok(nav)
    }

    /// Generate a single page
    async fn generate_page(&self, page_def: &PageDefinition) -> Result<GeneratedPage> {
        println!("📄 Generating page: {}", page_def.name);

        // Load template
        let template_content = self.load_template(&page_def.template).await?;

        // Create template context
        let context = self.create_template_context(page_def)?;

        // Render template with Jsonnet
        let rendered_html = self.render_template(&template_content, &context).await?;

        // Generate URL
        let url = self.generate_page_url(&page_def.name);

        Ok(GeneratedPage {
            url,
            title: page_def.title.clone(),
            html_content: rendered_html,
            metadata: page_def.metadata.clone(),
        })
    }

    /// Load template content
    async fn load_template(&self, template_name: &str) -> Result<String> {
        let template_path = Path::new(&self.config.build.template_dir)
            .join(format!("{}.html.jsonnet", template_name));

        if !template_path.exists() {
            // Return default template if custom template doesn't exist
            return Ok(self.get_default_template(template_name));
        }

        tokio::fs::read_to_string(&template_path)
            .await
            .map_err(|e| KotobaNetError::Io(e))
    }

    /// Create template context
    fn create_template_context(&self, page_def: &PageDefinition) -> Result<serde_json::Value> {
        let context = serde_json::json!({
            "site": {
                "name": self.config.name,
                "description": self.config.description,
                "base_url": self.config.base_url,
                "github_repo": self.config.github_repo,
            },
            "page": {
                "title": page_def.title,
                "description": page_def.description,
                "url": self.generate_page_url(&page_def.name),
                "content": page_def.content,
                "metadata": page_def.metadata,
            },
            "config": self.config,
        });

        Ok(context)
    }

    /// Render template with Jsonnet
    async fn render_template(&self, template: &str, context: &serde_json::Value) -> Result<String> {
        let context_json = serde_json::to_string(context)
            .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "Failed to serialize context".to_string()
            }))?;

        let jsonnet_code = format!(r#"
local context = {};
local site = context.site;
local page = context.page;
local config = context.config;

{}
"#, context_json, template);

        let result = crate::evaluate_kotoba_to_json(&jsonnet_code)?;
        let result_value: serde_json::Value = serde_json::from_str(&result)
            .map_err(|_e| KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "Failed to parse template result".to_string()
            }))?;

        // Extract HTML content
        if let Some(html) = result_value.get("html").and_then(|v| v.as_str()) {
            Ok(html.to_string())
        } else if result_value.is_string() {
            Ok(result_value.as_str().unwrap().to_string())
        } else {
            Err(KotobaNetError::Jsonnet(kotoba_jsonnet::JsonnetError::RuntimeError {
                message: "Template must return an object with 'html' field or a string".to_string()
            }))
        }
    }

    /// Generate page URL from page name
    fn generate_page_url(&self, page_name: &str) -> String {
        if page_name == "index" {
            "/".to_string()
        } else {
            format!("/{}/", page_name)
        }
    }

    /// Generate assets
    async fn generate_assets(&self) -> Result<Vec<String>> {
        println!("🎨 Generating assets...");

        // Generate CSS
        let css_content = self.generate_css().await?;
        let js_content = self.generate_javascript().await?;

        // Save assets
        let css_path = "assets/style.css";
        let js_path = "assets/main.js";

        tokio::fs::create_dir_all("assets").await
            .map_err(KotobaNetError::Io)?;

        tokio::fs::write(css_path, &css_content).await
            .map_err(KotobaNetError::Io)?;
        tokio::fs::write(js_path, &js_content).await
            .map_err(KotobaNetError::Io)?;

        Ok(vec![css_path.to_string(), js_path.to_string()])
    }

    /// Generate CSS
    async fn generate_css(&self) -> Result<String> {
        let css = r#"
/* Kotoba GitHub Pages Styles */
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
}

.nav-link {
  color: white;
  text-decoration: none;
  padding: 0.5rem 1rem;
  border-radius: 4px;
  transition: background-color 0.2s;
}

.nav-link:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.hero-section {
  background: linear-gradient(135deg, var(--primary-color), #28a745);
  color: white;
  padding: 4rem 0;
  text-align: center;
}

.hero-title {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.hero-subtitle {
  font-size: 1.25rem;
  margin-bottom: 2rem;
  opacity: 0.9;
}

.feature-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
  gap: 2rem;
  margin-top: 3rem;
}

.feature-card {
  background: white;
  color: var(--text-color);
  padding: 2rem;
  border-radius: 8px;
  box-shadow: 0 4px 6px rgba(0, 0, 0, 0.1);
  text-align: center;
}

.feature-icon {
  font-size: 3rem;
  margin-bottom: 1rem;
}

.section {
  padding: 4rem 0;
}

.section h2 {
  text-align: center;
  margin-bottom: 3rem;
  font-size: 2.5rem;
}

.footer {
  background-color: #f6f8fa;
  padding: 2rem 0;
  text-align: center;
  border-top: 1px solid var(--border-color);
}
"#.to_string();

        Ok(css)
    }

    /// Generate JavaScript
    async fn generate_javascript(&self) -> Result<String> {
        let js = "// Kotoba GitHub Pages JavaScript
console.log('🚀 Kotoba GitHub Pages loaded with basic functionality');".to_string();

        Ok(js)
    }

    /// Generate sitemap
    async fn generate_sitemap(&self, pages: &[GeneratedPage]) -> Result<GeneratedPage> {
        let mut sitemap = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#);

        for page in pages {
            if !page.url.starts_with("/api/") && !page.url.contains("private") {
                sitemap.push_str(&format!(
                    "  <url>\n    <loc>{}{}</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>weekly</changefreq>\n    <priority>0.8</priority>\n  </url>\n",
                    self.config.base_url.trim_end_matches('/'),
                    page.url.trim_end_matches('/'),
                    chrono::Utc::now().format("%Y-%m-%d")
                ));
            }
        }

        sitemap.push_str("</urlset>");

        Ok(GeneratedPage {
            url: "/sitemap.xml".to_string(),
            title: "Sitemap".to_string(),
            html_content: sitemap,
            metadata: HashMap::new(),
        })
    }

    /// Generate RSS feed
    async fn generate_feed(&self, pages: &[GeneratedPage]) -> Result<GeneratedPage> {
        let mut feed = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{}</title>
    <description>{}</description>
    <link>{}</link>
    <atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
    <lastBuildDate>{}</lastBuildDate>
"#,
            self.config.name,
            self.config.description,
            self.config.base_url,
            self.config.base_url,
            chrono::Utc::now().to_rfc2822()
        );

        // Add recent pages (limit to 10)
        for page in pages.iter().take(10) {
            if !page.url.starts_with("/api/") && page.url != "/sitemap.xml" && page.url != "/feed.xml" {
                feed.push_str(&format!(
                    r#"    <item>
      <title>{}</title>
      <link>{}{}</link>
      <guid>{}{}</guid>
      <pubDate>{}</pubDate>
    </item>
"#,
                    page.title,
                    self.config.base_url.trim_end_matches('/'),
                    page.url,
                    self.config.base_url.trim_end_matches('/'),
                    page.url,
                    chrono::Utc::now().to_rfc2822()
                ));
            }
        }

        feed.push_str("  </channel>\n</rss>");

        Ok(GeneratedPage {
            url: "/feed.xml".to_string(),
            title: "RSS Feed".to_string(),
            html_content: feed,
            metadata: HashMap::new(),
        })
    }

    /// Get default template for a page type
    fn get_default_template(&self, template_name: &str) -> String {
        match template_name {
            "home" => r#"
{
  html: std.join("\n", [
    "<div class='hero-section'>",
    "  <div class='container'>",
    "    <h1 class='hero-title'>" + page.title + "</h1>",
    "    <p class='hero-subtitle'>" + (page.description // "Welcome to our site") + "</p>",
    "    <div class='feature-list'>",
    std.join("\n", [
      item.label + ": " + item.description
      for item in (page.content.features // [])
    ]),
    "    </div>",
    "  </div>",
    "</div>",
    std.join("\n", [
      "<section class='section'>",
      "  <div class='container'>",
      "    <h2>" + section.title + "</h2>",
      "    <div class='feature-grid'>",
      std.join("\n", [
        "<div class='feature-card'>",
        "  <div class='feature-icon'>" + item.icon + "</div>",
        "  <h3>" + item.title + "</h3>",
        "  <p>" + item.description + "</p>",
        "</div>"
        for item in (section.items // [])
      ]),
      "    </div>",
      "  </div>",
      "</section>"
      for section in (page.content.sections // [])
    ])
  ])
}
"#.to_string(),

            "docs" => r#"
{
  html: std.join("\n", [
    "<div class='container'>",
    "  <h1>" + page.title + "</h1>",
    "  <p>" + (page.description // "") + "</p>",
    "  <div class='docs-content'>",
    "    <!-- Documentation content goes here -->",
    "    <p>This is a documentation page generated by Kotoba.</p>",
    "  </div>",
    "</div>"
  ])
}
"#.to_string(),

            "page" => r#"
{
  html: std.join("\n", [
    "<div class='container'>",
    "  <h1>" + page.title + "</h1>",
    "  <div class='page-content'>",
    "    " + (page.content.body // "Page content goes here"),
    "  </div>",
    "</div>"
  ])
}
"#.to_string(),

            _ => r#"
{
  html: std.join("\n", [
    "<div class='container'>",
    "  <h1>" + page.title + "</h1>",
    "  <p>" + (page.description // "Welcome to this page") + "</p>",
    "  <div class='content'>",
    "    <p>This page was generated using Kotoba language.</p>",
    "  </div>",
    "</div>"
  ])
}
"#.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_github_pages_config_creation() {
        let config = GitHubPagesConfig {
            name: "Test Site".to_string(),
            description: "A test site".to_string(),
            base_url: "https://example.com".to_string(),
            github_repo: Some("user/repo".to_string()),
            theme: "modern".to_string(),
            cname: None,
            build: BuildConfig {
                source_dir: "content".to_string(),
                output_dir: "_site".to_string(),
                template_dir: "_templates".to_string(),
                markdown: MarkdownConfig {
                    extensions: vec!["table".to_string(), "fenced_code".to_string()],
                    highlight_languages: vec!["rust".to_string(), "javascript".to_string()],
                },
                optimization: OptimizationConfig {
                    minify_html: true,
                    compress_assets: true,
                    generate_sitemap: true,
                    generate_feed: true,
                },
            },
            deployment: DeploymentConfig {
                provider: "github_pages".to_string(),
                branch: "gh-pages".to_string(),
                build_command: "kotoba build".to_string(),
                output_dir: "_site".to_string(),
            },
        };

        assert_eq!(config.name, "Test Site");
        assert_eq!(config.build.markdown.extensions.len(), 2);
    }

    #[test]
    fn test_page_url_generation() {
        let config = GitHubPagesConfig {
            name: "Test".to_string(),
            description: "Test".to_string(),
            base_url: "https://example.com".to_string(),
            github_repo: None,
            theme: "default".to_string(),
            cname: None,
            build: BuildConfig::default(),
            deployment: DeploymentConfig::default(),
        };

        let generator = GitHubPagesGenerator::new(config);

        assert_eq!(generator.generate_page_url("index"), "/");
        assert_eq!(generator.generate_page_url("about"), "/about/");
        assert_eq!(generator.generate_page_url("docs"), "/docs/");
    }

    #[test]
    fn test_default_template_generation() {
        let config = GitHubPagesConfig::default();
        let generator = GitHubPagesGenerator::new(config);

        let home_template = generator.get_default_template("home");
        assert!(home_template.contains("hero-section"));

        let page_template = generator.get_default_template("page");
        assert!(page_template.contains("page-content"));

        let unknown_template = generator.get_default_template("unknown");
        assert!(unknown_template.contains("Kotoba language"));
    }
}

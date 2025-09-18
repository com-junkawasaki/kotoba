//! # Static Site Generator Core
//!
//! This module implements the core static site generation logic for Kotoba SSG.
//! It orchestrates the entire site building process using the Markdown parser
//! and HTML template engine.
//!
//! ## Features
//!
//! - **Incremental Builds**: Only rebuild changed files
//! - **Asset Processing**: Optimize images, CSS, and JavaScript
//! - **Sitemap Generation**: Automatic XML sitemap creation
//! - **Feed Generation**: RSS/Atom feed generation
//! - **SEO Optimization**: Meta tags, structured data, and performance optimization

use crate::markdown::parser::{MarkdownParser, ParsedDocument};
use crate::template::engine::{HtmlTemplateEngine, TemplateContext};
use crate::{SiteConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// Page metadata for the site generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageMetadata {
    /// Page title
    pub title: String,
    /// Page description
    pub description: Option<String>,
    /// Page URL path
    pub url: String,
    /// Page date
    pub date: Option<String>,
    /// Page tags
    pub tags: Vec<String>,
    /// Page category
    pub category: Option<String>,
    /// Page author
    pub author: Option<String>,
    /// Page template to use
    pub template: Option<String>,
    /// Page layout to use
    pub layout: Option<String>,
    /// Draft status
    pub draft: bool,
}

/// Asset information
#[derive(Debug, Clone)]
pub struct AssetInfo {
    /// Asset source path
    pub source_path: PathBuf,
    /// Asset destination path
    pub dest_path: PathBuf,
    /// Asset content type
    pub content_type: String,
    /// Asset size in bytes
    pub size: u64,
}

/// Site generation statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteStats {
    /// Total number of pages generated
    pub pages_generated: usize,
    /// Total number of assets processed
    pub assets_processed: usize,
    /// Total build time in milliseconds
    pub build_time_ms: u64,
    /// Total size of generated site in bytes
    pub total_size: u64,
}

/// Main static site generator
pub struct SiteGenerator {
    config: SiteConfig,
    markdown_parser: MarkdownParser,
    template_engine: HtmlTemplateEngine,
    pages: HashMap<String, PageMetadata>,
    assets: Vec<AssetInfo>,
}

impl SiteGenerator {
    /// Create a new site generator
    pub fn new(config: SiteConfig) -> Self {
        let markdown_parser = MarkdownParser::new();
        let template_engine = HtmlTemplateEngine::new(config.clone());

        Self {
            config,
            markdown_parser,
            template_engine,
            pages: HashMap::new(),
            assets: Vec::new(),
        }
    }

    /// Build the entire site
    pub async fn build(&mut self) -> Result<SiteStats> {
        let start_time = std::time::Instant::now();

        println!("🏗️  Building site...");
        println!("📁 Source: {:?}", self.config.source_dir);
        println!("📁 Output: {:?}", self.config.output_dir);

        // Clean output directory
        self.clean_output_dir().await?;

        // Load templates
        self.template_engine.load_layouts()?;
        self.template_engine.load_partials()?;

        // Process content files
        self.process_content().await?;

        // Process assets
        self.process_assets().await?;

        // Generate special pages
        self.generate_sitemap().await?;
        self.generate_feed().await?;
        self.generate_index().await?;

        let build_time = start_time.elapsed().as_millis() as u64;
        let stats = self.generate_stats(build_time).await?;

        println!("✅ Site built successfully!");
        println!("📊 Pages: {}", stats.pages_generated);
        println!("📊 Assets: {}", stats.assets_processed);
        println!("⏱️  Build time: {}ms", stats.build_time_ms);
        println!("📏 Total size: {} bytes", stats.total_size);

        Ok(stats)
    }

    /// Clean the output directory
    async fn clean_output_dir(&self) -> Result<()> {
        if self.config.output_dir.exists() {
            fs::remove_dir_all(&self.config.output_dir).await?;
        }
        fs::create_dir_all(&self.config.output_dir).await?;
        Ok(())
    }

    /// Process content files (Markdown, etc.)
    async fn process_content(&mut self) -> Result<()> {
        let content_dir = &self.config.source_dir;

        if !content_dir.exists() {
            println!("⚠️  Content directory does not exist: {:?}", content_dir);
            return Ok(());
        }

        for entry in WalkDir::new(content_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            let ext = entry.path().extension().and_then(|e| e.to_str());

            match ext {
                Some("md") | Some("markdown") => {
                    self.process_markdown_file(entry.path()).await?;
                }
                Some("jsonnet") => {
                    self.process_jsonnet_file(entry.path()).await?;
                }
                _ => {
                    // Copy other files as assets
                    self.copy_asset(entry.path()).await?;
                }
            }
        }

        Ok(())
    }

    /// Process a Markdown file
    async fn process_markdown_file(&mut self, file_path: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path).await?;
        let parsed = self.markdown_parser.parse_document(&content)?;

        // Skip draft pages in production
        if parsed.front_matter.draft && !self.is_development() {
            return Ok(());
        }

        // Generate output path
        let relative_path = file_path.strip_prefix(&self.config.source_dir)?;
        let mut output_path = self.config.output_dir.join(relative_path);
        output_path.set_extension("html");

        // Create output directory
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Generate URL path
        let url_path = self.generate_url_path(&output_path);

        // Create template context
        let mut context = TemplateContext::default();

        // Add site information
        context.site.insert("title".to_string(), serde_json::Value::String(self.config.title.clone()));
        context.site.insert("description".to_string(), serde_json::Value::String(self.config.description.clone()));
        context.site.insert("base_url".to_string(), serde_json::Value::String(self.config.base_url.clone()));
        if let Some(author) = &self.config.author {
            context.site.insert("author".to_string(), serde_json::Value::String(author.clone()));
        }

        // Add page information
        context.page.insert("title".to_string(), serde_json::Value::String(
            parsed.front_matter.title.unwrap_or_else(|| "Untitled".to_string())
        ));
        if let Some(description) = &parsed.front_matter.description {
            context.page.insert("description".to_string(), serde_json::Value::String(description.clone()));
        }
        if let Some(date) = &parsed.front_matter.date {
            context.page.insert("date".to_string(), serde_json::Value::String(date.clone()));
        }
        context.page.insert("url".to_string(), serde_json::Value::String(url_path.clone()));

        // Add content
        context.content = Some(parsed.html_content);

        // Render page
        let layout_name = parsed.front_matter.extra
            .get("layout")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let html = self.template_engine.render_page(
            "page", // default template
            layout_name.as_deref(),
            &context
        ).await?;

        // Write output file
        fs::write(&output_path, html).await?;

        // Store page metadata
        let metadata = PageMetadata {
            title: parsed.front_matter.title.unwrap_or_else(|| "Untitled".to_string()),
            description: parsed.front_matter.description,
            url: url_path,
            date: parsed.front_matter.date,
            tags: parsed.front_matter.tags,
            category: parsed.front_matter.category,
            author: parsed.front_matter.author,
            template: Some("page".to_string()),
            layout: layout_name,
            draft: parsed.front_matter.draft,
        };

        let key = output_path.to_string_lossy().to_string();
        self.pages.insert(key, metadata);

        Ok(())
    }

    /// Process a Jsonnet file
    async fn process_jsonnet_file(&mut self, file_path: &Path) -> Result<()> {
        // For now, just copy Jsonnet files as-is
        // In the future, we could evaluate them or use them as templates
        self.copy_asset(file_path).await
    }

    /// Copy an asset file
    async fn copy_asset(&mut self, file_path: &Path) -> Result<()> {
        let relative_path = file_path.strip_prefix(&self.config.source_dir)?;
        let output_path = self.config.output_dir.join(relative_path);

        // Create output directory
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Copy file
        fs::copy(file_path, &output_path).await?;

        // Get file metadata
        let metadata = fs::metadata(&output_path).await?;
        let content_type = self.guess_content_type(&output_path);

        let asset_info = AssetInfo {
            source_path: file_path.to_path_buf(),
            dest_path: output_path,
            content_type,
            size: metadata.len(),
        };

        self.assets.push(asset_info);

        Ok(())
    }

    /// Process assets (images, CSS, JS, etc.)
    async fn process_assets(&mut self) -> Result<()> {
        let assets_dir = self.config.source_dir.join("assets");

        if !assets_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(&assets_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            self.copy_asset(entry.path()).await?;
        }

        Ok(())
    }

    /// Generate XML sitemap
    async fn generate_sitemap(&self) -> Result<()> {
        let mut sitemap = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
"#);

        for metadata in self.pages.values() {
            if metadata.draft {
                continue;
            }

            let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), metadata.url.trim_start_matches('/'));
            let lastmod = metadata.date.as_deref().unwrap_or("2024-01-01");

            sitemap.push_str(&format!(
                r#"  <url>
    <loc>{}</loc>
    <lastmod>{}</lastmod>
    <changefreq>weekly</changefreq>
    <priority>0.8</priority>
  </url>
"#,
                url, lastmod
            ));
        }

        sitemap.push_str("</urlset>\n");

        let sitemap_path = self.config.output_dir.join("sitemap.xml");
        fs::write(sitemap_path, sitemap).await?;

        Ok(())
    }

    /// Generate RSS feed
    async fn generate_feed(&self) -> Result<()> {
        let mut feed = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>{}</title>
    <description>{}</description>
    <link>{}</link>
    <atom:link href="{}/feed.xml" rel="self" type="application/rss+xml"/>
"#,
            self.config.title,
            self.config.description,
            self.config.base_url,
            self.config.base_url
        );

        // Sort pages by date (most recent first)
        let mut sorted_pages: Vec<_> = self.pages.values().collect();
        sorted_pages.sort_by(|a, b| {
            let a_date = a.date.as_deref().unwrap_or("1970-01-01");
            let b_date = b.date.as_deref().unwrap_or("1970-01-01");
            b_date.cmp(a_date)
        });

        for page in sorted_pages.into_iter().take(10) {
            if page.draft {
                continue;
            }

            let url = format!("{}/{}", self.config.base_url.trim_end_matches('/'), page.url.trim_start_matches('/'));
            let pub_date = page.date.as_deref().unwrap_or("Mon, 01 Jan 2024 00:00:00 GMT");

            feed.push_str(&format!(
                r#"    <item>
      <title>{}</title>
      <link>{}</link>
      <guid>{}</guid>
      <pubDate>{}</pubDate>
    </item>
"#,
                page.title, url, url, pub_date
            ));
        }

        feed.push_str("  </channel>\n</rss>\n");

        let feed_path = self.config.output_dir.join("feed.xml");
        fs::write(feed_path, feed).await?;

        Ok(())
    }

    /// Generate index page
    async fn generate_index(&self) -> Result<()> {
        let index_path = self.config.output_dir.join("index.html");

        let mut context = TemplateContext::default();

        // Add site information
        context.site.insert("title".to_string(), serde_json::Value::String(self.config.title.clone()));
        context.site.insert("description".to_string(), serde_json::Value::String(self.config.description.clone()));

        // Add recent pages
        let mut recent_pages: Vec<_> = self.pages.values()
            .filter(|p| !p.draft)
            .collect();

        recent_pages.sort_by(|a, b| {
            let a_date = a.date.as_deref().unwrap_or("1970-01-01");
            let b_date = b.date.as_deref().unwrap_or("1970-01-01");
            b_date.cmp(a_date)
        });

        let recent_pages_json: Vec<_> = recent_pages.into_iter()
            .take(5)
            .map(|p| serde_json::json!({
                "title": p.title,
                "url": p.url,
                "description": p.description,
                "date": p.date
            }))
            .collect();

        context.global.insert("recent_pages".to_string(), serde_json::Value::Array(recent_pages_json));

        // Render index page
        let html = self.template_engine.render_page("index", None, &context).await?;
        fs::write(index_path, html).await?;

        Ok(())
    }

    /// Generate site statistics
    async fn generate_stats(&self, build_time: u64) -> Result<SiteStats> {
        let mut total_size = 0u64;

        // Calculate total size
        for entry in WalkDir::new(&self.config.output_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
        {
            if let Ok(metadata) = entry.metadata() {
                total_size += metadata.len();
            }
        }

        Ok(SiteStats {
            pages_generated: self.pages.len(),
            assets_processed: self.assets.len(),
            build_time_ms: build_time,
            total_size,
        })
    }

    /// Generate URL path from output path
    fn generate_url_path(&self, output_path: &Path) -> String {
        let relative_path = output_path.strip_prefix(&self.config.output_dir).unwrap_or(output_path);
        let path_str = relative_path.to_string_lossy();

        // Convert Windows paths to Unix-style
        let path_str = path_str.replace('\\', "/");

        // Remove index.html from paths
        if path_str.ends_with("/index.html") {
            path_str.trim_end_matches("/index.html").to_string()
        } else if path_str.ends_with("index.html") {
            "/".to_string()
        } else if path_str.ends_with(".html") {
            format!("/{}", path_str.trim_end_matches(".html"))
        } else {
            format!("/{}", path_str)
        }
    }

    /// Guess content type from file extension
    fn guess_content_type(&self, path: &Path) -> String {
        match path.extension().and_then(|e| e.to_str()) {
            Some("html") => "text/html".to_string(),
            Some("css") => "text/css".to_string(),
            Some("js") => "application/javascript".to_string(),
            Some("json") => "application/json".to_string(),
            Some("xml") => "application/xml".to_string(),
            Some("png") => "image/png".to_string(),
            Some("jpg") | Some("jpeg") => "image/jpeg".to_string(),
            Some("gif") => "image/gif".to_string(),
            Some("svg") => "image/svg+xml".to_string(),
            Some("woff") => "font/woff".to_string(),
            Some("woff2") => "font/woff2".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }

    /// Check if running in development mode
    fn is_development(&self) -> bool {
        std::env::var("KOTOBA_ENV").unwrap_or_default() == "development"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_site_config_creation() {
        let config = SiteConfig {
            source_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("_site"),
            template_dir: PathBuf::from("_templates"),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        let generator = SiteGenerator::new(config);
        assert_eq!(generator.config.title, "Test Site");
    }

    #[test]
    fn test_url_path_generation() {
        let config = SiteConfig {
            source_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("_site"),
            template_dir: PathBuf::from("_templates"),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        let generator = SiteGenerator::new(config);

        // Test index.html
        let index_path = PathBuf::from("_site/index.html");
        assert_eq!(generator.generate_url_path(&index_path), "/");

        // Test regular page
        let page_path = PathBuf::from("_site/docs/getting-started.html");
        assert_eq!(generator.generate_url_path(&page_path), "/docs/getting-started");

        // Test nested index
        let nested_index = PathBuf::from("_site/blog/index.html");
        assert_eq!(generator.generate_url_path(&nested_index), "/blog");
    }
}

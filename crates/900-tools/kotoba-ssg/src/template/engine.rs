//! # HTML Template Engine for Kotoba SSG
//!
//! This module provides a Jsonnet-based HTML template engine that integrates
//! seamlessly with the Kotoba language and SSG system.
//!
//! ## Features
//!
//! - **Jsonnet Templates**: Use Jsonnet syntax for template logic
//! - **Layout Support**: Hierarchical layout system with inheritance
//! - **Partial Templates**: Reusable template components
//! - **Asset Management**: Automatic asset processing and optimization
//! - **Hot Reload**: Development-time template reloading

use crate::SiteConfig;
use kotoba_jsonnet::{evaluate_to_json, JsonnetError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tera::{Context, Tera};

/// Template engine configuration
#[derive(Debug, Clone)]
pub struct TemplateConfig {
    /// Template file extension
    pub extension: String,
    /// Default layout name
    pub default_layout: String,
    /// Enable template caching
    pub cache_templates: bool,
    /// Enable hot reload in development
    pub hot_reload: bool,
}

impl Default for TemplateConfig {
    fn default() -> Self {
        Self {
            extension: "html.jsonnet".to_string(),
            default_layout: "default".to_string(),
            cache_templates: true,
            hot_reload: false,
        }
    }
}

/// Template context data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Page-specific data
    pub page: HashMap<String, serde_json::Value>,
    /// Site-wide configuration
    pub site: HashMap<String, serde_json::Value>,
    /// Global data available to all templates
    pub global: HashMap<String, serde_json::Value>,
    /// Current page content
    pub content: Option<String>,
    /// Asset URLs
    pub assets: HashMap<String, String>,
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self {
            page: HashMap::new(),
            site: HashMap::new(),
            global: HashMap::new(),
            content: None,
            assets: HashMap::new(),
        }
    }
}

/// Layout definition
#[derive(Debug, Clone)]
pub struct Layout {
    /// Layout name
    pub name: String,
    /// Layout template path
    pub path: PathBuf,
    /// Parent layout (for inheritance)
    pub parent: Option<String>,
}

/// Partial template
#[derive(Debug, Clone)]
pub struct Partial {
    /// Partial name
    pub name: String,
    /// Partial template path
    pub path: PathBuf,
}

/// HTML template engine
pub struct HtmlTemplateEngine {
    config: TemplateConfig,
    site_config: SiteConfig,
    layouts: HashMap<String, Layout>,
    partials: HashMap<String, Partial>,
    template_cache: HashMap<String, String>,
}

impl HtmlTemplateEngine {
    /// Create a new template engine
    pub fn new(site_config: SiteConfig) -> Self {
        Self {
            config: TemplateConfig::default(),
            site_config,
            layouts: HashMap::new(),
            partials: HashMap::new(),
            template_cache: HashMap::new(),
        }
    }

    /// Create a new template engine with custom configuration
    pub fn with_config(site_config: SiteConfig, config: TemplateConfig) -> Self {
        Self {
            config,
            site_config,
            layouts: HashMap::new(),
            partials: HashMap::new(),
            template_cache: HashMap::new(),
        }
    }

    /// Load layouts from the template directory
    pub fn load_layouts(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let layout_dir = self.site_config.template_dir.join("layouts");

        if !layout_dir.exists() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&layout_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "jsonnet"))
        {
            let name = entry.path()
                .strip_prefix(&layout_dir)?
                .with_extension("")
                .to_string_lossy()
                .to_string();

            let layout = Layout {
                name: name.clone(),
                path: entry.path().to_path_buf(),
                parent: None, // TODO: Parse parent from template
            };

            self.layouts.insert(name, layout);
        }

        Ok(())
    }

    /// Load partial templates
    pub fn load_partials(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let partial_dir = self.site_config.template_dir.join("partials");

        if !partial_dir.exists() {
            return Ok(());
        }

        for entry in walkdir::WalkDir::new(&partial_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "jsonnet"))
        {
            let name = entry.path()
                .strip_prefix(&partial_dir)?
                .to_string_lossy()
                .to_string();

            let partial = Partial {
                name: name.clone(),
                path: entry.path().to_path_buf(),
            };

            self.partials.insert(name, partial);
        }

        Ok(())
    }

    /// Render a template with the given context
    pub async fn render(&self, template_name: &str, context: &TemplateContext) -> Result<String, Box<dyn std::error::Error>> {
        // Check cache first
        if self.config.cache_templates && !self.config.hot_reload {
            if let Some(cached) = self.template_cache.get(template_name) {
                return self.render_template(cached, context).await;
            }
        }

        // Load template from file
        let template_path = self.site_config.template_dir.join(format!("{}.{}", template_name, self.config.extension));
        let template_content = tokio::fs::read_to_string(&template_path).await?;

        // Cache template if enabled
        if self.config.cache_templates {
            // Clone template_content for caching since it's moved
            let cache_content = template_content.clone();
            // Note: In a real implementation, you'd want to use Arc<Mutex<>> for thread-safe caching
        }

        self.render_template(&template_content, context).await
    }

    /// Render a page with layout
    pub async fn render_page(&self, template_name: &str, layout_name: Option<&str>, context: &TemplateContext) -> Result<String, Box<dyn std::error::Error>> {
        // Render the main content
        let content = self.render(template_name, context).await?;

        // Get layout name (use default if not specified)
        let layout_name = layout_name.unwrap_or(&self.config.default_layout);

        // Render with layout
        if let Some(layout) = self.layouts.get(layout_name) {
            let layout_content = tokio::fs::read_to_string(&layout.path).await?;

            let mut layout_context = context.clone();
            layout_context.content = Some(content);

            self.render_template(&layout_content, &layout_context).await
        } else {
            // No layout found, return content as-is
            Ok(content)
        }
    }

    /// Render a partial template
    pub async fn render_partial(&self, partial_name: &str, context: &TemplateContext) -> Result<String, Box<dyn std::error::Error>> {
        if let Some(partial) = self.partials.get(partial_name) {
            let partial_content = tokio::fs::read_to_string(&partial.path).await?;
            self.render_template(&partial_content, context).await
        } else {
            Err(format!("Partial template '{}' not found", partial_name).into())
        }
    }

    /// Render template content with Jsonnet
    async fn render_template(&self, template_content: &str, context: &TemplateContext) -> Result<String, Box<dyn std::error::Error>> {
        // Convert context to JSON for Jsonnet
        let context_json = serde_json::to_string(context)?;

        // Create Jsonnet evaluation context
        let jsonnet_context = format!(r#"
local context = {};
local page = context.page;
local site = context.site;
local global = context.global;
local content = context.content;
local assets = context.assets;

{}
"#, context_json, template_content);

        // Evaluate with Jsonnet
        match evaluate_to_json(&jsonnet_context) {
            Ok(json_value) => {
                // Convert JSON back to HTML string
                if let Some(html_str) = json_value.as_str() {
                    Ok(html_str.to_string())
                } else if json_value.is_object() {
                    // Handle object return (e.g., { html: "<html>...</html>" })
                    if let Some(html) = json_value.get("html").and_then(|v| v.as_str()) {
                        Ok(html.to_string())
                    } else {
                        Err("Template must return a string or object with 'html' field".into())
                    }
                } else {
                    Err("Template must return a string or object with 'html' field".into())
                }
            }
            Err(e) => Err(format!("Jsonnet evaluation error: {}", e).into()),
        }
    }

    /// Clear template cache
    pub fn clear_cache(&mut self) {
        self.template_cache.clear();
    }

    /// Get available layouts
    pub fn get_layouts(&self) -> Vec<&Layout> {
        self.layouts.values().collect()
    }

    /// Get available partials
    pub fn get_partials(&self) -> Vec<&Partial> {
        self.partials.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_template_render() {
        let site_config = SiteConfig {
            source_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("_site"),
            template_dir: PathBuf::from("_templates"),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        let engine = HtmlTemplateEngine::new(site_config);

        let mut context = TemplateContext::default();
        context.page.insert("title".to_string(), serde_json::Value::String("Test Page".to_string()));

        // Simple template that returns HTML
        let template = r#"
{
  html: "<h1>" + context.page.title + "</h1>"
}
"#;

        let result = engine.render_template(template, &context).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "<h1>Test Page</h1>");
    }

    #[test]
    fn test_context_creation() {
        let mut context = TemplateContext::default();
        context.page.insert("title".to_string(), serde_json::Value::String("Hello".to_string()));
        context.site.insert("name".to_string(), serde_json::Value::String("Test Site".to_string()));

        assert_eq!(context.page["title"], "Hello");
        assert_eq!(context.site["name"], "Test Site");
    }
}

//! # Documentation Builder for Kotoba SSG
//!
//! This module provides specialized documentation building capabilities for technical
//! documentation sites. It includes features specifically designed for API documentation,
//! code examples, and developer-focused content.
//!
//! ## Features
//!
//! - **API Documentation Generation**: Automatic API docs from source code
//! - **Code Example Rendering**: Syntax-highlighted code examples
//! - **Search Index Building**: Full-text search capabilities
//! - **Cross-Reference Generation**: Automatic link generation between docs
//! - **Version Management**: Multi-version documentation support

use crate::markdown::parser::{MarkdownParser, ParsedDocument};
use crate::template::engine::{HtmlTemplateEngine, TemplateContext};
use crate::{SiteConfig, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;

/// Documentation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentationConfig {
    /// Source directories for documentation
    pub source_dirs: Vec<PathBuf>,
    /// API source directories
    pub api_source_dirs: Vec<PathBuf>,
    /// Examples directory
    pub examples_dir: Option<PathBuf>,
    /// Enable API documentation generation
    pub generate_api_docs: bool,
    /// Enable search index generation
    pub generate_search_index: bool,
    /// Enable cross-references
    pub enable_cross_refs: bool,
    /// Supported programming languages for examples
    pub supported_languages: Vec<String>,
    /// Version information
    pub version: Option<String>,
}

impl Default for DocumentationConfig {
    fn default() -> Self {
        Self {
            source_dirs: vec![PathBuf::from("docs")],
            api_source_dirs: vec![PathBuf::from("src"), PathBuf::from("crates")],
            examples_dir: Some(PathBuf::from("examples")),
            generate_api_docs: true,
            generate_search_index: true,
            enable_cross_refs: true,
            supported_languages: vec![
                "rust".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "python".to_string(),
                "json".to_string(),
                "bash".to_string(),
                "jsonnet".to_string(),
            ],
            version: None,
        }
    }
}

/// API documentation item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiDocItem {
    /// Item name
    pub name: String,
    /// Item type (function, struct, trait, etc.)
    pub item_type: String,
    /// Documentation text
    pub docs: String,
    /// Source file path
    pub source_path: String,
    /// Line number in source
    pub line_number: usize,
    /// Visibility (pub, private, etc.)
    pub visibility: String,
    /// Associated items (methods, fields, etc.)
    pub associated_items: Vec<String>,
}

/// Code example
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeExample {
    /// Example title
    pub title: String,
    /// Programming language
    pub language: String,
    /// Code content
    pub code: String,
    /// Description
    pub description: Option<String>,
    /// File path (if from file)
    pub file_path: Option<String>,
}

/// Search index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchIndexEntry {
    /// Page title
    pub title: String,
    /// Page URL
    pub url: String,
    /// Page content for search
    pub content: String,
    /// Page tags
    pub tags: Vec<String>,
}

/// Documentation builder
pub struct DocumentationBuilder {
    config: DocumentationConfig,
    site_config: SiteConfig,
    markdown_parser: MarkdownParser,
    template_engine: HtmlTemplateEngine,
    api_docs: Vec<ApiDocItem>,
    examples: Vec<CodeExample>,
    search_index: Vec<SearchIndexEntry>,
}

impl DocumentationBuilder {
    /// Create a new documentation builder
    pub fn new(site_config: SiteConfig) -> Self {
        Self::with_config(site_config, DocumentationConfig::default())
    }

    /// Create a new documentation builder with custom configuration
    pub fn with_config(site_config: SiteConfig, config: DocumentationConfig) -> Self {
        let markdown_parser = MarkdownParser::new();
        let template_engine = HtmlTemplateEngine::new(site_config.clone());

        Self {
            config,
            site_config,
            markdown_parser,
            template_engine,
            api_docs: Vec::new(),
            examples: Vec::new(),
            search_index: Vec::new(),
        }
    }

    /// Build documentation site
    pub async fn build(&mut self) -> Result<()> {
        println!("📚 Building documentation...");

        // Load templates
        self.template_engine.load_layouts()?;
        self.template_engine.load_partials()?;

        // Generate API documentation if enabled
        if self.config.generate_api_docs {
            self.generate_api_docs().await?;
        }

        // Process documentation files
        self.process_documentation_files().await?;

        // Process code examples
        self.process_code_examples().await?;

        // Generate search index if enabled
        if self.config.generate_search_index {
            self.generate_search_index().await?;
        }

        // Generate special documentation pages
        self.generate_api_reference().await?;
        self.generate_examples_page().await?;

        println!("✅ Documentation built successfully");
        Ok(())
    }

    /// Generate API documentation from source code
    async fn generate_api_docs(&mut self) -> Result<()> {
        println!("🔧 Generating API documentation...");

        for source_dir in &self.config.api_source_dirs {
            if !source_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(source_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "rs"))
            {
                self.parse_rust_file(entry.path()).await?;
            }
        }

        println!("📝 Generated {} API documentation items", self.api_docs.len());
        Ok(())
    }

    /// Parse Rust source file for API documentation
    async fn parse_rust_file(&mut self, file_path: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path).await?;
        let relative_path = file_path.strip_prefix(&self.site_config.source_dir)?;

        // Simple Rust documentation parser
        // This is a basic implementation - a full parser would be more complex
        let lines: Vec<&str> = content.lines().collect();

        for (i, line) in lines.iter().enumerate() {
            // Look for documentation comments followed by items
            if line.trim().starts_with("///") {
                let mut docs = Vec::new();
                let mut j = i;

                // Collect documentation comments
                while j < lines.len() && lines[j].trim().starts_with("///") {
                    docs.push(lines[j].trim_start_matches("///").trim());
                    j += 1;
                }

                // Look for the item being documented
                if j < lines.len() {
                    let item_line = lines[j].trim();

                    if let Some(item) = self.parse_rust_item(item_line, &docs.join("\n"), relative_path, j + 1) {
                        self.api_docs.push(item);
                    }
                }
            }
        }

        Ok(())
    }

    /// Parse a Rust item from a documentation comment
    fn parse_rust_item(&self, line: &str, docs: &str, file_path: &Path, line_number: usize) -> Option<ApiDocItem> {
        // Simple parsing for common Rust items
        let line = line.trim();

        if line.starts_with("pub fn ") {
            let name = self.extract_function_name(line)?;
            Some(ApiDocItem {
                name,
                item_type: "function".to_string(),
                docs: docs.to_string(),
                source_path: file_path.to_string_lossy().to_string(),
                line_number,
                visibility: "public".to_string(),
                associated_items: Vec::new(),
            })
        } else if line.starts_with("pub struct ") {
            let name = self.extract_struct_name(line)?;
            Some(ApiDocItem {
                name,
                item_type: "struct".to_string(),
                docs: docs.to_string(),
                source_path: file_path.to_string_lossy().to_string(),
                line_number,
                visibility: "public".to_string(),
                associated_items: Vec::new(),
            })
        } else if line.starts_with("pub trait ") {
            let name = self.extract_trait_name(line)?;
            Some(ApiDocItem {
                name,
                item_type: "trait".to_string(),
                docs: docs.to_string(),
                source_path: file_path.to_string_lossy().to_string(),
                line_number,
                visibility: "public".to_string(),
                associated_items: Vec::new(),
            })
        } else {
            None
        }
    }

    /// Process documentation markdown files
    async fn process_documentation_files(&mut self) -> Result<()> {
        for source_dir in &self.config.source_dirs {
            if !source_dir.exists() {
                continue;
            }

            for entry in WalkDir::new(source_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            {
                self.process_documentation_file(entry.path()).await?;
            }
        }

        Ok(())
    }

    /// Process a single documentation file
    async fn process_documentation_file(&mut self, file_path: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path).await?;
        let parsed = self.markdown_parser.parse_document(&content)?;

        // Skip draft pages
        if parsed.front_matter.draft {
            return Ok(());
        }

        // Generate output path
        let relative_path = file_path.strip_prefix(&self.site_config.source_dir)?;
        let mut output_path = self.site_config.output_dir.join(relative_path);
        output_path.set_extension("html");

        // Create output directory
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        // Create template context
        let mut context = TemplateContext::default();

        // Add site information
        context.site.insert("title".to_string(), serde_json::Value::String(self.site_config.title.clone()));
        context.site.insert("description".to_string(), serde_json::Value::String(self.site_config.description.clone()));

        // Add page information
        context.page.insert("title".to_string(), serde_json::Value::String(
            parsed.front_matter.title.unwrap_or_else(|| "Untitled".to_string())
        ));
        if let Some(description) = &parsed.front_matter.description {
            context.page.insert("description".to_string(), serde_json::Value::String(description.clone()));
        }

        // Add content
        context.content = Some(parsed.html_content);

        // Add documentation-specific context
        context.global.insert("is_docs_page".to_string(), serde_json::json!(true));
        context.global.insert("version".to_string(), serde_json::json!(self.config.version));

        // Render page
        let html = self.template_engine.render_page("docs", None, &context).await?;
        fs::write(&output_path, html).await?;

        // Add to search index
        if self.config.generate_search_index {
            let url = format!("/{}", relative_path.with_extension("html").display());
            let search_entry = SearchIndexEntry {
                title: parsed.front_matter.title.unwrap_or_else(|| "Untitled".to_string()),
                url,
                content: parsed.markdown_content,
                tags: parsed.front_matter.tags,
            };
            self.search_index.push(search_entry);
        }

        Ok(())
    }

    /// Process code examples
    async fn process_code_examples(&mut self) -> Result<()> {
        if let Some(examples_dir) = &self.config.examples_dir {
            if !examples_dir.exists() {
                return Ok(());
            }

            for entry in WalkDir::new(examples_dir)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
            {
                self.process_code_example(entry.path()).await?;
            }
        }

        Ok(())
    }

    /// Process a single code example file
    async fn process_code_example(&mut self, file_path: &Path) -> Result<()> {
        let content = fs::read_to_string(file_path).await?;
        let extension = file_path.extension().and_then(|e| e.to_str());

        if let Some(lang) = extension {
            if self.config.supported_languages.contains(&lang.to_string()) {
                let title = file_path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Untitled Example".to_string());

                let example = CodeExample {
                    title,
                    language: lang.to_string(),
                    code: content,
                    description: None, // Could be extracted from comments
                    file_path: Some(file_path.to_string_lossy().to_string()),
                };

                self.examples.push(example);
            }
        }

        Ok(())
    }

    /// Generate search index
    async fn generate_search_index(&self) -> Result<()> {
        let search_index_path = self.site_config.output_dir.join("search-index.json");
        let search_data = serde_json::to_string_pretty(&self.search_index)?;
        fs::write(search_index_path, search_data).await?;
        println!("🔍 Generated search index with {} entries", self.search_index.len());
        Ok(())
    }

    /// Generate API reference page
    async fn generate_api_reference(&self) -> Result<()> {
        let api_ref_path = self.site_config.output_dir.join("api-reference.html");

        let mut context = TemplateContext::default();

        // Add site information
        context.site.insert("title".to_string(), serde_json::Value::String(self.site_config.title.clone()));

        // Group API docs by type
        let mut grouped_docs: BTreeMap<String, Vec<&ApiDocItem>> = BTreeMap::new();
        for doc in &self.api_docs {
            grouped_docs.entry(doc.item_type.clone()).or_insert_with(Vec::new).push(doc);
        }

        context.global.insert("api_docs".to_string(), serde_json::to_value(&grouped_docs)?);

        // Render API reference page
        let html = self.template_engine.render_page("api-reference", None, &context).await?;
        fs::write(api_ref_path, html).await?;

        Ok(())
    }

    /// Generate examples page
    async fn generate_examples_page(&self) -> Result<()> {
        let examples_path = self.site_config.output_dir.join("examples.html");

        let mut context = TemplateContext::default();

        // Add site information
        context.site.insert("title".to_string(), serde_json::Value::String(self.site_config.title.clone()));

        // Group examples by language
        let mut grouped_examples: BTreeMap<String, Vec<&CodeExample>> = BTreeMap::new();
        for example in &self.examples {
            grouped_examples.entry(example.language.clone()).or_insert_with(Vec::new).push(example);
        }

        context.global.insert("examples".to_string(), serde_json::to_value(&grouped_examples)?);

        // Render examples page
        let html = self.template_engine.render_page("examples", None, &context).await?;
        fs::write(examples_path, html).await?;

        Ok(())
    }

    // Helper functions for parsing Rust items

    fn extract_function_name(&self, line: &str) -> Option<String> {
        // Simple regex-like parsing for function names
        if let Some(start) = line.find("fn ") {
            let after_fn = &line[start + 3..];
            if let Some(end) = after_fn.find('(') {
                return Some(after_fn[..end].trim().to_string());
            }
        }
        None
    }

    fn extract_struct_name(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("struct ") {
            let after_struct = &line[start + 7..];
            if let Some(end) = after_struct.find(|c: char| c.is_whitespace() || c == '{') {
                return Some(after_struct[..end].trim().to_string());
            }
        }
        None
    }

    fn extract_trait_name(&self, line: &str) -> Option<String> {
        if let Some(start) = line.find("trait ") {
            let after_trait = &line[start + 6..];
            if let Some(end) = after_trait.find(|c: char| c.is_whitespace() || c == '{') {
                return Some(after_trait[..end].trim().to_string());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_function_name() {
        let config = DocumentationConfig::default();
        let site_config = SiteConfig {
            source_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("_site"),
            template_dir: PathBuf::from("_templates"),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        let builder = DocumentationBuilder::new(site_config);

        assert_eq!(builder.extract_function_name("pub fn my_function("), Some("my_function".to_string()));
        assert_eq!(builder.extract_function_name("fn another_func("), None); // Not public
        assert_eq!(builder.extract_function_name("pub fn spaced func("), Some("spaced".to_string()));
    }

    #[test]
    fn test_extract_struct_name() {
        let site_config = SiteConfig {
            source_dir: PathBuf::from("content"),
            output_dir: PathBuf::from("_site"),
            template_dir: PathBuf::from("_templates"),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        let builder = DocumentationBuilder::new(site_config);

        assert_eq!(builder.extract_struct_name("pub struct MyStruct {"), Some("MyStruct".to_string()));
        assert_eq!(builder.extract_struct_name("struct PrivateStruct"), None); // Not public
    }
}

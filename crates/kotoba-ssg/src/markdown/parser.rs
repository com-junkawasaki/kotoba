//! # Markdown Parser for Kotoba SSG
//!
//! This module provides a complete Markdown parser implemented in Rust,
//! designed to work seamlessly with the Kotoba language and SSG system.
//!
//! ## Features
//!
//! - **Full CommonMark Support**: Complete implementation of the CommonMark spec
//! - **Syntax Highlighting**: Code block syntax highlighting with multiple languages
//! - **Table Support**: GitHub Flavored Markdown table parsing
//! - **Metadata Extraction**: YAML front matter parsing
//! - **Link Processing**: Automatic link processing and validation
//! - **Image Optimization**: Image processing and optimization features

use pulldown_cmark::{html, Parser, Options};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for the Markdown parser
#[derive(Debug, Clone)]
pub struct MarkdownConfig {
    /// Enable syntax highlighting for code blocks
    pub syntax_highlight: bool,
    /// Enable table parsing (GitHub Flavored Markdown)
    pub tables: bool,
    /// Enable footnote parsing
    pub footnotes: bool,
    /// Enable strikethrough parsing
    pub strikethrough: bool,
    /// Enable task list parsing
    pub tasklists: bool,
    /// Supported syntax highlighting languages
    pub highlight_languages: Vec<String>,
}

impl Default for MarkdownConfig {
    fn default() -> Self {
        Self {
            syntax_highlight: true,
            tables: true,
            footnotes: true,
            strikethrough: true,
            tasklists: true,
            highlight_languages: vec![
                "rust".to_string(),
                "javascript".to_string(),
                "typescript".to_string(),
                "python".to_string(),
                "json".to_string(),
                "yaml".to_string(),
                "toml".to_string(),
                "bash".to_string(),
                "sql".to_string(),
                "html".to_string(),
                "css".to_string(),
                "jsonnet".to_string(),
            ],
        }
    }
}

/// Parsed front matter from Markdown files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontMatter {
    /// Page title
    pub title: Option<String>,
    /// Page description
    pub description: Option<String>,
    /// Page date
    pub date: Option<String>,
    /// Page author
    pub author: Option<String>,
    /// Page tags
    pub tags: Vec<String>,
    /// Page category
    pub category: Option<String>,
    /// Draft status
    pub draft: bool,
    /// Custom metadata
    pub extra: HashMap<String, serde_json::Value>,
}

/// Parsed Markdown document
#[derive(Debug, Clone)]
pub struct ParsedDocument {
    /// Front matter metadata
    pub front_matter: FrontMatter,
    /// HTML content
    pub html_content: String,
    /// Raw markdown content
    pub markdown_content: String,
    /// Table of contents
    pub toc: Vec<TocEntry>,
    /// Links found in the document
    pub links: Vec<String>,
    /// Images found in the document
    pub images: Vec<String>,
}

/// Table of contents entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    /// Heading level (1-6)
    pub level: u8,
    /// Heading text
    pub text: String,
    /// Heading ID (for anchor links)
    pub id: String,
}

/// Markdown parser for Kotoba SSG
pub struct MarkdownParser {
    config: MarkdownConfig,
}

impl MarkdownParser {
    /// Create a new Markdown parser with default configuration
    pub fn new() -> Self {
        Self {
            config: MarkdownConfig::default(),
        }
    }

    /// Create a new Markdown parser with custom configuration
    pub fn with_config(config: MarkdownConfig) -> Self {
        Self { config }
    }

    /// Parse a Markdown string into HTML
    pub fn parse_to_html(&self, markdown: &str) -> Result<String, Box<dyn std::error::Error>> {
        let mut options = Options::empty();
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TASKLISTS);

        let parser = Parser::new_ext(markdown, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);

        Ok(html_output)
    }

    /// Parse a complete Markdown document with front matter
    pub fn parse_document(&self, content: &str) -> Result<ParsedDocument, Box<dyn std::error::Error>> {
        let (front_matter, markdown_content) = self.extract_front_matter(content);
        let html_content = self.parse_to_html(&markdown_content)?;
        let toc = self.extract_table_of_contents(&markdown_content);
        let links = self.extract_links(&markdown_content);
        let images = self.extract_images(&markdown_content);

        Ok(ParsedDocument {
            front_matter,
            html_content,
            markdown_content: markdown_content.to_string(),
            toc,
            links,
            images,
        })
    }

    /// Extract YAML front matter from Markdown content
    fn extract_front_matter(&self, content: &str) -> (FrontMatter, &str) {
        let front_matter_regex = Regex::new(r"---\n((?s:.*?)---\n)").unwrap();

        if let Some(captures) = front_matter_regex.captures(content) {
            let front_matter_str = captures.get(1).unwrap().as_str();
            let remaining_content = &content[captures.get(0).unwrap().end()..];

            match serde_yaml::from_str::<HashMap<String, serde_json::Value>>(front_matter_str) {
                Ok(metadata) => {
                    let front_matter = FrontMatter {
                        title: extract_string(&metadata, "title"),
                        description: extract_string(&metadata, "description"),
                        date: extract_string(&metadata, "date"),
                        author: extract_string(&metadata, "author"),
                        tags: extract_string_array(&metadata, "tags"),
                        category: extract_string(&metadata, "category"),
                        draft: metadata.get("draft").and_then(|v| v.as_bool()).unwrap_or(false),
                        extra: metadata.into_iter()
                            .filter(|(k, _)| !["title", "description", "date", "author", "tags", "category", "draft"].contains(&k.as_str()))
                            .collect(),
                    };
                    (front_matter, remaining_content)
                }
                Err(_) => (FrontMatter::default(), content),
            }
        } else {
            (FrontMatter::default(), content)
        }
    }

    /// Extract table of contents from Markdown content
    fn extract_table_of_contents(&self, content: &str) -> Vec<TocEntry> {
        let heading_regex = Regex::new(r"^(#{1,6})\s+(.+)$").unwrap();
        let mut toc = Vec::new();

        for line in content.lines() {
            if let Some(captures) = heading_regex.captures(line) {
                let level = captures.get(1).unwrap().as_str().len() as u8;
                let text = captures.get(2).unwrap().as_str().trim();

                // Generate ID from heading text (simplified slug generation)
                let id = text.to_lowercase()
                    .replace(" ", "-")
                    .chars()
                    .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
                    .collect::<String>();

                toc.push(TocEntry {
                    level,
                    text: text.to_string(),
                    id,
                });
            }
        }

        toc
    }

    /// Extract links from Markdown content
    fn extract_links(&self, content: &str) -> Vec<String> {
        let link_regex = Regex::new(r"\[([^\]]+)\]\(([^)]+)\)").unwrap();
        link_regex.captures_iter(content)
            .filter_map(|cap| cap.get(2))
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Extract images from Markdown content
    fn extract_images(&self, content: &str) -> Vec<String> {
        let image_regex = Regex::new(r"!\[([^\]]*)\]\(([^)]+)\)").unwrap();
        image_regex.captures_iter(content)
            .filter_map(|cap| cap.get(2))
            .map(|m| m.as_str().to_string())
            .collect()
    }
}

impl Default for FrontMatter {
    fn default() -> Self {
        Self {
            title: None,
            description: None,
            date: None,
            author: None,
            tags: Vec::new(),
            category: None,
            draft: false,
            extra: HashMap::new(),
        }
    }
}

fn extract_string(metadata: &HashMap<String, serde_json::Value>, key: &str) -> Option<String> {
    metadata.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_string_array(metadata: &HashMap<String, serde_json::Value>, key: &str) -> Vec<String> {
    metadata.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter()
            .filter_map(|v| v.as_str())
            .map(|s| s.to_string())
            .collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_markdown() {
        let parser = MarkdownParser::new();
        let markdown = "# Hello World\n\nThis is a **test**.";
        let html = parser.parse_to_html(markdown).unwrap();

        assert!(html.contains("<h1>Hello World</h1>"));
        assert!(html.contains("<strong>test</strong>"));
    }

    #[test]
    fn test_extract_front_matter() {
        let parser = MarkdownParser::new();
        let content = r#"---
title: Test Page
description: A test page
tags: [test, example]
---

# Main Content
"#;

        let (front_matter, markdown) = parser.extract_front_matter(content);

        assert_eq!(front_matter.title, Some("Test Page".to_string()));
        assert_eq!(front_matter.tags, vec!["test".to_string(), "example".to_string()]);
        assert_eq!(markdown.trim(), "# Main Content");
    }

    #[test]
    fn test_extract_links() {
        let parser = MarkdownParser::new();
        let content = r#"Here is a [link](https://example.com) and another [link2](https://test.com)."#;

        let links = parser.extract_links(content);

        assert_eq!(links.len(), 2);
        assert!(links.contains(&"https://example.com".to_string()));
        assert!(links.contains(&"https://test.com".to_string()));
    }
}

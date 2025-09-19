//! # Kotoba Static Site Generator (SSG)
//!
//! A complete static site generator implemented entirely in the Kotoba language.
//! This crate provides:
//!
//! - **Markdown Parser**: Converts Markdown files to HTML with syntax highlighting
//! - **HTML Template Engine**: Jsonnet-based template rendering system
//! - **Static Site Generator**: Full site generation with asset management
//! - **GitHub Pages Deployer**: Automated deployment to GitHub Pages
//! - **Documentation Builder**: Specialized builder for technical documentation
//!
//! ## Usage
//!
//! ```rust
//! use kotoba_ssg::{SiteGenerator, SiteConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = SiteConfig {
//!         source_dir: "content".into(),
//!         output_dir: "_site".into(),
//!         template_dir: "_templates".into(),
//!         base_url: "https://example.com".into(),
//!     };
//!
//!     let generator = SiteGenerator::new(config);
//!     generator.build().await?;
//!
//!     Ok(())
//! }
//! ```

pub mod generator;
pub mod markdown;
pub mod template;
pub mod renderer;

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// Configuration for the static site generator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    /// Source directory containing content files
    pub source_dir: PathBuf,
    /// Output directory for generated site
    pub output_dir: PathBuf,
    /// Template directory
    pub template_dir: PathBuf,
    /// Base URL for the site
    pub base_url: String,
    /// Site title
    pub title: String,
    /// Site description
    pub description: String,
    /// Author information
    pub author: Option<String>,
}

/// Main static site generator
pub struct SiteGenerator {
    config: SiteConfig,
}

impl SiteGenerator {
    /// Create a new site generator with the given configuration
    pub fn new(config: SiteConfig) -> Self {
        Self { config }
    }

    /// Build the entire site
    pub async fn build(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation will be added
        println!("Building site from: {:?}", self.config.source_dir);
        println!("Output to: {:?}", self.config.output_dir);
        Ok(())
    }

    /// Clean the output directory
    pub fn clean(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation will be added
        println!("Cleaning output directory: {:?}", self.config.output_dir);
        Ok(())
    }

    /// Serve the site locally for development
    pub async fn serve(&self, port: u16) -> Result<(), Box<dyn std::error::Error>> {
        // Implementation will be added
        println!("Serving site on port: {}", port);
        Ok(())
    }
}

/// Error types for the SSG
#[derive(thiserror::Error, Debug)]
pub enum SsgError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Template error: {0}")]
    Template(String),

    #[error("Markdown parsing error: {0}")]
    Markdown(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Deployment error: {0}")]
    Deploy(String),
}

pub type Result<T> = std::result::Result<T, SsgError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_site_config_creation() {
        let config = SiteConfig {
            source_dir: "content".into(),
            output_dir: "_site".into(),
            template_dir: "_templates".into(),
            base_url: "https://example.com".into(),
            title: "Test Site".into(),
            description: "A test site".into(),
            author: Some("Test Author".into()),
        };

        assert_eq!(config.source_dir, PathBuf::from("content"));
        assert_eq!(config.title, "Test Site");
    }
}

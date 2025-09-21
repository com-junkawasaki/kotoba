//! # Kotoba Language - Unified Language Processing
//!
//! すべての言語機能を統合的に提供するクレートです。
//! graphをプログラミング言語として扱うための統一APIを提供します。

use std::collections::HashMap;
use serde_json::Value;
use thiserror::Error;
use async_trait::async_trait;

/// Language processing errors
#[derive(Error, Debug)]
pub enum LanguageError {
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),
    #[error("Language not supported: {0}")]
    LanguageNotSupported(String),
    #[error("Processing failed: {0}")]
    ProcessingFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Language processing result
pub type Result<T> = std::result::Result<T, LanguageError>;

/// Supported language types
#[derive(Debug, Clone)]
pub enum LanguageType {
    /// Kotobas - HTTP設定言語
    Kotobas,
    /// Jsonnet - 設定言語
    Jsonnet,
    /// TypeScript変換
    TypeScript,
    /// フォーマッター
    Formatter,
    /// リンター
    Linter,
    /// REPL
    Repl,
    /// WASM
    Wasm,
}

/// Language processing configuration
#[derive(Debug, Clone)]
pub struct LanguageConfig {
    /// Enabled language features
    pub features: Vec<LanguageType>,
    /// Additional options
    pub options: HashMap<String, Value>,
}

impl Default for LanguageConfig {
    fn default() -> Self {
        Self {
            features: vec![
                LanguageType::Kotobas,
                LanguageType::Jsonnet,
                LanguageType::TypeScript,
                LanguageType::Formatter,
                LanguageType::Linter,
                LanguageType::Repl,
                LanguageType::Wasm,
            ],
            options: HashMap::new(),
        }
    }
}

/// Unified language processor trait
#[async_trait]
pub trait LanguageProcessor {
    /// Process language content
    async fn process(&self, language: LanguageType, content: &str) -> Result<String>;

    /// Format code
    async fn format(&self, language: LanguageType, content: &str) -> Result<String>;

    /// Lint code
    async fn lint(&self, language: LanguageType, content: &str) -> Result<Vec<String>>;

    /// Validate syntax
    async fn validate(&self, language: LanguageType, content: &str) -> Result<bool>;

    /// Get supported languages
    fn supported_languages(&self) -> Vec<LanguageType>;

    /// Get language configuration
    fn config(&self) -> &LanguageConfig;
}

/// Main language processor implementation
pub struct KotobaLanguageProcessor {
    config: LanguageConfig,
}

impl KotobaLanguageProcessor {
    /// Create new language processor
    pub fn new() -> Self {
        Self {
            config: LanguageConfig::default(),
        }
    }

    /// Create processor with custom config
    pub fn with_config(config: LanguageConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl LanguageProcessor for KotobaLanguageProcessor {
    async fn process(&self, language: LanguageType, content: &str) -> Result<String> {
        if !self.supported_languages().contains(&language) {
            return Err(LanguageError::LanguageNotSupported(format!("{:?}", language)));
        }

        match language {
            LanguageType::Kotobas => {
                // TODO: Integrate kotoba-kotobas functionality
                Err(LanguageError::FeatureNotEnabled("Kotobas".to_string()))
            },
            LanguageType::Jsonnet => {
                // TODO: Integrate kotoba-jsonnet functionality
                Err(LanguageError::FeatureNotEnabled("Jsonnet".to_string()))
            },
            LanguageType::TypeScript => {
                // TODO: Integrate kotoba2tsx functionality
                Err(LanguageError::FeatureNotEnabled("TypeScript".to_string()))
            },
            _ => Err(LanguageError::FeatureNotEnabled(format!("{:?}", language))),
        }
    }

    async fn format(&self, language: LanguageType, content: &str) -> Result<String> {
        if !self.supported_languages().contains(&language) {
            return Err(LanguageError::LanguageNotSupported(format!("{:?}", language)));
        }

        match language {
            LanguageType::Formatter => {
                // TODO: Integrate kotoba-formatter functionality
                Err(LanguageError::FeatureNotEnabled("Formatter".to_string()))
            },
            _ => Err(LanguageError::FeatureNotEnabled(format!("{:?}", language))),
        }
    }

    async fn lint(&self, language: LanguageType, content: &str) -> Result<Vec<String>> {
        if !self.supported_languages().contains(&language) {
            return Err(LanguageError::LanguageNotSupported(format!("{:?}", language)));
        }

        match language {
            #[cfg(feature = "linter")]
            LanguageType::Linter => {
                // Use linter for code analysis
                // This would call kotoba-linter functionality
                Ok(vec![])
            },
            _ => Err(LanguageError::FeatureNotEnabled(format!("{:?}", language))),
        }
    }

    async fn validate(&self, language: LanguageType, content: &str) -> Result<bool> {
        match self.lint(language, content).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn supported_languages(&self) -> Vec<LanguageType> {
        self.config.features.clone()
    }

    fn config(&self) -> &LanguageConfig {
        &self.config
    }
}

/// Prelude for convenient imports
pub mod prelude {
    pub use super::{LanguageProcessor, LanguageType, LanguageConfig, LanguageError};
    pub use super::KotobaLanguageProcessor;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = KotobaLanguageProcessor::new();
        assert_eq!(processor.supported_languages().len(), 7);
    }

    #[test]
    fn test_config_with_features() {
        let config = LanguageConfig {
            features: vec![LanguageType::Kotobas, LanguageType::Jsonnet],
            options: HashMap::new(),
        };
        let processor = KotobaLanguageProcessor::with_config(config);
        assert_eq!(processor.supported_languages().len(), 2);
    }
}

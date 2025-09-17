//! # KotobaDB Configuration Management
//!
//! Dynamic configuration management system for runtime configuration updates.
//! Provides configuration storage, validation, hot reloading, and change notifications.
//!
//! ## Features
//!
//! - **Dynamic Configuration**: Runtime configuration updates without restart
//! - **Multiple Formats**: JSON, YAML, TOML, and environment variables support
//! - **Configuration Validation**: Schema-based validation with custom rules
//! - **Hot Reloading**: Automatic configuration reloading on file changes
//! - **Change Notifications**: Event-driven configuration change handling
//! - **Hierarchical Config**: Environment-specific and layered configurations
//! - **Secure Storage**: Encrypted configuration storage options
//! - **Version Control**: Configuration versioning and rollback support
//!
//! ## Configuration Sources
//!
//! - **Files**: JSON, YAML, TOML configuration files
//! - **Environment Variables**: OS environment variables with prefix support
//! - **Database**: Persistent configuration storage in KotobaDB
//! - **Remote**: HTTP-based remote configuration services
//! - **Command Line**: Runtime configuration via CLI arguments

pub mod config_manager;
pub mod config_store;
pub mod config_validator;
pub mod hot_reload;

pub use config_manager::*;
pub use config_store::*;
pub use config_validator::*;
pub use hot_reload::*;

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Configuration metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMetadata {
    /// Configuration key
    pub key: String,
    /// Configuration value as JSON
    pub value: serde_json::Value,
    /// Configuration version
    pub version: u64,
    /// Last modified timestamp
    pub last_modified: DateTime<Utc>,
    /// Configuration source
    pub source: ConfigSource,
    /// Is configuration encrypted
    pub encrypted: bool,
    /// Configuration tags
    pub tags: HashMap<String, String>,
}

/// Configuration source types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigSource {
    /// File-based configuration
    File,
    /// Environment variables
    Environment,
    /// Database storage
    Database,
    /// Remote service
    Remote,
    /// Command line arguments
    Cli,
    /// Runtime update
    Runtime,
}

/// Configuration update event
#[derive(Debug, Clone)]
pub struct ConfigUpdateEvent {
    /// Configuration key
    pub key: String,
    /// Old value (if any)
    pub old_value: Option<serde_json::Value>,
    /// New value
    pub new_value: serde_json::Value,
    /// Update source
    pub source: ConfigSource,
    /// Update timestamp
    pub timestamp: DateTime<Utc>,
}

/// Configuration validation rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    /// Rule name
    pub name: String,
    /// Rule description
    pub description: String,
    /// JSON schema for validation
    pub schema: serde_json::Value,
    /// Is rule enabled
    pub enabled: bool,
}

/// Configuration change listener
#[async_trait::async_trait]
pub trait ConfigChangeListener: Send + Sync {
    /// Handle configuration change
    async fn on_config_change(&self, event: ConfigUpdateEvent) -> Result<(), ConfigError>;
}

/// Configuration format types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Json,
    Yaml,
    Toml,
}

/// Configuration manager settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSettings {
    /// Enable hot reloading
    pub enable_hot_reload: bool,
    /// Configuration file paths to watch
    pub watch_paths: Vec<String>,
    /// Environment variable prefix
    pub env_prefix: Option<String>,
    /// Enable validation
    pub enable_validation: bool,
    /// Validation rules
    pub validation_rules: Vec<ValidationRule>,
    /// Configuration cache TTL
    pub cache_ttl_seconds: u64,
    /// Enable encryption for sensitive configs
    pub enable_encryption: bool,
    /// Encryption key (if encryption enabled)
    pub encryption_key: Option<String>,
}

impl Default for ConfigSettings {
    fn default() -> Self {
        Self {
            enable_hot_reload: true,
            watch_paths: vec!["config".to_string()],
            env_prefix: Some("KOTOBA_".to_string()),
            enable_validation: true,
            validation_rules: Vec::new(),
            cache_ttl_seconds: 300, // 5 minutes
            enable_encryption: false,
            encryption_key: None,
        }
    }
}

/// Configuration snapshot for atomic updates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// Snapshot ID
    pub id: String,
    /// Configuration data
    pub data: HashMap<String, ConfigMetadata>,
    /// Snapshot timestamp
    pub timestamp: DateTime<Utc>,
    /// Snapshot description
    pub description: String,
}

/// Configuration errors
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Configuration not found: {0}")]
    NotFound(String),

    #[error("Configuration validation failed: {0}")]
    Validation(String),

    #[error("Configuration parse error: {0}")]
    Parse(String),

    #[error("Configuration I/O error: {0}")]
    Io(String),

    #[error("Configuration encryption error: {0}")]
    Encryption(String),

    #[error("Configuration storage error: {0}")]
    Storage(String),

    #[error("Configuration reload error: {0}")]
    Reload(String),

    #[error("Configuration format error: {0}")]
    Format(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_metadata_creation() {
        let metadata = ConfigMetadata {
            key: "database.url".to_string(),
            value: serde_json::Value::String("postgresql://localhost".to_string()),
            version: 1,
            last_modified: Utc::now(),
            source: ConfigSource::File,
            encrypted: false,
            tags: HashMap::new(),
        };

        assert_eq!(metadata.key, "database.url");
        assert_eq!(metadata.version, 1);
        assert!(!metadata.encrypted);
    }

    #[test]
    fn test_config_settings_default() {
        let settings = ConfigSettings::default();
        assert!(settings.enable_hot_reload);
        assert!(settings.enable_validation);
        assert_eq!(settings.cache_ttl_seconds, 300);
    }

    #[test]
    fn test_config_source_ordering() {
        assert!(ConfigSource::File < ConfigSource::Environment);
        assert!(ConfigSource::Environment < ConfigSource::Database);
    }
}

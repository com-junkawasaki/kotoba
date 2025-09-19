//! # Configuration Manager
//!
//! Central configuration management system with support for multiple sources,
//! validation, hot reloading, and change notifications.

use crate::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};
use chrono::{DateTime, Utc};

/// Configuration manager for centralized config management
pub struct ConfigManager {
    /// Configuration settings
    settings: ConfigSettings,
    /// Configuration store
    store: Arc<dyn ConfigStore>,
    /// Configuration validator
    validator: Arc<dyn ConfigValidator>,
    /// Hot reload manager
    hot_reload: Option<Arc<HotReloadManager>>,
    /// Configuration cache
    cache: Arc<RwLock<HashMap<String, CachedConfig>>>,
    /// Change listeners
    listeners: Arc<RwLock<Vec<Box<dyn ConfigChangeListener>>>>,
    /// Update event channel
    update_tx: mpsc::Sender<ConfigUpdateEvent>,
    update_rx: mpsc::Receiver<ConfigUpdateEvent>,
}

impl ConfigManager {
    /// Create a new configuration manager
    pub async fn new(settings: ConfigSettings, store: Arc<dyn ConfigStore>, validator: Arc<dyn ConfigValidator>) -> Result<Self, ConfigError> {
        let (update_tx, update_rx) = mpsc::channel(100);

        let mut manager = Self {
            settings,
            store,
            validator,
            hot_reload: None,
            cache: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
            update_tx,
            update_rx,
        };

        // Initialize hot reload if enabled
        if manager.settings.enable_hot_reload {
            let hot_reload = Arc::new(HotReloadManager::new(manager.settings.clone(), manager.store.clone()));
            manager.hot_reload = Some(hot_reload);
        }

        // Load initial configuration
        manager.load_initial_config().await?;

        Ok(manager)
    }

    /// Start the configuration manager
    pub async fn start(&mut self) -> Result<(), ConfigError> {
        // Start hot reload monitoring
        if let Some(hot_reload) = &self.hot_reload {
            hot_reload.start().await?;
        }

        // Start update event processing
        self.start_update_processing().await?;

        Ok(())
    }

    /// Stop the configuration manager
    pub async fn stop(&self) -> Result<(), ConfigError> {
        if let Some(hot_reload) = &self.hot_reload {
            hot_reload.stop().await?;
        }
        Ok(())
    }

    /// Get configuration value
    pub async fn get<T: for<'de> serde::Deserialize<'de>>(&self, key: &str) -> Result<Option<T>, ConfigError> {
        // Check cache first
        if let Some(cached) = self.get_cached(key).await {
            if !cached.is_expired() {
                return Ok(Some(cached.value.clone()));
            }
        }

        // Load from store
        match self.store.get(key).await? {
            Some(metadata) => {
                // Validate if enabled
                if self.settings.enable_validation {
                    self.validator.validate(key, &metadata.value).await?;
                }

                // Cache the value
                let cached = CachedConfig::new(metadata.value.clone());
                self.cache.write().await.insert(key.to_string(), cached);

                // Deserialize and return
                let value: T = serde_json::from_value(metadata.value)
                    .map_err(|e| ConfigError::Parse(format!("Failed to deserialize {}: {}", key, e)))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Set configuration value
    pub async fn set<T: serde::Serialize>(&self, key: &str, value: T, source: ConfigSource) -> Result<(), ConfigError> {
        let json_value = serde_json::to_value(&value)
            .map_err(|e| ConfigError::Parse(format!("Failed to serialize value: {}", e)))?;

        // Validate if enabled
        if self.settings.enable_validation {
            self.validator.validate(key, &json_value).await?;
        }

        // Create metadata
        let metadata = ConfigMetadata {
            key: key.to_string(),
            value: json_value.clone(),
            version: 1, // Simplified versioning
            last_modified: Utc::now(),
            source,
            encrypted: false, // TODO: Implement encryption
            tags: HashMap::new(),
        };

        // Store configuration
        self.store.set(&metadata).await?;

        // Update cache
        let cached = CachedConfig::new(json_value.clone());
        self.cache.write().await.insert(key.to_string(), cached);

        // Notify listeners
        let event = ConfigUpdateEvent {
            key: key.to_string(),
            old_value: None, // TODO: Track old values
            new_value: json_value,
            source,
            timestamp: Utc::now(),
        };

        self.notify_listeners(event).await?;

        Ok(())
    }

    /// Delete configuration value
    pub async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        let deleted = self.store.delete(key).await?;

        if deleted {
            // Remove from cache
            self.cache.write().await.remove(key);

            // Notify listeners
            let event = ConfigUpdateEvent {
                key: key.to_string(),
                old_value: None,
                new_value: serde_json::Value::Null,
                source: ConfigSource::Runtime,
                timestamp: Utc::now(),
            };

            self.notify_listeners(event).await?;
        }

        Ok(deleted)
    }

    /// List all configuration keys
    pub async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        self.store.list_keys(prefix).await
    }

    /// Get configuration metadata
    pub async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        self.store.get_metadata(key).await
    }

    /// Add configuration change listener
    pub async fn add_listener(&self, listener: Box<dyn ConfigChangeListener>) -> Result<(), ConfigError> {
        self.listeners.write().await.push(listener);
        Ok(())
    }

    /// Create configuration snapshot
    pub async fn create_snapshot(&self, description: &str) -> Result<ConfigSnapshot, ConfigError> {
        let keys = self.store.list_keys(None).await?;
        let mut data = HashMap::new();

        for key in keys {
            if let Some(metadata) = self.store.get_metadata(&key).await? {
                data.insert(key, metadata);
            }
        }

        let snapshot = ConfigSnapshot {
            id: uuid::Uuid::new_v4().to_string(),
            data,
            timestamp: Utc::now(),
            description: description.to_string(),
        };

        Ok(snapshot)
    }

    /// Load configuration from file
    pub async fn load_from_file(&self, path: &PathBuf, format: ConfigFormat) -> Result<(), ConfigError> {
        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| ConfigError::Io(format!("Failed to read config file: {}", e)))?;

        let json_value: serde_json::Value = match format {
            ConfigFormat::Json => serde_json::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid JSON: {}", e)))?,
            ConfigFormat::Yaml => serde_yaml::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid YAML: {}", e)))?,
            ConfigFormat::Toml => toml::from_str(&content)
                .map_err(|e| ConfigError::Parse(format!("Invalid TOML: {}", e)))?,
        };

        // Flatten nested structure into key-value pairs
        let flat_config = flatten_config(json_value);

        for (key, value) in flat_config {
            self.set(&key, value, ConfigSource::File).await?;
        }

        Ok(())
    }

    /// Load configuration from environment variables
    pub async fn load_from_env(&self) -> Result<(), ConfigError> {
        if let Some(prefix) = &self.settings.env_prefix {
            for (key, value) in std::env::vars() {
                if key.starts_with(prefix) {
                    let config_key = key[prefix.len()..].to_lowercase().replace('_', ".");
                    self.set(&config_key, value, ConfigSource::Environment).await?;
                }
            }
        }

        Ok(())
    }

    // Internal methods

    async fn load_initial_config(&self) -> Result<(), ConfigError> {
        // Load from environment variables
        self.load_from_env().await?;

        // Load from files in watch paths
        for path_str in &self.settings.watch_paths {
            let path = PathBuf::from(path_str);

            // Try different formats
            let formats = [
                (path.with_extension("json"), ConfigFormat::Json),
                (path.with_extension("yaml"), ConfigFormat::Yaml),
                (path.with_extension("yml"), ConfigFormat::Yaml),
                (path.with_extension("toml"), ConfigFormat::Toml),
            ];

            for (file_path, format) in &formats {
                if file_path.exists() {
                    self.load_from_file(file_path, *format).await?;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn get_cached(&self, key: &str) -> Option<CachedConfig> {
        self.cache.read().await.get(key).cloned()
    }

    async fn notify_listeners(&self, event: ConfigUpdateEvent) -> Result<(), ConfigError> {
        let listeners = self.listeners.read().await.clone();

        for listener in listeners {
            if let Err(e) = listener.on_config_change(event.clone()).await {
                eprintln!("Config listener error: {:?}", e);
            }
        }

        Ok(())
    }

    async fn start_update_processing(&mut self) -> Result<(), ConfigError> {
        let listeners = Arc::clone(&self.listeners);
        let mut rx = self.update_rx;

        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                let listeners = listeners.read().await.clone();

                for listener in listeners {
                    if let Err(e) = listener.on_config_change(event.clone()).await {
                        eprintln!("Config update listener error: {:?}", e);
                    }
                }
            }
        });

        Ok(())
    }
}

/// Cached configuration value
#[derive(Debug, Clone)]
struct CachedConfig {
    value: serde_json::Value,
    cached_at: DateTime<Utc>,
    ttl_seconds: u64,
}

impl CachedConfig {
    fn new(value: serde_json::Value) -> Self {
        Self {
            value,
            cached_at: Utc::now(),
            ttl_seconds: 300, // 5 minutes default
        }
    }

    fn is_expired(&self) -> bool {
        let elapsed = Utc::now().signed_duration_since(self.cached_at);
        elapsed.num_seconds() as u64 >= self.ttl_seconds
    }
}

/// Flatten nested JSON configuration into dot-separated keys
fn flatten_config(value: serde_json::Value) -> HashMap<String, serde_json::Value> {
    let mut result = HashMap::new();
    flatten_config_recursive(value, String::new(), &mut result);
    result
}

fn flatten_config_recursive(value: serde_json::Value, prefix: String, result: &mut HashMap<String, serde_json::Value>) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, value) in map {
                let new_prefix = if prefix.is_empty() {
                    key
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_config_recursive(value, new_prefix, result);
            }
        }
        _ => {
            result.insert(prefix, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    struct MockConfigStore {
        data: Arc<Mutex<HashMap<String, ConfigMetadata>>>,
    }

    impl MockConfigStore {
        fn new() -> Self {
            Self {
                data: Arc::new(Mutex::new(HashMap::new())),
            }
        }
    }

    #[async_trait::async_trait]
    impl ConfigStore for MockConfigStore {
        async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
            Ok(self.data.lock().await.get(key).cloned())
        }

        async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
            self.data.lock().await.insert(metadata.key.clone(), metadata.clone());
            Ok(())
        }

        async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
            Ok(self.data.lock().await.remove(key).is_some())
        }

        async fn list_keys(&self, _prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
            Ok(self.data.lock().await.keys().cloned().collect())
        }

        async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
            Ok(self.data.lock().await.get(key).cloned())
        }
    }

    struct MockConfigValidator;

    #[async_trait::async_trait]
    impl ConfigValidator for MockConfigValidator {
        async fn validate(&self, _key: &str, _value: &serde_json::Value) -> Result<(), ConfigError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn test_config_manager_creation() {
        let store = Arc::new(MockConfigStore::new());
        let validator = Arc::new(MockConfigValidator);
        let settings = ConfigSettings::default();

        let manager = ConfigManager::new(settings, store, validator).await;
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_config_set_and_get() {
        let store = Arc::new(MockConfigStore::new());
        let validator = Arc::new(MockConfigValidator);
        let settings = ConfigSettings::default();

        let manager = ConfigManager::new(settings, store, validator).await.unwrap();

        // Set configuration
        manager.set("database.url", "postgresql://localhost", ConfigSource::Runtime).await.unwrap();

        // Get configuration
        let value: Option<String> = manager.get("database.url").await.unwrap();
        assert_eq!(value, Some("postgresql://localhost".to_string()));
    }

    #[test]
    fn test_flatten_config() {
        let json: serde_json::Value = serde_json::json!({
            "database": {
                "host": "localhost",
                "port": 5432
            },
            "cache": {
                "enabled": true
            }
        });

        let flattened = flatten_config(json);
        assert_eq!(flattened.get("database.host"), Some(&serde_json::json!("localhost")));
        assert_eq!(flattened.get("database.port"), Some(&serde_json::json!(5432)));
        assert_eq!(flattened.get("cache.enabled"), Some(&serde_json::json!(true)));
    }
}

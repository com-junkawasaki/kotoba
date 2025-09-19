//! # Configuration Store
//!
//! Persistent storage backend for configuration data with support for
//! multiple storage engines and data sources.

use crate::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Configuration store trait for different storage backends
#[async_trait::async_trait]
pub trait ConfigStore: Send + Sync {
    /// Get configuration value
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError>;

    /// Set configuration value
    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError>;

    /// Delete configuration value
    async fn delete(&self, key: &str) -> Result<bool, ConfigError>;

    /// List all configuration keys with optional prefix
    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError>;

    /// Get configuration metadata
    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError>;

    /// Get all configurations
    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError>;

    /// Clear all configurations
    async fn clear(&self) -> Result<(), ConfigError>;
}

/// In-memory configuration store for testing and development
pub struct MemoryConfigStore {
    data: Arc<parking_lot::RwLock<HashMap<String, ConfigMetadata>>>,
}

impl MemoryConfigStore {
    pub fn new() -> Self {
        Self {
            data: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn with_initial_data(data: HashMap<String, ConfigMetadata>) -> Self {
        Self {
            data: Arc::new(parking_lot::RwLock::new(data)),
        }
    }
}

#[async_trait::async_trait]
impl ConfigStore for MemoryConfigStore {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        self.data.write().insert(metadata.key.clone(), metadata.clone());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        Ok(self.data.write().remove(key).is_some())
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        let keys: Vec<String> = self.data.read().keys()
            .filter(|key| prefix.map_or(true, |p| key.starts_with(p)))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError> {
        Ok(self.data.read().clone())
    }

    async fn clear(&self) -> Result<(), ConfigError> {
        self.data.write().clear();
        Ok(())
    }
}

/// File-based configuration store
pub struct FileConfigStore {
    file_path: PathBuf,
    data: Arc<parking_lot::RwLock<HashMap<String, ConfigMetadata>>>,
}

impl FileConfigStore {
    pub fn new(file_path: PathBuf) -> Self {
        let data = Self::load_from_file(&file_path);
        Self {
            file_path,
            data: Arc::new(parking_lot::RwLock::new(data)),
        }
    }

    fn load_from_file(file_path: &PathBuf) -> HashMap<String, ConfigMetadata> {
        if !file_path.exists() {
            return HashMap::new();
        }

        match std::fs::read_to_string(file_path) {
            Ok(content) => {
                match serde_json::from_str::<HashMap<String, ConfigMetadata>>(&content) {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Failed to parse config file {}: {}", file_path.display(), e);
                        HashMap::new()
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read config file {}: {}", file_path.display(), e);
                HashMap::new()
            }
        }
    }

    fn save_to_file(&self) -> Result<(), ConfigError> {
        let data = self.data.read();
        let content = serde_json::to_string_pretty(&*data)
            .map_err(|e| ConfigError::Format(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&self.file_path, content)
            .map_err(|e| ConfigError::Io(format!("Failed to write config file: {}", e)))?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl ConfigStore for FileConfigStore {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        self.data.write().insert(metadata.key.clone(), metadata.clone());
        self.save_to_file()?;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        let removed = self.data.write().remove(key).is_some();
        if removed {
            self.save_to_file()?;
        }
        Ok(removed)
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        let keys: Vec<String> = self.data.read().keys()
            .filter(|key| prefix.map_or(true, |p| key.starts_with(p)))
            .cloned()
            .collect();
        Ok(keys)
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        Ok(self.data.read().get(key).cloned())
    }

    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError> {
        Ok(self.data.read().clone())
    }

    async fn clear(&self) -> Result<(), ConfigError> {
        self.data.write().clear();
        self.save_to_file()?;
        Ok(())
    }
}

/// Database-backed configuration store using KotobaDB
pub struct DatabaseConfigStore {
    db: Arc<dyn DatabaseConfigInterface>,
}

impl DatabaseConfigStore {
    pub fn new(db: Arc<dyn DatabaseConfigInterface>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl ConfigStore for DatabaseConfigStore {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        self.db.get_config(key).await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        self.db.set_config(metadata).await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        self.db.delete_config(key).await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        self.db.list_config_keys(prefix).await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        self.db.get_config_metadata(key).await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError> {
        self.db.get_all_configs().await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }

    async fn clear(&self) -> Result<(), ConfigError> {
        self.db.clear_all_configs().await
            .map_err(|e| ConfigError::Storage(format!("Database error: {}", e)))
    }
}

/// Database interface for configuration storage
#[async_trait::async_trait]
pub trait DatabaseConfigInterface: Send + Sync {
    /// Get configuration by key
    async fn get_config(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError>;

    /// Set configuration
    async fn set_config(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError>;

    /// Delete configuration
    async fn delete_config(&self, key: &str) -> Result<bool, ConfigError>;

    /// List configuration keys
    async fn list_config_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError>;

    /// Get configuration metadata
    async fn get_config_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError>;

    /// Get all configurations
    async fn get_all_configs(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError>;

    /// Clear all configurations
    async fn clear_all_configs(&self) -> Result<(), ConfigError>;
}

/// Layered configuration store that combines multiple sources
pub struct LayeredConfigStore {
    layers: Vec<Arc<dyn ConfigStore>>,
}

impl LayeredConfigStore {
    pub fn new(layers: Vec<Arc<dyn ConfigStore>>) -> Self {
        Self { layers }
    }

    pub fn add_layer(&mut self, store: Arc<dyn ConfigStore>) {
        self.layers.push(store);
    }
}

#[async_trait::async_trait]
impl ConfigStore for LayeredConfigStore {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        // Check layers in order (last added has highest priority)
        for layer in self.layers.iter().rev() {
            if let Some(metadata) = layer.get(key).await? {
                return Ok(Some(metadata));
            }
        }
        Ok(None)
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        // Set in the first (highest priority) layer that supports writes
        if let Some(layer) = self.layers.last() {
            layer.set(metadata).await?;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        // Delete from all layers
        let mut deleted = false;
        for layer in &self.layers {
            if layer.delete(key).await? {
                deleted = true;
            }
        }
        Ok(deleted)
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        let mut all_keys = HashMap::new();

        // Collect keys from all layers
        for layer in &self.layers {
            for key in layer.list_keys(prefix).await? {
                all_keys.insert(key, ());
            }
        }

        Ok(all_keys.into_keys().collect())
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        self.get(key).await
    }

    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError> {
        let mut all_configs = HashMap::new();

        // Collect configs from all layers (higher layers override lower ones)
        for layer in &self.layers {
            for (key, metadata) in layer.get_all().await? {
                all_configs.insert(key, metadata);
            }
        }

        Ok(all_configs)
    }

    async fn clear(&self) -> Result<(), ConfigError> {
        // Clear all layers
        for layer in &self.layers {
            layer.clear().await?;
        }
        Ok(())
    }
}

/// Encrypted configuration store wrapper
pub struct EncryptedConfigStore<S: ConfigStore> {
    inner: S,
    encryption_key: Vec<u8>,
}

impl<S: ConfigStore> EncryptedConfigStore<S> {
    pub fn new(inner: S, encryption_key: &[u8]) -> Self {
        Self {
            inner,
            encryption_key: encryption_key.to_vec(),
        }
    }

    fn encrypt(&self, data: &[u8]) -> Result<Vec<u8>, ConfigError> {
        // Simple XOR encryption for demonstration
        // In production, use proper encryption like AES
        let mut encrypted = Vec::with_capacity(data.len());
        for (i, &byte) in data.iter().enumerate() {
            let key_byte = self.encryption_key[i % self.encryption_key.len()];
            encrypted.push(byte ^ key_byte);
        }
        Ok(encrypted)
    }

    fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, ConfigError> {
        // Simple XOR decryption
        self.encrypt(data) // XOR is symmetric
    }
}

#[async_trait::async_trait]
impl<S: ConfigStore> ConfigStore for EncryptedConfigStore<S> {
    async fn get(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        match self.inner.get(key).await? {
            Some(mut metadata) => {
                if metadata.encrypted {
                    // Decrypt the value
                    let encrypted_bytes = serde_json::to_vec(&metadata.value)
                        .map_err(|e| ConfigError::Encryption(format!("Serialize error: {}", e)))?;

                    let decrypted_bytes = self.decrypt(&encrypted_bytes)?;
                    metadata.value = serde_json::from_slice(&decrypted_bytes)
                        .map_err(|e| ConfigError::Encryption(format!("Deserialize error: {}", e)))?;
                }
                Ok(Some(metadata))
            }
            None => Ok(None),
        }
    }

    async fn set(&self, metadata: &ConfigMetadata) -> Result<(), ConfigError> {
        let mut encrypted_metadata = metadata.clone();

        if self.encryption_key.is_empty() {
            // No encryption
            self.inner.set(metadata).await?;
        } else {
            // Encrypt the value
            let json_bytes = serde_json::to_vec(&metadata.value)
                .map_err(|e| ConfigError::Encryption(format!("Serialize error: {}", e)))?;

            let encrypted_bytes = self.encrypt(&json_bytes)?;
            encrypted_metadata.value = serde_json::Value::String(base64::encode(&encrypted_bytes));
            encrypted_metadata.encrypted = true;

            self.inner.set(&encrypted_metadata).await?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, ConfigError> {
        self.inner.delete(key).await
    }

    async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>, ConfigError> {
        self.inner.list_keys(prefix).await
    }

    async fn get_metadata(&self, key: &str) -> Result<Option<ConfigMetadata>, ConfigError> {
        self.inner.get_metadata(key).await
    }

    async fn get_all(&self) -> Result<HashMap<String, ConfigMetadata>, ConfigError> {
        self.inner.get_all().await
    }

    async fn clear(&self) -> Result<(), ConfigError> {
        self.inner.clear().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_memory_config_store() {
        let store = MemoryConfigStore::new();

        let metadata = ConfigMetadata {
            key: "test.key".to_string(),
            value: serde_json::Value::String("test_value".to_string()),
            version: 1,
            last_modified: Utc::now(),
            source: ConfigSource::Runtime,
            encrypted: false,
            tags: HashMap::new(),
        };

        // Test set and get
        store.set(&metadata).await.unwrap();
        let retrieved = store.get("test.key").await.unwrap();
        assert_eq!(retrieved.unwrap().value, metadata.value);

        // Test list keys
        let keys = store.list_keys(None).await.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0], "test.key");

        // Test delete
        assert!(store.delete("test.key").await.unwrap());
        assert!(!store.delete("test.key").await.unwrap());
    }

    #[tokio::test]
    async fn test_file_config_store() {
        let temp_file = NamedTempFile::new().unwrap();
        let store = FileConfigStore::new(temp_file.path().to_path_buf());

        let metadata = ConfigMetadata {
            key: "file.key".to_string(),
            value: serde_json::Value::String("file_value".to_string()),
            version: 1,
            last_modified: Utc::now(),
            source: ConfigSource::File,
            encrypted: false,
            tags: HashMap::new(),
        };

        // Test set and get
        store.set(&metadata).await.unwrap();
        let retrieved = store.get("file.key").await.unwrap();
        assert_eq!(retrieved.unwrap().value, metadata.value);

        // Test persistence by creating new store instance
        let store2 = FileConfigStore::new(temp_file.path().to_path_buf());
        let retrieved2 = store2.get("file.key").await.unwrap();
        assert_eq!(retrieved2.unwrap().value, metadata.value);
    }

    #[tokio::test]
    async fn test_layered_config_store() {
        let store1 = Arc::new(MemoryConfigStore::new());
        let store2 = Arc::new(MemoryConfigStore::new());

        // Add configs to different layers
        let metadata1 = ConfigMetadata {
            key: "shared.key".to_string(),
            value: serde_json::Value::String("value1".to_string()),
            version: 1,
            last_modified: Utc::now(),
            source: ConfigSource::File,
            encrypted: false,
            tags: HashMap::new(),
        };

        let metadata2 = ConfigMetadata {
            key: "shared.key".to_string(),
            value: serde_json::Value::String("value2".to_string()),
            version: 1,
            last_modified: Utc::now(),
            source: ConfigSource::Environment,
            encrypted: false,
            tags: HashMap::new(),
        };

        store1.set(&metadata1).await.unwrap();
        store2.set(&metadata2).await.unwrap();

        let layered = LayeredConfigStore::new(vec![store1, store2]);

        // Should get value from higher priority layer (store2)
        let retrieved = layered.get("shared.key").await.unwrap();
        assert_eq!(retrieved.unwrap().value, metadata2.value);
    }
}

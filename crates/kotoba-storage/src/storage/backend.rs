//! Storage Backend Abstraction Layer
//!
//! This module provides a unified interface for different storage backends
//! (RocksDB, Redis, etc.) allowing the application to switch between them
//! based on configuration.

use std::collections::HashMap;

use kotoba_core::types::{Result, Value};
use kotoba_errors::KotobaError;

pub trait StorageBackend: Send + Sync {
    fn get(&self, key: &str) -> Result<Option<Value>>;
    fn set(&mut self, key: &str, value: Value) -> Result<()>;
    fn delete(&mut self, key: &str) -> Result<()>;
    fn list(&self) -> Result<Vec<String>>;
}

/// Statistics about the storage backend
#[derive(Debug, Clone)]
pub struct BackendStats {
    pub backend_type: String,
    pub total_keys: Option<u64>,
    pub memory_usage: Option<u64>,
    pub disk_usage: Option<u64>,
    pub connection_count: Option<u32>,
}

/// Factory for creating storage backends
pub struct StorageBackendFactory;

impl StorageBackendFactory {
    /// Create a storage backend based on configuration
    pub async fn create(config: &super::StorageConfig) -> Result<Box<dyn StorageBackend>> {
        match config.backend_type {
            #[cfg(feature = "rocksdb")]
            super::BackendType::RocksDB => {
                let rocksdb_backend = super::lsm::RocksDBBackend::new(config).await?;
                Ok(Box::new(rocksdb_backend))
            }
            #[cfg(not(feature = "rocksdb"))]
            super::BackendType::RocksDB => {
                Err(kotoba_core::types::KotobaError::Storage(
                    "RocksDB backend not available - compile with 'rocksdb' feature".to_string()
                ))
            }
            super::BackendType::Redis => {
                let redis_backend = super::redis::RedisBackend::new(config).await?;
                Ok(Box::new(redis_backend))
            }
        }
    }
}

/// High-level storage manager that provides a unified interface
/// and handles backend selection and management
pub struct StorageManager {
    backend: Box<dyn StorageBackend>,
    config: super::StorageConfig,
}

impl StorageManager {
    /// Create a new storage manager with the specified configuration
    pub async fn new(config: super::StorageConfig) -> Result<Self> {
        let backend = StorageBackendFactory::create(&config).await?;
        Ok(Self { backend, config })
    }

    /// Create a storage manager with default configuration (RocksDB)
    pub async fn default() -> Result<Self> {
        Self::new(super::StorageConfig::default()).await
    }

    /// Get the current backend type
    pub fn backend_type(&self) -> &super::BackendType {
        &self.config.backend_type
    }

    /// Get the current configuration
    pub fn config(&self) -> &super::StorageConfig {
        &self.config
    }

    /// Store a key-value pair
    pub async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        self.backend.put(key, value).await
    }

    /// Retrieve a value by key
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        self.backend.get(key).await
    }

    /// Delete a key-value pair
    pub async fn delete(&self, key: String) -> Result<()> {
        self.backend.delete(key).await
    }

    /// Check if a key exists
    pub async fn exists(&self, key: &str) -> Result<bool> {
        self.backend.exists(key).await
    }

    /// Get all keys with a prefix
    pub async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        self.backend.get_keys_with_prefix(prefix).await
    }

    /// Clear all data (for testing purposes)
    pub async fn clear(&self) -> Result<()> {
        self.backend.clear().await
    }

    /// Get backend statistics
    pub async fn stats(&self) -> Result<BackendStats> {
        self.backend.stats().await
    }

    /// Create a storage manager configured for Upstash Redis
    pub async fn with_upstash(redis_url: String) -> Result<Self> {
        let config = super::StorageConfig {
            backend_type: super::BackendType::Redis,
            redis_url: Some(redis_url),
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Create a storage manager configured for local RocksDB
    pub async fn with_rocksdb(data_dir: std::path::PathBuf) -> Result<Self> {
        let config = super::StorageConfig {
            backend_type: super::BackendType::RocksDB,
            rocksdb_path: Some(data_dir),
            ..Default::default()
        };
        Self::new(config).await
    }
}

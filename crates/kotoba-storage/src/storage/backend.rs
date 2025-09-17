//! Abstract storage backend trait and factory.
use async_trait::async_trait;
use kotoba_core::types::Result;
use kotoba_errors::KotobaError;

/// Storage backend types
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    RocksDB,
    Redis,
    ObjectStorage,
}

/// Object storage provider types
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectStorageProvider {
    AWS,
    GCP,
    Azure,
    Local, // MinIO, LocalStack, etc.
}

/// Configuration for storage backends
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend_type: BackendType,
    pub rocksdb_path: Option<std::path::PathBuf>,
    pub redis_url: Option<String>,
    pub object_storage_provider: Option<ObjectStorageProvider>,
    pub object_storage_bucket: Option<String>,
    pub object_storage_region: Option<String>,
    pub object_storage_access_key_id: Option<String>,
    pub object_storage_secret_access_key: Option<String>,
    pub object_storage_service_account_key: Option<String>,
    pub object_storage_client_id: Option<String>,
    pub object_storage_client_secret: Option<String>,
    pub object_storage_tenant_id: Option<String>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::RocksDB,
            rocksdb_path: Some(std::path::PathBuf::from("./data")),
            redis_url: Some("redis://localhost:6379".to_string()),
            object_storage_provider: None,
            object_storage_bucket: None,
            object_storage_region: None,
            object_storage_access_key_id: None,
            object_storage_secret_access_key: None,
            object_storage_service_account_key: None,
            object_storage_client_id: None,
            object_storage_client_secret: None,
            object_storage_tenant_id: None,
        }
    }
}


/// Abstract storage backend trait that all storage implementations must satisfy
#[async_trait]
pub trait StorageBackend: Send + Sync {
    /// Store a key-value pair
    async fn put(&self, key: String, value: Vec<u8>) -> Result<()>;

    /// Retrieve a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a key-value pair
    async fn delete(&self, key: String) -> Result<()>;

    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool> {
        Ok(self.get(key).await?.is_some())
    }

    /// Get all keys with a prefix (for scanning operations)
    async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>>;

    /// Clear all data (for testing purposes)
    async fn clear(&self) -> Result<()>;

    /// Get backend statistics
    async fn stats(&self) -> Result<BackendStats>;
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
    pub async fn create(config: &StorageConfig) -> Result<Box<dyn StorageBackend>> {
        match config.backend_type {
            #[cfg(feature = "rocksdb")]
            BackendType::RocksDB => {
                let rocksdb_backend = super::lsm::RocksDBBackend::new(config).await?;
                Ok(Box::new(rocksdb_backend))
            }
            #[cfg(not(feature = "rocksdb"))]
            BackendType::RocksDB => {
                Err(KotobaError::Storage(
                    "RocksDB backend not available - compile with 'rocksdb' feature".to_string()
                ))
            }
            BackendType::Redis => {
                let redis_backend = super::redis::RedisBackend::new(config).await?;
                Ok(Box::new(redis_backend))
            }
            #[cfg(feature = "object_storage")]
            BackendType::ObjectStorage => {
                let object_storage_backend = super::object::ObjectStorageBackend::new(config).await?;
                Ok(Box::new(object_storage_backend))
            }
            #[cfg(not(feature = "object_storage"))]
            BackendType::ObjectStorage => {
                Err(KotobaError::Storage(
                    "Object storage backend not available - compile with 'object_storage' feature".to_string()
                ))
            }
        }
    }
}

/// High-level storage manager that provides a unified interface
/// and handles backend selection and management
pub struct StorageManager {
    backend: Box<dyn StorageBackend>,
    config: StorageConfig,
}

impl StorageManager {
    /// Create a new storage manager with the specified configuration
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let backend = StorageBackendFactory::create(&config).await?;
        Ok(Self { backend, config })
    }

    /// Create a storage manager with default configuration (RocksDB)
    pub async fn default() -> Result<Self> {
        Self::new(StorageConfig::default()).await
    }

    /// Get the current backend type
    pub fn backend_type(&self) -> &BackendType {
        &self.config.backend_type
    }

    /// Get the current configuration
    pub fn config(&self) -> &StorageConfig {
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
        let config = StorageConfig {
            backend_type: BackendType::Redis,
            redis_url: Some(redis_url),
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Create a storage manager configured for local RocksDB
    pub async fn with_rocksdb(data_dir: std::path::PathBuf) -> Result<Self> {
        let config = StorageConfig {
            backend_type: BackendType::RocksDB,
            rocksdb_path: Some(data_dir),
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Create a storage manager configured for AWS S3
    pub async fn with_s3(bucket: String, region: Option<String>) -> Result<Self> {
        let config = StorageConfig {
            backend_type: BackendType::ObjectStorage,
            object_storage_provider: Some(ObjectStorageProvider::AWS),
            object_storage_bucket: Some(bucket),
            object_storage_region: region,
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Create a storage manager configured for Google Cloud Storage
    pub async fn with_gcs(bucket: String) -> Result<Self> {
        let config = StorageConfig {
            backend_type: BackendType::ObjectStorage,
            object_storage_provider: Some(ObjectStorageProvider::GCP),
            object_storage_bucket: Some(bucket),
            ..Default::default()
        };
        Self::new(config).await
    }

    /// Create a storage manager configured for Azure Blob Storage
    pub async fn with_azure(container: String, account_name: String) -> Result<Self> {
        let config = StorageConfig {
            backend_type: BackendType::ObjectStorage,
            object_storage_provider: Some(ObjectStorageProvider::Azure),
            object_storage_bucket: Some(container),
            object_storage_access_key_id: Some(account_name),
            ..Default::default()
        };
        Self::new(config).await
    }
}

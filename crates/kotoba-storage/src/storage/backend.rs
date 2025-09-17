//! Abstract storage backend trait and factory.
use async_trait::async_trait;
use kotoba_core::types::Result;
use kotoba_errors::KotobaError;

/// KotobaDB backend implementation
#[cfg(feature = "kotoba_db")]
mod kotoba_db_backend {
    use super::*;
    use kotoba_db::DB;

    pub struct KotobaDBBackend {
        db: DB,
    }

    impl KotobaDBBackend {
        pub async fn new(path: &std::path::Path) -> Result<Self> {
            let db = DB::open_lsm(path).await
                .map_err(|e| KotobaError::Storage(format!("Failed to open KotobaDB: {}", e)))?;
            Ok(Self { db })
        }
    }

    #[async_trait]
    impl StorageBackend for KotobaDBBackend {
        async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
            // For basic key-value operations, we store as simple nodes
            // In a real implementation, this might use a dedicated key-value table
            // For now, we'll use the key as a property and store the value
            use kotoba_db::{Value, Operation};
            use std::collections::BTreeMap;

            let mut properties = BTreeMap::new();
            properties.insert("key".to_string(), Value::String(key.clone()));
            properties.insert("value".to_string(), Value::Bytes(value));

            // Check if key already exists, update if it does
            let existing_nodes = self.db.find_nodes(&[("key".to_string(), Value::String(key.clone()))]).await
                .map_err(|e| KotobaError::Storage(format!("Failed to query KotobaDB: {}", e)))?;

            if let Some((cid, _)) = existing_nodes.first() {
                // Update existing node
                let mut update_props = BTreeMap::new();
                update_props.insert("value".to_string(), Value::Bytes(value));
                let txn_id = self.db.begin_transaction().await
                    .map_err(|e| KotobaError::Storage(format!("Failed to begin transaction: {}", e)))?;
                self.db.add_operation(txn_id, Operation::UpdateNode { cid: *cid, properties: update_props }).await
                    .map_err(|e| KotobaError::Storage(format!("Failed to add operation: {}", e)))?;
                self.db.commit_transaction(txn_id).await
                    .map_err(|e| KotobaError::Storage(format!("Failed to commit transaction: {}", e)))?;
            } else {
                // Create new node
                self.db.create_node(properties).await
                    .map_err(|e| KotobaError::Storage(format!("Failed to create node: {}", e)))?;
            }

            Ok(())
        }

        async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
            let nodes = self.db.find_nodes(&[("key".to_string(), Value::String(key.to_string()))]).await
                .map_err(|e| KotobaError::Storage(format!("Failed to query KotobaDB: {}", e)))?;

            if let Some((_, node)) = nodes.first() {
                if let Some(Value::Bytes(value)) = node.properties.get("value") {
                    Ok(Some(value.clone()))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }

        async fn delete(&self, key: &str) -> Result<()> {
            let nodes = self.db.find_nodes(&[("key".to_string(), Value::String(key.to_string()))]).await
                .map_err(|e| KotobaError::Storage(format!("Failed to query KotobaDB: {}", e)))?;

            if let Some((cid, _)) = nodes.first() {
                let txn_id = self.db.begin_transaction().await
                    .map_err(|e| KotobaError::Storage(format!("Failed to begin transaction: {}", e)))?;
                self.db.add_operation(txn_id, Operation::DeleteNode { cid: *cid }).await
                    .map_err(|e| KotobaError::Storage(format!("Failed to add operation: {}", e)))?;
                self.db.commit_transaction(txn_id).await
                    .map_err(|e| KotobaError::Storage(format!("Failed to commit transaction: {}", e)))?;
            }

            Ok(())
        }

        async fn exists(&self, key: &str) -> Result<bool> {
            let nodes = self.db.find_nodes(&[("key".to_string(), Value::String(key.to_string()))]).await
                .map_err(|e| KotobaError::Storage(format!("Failed to query KotobaDB: {}", e)))?;
            Ok(!nodes.is_empty())
        }

        async fn list_keys(&self, prefix: Option<&str>) -> Result<Vec<String>> {
            // For simplicity, return all keys (in a real implementation, this would be optimized)
            // This is not efficient for large datasets
            let all_keys = self.db.engine.scan(b"").await
                .map_err(|e| KotobaError::Storage(format!("Failed to scan KotobaDB: {}", e)))?;

            let mut keys = Vec::new();
            for (key_bytes, _) in all_keys {
                if let Ok(cid) = <[u8; 32]>::try_from(&key_bytes[..]) {
                    if let Some(block) = self.db.get_block(&cid).await
                        .map_err(|e| KotobaError::Storage(format!("Failed to get block: {}", e)))? {
                        if let kotoba_db_core::types::Block::Node(node) = block {
                            if let Some(Value::String(key_str)) = node.properties.get("key") {
                                if let Some(prefix_str) = prefix {
                                    if key_str.starts_with(prefix_str) {
                                        keys.push(key_str.clone());
                                    }
                                } else {
                                    keys.push(key_str.clone());
                                }
                            }
                        }
                    }
                }
            }

            Ok(keys)
        }

        async fn get_stats(&self) -> Result<BackendStats> {
            // Basic stats - in a real implementation, this would provide more detailed metrics
            Ok(BackendStats {
                total_keys: 0, // TODO: implement proper counting
                total_size: 0,
                read_operations: 0,
                write_operations: 0,
                cache_hit_rate: 0.0,
                avg_response_time_ms: 0.0,
            })
        }
    }
}

/// Storage backend types
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    RocksDB,
    Redis,
    ObjectStorage,
    KotobaDB, // New graph-native database backend
}

/// Object storage provider types
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectStorageProvider {
    AWS,
    GCP,
    Azure,
    Local, // MinIO, LocalStack, etc.
}

/// Storage tier type
#[derive(Debug, Clone, PartialEq)]
pub enum StorageTier {
    /// Hot tier - fast local storage (RocksDB, Redis)
    Hot,
    /// Cold tier - cost-effective object storage (S3, GCS, Azure)
    Cold,
    /// Cache tier - temporary fast storage for frequently accessed data
    Cache,
}

/// Hybrid storage configuration
#[derive(Debug, Clone)]
pub struct HybridStorageConfig {
    /// Hot tier backend type
    pub hot_backend: Option<BackendType>,
    /// Cold tier backend type
    pub cold_backend: Option<BackendType>,
    /// Cache backend type (optional)
    pub cache_backend: Option<BackendType>,
    /// Cache size limit in bytes
    pub cache_size_limit: Option<u64>,
    /// Data migration policy (hot -> cold threshold in days)
    pub cold_migration_threshold_days: Option<u64>,
    /// Enable automatic tiering
    pub enable_auto_tiering: bool,
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
    /// KotobaDB path for graph-native storage
    pub kotoba_db_path: Option<std::path::PathBuf>,
    /// Hybrid storage configuration
    pub hybrid_config: Option<HybridStorageConfig>,
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
            kotoba_db_path: Some(std::path::PathBuf::from("./kotoba_db")),
            hybrid_config: None,
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
            BackendType::KotobaDB => {
                #[cfg(feature = "kotoba_db")]
                {
                    let path = config.kotoba_db_path.as_ref()
                        .ok_or_else(|| KotobaError::Storage("KotobaDB path not configured".to_string()))?;
                    let kotoba_db_backend = kotoba_db_backend::KotobaDBBackend::new(path).await?;
                    Ok(Box::new(kotoba_db_backend))
                }
                #[cfg(not(feature = "kotoba_db"))]
                {
                    Err(KotobaError::Storage(
                        "KotobaDB backend not available - compile with 'kotoba_db' feature".to_string()
                    ))
                }
            }
        }
    }
}

/// Data routing policy for hybrid storage
#[derive(Debug, Clone)]
pub enum DataRoutingPolicy {
    /// Route based on data age (time-based tiering)
    AgeBased {
        /// Hot data threshold in days
        hot_threshold_days: u64,
    },
    /// Route based on access frequency (LRU cache)
    AccessFrequency {
        /// Minimum access count to stay in hot tier
        min_access_count: u64,
    },
    /// Route based on data size
    SizeBased {
        /// Maximum size for hot tier in bytes
        hot_max_size: u64,
    },
    /// Manual routing - explicit tier specification
    Manual,
}

/// Hybrid storage manager that handles multiple storage tiers
pub struct HybridStorageManager {
    /// Hot tier storage (fast, expensive)
    hot_backend: Option<Box<dyn StorageBackend>>,
    /// Cold tier storage (slow, cheap)
    cold_backend: Option<Box<dyn StorageBackend>>,
    /// Cache tier storage (fast, volatile)
    cache_backend: Option<Box<dyn StorageBackend>>,
    /// Configuration
    config: StorageConfig,
    /// Routing policy
    routing_policy: DataRoutingPolicy,
}

impl HybridStorageManager {
    /// Create a new hybrid storage manager
    pub async fn new(hybrid_config: HybridStorageConfig, base_config: StorageConfig) -> Result<Self> {
        let mut hot_backend = None;
        let mut cold_backend = None;
        let mut cache_backend = None;

        // Create hot tier backend
        if let Some(hot_type) = hybrid_config.hot_backend {
            let hot_config = StorageConfig {
                backend_type: hot_type,
                ..base_config.clone()
            };
            hot_backend = Some(StorageBackendFactory::create(&hot_config).await?);
        }

        // Create cold tier backend
        if let Some(cold_type) = hybrid_config.cold_backend {
            let cold_config = StorageConfig {
                backend_type: cold_type,
                ..base_config.clone()
            };
            cold_backend = Some(StorageBackendFactory::create(&cold_config).await?);
        }

        // Create cache tier backend
        if let Some(cache_type) = hybrid_config.cache_backend {
            let cache_config = StorageConfig {
                backend_type: cache_type,
                ..base_config.clone()
            };
            cache_backend = Some(StorageBackendFactory::create(&cache_config).await?);
        }

        // Default routing policy
        let routing_policy = if hybrid_config.enable_auto_tiering {
            DataRoutingPolicy::AgeBased {
                hot_threshold_days: hybrid_config.cold_migration_threshold_days.unwrap_or(30),
            }
        } else {
            DataRoutingPolicy::Manual
        };

        Ok(Self {
            hot_backend,
            cold_backend,
            cache_backend,
            config: base_config,
            routing_policy,
        })
    }

    /// Determine which tier to route data to
    pub fn route_to_tier(&self, key: &str, access_count: u64, data_age_days: u64, data_size: u64) -> StorageTier {
        match &self.routing_policy {
            DataRoutingPolicy::AgeBased { hot_threshold_days } => {
                if data_age_days <= *hot_threshold_days {
                    StorageTier::Hot
                } else {
                    StorageTier::Cold
                }
            }
            DataRoutingPolicy::AccessFrequency { min_access_count } => {
                if access_count >= *min_access_count {
                    StorageTier::Hot
                } else {
                    StorageTier::Cold
                }
            }
            DataRoutingPolicy::SizeBased { hot_max_size } => {
                if data_size <= *hot_max_size {
                    StorageTier::Hot
                } else {
                    StorageTier::Cold
                }
            }
            DataRoutingPolicy::Manual => {
                // For manual routing, check cache first, then hot, then cold
                if self.cache_backend.is_some() {
                    StorageTier::Cache
                } else if self.hot_backend.is_some() {
                    StorageTier::Hot
                } else {
                    StorageTier::Cold
                }
            }
        }
    }
}

/// High-level storage manager that provides a unified interface
/// and handles backend selection and management
pub struct StorageManager {
    backend: Box<dyn StorageBackend>,
    config: StorageConfig,
    /// Optional hybrid manager for multi-tier storage
    hybrid_manager: Option<HybridStorageManager>,
}

impl StorageManager {
    /// Create a new storage manager with the specified configuration
    pub async fn new(config: StorageConfig) -> Result<Self> {
        let backend = StorageBackendFactory::create(&config).await?;
        let hybrid_manager = if let Some(hybrid_config) = &config.hybrid_config {
            Some(HybridStorageManager::new(hybrid_config.clone(), config.clone()).await?)
        } else {
            None
        };
        Ok(Self { backend, config, hybrid_manager })
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
        if let Some(hybrid) = &self.hybrid_manager {
            hybrid.put(key, value).await
        } else {
            self.backend.put(key, value).await
        }
    }

    /// Retrieve a value by key
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        if let Some(hybrid) = &self.hybrid_manager {
            hybrid.get(key).await
        } else {
            self.backend.get(key).await
        }
    }

    /// Delete a key-value pair
    pub async fn delete(&self, key: String) -> Result<()> {
        if let Some(hybrid) = &self.hybrid_manager {
            hybrid.delete(key).await
        } else {
            self.backend.delete(key).await
        }
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

    /// Create a hybrid storage manager with hot (local) and cold (object storage) tiers
    pub async fn with_hybrid_redis_s3(
        redis_url: String,
        s3_bucket: String,
        s3_region: Option<String>,
        cache_size_limit: Option<u64>
    ) -> Result<Self> {
        let hybrid_config = HybridStorageConfig {
            hot_backend: Some(BackendType::Redis),
            cold_backend: Some(BackendType::ObjectStorage),
            cache_backend: None,
            cache_size_limit,
            cold_migration_threshold_days: Some(30), // 30 days
            enable_auto_tiering: true,
        };

        let config = StorageConfig {
            backend_type: BackendType::Redis, // Primary backend
            redis_url: Some(redis_url),
            object_storage_provider: Some(ObjectStorageProvider::AWS),
            object_storage_bucket: Some(s3_bucket),
            object_storage_region: s3_region,
            hybrid_config: Some(hybrid_config),
            ..Default::default()
        };

        Self::new(config).await
    }

    /// Create a hybrid storage manager with RocksDB (hot) and S3 (cold)
    pub async fn with_hybrid_rocksdb_s3(
        rocksdb_path: std::path::PathBuf,
        s3_bucket: String,
        s3_region: Option<String>
    ) -> Result<Self> {
        let hybrid_config = HybridStorageConfig {
            hot_backend: Some(BackendType::RocksDB),
            cold_backend: Some(BackendType::ObjectStorage),
            cache_backend: None,
            cache_size_limit: None,
            cold_migration_threshold_days: Some(30),
            enable_auto_tiering: true,
        };

        let config = StorageConfig {
            backend_type: BackendType::RocksDB, // Primary backend
            rocksdb_path: Some(rocksdb_path),
            object_storage_provider: Some(ObjectStorageProvider::AWS),
            object_storage_bucket: Some(s3_bucket),
            object_storage_region: s3_region,
            hybrid_config: Some(hybrid_config),
            ..Default::default()
        };

        Self::new(config).await
    }

    /// Create a hybrid storage manager with cache, hot, and cold tiers
    pub async fn with_three_tier(
        cache_backend: BackendType,
        hot_backend: BackendType,
        cold_backend: BackendType,
        cache_size_limit: u64,
        cold_migration_days: u64
    ) -> Result<Self> {
        let hybrid_config = HybridStorageConfig {
            hot_backend: Some(hot_backend.clone()),
            cold_backend: Some(cold_backend.clone()),
            cache_backend: Some(cache_backend.clone()),
            cache_size_limit: Some(cache_size_limit),
            cold_migration_threshold_days: Some(cold_migration_days),
            enable_auto_tiering: true,
        };

        let config = StorageConfig {
            backend_type: cache_backend, // Primary backend is cache
            hybrid_config: Some(hybrid_config),
            ..Default::default()
        };

        Self::new(config).await
    }

    /// Store a key-value pair with tier routing
    pub async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        let data_size = value.len() as u64;
        let tier = self.route_to_tier(&key, 0, 0, data_size); // Simplified routing

        match tier {
            StorageTier::Cache => {
                if let Some(cache) = &self.cache_backend {
                    // Try cache first
                    cache.put(key.clone(), value.clone()).await?;
                }
                // Also store in hot tier as backup
                if let Some(hot) = &self.hot_backend {
                    hot.put(key, value).await?;
                } else {
                    return Err(KotobaError::Storage("No hot tier configured".to_string()));
                }
            }
            StorageTier::Hot => {
                if let Some(hot) = &self.hot_backend {
                    hot.put(key, value).await?;
                } else {
                    return Err(KotobaError::Storage("No hot tier configured".to_string()));
                }
            }
            StorageTier::Cold => {
                if let Some(cold) = &self.cold_backend {
                    cold.put(key, value).await?;
                } else {
                    return Err(KotobaError::Storage("No cold tier configured".to_string()));
                }
            }
        }

        Ok(())
    }

    /// Retrieve a value by key with tier fallback
    pub async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Try cache first
        if let Some(cache) = &self.cache_backend {
            if let Some(value) = cache.get(key).await? {
                return Ok(Some(value));
            }
        }

        // Try hot tier
        if let Some(hot) = &self.hot_backend {
            if let Some(value) = hot.get(key).await? {
                // Promote to cache if cache exists
                if let Some(cache) = &self.cache_backend {
                    if let Ok(cached_value) = cache.get(key).await {
                        if cached_value.is_none() {
                            let _ = cache.put(key.to_string(), value.clone()).await;
                        }
                    }
                }
                return Ok(Some(value));
            }
        }

        // Try cold tier
        if let Some(cold) = &self.cold_backend {
            if let Some(value) = cold.get(key).await? {
                // Promote to hot tier and cache
                if let Some(hot) = &self.hot_backend {
                    let _ = hot.put(key.to_string(), value.clone()).await;
                }
                if let Some(cache) = &self.cache_backend {
                    let _ = cache.put(key.to_string(), value.clone()).await;
                }
                return Ok(Some(value));
            }
        }

        Ok(None)
    }

    /// Delete a key-value pair from all tiers
    pub async fn delete(&self, key: String) -> Result<()> {
        let mut result = Ok(());

        // Delete from cache
        if let Some(cache) = &self.cache_backend {
            if let Err(e) = cache.delete(key.clone()).await {
                result = Err(e);
            }
        }

        // Delete from hot tier
        if let Some(hot) = &self.hot_backend {
            if let Err(e) = hot.delete(key.clone()).await {
                result = Err(e);
            }
        }

        // Delete from cold tier
        if let Some(cold) = &self.cold_backend {
            if let Err(e) = cold.delete(key).await {
                result = Err(e);
            }
        }

        result
    }
}

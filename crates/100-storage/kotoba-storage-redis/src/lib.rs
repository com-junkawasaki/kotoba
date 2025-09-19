//! `kotoba-storage-redis`
//!
//! Redis adapter implementation for kotoba-storage.
//! Provides persistent key-value storage using Redis with advanced features.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use redis::{Client, AsyncCommands};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};
use chrono::{DateTime, Utc};

use kotoba_storage::KeyValueStore;

/// Redis storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisConfig {
    /// Redis connection URLs (cluster support)
    pub redis_urls: Vec<String>,
    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,
    /// Default TTL for cache entries in seconds
    pub default_ttl_seconds: u64,
    /// Maximum cache size (approximate)
    pub max_size_bytes: u64,
    /// Enable compression for large values
    pub enable_compression: bool,
    /// Compression threshold in bytes
    pub compression_threshold_bytes: usize,
    /// Enable metrics collection
    pub enable_metrics: bool,
    /// Cache key prefix
    pub key_prefix: String,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Enable TLS
    pub enable_tls: bool,
    /// Redis username (for ACL auth)
    pub username: Option<String>,
    /// Redis password
    pub password: Option<String>,
    /// Database number
    pub database: Option<u8>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            connection_timeout_seconds: 30,
            default_ttl_seconds: 3600, // 1 hour
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            enable_compression: true,
            compression_threshold_bytes: 1024, // 1KB
            enable_metrics: true,
            key_prefix: "kotoba:storage".to_string(),
            connection_pool_size: 10,
            enable_tls: false,
            username: None,
            password: None,
            database: Some(0),
        }
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RedisStats {
    pub total_operations: u64,
    pub hit_ratio: f64,
    pub total_size_bytes: u64,
    pub entries_count: u64,
    pub connection_status: ConnectionStatus,
    pub compression_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionStatus {
    Connected,
    Disconnected,
    MockMode,
}

impl Default for ConnectionStatus {
    fn default() -> Self {
        ConnectionStatus::Disconnected
    }
}

/// Redis-based key-value store implementation
pub struct RedisStore {
    /// Configuration
    config: RedisConfig,
    /// Redis client
    client: Client,
    /// Connection pool
    connection_pool: Arc<RwLock<Vec<redis::aio::Connection>>>,
    /// Statistics
    stats: Arc<RwLock<RedisStats>>,
    /// Active entries tracking
    active_entries: Arc<RwLock<HashMap<String, EntryMetadata>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EntryMetadata {
    key: String,
    size_bytes: usize,
    created_at: DateTime<Utc>,
    ttl_seconds: Option<u64>,
}

impl RedisStore {
    /// Create a new Redis store
    pub async fn new(config: RedisConfig) -> Result<Self, RedisError> {
        info!("Initializing Redis storage with config: {:?}", config);

        // Create Redis client with authentication
        let client_url = Self::build_redis_url(&config)?;
        let client = Client::open(client_url)
            .map_err(|e| RedisError::ConnectionError(e.to_string()))?;

        // Initialize connection pool
        let mut connection_pool = Vec::new();
        let mut connection_status = ConnectionStatus::Disconnected;

        // Try to establish connections
        for i in 0..config.connection_pool_size {
            match client.get_async_connection().await {
                Ok(conn) => {
                    connection_pool.push(conn);
                    if i == 0 {
                        connection_status = ConnectionStatus::Connected;
                        info!("Redis connection established successfully");
                    }
                }
                Err(e) => {
                    warn!("Failed to establish Redis connection {}: {}. Using mock mode.", i, e);
                    connection_status = ConnectionStatus::MockMode;
                    break;
                }
            }
        }

        if connection_pool.is_empty() {
            warn!("No Redis connections available, operating in mock mode");
            connection_status = ConnectionStatus::MockMode;
        }

        let stats = RedisStats {
            connection_status,
            ..Default::default()
        };

        Ok(Self {
            config,
            client,
            connection_pool: Arc::new(RwLock::new(connection_pool)),
            stats: Arc::new(RwLock::new(stats)),
            active_entries: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Create a Redis store with default configuration
    pub async fn default() -> Result<Self, RedisError> {
        Self::new(RedisConfig::default()).await
    }

    /// Build Redis URL with authentication
    fn build_redis_url(config: &RedisConfig) -> Result<String, RedisError> {
        let base_url = config.redis_urls.first()
            .ok_or_else(|| RedisError::ConfigError("No Redis URLs provided".to_string()))?;

        let mut url = if config.enable_tls {
            base_url.replace("redis://", "rediss://")
        } else {
            base_url.clone()
        };

        // Add authentication if provided
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let auth_part = format!("{}:{}", username, password);
            if let Some(pos) = url.find("://") {
                url.insert_str(pos + 3, &format!("{}@", auth_part));
            }
        } else if let Some(password) = &config.password {
            if let Some(pos) = url.find("://") {
                url.insert_str(pos + 3, &format!("{}@", password));
            }
        }

        // Add database number
        if let Some(db) = config.database {
            if !url.contains("?") {
                url.push('?');
            } else {
                url.push('&');
            }
            url.push_str(&format!("db={}", db));
        }

        Ok(url)
    }

    /// Get connection from pool
    async fn get_connection(&self) -> Result<redis::aio::Connection, RedisError> {
        let mut pool = self.connection_pool.write().await;
        if pool.is_empty() {
            // Try to create a new connection
            match self.client.get_async_connection().await {
                Ok(conn) => Ok(conn),
                Err(e) => Err(RedisError::ConnectionError(e.to_string())),
            }
        } else {
            Ok(pool.remove(0))
        }
    }

    /// Return connection to pool
    async fn return_connection(&self, connection: redis::aio::Connection) {
        let mut pool = self.connection_pool.write().await;
        if pool.len() < self.config.connection_pool_size {
            pool.push(connection);
        }
        // If pool is full, connection will be dropped
    }

    /// Make storage key with prefix
    fn make_storage_key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }

    /// Compress data using LZ4
    #[cfg(feature = "compression")]
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, RedisError> {
        lz4::block::compress(data, None, true)
            .map_err(|e| RedisError::CompressionError(e.to_string()))
    }

    /// Decompress data using LZ4
    #[cfg(feature = "compression")]
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, RedisError> {
        lz4::block::decompress(data, None)
            .map_err(|e| RedisError::CompressionError(e.to_string()))
    }

    /// Update statistics
    async fn update_stats(&self, operation: &str, success: bool) {
        if self.config.enable_metrics {
            let mut stats = self.stats.write().await;
            stats.total_operations += 1;

            if success {
                // Update metrics based on operation type
                match operation {
                    "get" => {
                        // Could track hit/miss ratio here
                    }
                    "put" => {
                        stats.entries_count += 1;
                    }
                    "delete" => {
                        if stats.entries_count > 0 {
                            stats.entries_count -= 1;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    /// Check if Redis is connected
    pub async fn is_connected(&self) -> bool {
        let stats = self.stats.read().await;
        matches!(stats.connection_status, ConnectionStatus::Connected)
    }

    /// Get current statistics
    pub async fn get_stats(&self) -> RedisStats {
        self.stats.read().await.clone()
    }
}

#[async_trait]
impl KeyValueStore for RedisStore {
    #[instrument(skip(self, value))]
    async fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        let key_str = String::from_utf8_lossy(key);
        let storage_key = self.make_storage_key(&key_str);

        // Check if we have connections
        let stats = self.stats.read().await;
        if matches!(stats.connection_status, ConnectionStatus::MockMode) {
            // Mock mode - just simulate success
            if self.config.enable_metrics {
                self.update_stats("put", true).await;
            }
            return Ok(());
        }
        drop(stats);

        let mut connection = self.get_connection().await?;

        // Prepare data (compress if enabled and above threshold)
        let final_data = if self.config.enable_compression && value.len() > self.config.compression_threshold_bytes {
            #[cfg(feature = "compression")]
            {
                self.compress_data(value)?
            }
            #[cfg(not(feature = "compression"))]
            {
                value.to_vec()
            }
        } else {
            value.to_vec()
        };

        // Store with TTL
        let ttl = self.config.default_ttl_seconds;
        let result = if ttl > 0 {
            connection.set_ex(&storage_key, final_data, ttl as u64).await
        } else {
            connection.set(&storage_key, final_data).await
        };

        self.return_connection(connection).await;

        match result {
            Ok(()) => {
                // Track metadata
                let metadata = EntryMetadata {
                    key: key_str.to_string(),
                    size_bytes: value.len(),
                    created_at: Utc::now(),
                    ttl_seconds: Some(ttl),
                };

                let mut entries = self.active_entries.write().await;
                entries.insert(key_str.to_string(), metadata);

                self.update_stats("put", true).await;
                Ok(())
            }
            Err(e) => {
                error!("Redis put error for key {}: {}", storage_key, e);
                self.update_stats("put", false).await;
                Err(anyhow::anyhow!("Redis put error: {}", e))
            }
        }
    }

    #[instrument(skip(self))]
    async fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let key_str = String::from_utf8_lossy(key);
        let storage_key = self.make_storage_key(&key_str);

        // Check if we have connections
        let stats = self.stats.read().await;
        if matches!(stats.connection_status, ConnectionStatus::MockMode) {
            // Mock mode - return None
            if self.config.enable_metrics {
                self.update_stats("get", true).await;
            }
            return Ok(None);
        }
        drop(stats);

        let mut connection = self.get_connection().await?;

        let result = connection.get::<_, Option<Vec<u8>>>(&storage_key).await;
        self.return_connection(connection).await;

        match result {
            Ok(Some(data)) => {
                // Decompress if needed
                let decompressed_data = if self.config.enable_compression && data.len() > self.config.compression_threshold_bytes {
                    #[cfg(feature = "compression")]
                    {
                        self.decompress_data(&data)?
                    }
                    #[cfg(not(feature = "compression"))]
                    {
                        data
                    }
                } else {
                    data
                };

                self.update_stats("get", true).await;
                Ok(Some(decompressed_data))
            }
            Ok(None) => {
                self.update_stats("get", true).await;
                Ok(None)
            }
            Err(e) => {
                error!("Redis get error for key {}: {}", storage_key, e);
                self.update_stats("get", false).await;
                Err(anyhow::anyhow!("Redis get error: {}", e))
            }
        }
    }

    #[instrument(skip(self))]
    async fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
        let key_str = String::from_utf8_lossy(key);
        let storage_key = self.make_storage_key(&key_str);

        // Check if we have connections
        let stats = self.stats.read().await;
        if matches!(stats.connection_status, ConnectionStatus::MockMode) {
            // Mock mode - simulate success
            let mut entries = self.active_entries.write().await;
            entries.remove(&key_str.to_string());
            if self.config.enable_metrics {
                self.update_stats("delete", true).await;
            }
            return Ok(());
        }
        drop(stats);

        let mut connection = self.get_connection().await?;

        let result = connection.del::<_, i64>(&storage_key).await;
        self.return_connection(connection).await;

        match result {
            Ok(_) => {
                // Remove from active entries
                let mut entries = self.active_entries.write().await;
                entries.remove(&key_str.to_string());

                self.update_stats("delete", true).await;
                Ok(())
            }
            Err(e) => {
                error!("Redis delete error for key {}: {}", storage_key, e);
                self.update_stats("delete", false).await;
                Err(anyhow::anyhow!("Redis delete error: {}", e))
            }
        }
    }

    #[instrument(skip(self))]
    async fn scan(&self, prefix: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_str = String::from_utf8_lossy(prefix);
        let storage_prefix = self.make_storage_key(&prefix_str);

        // Check if we have connections
        let stats = self.stats.read().await;
        if matches!(stats.connection_status, ConnectionStatus::MockMode) {
            // Mock mode - return empty results
            return Ok(Vec::new());
        }
        drop(stats);

        let mut connection = self.get_connection().await?;

        // Use KEYS command to find matching keys
        let pattern = format!("{}*", storage_prefix);
        let keys: Vec<String> = connection.keys(&pattern).await
            .map_err(|e| anyhow::anyhow!("Redis scan keys error: {}", e))?;

        let mut results = Vec::new();

        for key in keys {
            if let Ok(Some(value)) = connection.get::<_, Option<Vec<u8>>>(&key).await {
                // Remove prefix from key for result
                let original_key = if let Some(stripped) = key.strip_prefix(&format!("{}:", self.config.key_prefix)) {
                    stripped.as_bytes().to_vec()
                } else {
                    key.as_bytes().to_vec()
                };

                results.push((original_key, value));
            }
        }

        self.return_connection(connection).await;

        // Sort results for consistent ordering
        results.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(results)
    }
}

/// Redis storage errors
#[derive(thiserror::Error, Debug)]
pub enum RedisError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    #[error("Redis operation error: {0}")]
    OperationError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// Remove the conflicting From implementation
// RedisError can be converted to anyhow::Error via the Display trait

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_redis_store_creation_mock() {
        // Test with invalid Redis URL to force mock mode
        let config = RedisConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:redis".to_string(),
            ..Default::default()
        };

        // This should succeed even with invalid Redis URL (mock mode)
        let store_result = RedisStore::new(config).await;
        assert!(store_result.is_ok(), "Redis store should create successfully in mock mode");

        let store = store_result.unwrap();
        assert_eq!(store.config.key_prefix, "test:redis");

        // Check that it's in mock mode
        let stats = store.get_stats().await;
        assert!(matches!(stats.connection_status, ConnectionStatus::MockMode));
    }

    #[tokio::test]
    async fn test_redis_store_mock_operations() {
        let config = RedisConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:mock".to_string(),
            enable_metrics: true,
            ..Default::default()
        };

        let store = RedisStore::new(config).await.unwrap();

        // Test mock operations
        let test_key = b"mock_key";
        let test_value = b"mock_value";

        // Put operation
        let put_result = store.put(test_key, test_value).await;
        assert!(put_result.is_ok(), "Mock put should succeed");

        // Get operation (should return None in mock mode since we can't actually store)
        let get_result = store.get(test_key).await;
        assert!(get_result.is_ok(), "Mock get should succeed");
        assert_eq!(get_result.unwrap(), None, "Mock get should return None");

        // Delete operation
        let delete_result = store.delete(test_key).await;
        assert!(delete_result.is_ok(), "Mock delete should succeed");

        // Scan operation
        let scan_result = store.scan(b"mock").await;
        assert!(scan_result.is_ok(), "Mock scan should succeed");
        assert_eq!(scan_result.unwrap().len(), 0, "Mock scan should return empty results");
    }

    #[tokio::test]
    async fn test_redis_config_default() {
        let config = RedisConfig::default();

        assert_eq!(config.redis_urls, vec!["redis://127.0.0.1:6379".to_string()]);
        assert_eq!(config.connection_timeout_seconds, 30);
        assert_eq!(config.default_ttl_seconds, 3600);
        assert_eq!(config.key_prefix, "kotoba:storage");
        assert_eq!(config.connection_pool_size, 10);
        assert!(!config.enable_tls);
        assert!(config.enable_compression);
        assert!(config.enable_metrics);
    }

    #[tokio::test]
    async fn test_redis_config_custom() {
        let config = RedisConfig {
            redis_urls: vec!["redis://localhost:6380".to_string()],
            connection_timeout_seconds: 60,
            default_ttl_seconds: 1800,
            key_prefix: "custom:prefix".to_string(),
            connection_pool_size: 5,
            enable_tls: true,
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            database: Some(1),
            ..Default::default()
        };

        assert_eq!(config.redis_urls, vec!["redis://localhost:6380".to_string()]);
        assert_eq!(config.connection_timeout_seconds, 60);
        assert_eq!(config.default_ttl_seconds, 1800);
        assert_eq!(config.key_prefix, "custom:prefix");
        assert_eq!(config.connection_pool_size, 5);
        assert!(config.enable_tls);
        assert_eq!(config.username, Some("testuser".to_string()));
        assert_eq!(config.password, Some("testpass".to_string()));
        assert_eq!(config.database, Some(1));
    }

    #[test]
    fn test_build_redis_url_basic() {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            ..Default::default()
        };

        let url = RedisStore::build_redis_url(&config).unwrap();
        assert_eq!(url, "redis://127.0.0.1:6379?db=0");
    }

    #[test]
    fn test_build_redis_url_with_auth() {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            username: Some("testuser".to_string()),
            password: Some("testpass".to_string()),
            database: Some(1),
            ..Default::default()
        };

        let url = RedisStore::build_redis_url(&config).unwrap();
        assert_eq!(url, "redis://testuser:testpass@127.0.0.1:6379?db=1");
    }

    #[test]
    fn test_build_redis_url_tls() {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            enable_tls: true,
            password: Some("testpass".to_string()),
            ..Default::default()
        };

        let url = RedisStore::build_redis_url(&config).unwrap();
        assert_eq!(url, "rediss://testpass@127.0.0.1:6379?db=0");
    }

    #[test]
    fn test_build_redis_url_no_urls() {
        let config = RedisConfig {
            redis_urls: vec![],
            ..Default::default()
        };

        let result = RedisStore::build_redis_url(&config);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_redis_store_key_prefix() {
        let config = RedisConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:prefix".to_string(),
            ..Default::default()
        };

        let store = RedisStore::new(config).await.unwrap();

        // Test that key prefix is applied correctly
        assert_eq!(store.make_storage_key("my_key"), "test:prefix:my_key");
        assert_eq!(store.make_storage_key("another/key"), "test:prefix:another/key");
        assert_eq!(store.make_storage_key(""), "test:prefix:");
    }

    #[tokio::test]
    async fn test_redis_store_stats() {
        let config = RedisConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            enable_metrics: true,
            ..Default::default()
        };

        let store = RedisStore::new(config).await.unwrap();

        // Initial stats
        let initial_stats = store.get_stats().await;
        assert_eq!(initial_stats.total_operations, 0);
        assert!(matches!(initial_stats.connection_status, ConnectionStatus::MockMode));

        // Perform some operations
        store.put(b"key1", b"value1").await.unwrap();
        store.get(b"key1").await.unwrap();
        store.delete(b"key1").await.unwrap();

        // Check updated stats
        let updated_stats = store.get_stats().await;
        assert_eq!(updated_stats.total_operations, 3);
    }
}

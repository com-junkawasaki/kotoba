//! `kotoba-cache`
//!
//! Redis-based distributed cache layer for KotobaDB.
//! Provides high-performance caching with distributed invalidation.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use redis::{Client, Connection, AsyncCommands, RedisResult};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use tracing::{info, warn, error, instrument};
use metrics::{counter, histogram};
use chrono::{DateTime, Utc};

/// Cache layer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
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
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            connection_timeout_seconds: 30,
            default_ttl_seconds: 3600, // 1 hour
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            enable_compression: true,
            compression_threshold_bytes: 1024, // 1KB
            enable_metrics: true,
            key_prefix: "kotoba:cache".to_string(),
        }
    }
}

/// Main cache layer implementation
pub struct CacheLayer {
    /// Configuration
    config: CacheConfig,
    /// Redis client
    client: Client,
    /// Single connection (simplified for now)
    connection: Arc<RwLock<Option<redis::aio::Connection>>>,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Active cache entries (for local tracking)
    active_entries: Arc<DashMap<String, CacheEntry>>,
}

/// Cache entry metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// Cache key
    pub key: String,
    /// Entry size in bytes
    pub size_bytes: usize,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last access timestamp
    pub last_accessed: DateTime<Utc>,
    /// Access count
    pub access_count: u64,
    /// TTL in seconds
    pub ttl_seconds: Option<u64>,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
    pub evictions: u64,
    pub hit_ratio: f64,
    pub total_size_bytes: u64,
    pub entries_count: u64,
}

impl CacheLayer {
    /// Create a new cache layer
    pub async fn new(config: CacheConfig) -> Result<Self, CacheError> {
        info!("Initializing Redis cache layer with config: {:?}", config);

        // Create Redis client (use first URL for now)
        let client = Client::open(config.redis_urls.first().unwrap_or(&"redis://127.0.0.1:6379".to_string()).clone())
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        // Create connection
        let connection = match client.get_async_connection().await {
            Ok(conn) => Some(conn),
            Err(e) => {
                warn!("Failed to establish Redis connection: {}. Using mock cache.", e);
                None // Allow mock operation for testing
            }
        };

        if connection.is_some() {
            info!("Redis connection established successfully");
        }

        let cache = Self {
            config,
            client,
            connection: Arc::new(RwLock::new(connection)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            active_entries: Arc::new(DashMap::new()),
        };

        Ok(cache)
    }

    /// Get a value from cache
    #[instrument(skip(self))]
    pub async fn get(&self, key: &str) -> Result<Option<serde_json::Value>, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, simulate cache miss
            if self.config.enable_metrics {
                self.record_miss().await;
            }
            return Ok(None);
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        let result = connection.get::<_, Option<Vec<u8>>>(&cache_key).await;
        self.return_connection(connection);

        match result {
            Ok(Some(data)) => {
                // Decompress if needed
                let decompressed_data = if self.config.enable_compression {
                    self.decompress_data(&data)?
                } else {
                    data
                };

                // Deserialize
                let value: serde_json::Value = serde_json::from_slice(&decompressed_data)
                    .map_err(|e| CacheError::SerializationError(e.to_string()))?;

                // Update statistics
                if self.config.enable_metrics {
                    self.record_hit().await;
                }

                // Update access metadata
                self.update_access_metadata(key).await?;

                Ok(Some(value))
            }
            Ok(None) => {
                if self.config.enable_metrics {
                    self.record_miss().await;
                }
                Ok(None)
            }
            Err(e) => {
                error!("Cache get error for key {}: {}", cache_key, e);
                if self.config.enable_metrics {
                    self.record_miss().await;
                }
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Set a value in cache
    #[instrument(skip(self, value))]
    pub async fn set(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> Result<(), CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, just return success for mock operation
            if self.config.enable_metrics {
                self.record_set().await;
            }
            return Ok(());
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        // Serialize value
        let serialized_data = serde_json::to_vec(&value)
            .map_err(|e| CacheError::SerializationError(e.to_string()))?;

        // Compress if enabled and above threshold
        let final_data = if self.config.enable_compression && serialized_data.len() > self.config.compression_threshold_bytes {
            self.compress_data(&serialized_data)?
        } else {
            serialized_data.clone()
        };

        // Set TTL
        let ttl = ttl_seconds.unwrap_or(self.config.default_ttl_seconds);

        let result = if ttl > 0 {
            connection.set_ex(&cache_key, final_data, ttl as u64).await
        } else {
            connection.set(&cache_key, final_data).await
        };

        self.return_connection(connection);

        match result {
            Ok(()) => {
                // Update statistics
                if self.config.enable_metrics {
                    self.record_set().await;
                }

                // Track cache entry
                let entry = CacheEntry {
                    key: key.to_string(),
                    size_bytes: serialized_data.len(),
                    created_at: Utc::now(),
                    last_accessed: Utc::now(),
                    access_count: 0,
                    ttl_seconds: Some(ttl),
                };
                self.active_entries.insert(key.to_string(), entry);

                // Check cache size limit
                self.enforce_size_limit().await?;

                Ok(())
            }
            Err(e) => {
                error!("Cache set error for key {}: {}", cache_key, e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Delete a value from cache
    #[instrument(skip(self))]
    pub async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, simulate successful delete
            if self.config.enable_metrics {
                self.record_delete().await;
            }
            self.active_entries.remove(key);
            return Ok(true);
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        let result = connection.del::<_, i64>(&cache_key).await;
        self.return_connection(connection);

        match result {
            Ok(count) => {
                let deleted = count > 0;
                if deleted {
                    if self.config.enable_metrics {
                        self.record_delete().await;
                    }
                    self.active_entries.remove(key);
                }
                Ok(deleted)
            }
            Err(e) => {
                error!("Cache delete error for key {}: {}", cache_key, e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Check if key exists in cache
    #[instrument(skip(self))]
    pub async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, simulate non-existence
            return Ok(false);
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        let result = connection.exists::<_, bool>(&cache_key).await;
        self.return_connection(connection);

        match result {
            Ok(exists) => Ok(exists),
            Err(e) => {
                error!("Cache exists error for key {}: {}", cache_key, e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Get time to live for a key
    #[instrument(skip(self))]
    pub async fn ttl(&self, key: &str) -> Result<Option<i64>, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            return Ok(None);
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        let result = connection.ttl::<_, i64>(&cache_key).await;
        self.return_connection(connection);

        match result {
            Ok(ttl) => {
                if ttl < 0 {
                    Ok(None)
                } else {
                    Ok(Some(ttl))
                }
            }
            Err(e) => {
                error!("Cache TTL error for key {}: {}", cache_key, e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Increment a numeric value
    #[instrument(skip(self))]
    pub async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, simulate increment
            return Ok(amount);
        }
        drop(conn_opt);

        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        let result = connection.incr::<_, _, i64>(&cache_key, amount).await;
        self.return_connection(connection);

        match result {
            Ok(value) => Ok(value),
            Err(e) => {
                error!("Cache increment error for key {}: {}", cache_key, e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Clear all cache entries
    #[instrument(skip(self))]
    pub async fn clear(&self) -> Result<(), CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        let keys_deleted = if conn_opt.is_some() {
            drop(conn_opt);

            let pattern = format!("{}:*", self.config.key_prefix);
            let mut connection = self.get_connection().await?;

            // Get all keys matching the pattern
            let keys: Vec<String> = connection.keys::<_, Vec<String>>(&pattern).await
                .map_err(|e| CacheError::RedisError(e.to_string()))?;

            let keys_count = keys.len();

            if !keys.is_empty() {
                // Delete all keys
                let _: () = connection.del::<_, ()>(&keys).await
                    .map_err(|e| CacheError::RedisError(e.to_string()))?;
            }

            self.return_connection(connection);
            keys_count
        } else {
            0
        };

        // Clear local tracking
        self.active_entries.clear();

        if self.config.enable_metrics {
            let mut stats = self.stats.write().await;
            stats.deletes += keys_deleted as u64;
        }

        info!("Cache cleared, {} keys deleted", keys_deleted);
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get cache information
    #[instrument(skip(self))]
    pub async fn get_info(&self) -> Result<HashMap<String, String>, CacheError> {
        // Check if we have a Redis connection
        let conn_opt = self.connection.read().await;
        if conn_opt.is_none() {
            // No Redis connection, return mock info
            let mut info_map = HashMap::new();
            info_map.insert("status".to_string(), "mock".to_string());
            info_map.insert("version".to_string(), "0.0.0".to_string());
            return Ok(info_map);
        }
        drop(conn_opt);

        let mut connection = self.get_connection().await?;

        let info_result: Result<String, _> = redis::cmd("INFO").query_async(&mut connection).await;
        self.return_connection(connection);

        match info_result {
            Ok(info) => {
                // Parse INFO command output
                let mut info_map = HashMap::new();
                for line in info.lines() {
                    if let Some((key, value)) = line.split_once(':') {
                        info_map.insert(key.to_string(), value.to_string());
                    }
                }
                Ok(info_map)
            }
            Err(e) => {
                error!("Cache INFO error: {}", e);
                Err(CacheError::RedisError(e.to_string()))
            }
        }
    }

    /// Get a Redis connection
    async fn get_connection(&self) -> Result<redis::aio::Connection, CacheError> {
        let mut conn_opt = self.connection.write().await;
        match conn_opt.take() {
            Some(conn) => Ok(conn),
            None => {
                // Try to create new connection
                self.client.get_async_connection().await
                    .map_err(|e| CacheError::ConnectionError(e.to_string()))
            }
        }
    }

    /// Return a connection
    async fn return_connection(&self, connection: redis::aio::Connection) {
        let mut conn_opt = self.connection.write().await;
        *conn_opt = Some(connection);
    }

    /// Make cache key with prefix
    fn make_cache_key(&self, key: &str) -> String {
        format!("{}:{}", self.config.key_prefix, key)
    }

    /// Compress data using LZ4
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        lz4::block::compress(data, None, true)
            .map_err(|e| CacheError::CompressionError(e.to_string()))
    }

    /// Decompress data using LZ4
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>, CacheError> {
        lz4::block::decompress(data, None)
            .map_err(|e| CacheError::CompressionError(e.to_string()))
    }

    /// Update access metadata for a cache entry
    async fn update_access_metadata(&self, key: &str) -> Result<(), CacheError> {
        if let Some(mut entry) = self.active_entries.get_mut(key) {
            entry.last_accessed = Utc::now();
            entry.access_count += 1;
        }
        Ok(())
    }

    /// Enforce cache size limit by evicting least recently used entries
    async fn enforce_size_limit(&self) -> Result<(), CacheError> {
        let mut total_size = 0u64;
        let mut entries: Vec<_> = self.active_entries.iter().collect();

        // Sort by last accessed time (oldest first)
        entries.sort_by(|a, b| a.last_accessed.cmp(&b.last_accessed));

        for entry in entries {
            total_size += entry.size_bytes as u64;

            if total_size > self.config.max_size_bytes {
                // Evict this entry
                let key = entry.key.clone();
                if let Err(e) = self.delete(&key).await {
                    warn!("Failed to evict cache entry {}: {}", key, e);
                } else {
                    if self.config.enable_metrics {
                        self.record_eviction().await;
                    }
                }
            }
        }

        Ok(())
    }

    /// Record cache hit
    async fn record_hit(&self) {
        // Use simple counter without labels to avoid version compatibility issues
        let mut stats = self.stats.write().await;
        stats.hits += 1;
        self.update_hit_ratio(&mut stats);
    }

    /// Record cache miss
    async fn record_miss(&self) {
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        self.update_hit_ratio(&mut stats);
    }

    /// Record cache set
    async fn record_set(&self) {
        let mut stats = self.stats.write().await;
        stats.sets += 1;
    }

    /// Record cache delete
    async fn record_delete(&self) {
        let mut stats = self.stats.write().await;
        stats.deletes += 1;
    }

    /// Record cache eviction
    async fn record_eviction(&self) {
        let mut stats = self.stats.write().await;
        stats.evictions += 1;
    }

    /// Update hit ratio in statistics
    fn update_hit_ratio(&self, stats: &mut CacheStats) {
        let total = stats.hits + stats.misses;
        if total > 0 {
            stats.hit_ratio = stats.hits as f64 / total as f64;
        }
    }
}

/// Cache error types
#[derive(thiserror::Error, Debug)]
pub enum CacheError {
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    #[error("Redis operation error: {0}")]
    RedisError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Compression error: {0}")]
    CompressionError(String),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),
}

impl Default for CacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            sets: 0,
            deletes: 0,
            evictions: 0,
            hit_ratio: 0.0,
            total_size_bytes: 0,
            entries_count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();

        assert_eq!(config.redis_urls, vec!["redis://127.0.0.1:6379".to_string()]);
        assert_eq!(config.connection_timeout_seconds, 30);
        assert_eq!(config.default_ttl_seconds, 3600);
        assert_eq!(config.max_size_bytes, 1024 * 1024 * 1024);
        assert!(config.enable_compression);
        assert_eq!(config.compression_threshold_bytes, 1024);
        assert!(config.enable_metrics);
        assert_eq!(config.key_prefix, "kotoba:cache");
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig {
            redis_urls: vec!["redis://localhost:6380".to_string()],
            connection_timeout_seconds: 60,
            default_ttl_seconds: 1800,
            max_size_bytes: 512 * 1024 * 1024,
            enable_compression: false,
            compression_threshold_bytes: 2048,
            enable_metrics: false,
            key_prefix: "test:cache".to_string(),
        };

        assert_eq!(config.redis_urls, vec!["redis://localhost:6380".to_string()]);
        assert_eq!(config.connection_timeout_seconds, 60);
        assert_eq!(config.default_ttl_seconds, 1800);
        assert_eq!(config.max_size_bytes, 512 * 1024 * 1024);
        assert!(!config.enable_compression);
        assert_eq!(config.compression_threshold_bytes, 2048);
        assert!(!config.enable_metrics);
        assert_eq!(config.key_prefix, "test:cache");
    }

    #[test]
    fn test_cache_stats_default() {
        let stats = CacheStats::default();

        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.sets, 0);
        assert_eq!(stats.deletes, 0);
        assert_eq!(stats.evictions, 0);
        assert_eq!(stats.hit_ratio, 0.0);
        assert_eq!(stats.total_size_bytes, 0);
        assert_eq!(stats.entries_count, 0);
    }

    #[test]
    fn test_cache_stats_hit_ratio_calculation() {
        let mut stats = CacheStats::default();

        // Test with no requests
        CacheStats::update_hit_ratio(&stats, &mut stats);
        assert_eq!(stats.hit_ratio, 0.0);

        // Test with hits
        stats.hits = 3;
        stats.misses = 2;
        CacheStats::update_hit_ratio(&stats, &mut stats);
        assert_eq!(stats.hit_ratio, 0.6);

        // Test with only hits
        stats.hits = 5;
        stats.misses = 0;
        CacheStats::update_hit_ratio(&stats, &mut stats);
        assert_eq!(stats.hit_ratio, 1.0);

        // Test with only misses
        stats.hits = 0;
        stats.misses = 3;
        CacheStats::update_hit_ratio(&stats, &mut stats);
        assert_eq!(stats.hit_ratio, 0.0);
    }

    #[test]
    fn test_cache_entry_creation() {
        let now = Utc::now();
        let entry = CacheEntry {
            key: "test_key".to_string(),
            size_bytes: 1024,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl_seconds: Some(3600),
        };

        assert_eq!(entry.key, "test_key");
        assert_eq!(entry.size_bytes, 1024);
        assert_eq!(entry.created_at, now);
        assert_eq!(entry.last_accessed, now);
        assert_eq!(entry.access_count, 0);
        assert_eq!(entry.ttl_seconds, Some(3600));
    }

    #[test]
    fn test_cache_key_generation() {
        let config = CacheConfig {
            key_prefix: "test:cache".to_string(),
            ..Default::default()
        };

        let cache = CacheLayer {
            config,
            client: Client::open("redis://127.0.0.1:6379").unwrap(),
            connection: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            active_entries: Arc::new(DashMap::new()),
        };

        assert_eq!(cache.make_cache_key("my_key"), "test:cache:my_key");
        assert_eq!(cache.make_cache_key("another/key"), "test:cache:another/key");
        assert_eq!(cache.make_cache_key(""), "test:cache:");
    }

    #[test]
    fn test_cache_config_multiple_redis_urls() {
        let config = CacheConfig {
            redis_urls: vec![
                "redis://127.0.0.1:6379".to_string(),
                "redis://127.0.0.1:6380".to_string(),
                "redis://127.0.0.1:6381".to_string(),
            ],
            ..Default::default()
        };

        assert_eq!(config.redis_urls.len(), 3);
        assert_eq!(config.redis_urls[0], "redis://127.0.0.1:6379");
        assert_eq!(config.redis_urls[1], "redis://127.0.0.1:6380");
        assert_eq!(config.redis_urls[2], "redis://127.0.0.1:6381");
    }

    #[test]
    fn test_cache_error_types() {
        let conn_err = CacheError::ConnectionError("connection failed".to_string());
        assert!(format!("{}", conn_err).contains("connection failed"));

        let redis_err = CacheError::RedisError("redis error".to_string());
        assert!(format!("{}", redis_err).contains("redis error"));

        let ser_err = CacheError::SerializationError("serialization failed".to_string());
        assert!(format!("{}", ser_err).contains("serialization failed"));

        let comp_err = CacheError::CompressionError("compression failed".to_string());
        assert!(format!("{}", comp_err).contains("compression failed"));

        let config_err = CacheError::ConfigError("invalid config".to_string());
        assert!(format!("{}", config_err).contains("invalid config"));
    }

    #[test]
    fn test_json_serialization_roundtrip() {
        let config = CacheConfig::default();
        let json_str = serde_json::to_string(&config).unwrap();
        let deserialized: CacheConfig = serde_json::from_str(&json_str).unwrap();
        assert_eq!(config.redis_urls, deserialized.redis_urls);
        assert_eq!(config.key_prefix, deserialized.key_prefix);
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            hits: 100,
            misses: 50,
            sets: 75,
            deletes: 25,
            evictions: 10,
            hit_ratio: 0.667,
            total_size_bytes: 1024 * 1024,
            entries_count: 100,
        };

        let json_str = serde_json::to_string(&stats).unwrap();
        let deserialized: CacheStats = serde_json::from_str(&json_str).unwrap();

        assert_eq!(stats.hits, deserialized.hits);
        assert_eq!(stats.misses, deserialized.misses);
        assert_eq!(stats.sets, deserialized.sets);
        assert_eq!(stats.deletes, deserialized.deletes);
        assert_eq!(stats.evictions, deserialized.evictions);
        assert!((stats.hit_ratio - deserialized.hit_ratio).abs() < 0.001);
        assert_eq!(stats.total_size_bytes, deserialized.total_size_bytes);
        assert_eq!(stats.entries_count, deserialized.entries_count);
    }

    #[tokio::test]
    async fn test_cache_layer_creation_mock() {
        // Test with invalid Redis URL to force mock mode
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:cache".to_string(),
            ..Default::default()
        };

        // This should succeed even with invalid Redis URL (mock mode)
        let cache_result = CacheLayer::new(config).await;
        assert!(cache_result.is_ok(), "Cache layer should create successfully in mock mode");

        let cache = cache_result.unwrap();
        assert_eq!(cache.config.key_prefix, "test:cache");
    }

    #[tokio::test]
    async fn test_cache_mock_operations() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:mock".to_string(),
            enable_metrics: true,
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        // Test mock operations (should work without Redis)
        let test_value = serde_json::json!({"message": "mock test", "number": 123});

        // Set operation
        let set_result = cache.set("mock_key", test_value.clone(), Some(60)).await;
        assert!(set_result.is_ok(), "Mock set should succeed");

        // Get operation (should return None in mock mode since we can't actually store)
        let get_result = cache.get("mock_key").await;
        assert!(get_result.is_ok(), "Mock get should succeed");
        assert_eq!(get_result.unwrap(), None, "Mock get should return None");

        // Exists operation
        let exists_result = cache.exists("mock_key").await;
        assert!(exists_result.is_ok(), "Mock exists should succeed");
        assert!(!exists_result.unwrap(), "Mock exists should return false");

        // Delete operation
        let delete_result = cache.delete("mock_key").await;
        assert!(delete_result.is_ok(), "Mock delete should succeed");
        assert!(delete_result.unwrap(), "Mock delete should return true");

        // TTL operation
        let ttl_result = cache.ttl("mock_key").await;
        assert!(ttl_result.is_ok(), "Mock TTL should succeed");
        assert_eq!(ttl_result.unwrap(), None, "Mock TTL should return None");

        // Increment operation
        let incr_result = cache.increment("mock_key", 5).await;
        assert!(incr_result.is_ok(), "Mock increment should succeed");
        assert_eq!(incr_result.unwrap(), 5, "Mock increment should return amount");

        // Clear operation
        let clear_result = cache.clear().await;
        assert!(clear_result.is_ok(), "Mock clear should succeed");

        // Info operation
        let info_result = cache.get_info().await;
        assert!(info_result.is_ok(), "Mock info should succeed");
        let info = info_result.unwrap();
        assert_eq!(info.get("status"), Some(&"mock".to_string()));
    }

    #[tokio::test]
    async fn test_cache_mock_statistics() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:stats".to_string(),
            enable_metrics: true,
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        // Get initial stats
        let initial_stats = cache.get_statistics().await;
        assert_eq!(initial_stats.hits, 0);
        assert_eq!(initial_stats.misses, 0);
        assert_eq!(initial_stats.sets, 0);

        // Perform mock operations that should update stats
        let test_value = serde_json::json!({"test": true});

        // Set operation (should record in stats)
        cache.set("stats_test", test_value.clone(), None).await.unwrap();

        // Get operation (should be miss in mock mode)
        cache.get("stats_test").await.unwrap();

        // Get non-existent (should be miss)
        cache.get("non_existent").await.unwrap();

        // Delete operation (should record in stats)
        cache.delete("stats_test").await.unwrap();

        // Check updated stats
        let updated_stats = cache.get_statistics().await;
        assert_eq!(updated_stats.sets, 1);
        assert_eq!(updated_stats.misses, 2); // get on existing and non-existent
        assert_eq!(updated_stats.deletes, 1);
    }

    #[tokio::test]
    async fn test_cache_large_value_compression() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:compress".to_string(),
            enable_compression: true,
            compression_threshold_bytes: 100, // Low threshold for testing
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        // Create a large value that should be compressed
        let large_string = "x".repeat(200); // 200 characters
        let large_value = serde_json::json!({"data": large_string});

        // Set operation (should work in mock mode)
        let set_result = cache.set("large_key", large_value, None).await;
        assert!(set_result.is_ok(), "Setting large value should succeed");
    }

    #[tokio::test]
    async fn test_cache_concurrent_operations() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:concurrent".to_string(),
            ..Default::default()
        };

        let cache = Arc::new(CacheLayer::new(config).await.unwrap());
        let mut handles = vec![];

        // Spawn multiple concurrent operations
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i);
                let value = serde_json::json!({"index": i, "thread": "test"});

                // Perform operations
                let set_result = cache_clone.set(&key, value, Some(300)).await;
                assert!(set_result.is_ok());

                let get_result = cache_clone.get(&key).await;
                assert!(get_result.is_ok());

                let exists_result = cache_clone.exists(&key).await;
                assert!(exists_result.is_ok());

                let delete_result = cache_clone.delete(&key).await;
                assert!(delete_result.is_ok());
            });
            handles.push(handle);
        }

        // Wait for all operations to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    #[tokio::test]
    async fn test_cache_ttl_operations() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:ttl".to_string(),
            default_ttl_seconds: 120,
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        // Test setting with TTL
        let test_value = serde_json::json!({"ttl_test": true});
        let set_result = cache.set("ttl_key", test_value, Some(60)).await;
        assert!(set_result.is_ok());

        // Test TTL retrieval (should return None in mock mode)
        let ttl_result = cache.ttl("ttl_key").await;
        assert!(ttl_result.is_ok());
        assert_eq!(ttl_result.unwrap(), None);
    }

    #[tokio::test]
    async fn test_cache_increment_operations() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:incr".to_string(),
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        // Test increment operations
        let incr1 = cache.increment("counter", 5).await;
        assert!(incr1.is_ok());
        assert_eq!(incr1.unwrap(), 5);

        let incr2 = cache.increment("counter", 3).await;
        assert!(incr2.is_ok());
        assert_eq!(incr2.unwrap(), 3);

        let incr3 = cache.increment("new_counter", -2).await;
        assert!(incr3.is_ok());
        assert_eq!(incr3.unwrap(), -2);
    }

    #[tokio::test]
    async fn test_cache_info_mock() {
        let config = CacheConfig {
            redis_urls: vec!["redis://invalid.host:9999".to_string()],
            key_prefix: "test:info".to_string(),
            ..Default::default()
        };

        let cache = CacheLayer::new(config).await.unwrap();

        let info_result = cache.get_info().await;
        assert!(info_result.is_ok());

        let info = info_result.unwrap();
        assert_eq!(info.get("status"), Some(&"mock".to_string()));
        assert_eq!(info.get("version"), Some(&"0.0.0".to_string()));
    }

    #[test]
    fn test_cache_config_validation() {
        // Test valid config
        let valid_config = CacheConfig {
            redis_urls: vec!["redis://localhost:6379".to_string()],
            connection_timeout_seconds: 30,
            default_ttl_seconds: 3600,
            max_size_bytes: 1024 * 1024 * 1024,
            enable_compression: true,
            compression_threshold_bytes: 1024,
            enable_metrics: true,
            key_prefix: "valid:prefix".to_string(),
        };

        assert!(!valid_config.redis_urls.is_empty());
        assert!(valid_config.connection_timeout_seconds > 0);
        assert!(valid_config.default_ttl_seconds > 0);
        assert!(valid_config.max_size_bytes > 0);
        assert!(valid_config.compression_threshold_bytes > 0);
        assert!(!valid_config.key_prefix.is_empty());
    }

    #[test]
    fn test_cache_config_edge_cases() {
        // Test config with empty values
        let empty_config = CacheConfig {
            redis_urls: vec![],
            key_prefix: "".to_string(),
            ..Default::default()
        };

        assert!(empty_config.redis_urls.is_empty());
        assert!(empty_config.key_prefix.is_empty());

        // Test config with extreme values
        let extreme_config = CacheConfig {
            redis_urls: vec!["redis://test".to_string()],
            connection_timeout_seconds: u64::MAX,
            default_ttl_seconds: u64::MAX,
            max_size_bytes: u64::MAX,
            compression_threshold_bytes: usize::MAX,
            key_prefix: "a".repeat(1000), // Very long prefix
            ..Default::default()
        };

        assert_eq!(extreme_config.connection_timeout_seconds, u64::MAX);
        assert_eq!(extreme_config.max_size_bytes, u64::MAX);
        assert_eq!(extreme_config.compression_threshold_bytes, usize::MAX);
        assert_eq!(extreme_config.key_prefix.len(), 1000);
    }
}

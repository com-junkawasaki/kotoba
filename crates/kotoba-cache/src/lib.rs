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
    /// Redis connection URLs (single node or cluster)
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
    /// Connection pool
    connections: Arc<RwLock<Vec<Connection>>>,
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

        // Create Redis client
        let client = Client::open(config.redis_urls[0].clone())
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        // Test connection
        let mut connection = client.get_async_connection().await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        let _: String = redis::cmd("PING").query_async(&mut connection).await
            .map_err(|e| CacheError::ConnectionError(e.to_string()))?;

        info!("Redis connection established successfully");

        let cache = Self {
            config,
            client,
            connections: Arc::new(RwLock::new(vec![connection])),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            active_entries: Arc::new(DashMap::new()),
        };

        Ok(cache)
    }

    /// Get a value from cache
    #[instrument(skip(self))]
    pub async fn get(&self, key: &str) -> Result<Option<serde_json::Value>, CacheError> {
        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        match connection.get::<_, Option<Vec<u8>>>(&cache_key).await {
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
                    histogram!("cache.entry_size", data.len() as f64);
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
            connection.set_ex(&cache_key, final_data, ttl as usize).await
        } else {
            connection.set(&cache_key, final_data).await
        };

        match result {
            Ok(()) => {
                // Update statistics
                if self.config.enable_metrics {
                    self.record_set().await;
                    histogram!("cache.entry_size", serialized_data.len() as f64);
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
        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        match connection.del(&cache_key).await {
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
        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        match connection.exists(&cache_key).await {
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
        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        match connection.ttl(&cache_key).await {
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
        let cache_key = self.make_cache_key(key);
        let mut connection = self.get_connection().await?;

        match connection.incr(&cache_key, amount).await {
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
        let pattern = format!("{}:*", self.config.key_prefix);
        let mut connection = self.get_connection().await?;

        // Get all keys matching the pattern
        let keys: Vec<String> = connection.keys(&pattern).await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        if !keys.is_empty() {
            // Delete all keys
            let _: () = connection.del(keys).await
                .map_err(|e| CacheError::RedisError(e.to_string()))?;
        }

        // Clear local tracking
        self.active_entries.clear();

        if self.config.enable_metrics {
            let mut stats = self.stats.write().await;
            stats.deletes += keys.len() as u64;
        }

        info!("Cache cleared, {} keys deleted", keys.len());
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get cache information
    #[instrument(skip(self))]
    pub async fn get_info(&self) -> Result<HashMap<String, String>, CacheError> {
        let mut connection = self.get_connection().await?;

        let info: String = redis::cmd("INFO").query_async(&mut connection).await
            .map_err(|e| CacheError::RedisError(e.to_string()))?;

        // Parse INFO command output
        let mut info_map = HashMap::new();
        for line in info.lines() {
            if let Some((key, value)) = line.split_once(':') {
                info_map.insert(key.to_string(), value.to_string());
            }
        }

        Ok(info_map)
    }

    /// Get a Redis connection from the pool
    async fn get_connection(&self) -> Result<redis::aio::Connection, CacheError> {
        let mut connections = self.connections.write().await;

        if let Some(connection) = connections.pop() {
            Ok(connection)
        } else {
            // Create new connection
            self.client.get_async_connection().await
                .map_err(|e| CacheError::ConnectionError(e.to_string()))
        }
    }

    /// Return a connection to the pool
    async fn return_connection(&self, connection: redis::aio::Connection) {
        let mut connections = self.connections.write().await;
        if connections.len() < 10 { // Max pool size
            connections.push(connection);
        }
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
        counter!("cache.hits", 1);
        let mut stats = self.stats.write().await;
        stats.hits += 1;
        self.update_hit_ratio(&mut stats);
    }

    /// Record cache miss
    async fn record_miss(&self) {
        counter!("cache.misses", 1);
        let mut stats = self.stats.write().await;
        stats.misses += 1;
        self.update_hit_ratio(&mut stats);
    }

    /// Record cache set
    async fn record_set(&self) {
        counter!("cache.sets", 1);
        let mut stats = self.stats.write().await;
        stats.sets += 1;
    }

    /// Record cache delete
    async fn record_delete(&self) {
        counter!("cache.deletes", 1);
        let mut stats = self.stats.write().await;
        stats.deletes += 1;
    }

    /// Record cache eviction
    async fn record_eviction(&self) {
        counter!("cache.evictions", 1);
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

    #[tokio::test]
    async fn test_cache_layer_creation() {
        let config = CacheConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            ..Default::default()
        };

        // Skip test if Redis is not available
        let cache_result = CacheLayer::new(config).await;
        match cache_result {
            Ok(_) => println!("Cache layer created successfully"),
            Err(e) => {
                println!("Skipping cache test (Redis not available): {}", e);
                return;
            }
        }
    }

    #[tokio::test]
    async fn test_cache_operations() {
        let config = CacheConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            key_prefix: "test:cache".to_string(),
            ..Default::default()
        };

        let cache_result = CacheLayer::new(config).await;
        if cache_result.is_err() {
            println!("Skipping cache test (Redis not available)");
            return;
        }

        let cache = cache_result.unwrap();

        // Test set and get
        let test_value = serde_json::json!({"message": "hello world", "count": 42});
        let set_result = cache.set("test_key", test_value.clone(), Some(60)).await;
        assert!(set_result.is_ok(), "Should be able to set cache value");

        let get_result = cache.get("test_key").await;
        assert!(get_result.is_ok(), "Should be able to get cache value");
        assert_eq!(get_result.unwrap(), Some(test_value));

        // Test exists
        let exists_result = cache.exists("test_key").await;
        assert!(exists_result.is_ok(), "Should be able to check existence");
        assert!(exists_result.unwrap(), "Key should exist");

        // Test delete
        let delete_result = cache.delete("test_key").await;
        assert!(delete_result.is_ok(), "Should be able to delete cache value");
        assert!(delete_result.unwrap(), "Delete should return true");

        // Test exists after delete
        let exists_after_delete = cache.exists("test_key").await;
        assert!(exists_after_delete.is_ok(), "Should be able to check existence after delete");
        assert!(!exists_after_delete.unwrap(), "Key should not exist after delete");
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = CacheConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            key_prefix: "test:stats".to_string(),
            ..Default::default()
        };

        let cache_result = CacheLayer::new(config).await;
        if cache_result.is_err() {
            println!("Skipping cache test (Redis not available)");
            return;
        }

        let cache = cache_result.unwrap();

        // Get initial stats
        let initial_stats = cache.get_statistics().await;
        assert_eq!(initial_stats.hits, 0);
        assert_eq!(initial_stats.misses, 0);

        // Perform operations
        let test_value = serde_json::json!({"test": true});
        cache.set("stats_test", test_value.clone(), None).await.unwrap();

        // Get value (should be hit)
        cache.get("stats_test").await.unwrap();

        // Try to get non-existent value (should be miss)
        cache.get("non_existent").await.unwrap();

        // Check updated stats
        let updated_stats = cache.get_statistics().await;
        assert_eq!(updated_stats.sets, 1);
        assert_eq!(updated_stats.hits, 1);
        assert_eq!(updated_stats.misses, 1);
        assert_eq!(updated_stats.hit_ratio, 0.5);
    }
}

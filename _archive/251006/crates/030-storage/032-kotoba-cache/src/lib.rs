//! Kotoba Cache Layer
//!
//! Pure cache management following the Pure Kernel/Effects Shell pattern.
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate provides cache management with clear separation:
//!
//! - **Pure Kernel**: `CachePolicy`, `CacheKey`, `CacheEntry` - pure cache logic and data structures
//! - **Effects Shell**: `CacheManager` - handles actual storage and eviction with I/O
//!
//! ## Key Features
//!
//! - **Multiple Policies**: LRU, LFU, TTL-based eviction
//! - **Pure Logic**: Cache policies are pure functions
//! - **Thread-safe**: Concurrent access support
//! - **Configurable**: Pluggable storage backends

use kotoba_storage::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

/// Pure cache key representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    /// Cache namespace
    pub namespace: String,
    /// Cache key
    pub key: String,
}

impl CacheKey {
    /// Create a new cache key
    pub fn new(namespace: impl Into<String>, key: impl Into<String>) -> Self {
        Self {
            namespace: namespace.into(),
            key: key.into(),
        }
    }

    /// Convert to storage key
    pub fn to_storage_key(&self) -> StorageKey {
        StorageKey::new(format!("cache:{}", self.namespace), self.key.clone())
    }
}

/// Pure cache entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The cached value
    pub value: serde_json::Value,
    /// When this entry was created
    pub created_at: u64,
    /// When this entry expires (None = never)
    pub expires_at: Option<u64>,
    /// Access count for LFU policy
    pub access_count: u64,
    /// Last access time for LRU policy
    pub last_accessed: u64,
    /// Size estimate in bytes
    pub size_bytes: usize,
}

impl CacheEntry {
    /// Create a new cache entry
    pub fn new(value: serde_json::Value) -> Self {
        let now = Self::current_timestamp();
        Self {
            value,
            created_at: now,
            expires_at: None,
            access_count: 1,
            last_accessed: now,
            size_bytes: Self::estimate_size(&value),
        }
    }

    /// Create a cache entry with TTL
    pub fn with_ttl(value: serde_json::Value, ttl: Duration) -> Self {
        let now = Self::current_timestamp();
        Self {
            value,
            created_at: now,
            expires_at: Some(now + ttl.as_millis() as u64),
            access_count: 1,
            last_accessed: now,
            size_bytes: Self::estimate_size(&value),
        }
    }

    /// Check if entry is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Self::current_timestamp() > exp)
    }

    /// Mark as accessed (pure operation)
    pub fn accessed(&self) -> Self {
        let mut updated = self.clone();
        updated.access_count += 1;
        updated.last_accessed = Self::current_timestamp();
        updated
    }

    /// Get current timestamp (in milliseconds)
    fn current_timestamp() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    /// Estimate size of a JSON value
    fn estimate_size(value: &serde_json::Value) -> usize {
        match value {
            serde_json::Value::Null => 4,
            serde_json::Value::Bool(_) => 1,
            serde_json::Value::Number(n) => n.to_string().len(),
            serde_json::Value::String(s) => s.len(),
            serde_json::Value::Array(arr) => arr.iter().map(Self::estimate_size).sum::<usize>() + 8,
            serde_json::Value::Object(obj) => {
                obj.iter()
                    .map(|(k, v)| k.len() + Self::estimate_size(v))
                    .sum::<usize>() + 16
            }
        }
    }
}

/// Pure cache eviction policies
#[derive(Debug, Clone, PartialEq)]
pub enum CachePolicy {
    /// Least Recently Used - evict oldest accessed entries
    LRU,
    /// Least Frequently Used - evict least accessed entries
    LFU,
    /// First In First Out - evict oldest entries
    FIFO,
    /// Size-based - evict largest entries when over limit
    SizeBased,
    /// TTL-based - evict expired entries only
    TTLOnly,
}

/// Pure cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Maximum total size in bytes
    pub max_size_bytes: usize,
    /// Default TTL for entries
    pub default_ttl: Option<Duration>,
    /// Eviction policy
    pub policy: CachePolicy,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 10 * 1024 * 1024, // 10MB
            default_ttl: None,
            policy: CachePolicy::LRU,
        }
    }
}

/// Pure cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of entries
    pub entries: usize,
    /// Total size in bytes
    pub size_bytes: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
}

/// Pure cache operations - no side effects
pub struct PureCache {
    config: CacheConfig,
}

impl PureCache {
    /// Create a new pure cache
    pub fn new(config: CacheConfig) -> Self {
        Self { config }
    }

    /// Pure function to check if an entry should be evicted
    pub fn should_evict(&self, entry: &CacheEntry, all_entries: &[CacheEntry], stats: &CacheStats) -> bool {
        // Check TTL first
        if entry.is_expired() {
            return true;
        }

        // Check size and entry limits
        if stats.entries >= self.config.max_entries || stats.size_bytes >= self.config.max_size_bytes {
            match self.config.policy {
                CachePolicy::LRU => {
                    // Find oldest accessed entry
                    all_entries.iter().all(|e| e.last_accessed >= entry.last_accessed)
                }
                CachePolicy::LFU => {
                    // Find least frequently used entry
                    all_entries.iter().all(|e| e.access_count >= entry.access_count)
                }
                CachePolicy::FIFO => {
                    // Find oldest entry
                    all_entries.iter().all(|e| e.created_at >= entry.created_at)
                }
                CachePolicy::SizeBased => {
                    // Find largest entry
                    all_entries.iter().all(|e| e.size_bytes >= entry.size_bytes)
                }
                CachePolicy::TTLOnly => false, // TTL already checked above
            }
        } else {
            false
        }
    }

    /// Pure function to compute new statistics after operation
    pub fn compute_stats(&self, entries: &[CacheEntry]) -> CacheStats {
        CacheStats {
            entries: entries.len(),
            size_bytes: entries.iter().map(|e| e.size_bytes).sum(),
            hits: 0, // This would be tracked separately
            misses: 0,
            evictions: 0,
        }
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }
}

/// Effects Shell cache manager - handles I/O and state
pub struct CacheManager {
    /// Pure cache logic
    pure_cache: PureCache,
    /// Underlying storage engine
    storage: Box<dyn StorageEngine + Send + Sync>,
    /// Current cache statistics
    stats: std::sync::RwLock<CacheStats>,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(config: CacheConfig, storage: Box<dyn StorageEngine + Send + Sync>) -> Self {
        Self {
            pure_cache: PureCache::new(config),
            storage,
            stats: std::sync::RwLock::new(CacheStats::default()),
        }
    }

    /// Get a value from cache (effects: I/O)
    pub async fn get(&self, key: &CacheKey) -> Result<Option<serde_json::Value>, CacheError> {
        let storage_key = key.to_storage_key();
        let plan = StoragePlan::single(StorageOperation::Get(storage_key));

        match self.storage.execute_plan(&plan).await {
            Ok(result) => {
                match &result.results[0] {
                    OperationResult::Get(Some(value)) => {
                        // Deserialize cache entry
                        let entry: CacheEntry = serde_json::from_value(value.data.clone())
                            .map_err(|e| CacheError::SerializationError(e.to_string()))?;

                        if entry.is_expired() {
                            // Expired - delete and return None
                            self.delete(key).await?;
                            self.update_stats(|stats| stats.misses += 1);
                            Ok(None)
                        } else {
                            // Valid - update access and return value
                            let updated_entry = entry.accessed();
                            let updated_value = serde_json::to_value(&updated_entry)
                                .map_err(|e| CacheError::SerializationError(e.to_string()))?;
                            let storage_value = StorageValue::new(updated_value);

                            let put_plan = StoragePlan::single(StorageOperation::Put(
                                key.to_storage_key(),
                                storage_value,
                            ));
                            self.storage.execute_plan(&put_plan).await?;

                            self.update_stats(|stats| stats.hits += 1);
                            Ok(Some(entry.value))
                        }
                    }
                    OperationResult::Get(None) => {
                        self.update_stats(|stats| stats.misses += 1);
                        Ok(None)
                    }
                    _ => Err(CacheError::StorageError("Unexpected result".to_string())),
                }
            }
            Err(e) => Err(CacheError::StorageError(format!("{:?}", e))),
        }
    }

    /// Put a value in cache (effects: I/O)
    pub async fn put(&self, key: CacheKey, value: serde_json::Value) -> Result<(), CacheError> {
        let ttl = self.pure_cache.config().default_ttl;
        let entry = if let Some(ttl) = ttl {
            CacheEntry::with_ttl(value, ttl)
        } else {
            CacheEntry::new(value)
        };

        let storage_value = StorageValue::new(
            serde_json::to_value(&entry)
                .map_err(|e| CacheError::SerializationError(e.to_string()))?
        );

        let plan = StoragePlan::single(StorageOperation::Put(key.to_storage_key(), storage_value));

        self.storage.execute_plan(&plan).await?;
        self.update_stats(|stats| stats.entries += 1);

        // Trigger eviction if needed
        self.evict_if_needed().await?;

        Ok(())
    }

    /// Delete a value from cache (effects: I/O)
    pub async fn delete(&self, key: &CacheKey) -> Result<bool, CacheError> {
        let plan = StoragePlan::single(StorageOperation::Delete(key.to_storage_key()));

        match self.storage.execute_plan(&plan).await {
            Ok(result) => {
                match &result.results[0] {
                    OperationResult::Delete(existed) => {
                        if *existed {
                            self.update_stats(|stats| stats.entries = stats.entries.saturating_sub(1));
                        }
                        Ok(*existed)
                    }
                    _ => Err(CacheError::StorageError("Unexpected result".to_string())),
                }
            }
            Err(e) => Err(CacheError::StorageError(format!("{:?}", e))),
        }
    }

    /// Clear all cache entries (effects: I/O)
    pub async fn clear(&self) -> Result<(), CacheError> {
        // List all cache entries and delete them
        let query = QueryPlan {
            namespace: "cache".to_string(), // This would need to be adjusted based on actual storage
            conditions: vec![],
            sort_by: None,
            limit: None,
            offset: None,
        };

        match self.storage.execute_query(&query).await {
            Ok(result) => {
                for (key, _) in result.values {
                    let delete_plan = StoragePlan::single(StorageOperation::Delete(key));
                    self.storage.execute_plan(&delete_plan).await?;
                }
                self.update_stats(|stats| {
                    stats.entries = 0;
                    stats.size_bytes = 0;
                });
                Ok(())
            }
            Err(e) => Err(CacheError::StorageError(format!("{:?}", e))),
        }
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.read().unwrap().clone()
    }

    /// Trigger eviction if cache limits are exceeded (effects: I/O)
    async fn evict_if_needed(&self) -> Result<(), CacheError> {
        // In a real implementation, this would check current stats and evict entries
        // For now, this is a placeholder
        Ok(())
    }

    /// Update statistics
    fn update_stats<F>(&self, updater: F)
    where
        F: FnOnce(&mut CacheStats),
    {
        let mut stats = self.stats.write().unwrap();
        updater(&mut stats);
    }
}

/// Cache operation errors
#[derive(Debug, Clone)]
pub enum CacheError {
    StorageError(String),
    SerializationError(String),
    KeyNotFound(CacheKey),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_cache_key_creation() {
        let key = CacheKey::new("users", "alice");
        assert_eq!(key.namespace, "users");
        assert_eq!(key.key, "alice");

        let storage_key = key.to_storage_key();
        assert_eq!(storage_key.namespace, "cache:users");
        assert_eq!(storage_key.key, "alice");
    }

    #[test]
    fn test_cache_entry_creation() {
        let value = json!({"name": "Alice", "age": 30});
        let entry = CacheEntry::new(value.clone());

        assert_eq!(entry.value, value);
        assert!(!entry.is_expired());
        assert_eq!(entry.access_count, 1);
        assert!(entry.size_bytes > 0);
    }

    #[test]
    fn test_cache_entry_with_ttl() {
        let value = json!("test");
        let ttl = Duration::from_secs(60);
        let entry = CacheEntry::with_ttl(value.clone(), ttl);

        assert_eq!(entry.value, value);
        assert!(!entry.is_expired());
        assert!(entry.expires_at.is_some());
    }

    #[test]
    fn test_cache_entry_accessed() {
        let entry = CacheEntry::new(json!("test"));
        let accessed = entry.accessed();

        assert_eq!(accessed.access_count, 2);
        assert!(accessed.last_accessed >= entry.last_accessed);
    }

    #[test]
    fn test_pure_cache_should_evict() {
        let config = CacheConfig {
            max_entries: 2,
            max_size_bytes: 1000,
            default_ttl: None,
            policy: CachePolicy::LRU,
        };

        let pure_cache = PureCache::new(config);

        let entry1 = CacheEntry::new(json!("small"));
        let entry2 = CacheEntry::new(json!("small"));

        // With 2 entries and max of 2, should not evict
        let stats = CacheStats { entries: 2, size_bytes: 10, hits: 0, misses: 0, evictions: 0 };
        assert!(!pure_cache.should_evict(&entry1, &[entry1.clone(), entry2.clone()], &stats));
    }

    #[test]
    fn test_pure_cache_compute_stats() {
        let config = CacheConfig::default();
        let pure_cache = PureCache::new(config);

        let entries = vec![
            CacheEntry::new(json!("test1")),
            CacheEntry::new(json!("test2")),
        ];

        let stats = pure_cache.compute_stats(&entries);
        assert_eq!(stats.entries, 2);
        assert!(stats.size_bytes > 0);
    }
}

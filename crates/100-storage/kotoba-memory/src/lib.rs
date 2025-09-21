//! KotobaDB Memory Storage and Optimization
//!
//! Provides in-memory storage implementations and advanced memory management:
//! - In-memory KeyValueStore implementation
//! - Memory pooling for reduced allocation overhead
//! - Intelligent caching with multiple strategies
//! - Memory profiling and leak detection
//! - Garbage collection optimization
//! - Custom allocators and memory layouts

pub mod memory_pool;
pub mod cache_manager;
pub mod memory_profiler;
pub mod gc_optimizer;
pub mod allocators;

use std::collections::HashMap;
use std::sync::Arc;
use serde::{Deserialize, Serialize};

/// Memory optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Enable memory pooling
    pub enable_pooling: bool,

    /// Memory pool size in MB
    pub pool_size_mb: usize,

    /// Enable intelligent caching
    pub enable_caching: bool,

    /// Cache size limit in MB
    pub cache_size_mb: usize,

    /// Cache eviction policy
    pub cache_policy: CachePolicy,

    /// Enable custom allocators
    pub enable_custom_allocators: bool,

    /// Allocator type
    pub allocator_type: AllocatorType,

    /// Enable GC optimization
    pub enable_gc_optimization: bool,

    /// Target memory usage percentage
    pub target_memory_usage_percent: f64,

    /// Memory monitoring interval in milliseconds
    pub monitoring_interval_ms: u64,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            enable_pooling: true,
            pool_size_mb: 256,
            enable_caching: true,
            cache_size_mb: 512,
            cache_policy: CachePolicy::Lru,
            enable_custom_allocators: false,
            allocator_type: AllocatorType::System,
            enable_gc_optimization: true,
            target_memory_usage_percent: 75.0,
            monitoring_interval_ms: 1000,
        }
    }
}

/// Cache eviction policies
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CachePolicy {
    Lru,
    Lfu,
    Fifo,
    Adaptive,
}

/// Allocator types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AllocatorType {
    System,
    Jemalloc,
    Mimalloc,
    Custom,
}

/// Hybrid storage mode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HybridMode {
    /// Memory-only storage
    MemoryOnly,
    /// Redis-only storage
    RedisOnly,
    /// Memory with Redis backup
    MemoryWithRedisBackup,
    /// Tiered storage (hot data in memory, cold data in Redis)
    TieredMemoryRedis,
}

/// Memory optimization manager
pub struct MemoryOptimizer {
    config: MemoryConfig,
    memory_pool: Option<memory_pool::MemoryPool>,
    cache_manager: Option<cache_manager::CacheManager>,
    memory_profiler: Option<memory_profiler::MemoryProfiler>,
    gc_optimizer: Option<gc_optimizer::GcOptimizer>,
    allocator: Option<Box<dyn allocators::Allocator>>,
    #[cfg(feature = "redis")]
    redis_store: Option<std::sync::Arc<kotoba_storage_redis::RedisStore>>,
    hybrid_mode: HybridMode,
}

impl MemoryOptimizer {
    /// Create a new memory optimizer with default configuration
    pub fn new() -> Self {
        Self::with_config(MemoryConfig::default())
    }

    /// Create a memory optimizer with custom configuration
    pub fn with_config(config: MemoryConfig) -> Self {
        Self::with_hybrid_config(config, HybridMode::MemoryOnly)
    }

    /// Create a memory optimizer with hybrid configuration
    pub fn with_hybrid_config(config: MemoryConfig, hybrid_mode: HybridMode) -> Self {
        let mut optimizer = Self {
            config: config.clone(),
            memory_pool: None,
            cache_manager: None,
            memory_profiler: None,
            gc_optimizer: None,
            allocator: None,
            #[cfg(feature = "redis")]
            redis_store: None,
            hybrid_mode,
        };

        optimizer.initialize_components();
        optimizer
    }

    /// Create hybrid memory optimizer with Redis
    #[cfg(feature = "redis")]
    pub async fn with_redis(
        config: MemoryConfig,
        redis_config: kotoba_storage_redis::RedisConfig,
        hybrid_mode: HybridMode
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let redis_store = std::sync::Arc::new(
            kotoba_storage_redis::RedisStore::new(redis_config).await?
        );

        let mut optimizer = Self {
            config: config.clone(),
            memory_pool: None,
            cache_manager: None,
            memory_profiler: None,
            gc_optimizer: None,
            allocator: None,
            redis_store: Some(redis_store),
            hybrid_mode,
        };

        optimizer.initialize_components();
        Ok(optimizer)
    }

    /// Initialize memory optimization components
    fn initialize_components(&mut self) {
        // Initialize memory pool
        if self.config.enable_pooling {
            self.memory_pool = Some(memory_pool::MemoryPool::new(
                self.config.pool_size_mb * 1024 * 1024,
            ));
        }

        // Initialize cache manager
        if self.config.enable_caching {
            self.cache_manager = Some(cache_manager::CacheManager::new(
                self.config.cache_size_mb * 1024 * 1024,
                match self.config.cache_policy {
                    CachePolicy::Lru => cache_manager::CachePolicy::Lru,
                    CachePolicy::Lfu => cache_manager::CachePolicy::Lfu,
                    CachePolicy::Fifo => cache_manager::CachePolicy::Lru, // Default to LRU for now
                    CachePolicy::Adaptive => cache_manager::CachePolicy::Adaptive,
                },
            ));
        }

        // Initialize memory profiler
        self.memory_profiler = Some(memory_profiler::MemoryProfiler::new());

        // Initialize GC optimizer
        if self.config.enable_gc_optimization {
            self.gc_optimizer = Some(gc_optimizer::GcOptimizer::new());
        }

        // Initialize custom allocator
        if self.config.enable_custom_allocators {
            match self.config.allocator_type {
                AllocatorType::Jemalloc => {
                    #[cfg(feature = "jemalloc")]
                    {
                        self.allocator = Some(Box::new(allocators::JemallocAllocator::new()));
                    }
                }
                AllocatorType::Mimalloc => {
                    #[cfg(feature = "mimalloc")]
                    {
                        self.allocator = Some(Box::new(allocators::MimallocAllocator::new()));
                    }
                }
                AllocatorType::Custom => {
                    self.allocator = Some(Box::new(allocators::CustomAllocator::new()));
                }
                _ => {}
            }
        }
    }

    /// Start memory optimization
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🚀 Starting KotobaDB memory optimization...");

        if let Some(ref mut profiler) = self.memory_profiler {
            profiler.start().await?;
        }

        if let Some(ref mut gc_optimizer) = self.gc_optimizer {
            gc_optimizer.start().await?;
        }

        // Start monitoring loop
        self.start_monitoring().await;

        println!("✅ Memory optimization started");
        Ok(())
    }

    /// Stop memory optimization
    pub async fn stop(&mut self) -> Result<MemoryReport, Box<dyn std::error::Error>> {
        println!("🛑 Stopping memory optimization...");

        if let Some(ref mut profiler) = self.memory_profiler {
            profiler.stop().await?;
        }

        if let Some(ref mut gc_optimizer) = self.gc_optimizer {
            gc_optimizer.stop().await?;
        }

        let report = self.generate_report().await?;
        println!("✅ Memory optimization stopped");

        Ok(report)
    }

    /// Allocate memory from pool
    pub fn allocate(&self, size: usize) -> Result<memory_pool::MemoryBlock, Box<dyn std::error::Error>> {
        if let Some(ref pool) = self.memory_pool {
            pool.allocate(size)
        } else {
            Err("Memory pooling not enabled".into())
        }
    }

    /// Get value from cache
    pub fn get_cached(&self, key: &str) -> Option<cache_manager::CachedValue> {
        self.cache_manager.as_ref()?.get(key)
    }

    /// Put value in cache
    pub fn put_cached(&self, key: String, value: cache_manager::CachedValue) {
        if let Some(ref manager) = self.cache_manager {
            manager.put(key, value);
        }
    }

    /// Get memory statistics
    pub async fn memory_stats(&self) -> MemoryStats {
        let pool_stats = self.memory_pool.as_ref().map(|p| p.stats());
        let cache_stats = self.cache_manager.as_ref().map(|c| c.stats());
        let profiler_stats = if let Some(ref profiler) = self.memory_profiler {
            profiler.current_stats().await.profiler_stats
        } else {
            memory_profiler::MemoryStats::default()
        };

        MemoryStats {
            pool_stats,
            cache_stats,
            profiler_stats,
            total_memory_mb: self.get_total_memory_mb(),
            available_memory_mb: self.get_available_memory_mb(),
            memory_efficiency: self.calculate_memory_efficiency(),
        }
    }

    /// Generate optimization recommendations
    pub async fn get_recommendations(&self) -> Vec<MemoryRecommendation> {
        let mut recommendations = Vec::new();
        let stats = self.memory_stats().await;

        // Memory usage recommendations
        if stats.memory_efficiency < 0.7 {
            recommendations.push(MemoryRecommendation {
                category: RecommendationCategory::Efficiency,
                priority: Priority::High,
                title: "Low Memory Efficiency Detected".to_string(),
                description: format!("Memory efficiency is {:.1}%. Consider memory optimization.", stats.memory_efficiency * 100.0),
                actions: vec![
                    "Enable memory pooling if not already enabled".to_string(),
                    "Review data structures for memory overhead".to_string(),
                    "Implement object reuse patterns".to_string(),
                ],
            });
        }

        // Cache recommendations
        if let Some(cache_stats) = stats.cache_stats {
            if cache_stats.hit_rate < 0.5 {
                recommendations.push(MemoryRecommendation {
                    category: RecommendationCategory::Caching,
                    priority: Priority::Medium,
                    title: "Low Cache Hit Rate".to_string(),
                    description: format!("Cache hit rate is {:.1}%. Consider cache optimization.", cache_stats.hit_rate * 100.0),
                    actions: vec![
                        "Review cache key distribution".to_string(),
                        "Increase cache size if possible".to_string(),
                        "Implement cache warming strategies".to_string(),
                    ],
                });
            }
        }

        // Pool recommendations
        if let Some(pool_stats) = stats.pool_stats {
            if pool_stats.utilization < 0.6 {
                recommendations.push(MemoryRecommendation {
                    category: RecommendationCategory::Pooling,
                    priority: Priority::Low,
                    title: "Low Memory Pool Utilization".to_string(),
                    description: format!("Memory pool utilization is {:.1}%. Consider pool size adjustment.", pool_stats.utilization * 100.0),
                    actions: vec![
                        "Reduce pool size to free memory".to_string(),
                        "Review allocation patterns".to_string(),
                        "Consider different pool strategies".to_string(),
                    ],
                });
            }
        }

        recommendations
    }

    /// Force garbage collection optimization
    pub async fn optimize_gc(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref gc_optimizer) = self.gc_optimizer {
            gc_optimizer.optimize().await
        } else {
            Ok(())
        }
    }

    /// Get hybrid storage mode
    pub fn hybrid_mode(&self) -> &HybridMode {
        &self.hybrid_mode
    }

    /// Check if Redis is available
    #[cfg(feature = "redis")]
    pub async fn is_redis_available(&self) -> bool {
        if let Some(ref redis) = self.redis_store {
            redis.is_connected().await
        } else {
            false
        }
    }

    /// Get Redis statistics
    #[cfg(feature = "redis")]
    pub async fn redis_stats(&self) -> Option<kotoba_storage_redis::RedisStats> {
        if let Some(ref redis) = self.redis_store {
            Some(redis.get_stats().await)
        } else {
            None
        }
    }

    /// Hybrid put operation (chooses storage based on hybrid mode)
    #[cfg(feature = "redis")]
    pub async fn hybrid_put(&self, key: &[u8], value: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        match self.hybrid_mode {
            HybridMode::MemoryOnly => {
                if let Some(ref memory_store) = self.memory_pool {
                    // This is simplified - in practice you'd need proper memory storage
                    Ok(())
                } else {
                    Err("Memory pool not enabled".into())
                }
            }
            HybridMode::RedisOnly => {
                if let Some(ref redis) = self.redis_store {
                    redis.put(key, value).await?;
                    Ok(())
                } else {
                    Err("Redis not configured".into())
                }
            }
            HybridMode::MemoryWithRedisBackup => {
                // Try memory first, then Redis as backup
                let memory_result = if let Some(ref memory_store) = self.memory_pool {
                    Ok(())
                } else {
                    Err("Memory pool not enabled")
                };

                if memory_result.is_err() {
                    if let Some(ref redis) = self.redis_store {
                        redis.put(key, value).await?;
                    }
                }
                Ok(())
            }
            HybridMode::TieredMemoryRedis => {
                // Hot data in memory, cold data in Redis
                // This is a simplified implementation
                if let Some(ref redis) = self.redis_store {
                    redis.put(key, value).await?;
                }
                Ok(())
            }
        }
    }

    /// Hybrid get operation
    #[cfg(feature = "redis")]
    pub async fn hybrid_get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>> {
        match self.hybrid_mode {
            HybridMode::MemoryOnly => {
                // Simplified - would need actual memory lookup
                Ok(None)
            }
            HybridMode::RedisOnly => {
                if let Some(ref redis) = self.redis_store {
                    redis.get(key).await.map_err(Into::into)
                } else {
                    Err("Redis not configured".into())
                }
            }
            HybridMode::MemoryWithRedisBackup => {
                // Try memory first, then Redis
                let memory_result = None; // Simplified

                if memory_result.is_none() {
                    if let Some(ref redis) = self.redis_store {
                        redis.get(key).await.map_err(Into::into)
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(memory_result)
                }
            }
            HybridMode::TieredMemoryRedis => {
                // Check memory first, then Redis
                let memory_result = None; // Simplified

                if memory_result.is_none() {
                    if let Some(ref redis) = self.redis_store {
                        redis.get(key).await.map_err(Into::into)
                    } else {
                        Ok(None)
                    }
                } else {
                    Ok(memory_result)
                }
            }
        }
    }

    /// Start background monitoring
    async fn start_monitoring(&self) {
        // Note: Background monitoring is disabled due to Send trait issues with current implementation
        // TODO: Implement proper background monitoring with thread-safe components
    }

    /// Perform periodic optimization tasks
    async fn perform_periodic_optimization(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Check memory usage and trigger optimization if needed
        let stats = self.memory_stats().await;

        if stats.memory_efficiency < 0.5 {
            self.optimize_gc().await?;
        }

        // Clean up cache if memory usage is high
        if let Some(ref cache_manager) = self.cache_manager {
            if stats.profiler_stats.current_memory_mb > self.config.target_memory_usage_percent / 100.0 * stats.total_memory_mb {
                cache_manager.evict_old_entries();
            }
        }

        Ok(())
    }

    /// Generate comprehensive memory report
    async fn generate_report(&self) -> Result<MemoryReport, Box<dyn std::error::Error>> {
        let stats = self.memory_stats().await;
        let recommendations = self.get_recommendations().await;

        let pool_analysis = if let Some(ref pool) = self.memory_pool {
            Some(pool.analyze())
        } else {
            None
        };

        let cache_analysis = if let Some(ref cache) = self.cache_manager {
            Some(cache.analyze())
        } else {
            None
        };

        let gc_analysis = if let Some(ref gc) = self.gc_optimizer {
            Some(gc.analyze().await?)
        } else {
            None
        };

        let stats_clone = stats.clone();
        let recommendations_clone = recommendations.clone();

        Ok(MemoryReport {
            generated_at: chrono::Utc::now(),
            config: self.config.clone(),
            stats,
            pool_analysis,
            cache_analysis,
            gc_analysis,
            recommendations,
            optimization_score: self.calculate_optimization_score(&stats_clone, &recommendations_clone),
        })
    }

    // Helper methods
    fn get_total_memory_mb(&self) -> f64 {
        // In a real implementation, this would query system memory
        8192.0 // Assume 8GB for demonstration
    }

    fn get_available_memory_mb(&self) -> f64 {
        // In a real implementation, this would query available memory
        4096.0 // Assume 4GB available for demonstration
    }

    fn calculate_memory_efficiency(&self) -> f64 {
        // Simplified efficiency calculation
        let total = self.get_total_memory_mb();
        let available = self.get_available_memory_mb();
        let used = total - available;
        used / total
    }

    fn calculate_optimization_score(&self, stats: &MemoryStats, recommendations: &[MemoryRecommendation]) -> f64 {
        let mut score = 1.0f64;

        // Deduct points for high memory usage
        if stats.profiler_stats.current_memory_mb > 0.8 * stats.total_memory_mb {
            score -= 0.2;
        }

        // Deduct points for low efficiency
        if stats.memory_efficiency < 0.7 {
            score -= 0.3;
        }

        // Deduct points for recommendations
        for rec in recommendations {
            match rec.priority {
                Priority::Critical => score -= 0.3,
                Priority::High => score -= 0.2,
                Priority::Medium => score -= 0.1,
                Priority::Low => score -= 0.05,
            }
        }

        score.max(0.0).min(1.0)
    }
}

impl Clone for MemoryOptimizer {
    fn clone(&self) -> Self {
        // Note: This is a simplified clone - in practice, you'd need to handle internal state properly
        let mut optimizer = Self::with_hybrid_config(self.config.clone(), self.hybrid_mode.clone());

        // Copy Redis store reference if available
        #[cfg(feature = "redis")]
        {
            optimizer.redis_store = self.redis_store.clone();
        }

        optimizer.initialize_components();
        optimizer
    }
}

/// Memory statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MemoryStats {
    pub pool_stats: Option<memory_pool::PoolStats>,
    pub cache_stats: Option<cache_manager::CacheStats>,
    pub profiler_stats: memory_profiler::MemoryStats,
    pub total_memory_mb: f64,
    pub available_memory_mb: f64,
    pub memory_efficiency: f64,
}

/// Memory optimization report
#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryReport {
    pub generated_at: chrono::DateTime<chrono::Utc>,
    pub config: MemoryConfig,
    pub stats: MemoryStats,
    pub pool_analysis: Option<memory_pool::PoolAnalysis>,
    pub cache_analysis: Option<cache_manager::CacheAnalysis>,
    pub gc_analysis: Option<gc_optimizer::GcAnalysis>,
    pub recommendations: Vec<MemoryRecommendation>,
    pub optimization_score: f64,
}

/// Memory optimization recommendation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryRecommendation {
    pub category: RecommendationCategory,
    pub priority: Priority,
    pub title: String,
    pub description: String,
    pub actions: Vec<String>,
}

/// Recommendation categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecommendationCategory {
    Efficiency,
    Caching,
    Pooling,
    Allocation,
    GC,
}

/// Priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

impl MemoryReport {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        format!(
            "KotobaDB Memory Optimization Report\n"
        )
        .to_string()
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String, Box<dyn std::error::Error>> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Convenience function to create and configure memory optimization
pub fn create_optimizer(config: MemoryConfig) -> MemoryOptimizer {
    MemoryOptimizer::with_config(config)
}

/// Global memory optimizer instance (optional)
static mut GLOBAL_OPTIMIZER: Option<MemoryOptimizer> = None;

/// Initialize global memory optimizer
pub fn init_global_optimizer(config: MemoryConfig) {
    unsafe {
        GLOBAL_OPTIMIZER = Some(MemoryOptimizer::with_config(config));
    }
}

/// Get global memory optimizer instance
pub fn global_optimizer() -> Option<&'static MemoryOptimizer> {
    unsafe { GLOBAL_OPTIMIZER.as_ref() }
}

/// In-memory KeyValueStore implementation
#[derive(Debug, Clone)]
pub struct MemoryKeyValueStore {
    data: Arc<std::sync::RwLock<HashMap<Vec<u8>, Vec<u8>>>>,
}

impl MemoryKeyValueStore {
    /// Create a new empty in-memory key-value store
    pub fn new() -> Self {
        Self {
            data: Arc::new(std::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Create a new in-memory key-value store with initial data
    pub fn with_data(data: HashMap<Vec<u8>, Vec<u8>>) -> Self {
        Self {
            data: Arc::new(std::sync::RwLock::new(data)),
        }
    }

    /// Get the number of entries in the store
    pub fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.data.read().unwrap().is_empty()
    }

    /// Clear all data from the store
    pub fn clear(&self) {
        self.data.write().unwrap().clear();
    }
}

#[async_trait::async_trait]
impl kotoba_storage::KeyValueStore for MemoryKeyValueStore {
    async fn put(&self, key: &[u8], value: &[u8]) -> anyhow::Result<()> {
        let mut data = self.data.write().unwrap();
        data.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
        let data = self.data.read().unwrap();
        Ok(data.get(key).cloned())
    }

    async fn delete(&self, key: &[u8]) -> anyhow::Result<()> {
        let mut data = self.data.write().unwrap();
        data.remove(key);
        Ok(())
    }

    async fn scan(&self, prefix: &[u8]) -> anyhow::Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let data = self.data.read().unwrap();
        let mut results = Vec::new();

        for (key, value) in data.iter() {
            if key.starts_with(prefix) {
                results.push((key.clone(), value.clone()));
            }
        }

        // Sort results by key for consistent ordering
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_storage::KeyValueStore;

    #[tokio::test]
    async fn test_memory_kv_store_basic_operations() {
        let store = MemoryKeyValueStore::new();

        // Test put and get
        store.put(b"key1", b"value1").await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));

        // Test get non-existent key
        let value = store.get(b"nonexistent").await.unwrap();
        assert_eq!(value, None);

        // Test delete
        store.delete(b"key1").await.unwrap();
        let value = store.get(b"key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_memory_kv_store_multiple_keys() {
        let store = MemoryKeyValueStore::new();

        // Put multiple key-value pairs
        store.put(b"key1", b"value1").await.unwrap();
        store.put(b"key2", b"value2").await.unwrap();
        store.put(b"key3", b"value3").await.unwrap();

        // Verify all keys exist
        assert_eq!(store.get(b"key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(store.get(b"key2").await.unwrap(), Some(b"value2".to_vec()));
        assert_eq!(store.get(b"key3").await.unwrap(), Some(b"value3".to_vec()));

        // Check length
        assert_eq!(store.len(), 3);
        assert!(!store.is_empty());
    }

    #[tokio::test]
    async fn test_memory_kv_store_scan() {
        let store = MemoryKeyValueStore::new();

        // Put keys with common prefix
        store.put(b"prefix_key1", b"value1").await.unwrap();
        store.put(b"prefix_key2", b"value2").await.unwrap();
        store.put(b"prefix_key3", b"value3").await.unwrap();
        store.put(b"other_key", b"other_value").await.unwrap();

        // Scan with prefix
        let results = store.scan(b"prefix_").await.unwrap();
        assert_eq!(results.len(), 3);

        // Verify results are sorted
        assert_eq!(results[0], (b"prefix_key1".to_vec(), b"value1".to_vec()));
        assert_eq!(results[1], (b"prefix_key2".to_vec(), b"value2".to_vec()));
        assert_eq!(results[2], (b"prefix_key3".to_vec(), b"value3".to_vec()));

        // Scan with empty prefix (should return all)
        let all_results = store.scan(b"").await.unwrap();
        assert_eq!(all_results.len(), 4);
    }

    #[tokio::test]
    async fn test_memory_kv_store_overwrite() {
        let store = MemoryKeyValueStore::new();

        // Put initial value
        store.put(b"key", b"initial").await.unwrap();
        assert_eq!(store.get(b"key").await.unwrap(), Some(b"initial".to_vec()));

        // Overwrite with new value
        store.put(b"key", b"updated").await.unwrap();
        assert_eq!(store.get(b"key").await.unwrap(), Some(b"updated".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_kv_store_empty_keys_values() {
        let store = MemoryKeyValueStore::new();

        // Test empty key
        store.put(b"", b"empty_key_value").await.unwrap();
        assert_eq!(store.get(b"").await.unwrap(), Some(b"empty_key_value".to_vec()));

        // Test empty value
        store.put(b"empty_value_key", b"").await.unwrap();
        assert_eq!(store.get(b"empty_value_key").await.unwrap(), Some(b"".to_vec()));
    }

    #[tokio::test]
    async fn test_memory_kv_store_clear() {
        let store = MemoryKeyValueStore::new();

        // Add some data
        store.put(b"key1", b"value1").await.unwrap();
        store.put(b"key2", b"value2").await.unwrap();
        assert_eq!(store.len(), 2);

        // Clear all data
        store.clear();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn test_memory_kv_store_with_initial_data() {
        let mut initial_data = HashMap::new();
        initial_data.insert(b"key1".to_vec(), b"value1".to_vec());
        initial_data.insert(b"key2".to_vec(), b"value2".to_vec());

        let store = MemoryKeyValueStore::with_data(initial_data);
        assert_eq!(store.len(), 2);

        // Verify initial data
        assert_eq!(store.get(b"key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(store.get(b"key2").await.unwrap(), Some(b"value2".to_vec()));

        // Add more data
        store.put(b"key3", b"value3").await.unwrap();
        assert_eq!(store.len(), 3);
    }

    #[tokio::test]
    async fn test_memory_kv_store_large_data() {
        let store = MemoryKeyValueStore::new();

        // Test with large key and value
        let large_key = vec![42u8; 1024]; // 1KB key
        let large_value = vec![255u8; 1024 * 1024]; // 1MB value

        store.put(&large_key, &large_value).await.unwrap();
        let retrieved = store.get(&large_key).await.unwrap();

        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_memory_kv_store_unicode_keys_values() {
        let store = MemoryKeyValueStore::new();

        // Test Unicode keys and values
        let unicode_key = "🚀 котоба 🔥".as_bytes();
        let unicode_value = "Hello 世界 🌍".as_bytes();

        store.put(unicode_key, unicode_value).await.unwrap();
        let retrieved = store.get(unicode_key).await.unwrap();

        assert_eq!(retrieved, Some(unicode_value.to_vec()));
    }

    #[tokio::test]
    async fn test_memory_kv_store_concurrent_access() {
        let store = Arc::new(MemoryKeyValueStore::new());

        let mut handles = vec![];

        // Spawn multiple tasks to test concurrent access
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let key = format!("key{}", i).into_bytes();
                let value = format!("value{}", i).into_bytes();

                // Put operation
                store_clone.put(&key, &value).await.unwrap();

                // Get operation
                let retrieved = store_clone.get(&key).await.unwrap();
                assert_eq!(retrieved, Some(value));
            });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }

        // Verify all data was stored
        assert_eq!(store.len(), 10);
    }

    #[test]
    fn test_memory_config_default() {
        let config = MemoryConfig::default();

        assert!(config.enable_pooling);
        assert_eq!(config.pool_size_mb, 256);
        assert!(config.enable_caching);
        assert_eq!(config.cache_size_mb, 512);
        assert_eq!(config.cache_policy, CachePolicy::Lru);
        assert!(!config.enable_custom_allocators);
        assert_eq!(config.allocator_type, AllocatorType::System);
        assert!(config.enable_gc_optimization);
        assert_eq!(config.target_memory_usage_percent, 75.0);
        assert_eq!(config.monitoring_interval_ms, 1000);
    }

    #[test]
    fn test_memory_optimizer_creation() {
        let optimizer = MemoryOptimizer::new();

        // Check that components are initialized based on default config
        assert!(optimizer.memory_profiler.is_some());
        assert!(optimizer.gc_optimizer.is_some());
        assert!(optimizer.memory_pool.is_some());
        assert!(optimizer.cache_manager.is_some());
        assert!(optimizer.allocator.is_none()); // Custom allocators disabled by default
    }

    #[test]
    fn test_memory_optimizer_with_custom_config() {
        let config = MemoryConfig {
            enable_pooling: false,
            enable_caching: false,
            enable_custom_allocators: true,
            allocator_type: AllocatorType::Custom,
            ..Default::default()
        };

        let optimizer = MemoryOptimizer::with_config(config);

        // Check that components are initialized based on custom config
        assert!(optimizer.memory_profiler.is_some());
        assert!(optimizer.gc_optimizer.is_some());
        assert!(optimizer.memory_pool.is_none()); // Pooling disabled
        assert!(optimizer.cache_manager.is_none()); // Caching disabled
        assert!(optimizer.allocator.is_some()); // Custom allocator enabled
    }

    #[tokio::test]
    async fn test_memory_optimizer_stats() {
        let optimizer = MemoryOptimizer::new();
        let stats = optimizer.memory_stats().await;

        // Verify basic stats structure
        assert!(stats.total_memory_mb > 0.0);
        assert!(stats.available_memory_mb > 0.0);
        assert!(stats.memory_efficiency >= 0.0 && stats.memory_efficiency <= 1.0);
    }
}

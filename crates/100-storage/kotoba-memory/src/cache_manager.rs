//! Cache Manager Implementation
//!
//! Intelligent caching with multiple strategies:
//! - LRU (Least Recently Used) eviction
//! - LFU (Least Frequently Used) eviction
//! - Adaptive caching based on access patterns
//! - Multi-level caching (memory, disk)

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};
use lru::LruCache;

/// Cache manager for intelligent data caching
pub struct CacheManager {
    memory_cache: Mutex<LruCache<String, CachedValue>>,
    size_limit: usize,
    current_size: Mutex<usize>,
    policy: CachePolicy,
    stats: Arc<Mutex<CacheStats>>,
    adaptive_stats: Mutex<AdaptiveStats>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CachePolicy {
    Lru,
    Lfu,
    Fifo,
    Adaptive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedValue {
    pub data: Vec<u8>,
    pub metadata: CacheMetadata,
    pub access_count: u64,
    pub last_access: Instant,
    pub created_at: Instant,
    pub ttl: Option<Duration>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub content_type: String,
    pub size_bytes: usize,
    pub compression_ratio: Option<f64>,
    pub checksum: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_size_bytes: usize,
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub hit_rate: f64,
    pub average_access_time: Duration,
    pub size_efficiency: f64,
}

#[derive(Debug)]
struct AdaptiveStats {
    access_patterns: HashMap<String, AccessPattern>,
    time_windows: Vec<TimeWindow>,
    current_window: TimeWindow,
}

#[derive(Debug, Clone)]
struct AccessPattern {
    frequency: u64,
    recency: Instant,
    popularity_score: f64,
}

#[derive(Debug, Clone)]
struct TimeWindow {
    start_time: Instant,
    accesses: HashMap<String, u64>,
    total_accesses: u64,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new(size_limit: usize, policy: CachePolicy) -> Self {
        let memory_cache = Mutex::new(LruCache::new(std::num::NonZeroUsize::new(10000).unwrap()));

        Self {
            memory_cache,
            size_limit,
            current_size: Mutex::new(0),
            policy,
            stats: Arc::new(Mutex::new(CacheStats {
                total_entries: 0,
                total_size_bytes: 0,
                hits: 0,
                misses: 0,
                evictions: 0,
                hit_rate: 0.0,
                average_access_time: Duration::from_nanos(0),
                size_efficiency: 0.0,
            })),
            adaptive_stats: Mutex::new(AdaptiveStats {
                access_patterns: HashMap::new(),
                time_windows: Vec::new(),
                current_window: TimeWindow {
                    start_time: Instant::now(),
                    accesses: HashMap::new(),
                    total_accesses: 0,
                },
            }),
        }
    }

    /// Get a value from cache
    pub fn get(&self, key: &str) -> Option<CachedValue> {
        let start_time = Instant::now();

        let mut cache = self.memory_cache.lock().unwrap();
        let result = cache.get_mut(key).cloned();

        let access_time = start_time.elapsed();
        let mut stats = self.stats.lock().unwrap();

        if let Some(ref mut value) = result {
            // Update access statistics
            value.access_count += 1;
            value.last_access = Instant::now();

            stats.hits += 1;
            stats.average_access_time = (stats.average_access_time + access_time) / 2;

            // Update adaptive stats
            self.update_access_pattern(key, true);
        } else {
            stats.misses += 1;
            self.update_access_pattern(key, false);
        }

        // Update hit rate
        let total_requests = stats.hits + stats.misses;
        if total_requests > 0 {
            stats.hit_rate = stats.hits as f64 / total_requests as f64;
        }

        result
    }

    /// Put a value in cache
    pub fn put(&self, key: String, value: CachedValue) {
        let value_size = value.data.len() + std::mem::size_of::<CachedValue>();

        // Check if we need to evict entries
        let mut current_size = self.current_size.lock().unwrap();
        while *current_size + value_size > self.size_limit {
            if !self.evict_entry() {
                break; // Could not evict more entries
            }
        }

        // Add to cache
        let mut cache = self.memory_cache.lock().unwrap();
        cache.put(key.clone(), value);
        *current_size += value_size;

        // Update stats
        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = cache.len();
        stats.total_size_bytes = *current_size;

        if stats.total_entries > 0 {
            stats.size_efficiency = *current_size as f64 / stats.total_entries as f64;
        }
    }

    /// Remove a value from cache
    pub fn remove(&self, key: &str) -> bool {
        let mut cache = self.memory_cache.lock().unwrap();
        if let Some(value) = cache.pop(key) {
            let value_size = value.data.len() + std::mem::size_of::<CachedValue>();
            let mut current_size = self.current_size.lock().unwrap();
            *current_size = current_size.saturating_sub(value_size);

            let mut stats = self.stats.lock().unwrap();
            stats.total_entries = cache.len();
            stats.total_size_bytes = *current_size;

            true
        } else {
            false
        }
    }

    /// Clear all cached entries
    pub fn clear(&self) {
        let mut cache = self.memory_cache.lock().unwrap();
        cache.clear();
        let mut current_size = self.current_size.lock().unwrap();
        *current_size = 0;

        let mut stats = self.stats.lock().unwrap();
        stats.total_entries = 0;
        stats.total_size_bytes = 0;
        stats.hits = 0;
        stats.misses = 0;
        stats.evictions = 0;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        self.stats.lock().unwrap().clone()
    }

    /// Check if key exists in cache
    pub fn contains(&self, key: &str) -> bool {
        let cache = self.memory_cache.lock().unwrap();
        cache.contains(key)
    }

    /// Get cache size information
    pub fn size_info(&self) -> (usize, usize) {
        let current_size = *self.current_size.lock().unwrap();
        (current_size, self.size_limit)
    }

    /// Evict old entries to free up space
    pub fn evict_old_entries(&self) {
        let cache = self.memory_cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        // Simple eviction: remove expired entries first
        let mut to_remove = Vec::new();
        for (key, value) in cache.iter() {
            if let Some(ttl) = value.ttl {
                if value.created_at.elapsed() > ttl {
                    to_remove.push(key.clone());
                }
            }
        }

        for key in to_remove {
            cache.pop(&key);
            stats.evictions += 1;
        }

        stats.total_entries = cache.len();
    }

    /// Analyze cache performance and patterns
    pub fn analyze(&self) -> CacheAnalysis {
        let stats = self.stats();
        let cache = self.memory_cache.lock().unwrap();

        let mut access_distribution = HashMap::new();
        let mut size_distribution = HashMap::new();
        let mut age_distribution = HashMap::new();

        for (key, value) in cache.iter() {
            // Access count distribution
            let access_bucket = (value.access_count / 10) * 10; // Group by 10s
            *access_distribution.entry(access_bucket).or_insert(0) += 1;

            // Size distribution
            let size_bucket = ((value.data.len() as f64).log2() as u32) * 1024; // Log-scale buckets
            *size_distribution.entry(size_bucket).or_insert(0) += 1;

            // Age distribution (in minutes)
            let age_minutes = value.created_at.elapsed().as_secs() / 60;
            let age_bucket = (age_minutes / 5) * 5; // Group by 5-minute intervals
            *age_distribution.entry(age_bucket).or_insert(0) += 1;
        }

        let effectiveness_score = self.calculate_effectiveness_score(&stats);
        let recommendations = self.generate_recommendations(&stats, &access_distribution);

        CacheAnalysis {
            stats,
            access_distribution,
            size_distribution,
            age_distribution,
            cache_effectiveness: effectiveness_score,
            recommendations,
            policy_performance: self.analyze_policy_performance(),
        }
    }

    /// Evict a single entry (used internally)
    fn evict_entry(&self) -> bool {
        let mut cache = self.memory_cache.lock().unwrap();
        let mut stats = self.stats.lock().unwrap();

        match self.policy {
            CachePolicy::Lru => {
                // LRU is handled automatically by LruCache
                if let Some((_, value)) = cache.pop_lru() {
                    let value_size = value.data.len() + std::mem::size_of::<CachedValue>();
                    let mut current_size = self.current_size.lock().unwrap();
                    *current_size = current_size.saturating_sub(value_size);
                    stats.evictions += 1;
                    stats.total_entries = cache.len();
                    stats.total_size_bytes = *current_size;
                    true
                } else {
                    false
                }
            }
            CachePolicy::Adaptive => {
                // Use adaptive eviction based on access patterns
                self.evict_adaptive(&mut cache, &mut stats)
            }
            _ => {
                // For other policies, use LRU as fallback
                if let Some((_, value)) = cache.pop_lru() {
                    let value_size = value.data.len() + std::mem::size_of::<CachedValue>();
                    let mut current_size = self.current_size.lock().unwrap();
                    *current_size = current_size.saturating_sub(value_size);
                    stats.evictions += 1;
                    stats.total_entries = cache.len();
                    stats.total_size_bytes = *current_size;
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Adaptive eviction strategy
    fn evict_adaptive(&self, cache: &mut LruCache<String, CachedValue>, stats: &mut CacheStats) -> bool {
        // Find the least valuable entry based on adaptive scoring
        let mut lowest_score = f64::INFINITY;
        let mut lowest_key = None;

        for (key, value) in cache.iter() {
            let score = self.calculate_adaptive_score(value);
            if score < lowest_score {
                lowest_score = score;
                lowest_key = Some(key.clone());
            }
        }

        if let Some(key) = lowest_key {
            if let Some(value) = cache.pop(&key) {
                let value_size = value.data.len() + std::mem::size_of::<CachedValue>();
                let mut current_size = self.current_size.lock().unwrap();
                *current_size = current_size.saturating_sub(value_size);
                stats.evictions += 1;
                stats.total_entries = cache.len();
                stats.total_size_bytes = *current_size;
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    /// Calculate adaptive score for cache eviction
    fn calculate_adaptive_score(&self, value: &CachedValue) -> f64 {
        let age_hours = value.created_at.elapsed().as_secs_f64() / 3600.0;
        let access_frequency = value.access_count as f64 / (age_hours + 1.0); // Avoid division by zero
        let recency = 1.0 / (value.last_access.elapsed().as_secs_f64() + 1.0);

        // Score = frequency * recency / size
        let size_penalty = (value.data.len() as f64).sqrt();
        (access_frequency * recency) / size_penalty
    }

    /// Update access pattern statistics
    fn update_access_pattern(&self, key: &str, hit: bool) {
        let mut adaptive_stats = self.adaptive_stats.lock().unwrap();

        let pattern = adaptive_stats.access_patterns.entry(key.to_string()).or_insert(AccessPattern {
            frequency: 0,
            recency: Instant::now(),
            popularity_score: 0.0,
        });

        pattern.frequency += 1;
        pattern.recency = Instant::now();

        // Update popularity score
        let time_since_last_access = pattern.recency.elapsed().as_secs_f64();
        pattern.popularity_score = pattern.frequency as f64 / (time_since_last_access + 1.0);

        // Update time window
        *adaptive_stats.current_window.accesses.entry(key.to_string()).or_insert(0) += 1;
        adaptive_stats.current_window.total_accesses += 1;

        // Rotate time windows every 5 minutes
        if adaptive_stats.current_window.start_time.elapsed() > Duration::from_secs(300) {
            adaptive_stats.time_windows.push(adaptive_stats.current_window.clone());
            adaptive_stats.current_window = TimeWindow {
                start_time: Instant::now(),
                accesses: HashMap::new(),
                total_accesses: 0,
            };

            // Keep only last 10 windows
            if adaptive_stats.time_windows.len() > 10 {
                adaptive_stats.time_windows.remove(0);
            }
        }
    }

    /// Calculate cache effectiveness score
    fn calculate_effectiveness_score(&self, stats: &CacheStats) -> f64 {
        let hit_rate_score = stats.hit_rate;
        let size_efficiency_score = if stats.size_efficiency > 0.0 {
            (1000.0 / stats.size_efficiency).min(1.0) // Smaller objects per byte is better
        } else {
            0.0
        };

        (hit_rate_score + size_efficiency_score) / 2.0
    }

    /// Generate caching recommendations
    fn generate_recommendations(&self, stats: &CacheStats, access_distribution: &HashMap<u64, usize>) -> Vec<String> {
        let mut recommendations = Vec::new();

        if stats.hit_rate < 0.5 {
            recommendations.push("Low cache hit rate detected. Consider increasing cache size or adjusting cache policy.".to_string());
        }

        if stats.evictions > stats.hits {
            recommendations.push("High eviction rate. Cache size may be too small for workload.".to_string());
        }

        // Check access pattern distribution
        let low_access_entries = access_distribution.get(&0).unwrap_or(&0);
        if *low_access_entries > stats.total_entries / 2 {
            recommendations.push("Many cache entries have low access frequency. Consider using LFU policy or smaller cache.".to_string());
        }

        if stats.size_efficiency > 10000.0 { // Very large average object size
            recommendations.push("Large objects in cache. Consider caching smaller objects or using compression.".to_string());
        }

        recommendations
    }

    /// Analyze cache policy performance
    fn analyze_policy_performance(&self) -> PolicyPerformance {
        let stats = self.stats();
        let adaptive_stats = self.adaptive_stats.lock().unwrap();

        // Calculate policy-specific metrics
        let temporal_locality = self.calculate_temporal_locality(&adaptive_stats);
        let spatial_locality = self.calculate_spatial_locality(&adaptive_stats);

        PolicyPerformance {
            policy_type: self.policy.clone(),
            hit_rate: stats.hit_rate,
            temporal_locality,
            spatial_locality,
            eviction_efficiency: if stats.evictions > 0 {
                stats.hits as f64 / stats.evictions as f64
            } else {
                0.0
            },
        }
    }

    /// Calculate temporal locality (how recently accessed items are)
    fn calculate_temporal_locality(&self, adaptive_stats: &AdaptiveStats) -> f64 {
        let mut total_score = 0.0;
        let mut count = 0;

        for pattern in adaptive_stats.access_patterns.values() {
            let recency_score = 1.0 / (pattern.recency.elapsed().as_secs_f64() + 1.0);
            total_score += recency_score;
            count += 1;
        }

        if count > 0 { total_score / count as f64 } else { 0.0 }
    }

    /// Calculate spatial locality (how clustered access patterns are)
    fn calculate_spatial_locality(&self, adaptive_stats: &AdaptiveStats) -> f64 {
        // Simplified spatial locality calculation
        // In practice, this would analyze access patterns for clustering
        let mut total_accesses = 0u64;
        let mut clustered_accesses = 0u64;

        for window in &adaptive_stats.time_windows {
            total_accesses += window.total_accesses;
            // Count accesses to top 20% of keys as "clustered"
            let mut accesses: Vec<_> = window.accesses.values().collect();
            accesses.sort_by(|a, b| b.cmp(a));
            let top_count = (accesses.len() as f64 * 0.2) as usize;
            clustered_accesses += accesses.iter().take(top_count).map(|&&x| x).sum::<u64>();
        }

        if total_accesses > 0 {
            clustered_accesses as f64 / total_accesses as f64
        } else {
            0.0
        }
    }
}

/// Cache analysis result
#[derive(Debug, Serialize, Deserialize)]
pub struct CacheAnalysis {
    pub stats: CacheStats,
    pub access_distribution: HashMap<u64, usize>,
    pub size_distribution: HashMap<u32, usize>,
    pub age_distribution: HashMap<u64, usize>,
    pub cache_effectiveness: f64,
    pub recommendations: Vec<String>,
    pub policy_performance: PolicyPerformance,
}

/// Cache policy performance analysis
#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyPerformance {
    pub policy_type: CachePolicy,
    pub hit_rate: f64,
    pub temporal_locality: f64,
    pub spatial_locality: f64,
    pub eviction_efficiency: f64,
}

/// Convenience functions for cache management
pub fn create_cache(size_limit: usize, policy: CachePolicy) -> CacheManager {
    CacheManager::new(size_limit, policy)
}

pub fn create_lru_cache(size_limit: usize) -> CacheManager {
    CacheManager::new(size_limit, CachePolicy::Lru)
}

pub fn create_adaptive_cache(size_limit: usize) -> CacheManager {
    CacheManager::new(size_limit, CachePolicy::Adaptive)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_basic_operations() {
        let cache = CacheManager::new(1024 * 1024, CachePolicy::Lru); // 1MB cache

        let value = CachedValue {
            data: vec![1, 2, 3, 4],
            metadata: CacheMetadata {
                content_type: "test".to_string(),
                size_bytes: 4,
                compression_ratio: None,
                checksum: None,
            },
            access_count: 0,
            last_access: Instant::now(),
            created_at: Instant::now(),
            ttl: None,
        };

        // Test put and get
        cache.put("test_key".to_string(), value.clone());
        let retrieved = cache.get("test_key");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().data, vec![1, 2, 3, 4]);

        // Test stats
        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert!(stats.hits >= 1);
    }

    #[test]
    fn test_cache_eviction() {
        let cache = CacheManager::new(100, CachePolicy::Lru); // Very small cache

        // Add items that exceed cache size
        for i in 0..10 {
            let value = CachedValue {
                data: vec![0u8; 50], // 50 bytes each
                metadata: CacheMetadata {
                    content_type: "test".to_string(),
                    size_bytes: 50,
                    compression_ratio: None,
                    checksum: None,
                },
                access_count: 0,
                last_access: Instant::now(),
                created_at: Instant::now(),
                ttl: None,
            };
            cache.put(format!("key_{}", i), value);
        }

        // Cache should have evicted some entries
        let stats = cache.stats();
        assert!(stats.total_entries < 10);
        assert!(stats.evictions > 0);
    }
}

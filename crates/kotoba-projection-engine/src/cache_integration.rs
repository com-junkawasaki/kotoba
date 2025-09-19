//! Cache Integration
//!
//! Integration with the cache layer for projection results.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, error, instrument};

use kotoba_cache::CacheLayer;

/// Cache integration for projection results
pub struct CacheIntegration {
    cache: Arc<CacheLayer>,
}

impl CacheIntegration {
    /// Create new cache integration
    pub fn new(cache: Arc<CacheLayer>) -> Self {
        Self { cache }
    }

    /// Get cached result
    #[instrument(skip(self))]
    pub async fn get_cached_result(
        &self,
        namespace: &str,
        key: &serde_json::Value,
    ) -> Result<Option<crate::QueryResult>> {
        let cache_key = format!("{}:{}", namespace, serde_json::to_string(key)?);

        match self.cache.get(&cache_key).await {
            Ok(Some(value)) => {
                // Deserialize cached result
                let result: crate::QueryResult = serde_json::from_value(value)?;
                info!("Cache hit for key: {}", cache_key);
                Ok(Some(result))
            }
            Ok(None) => {
                info!("Cache miss for key: {}", cache_key);
                Ok(None)
            }
            Err(e) => {
                warn!("Cache error for key {}: {}", cache_key, e);
                Ok(None)
            }
        }
    }

    /// Cache result
    #[instrument(skip(self, result))]
    pub async fn cache_result(
        &self,
        namespace: &str,
        key: &serde_json::Value,
        result: &crate::QueryResult,
    ) -> Result<()> {
        let cache_key = format!("{}:{}", namespace, serde_json::to_string(key)?);

        // Serialize result
        let value = serde_json::to_value(result)?;

        // Cache with default TTL
        self.cache.set(&cache_key, value, None).await?;

        info!("Cached result for key: {}", cache_key);
        Ok(())
    }

    /// Invalidate cache entries
    #[instrument(skip(self))]
    pub async fn invalidate_cache(&self, pattern: &str) -> Result<()> {
        // For now, just log the invalidation request
        // In a full implementation, this would use Redis SCAN or similar
        info!("Cache invalidation requested for pattern: {}", pattern);
        Ok(())
    }

    /// Clear all cache entries
    #[instrument(skip(self))]
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear().await?;
        info!("Cache cleared");
        Ok(())
    }

    /// Get cache statistics
    pub async fn get_cache_stats(&self) -> Result<CacheStats> {
        let stats = self.cache.get_statistics().await;
        Ok(CacheStats {
            hits: stats.hits,
            misses: stats.misses,
            sets: stats.sets,
            deletes: stats.deletes,
            evictions: stats.evictions,
            total_size_bytes: stats.total_size_bytes,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
    pub evictions: u64,
    pub total_size_bytes: u64,
}

impl Default for CacheIntegration {
    fn default() -> Self {
        // Create a mock cache for default
        let cache_config = kotoba_cache::CacheConfig::default();
        let cache = Arc::new(tokio::sync::Mutex::new(
            futures::executor::block_on(async {
                CacheLayer::new(cache_config).await.unwrap()
            })
        ));

        Self { cache }
    }
}

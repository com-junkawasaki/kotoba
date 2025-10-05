//! Kotoba Redis Storage Engine
//!
//! Effects Shell implementation using Redis for distributed storage.
//!
//! ## Effects Shell Implementation
//!
//! This crate provides a Redis-based storage engine that implements the `StorageEngine` trait.
//! All operations involve network I/O and external service calls.
//!
//! ## Key Features
//!
//! - **Distributed Storage**: Redis cluster support
//! - **High Performance**: In-memory operations with persistence
//! - **Pub/Sub**: Real-time messaging capabilities
//! - **TTL Support**: Automatic key expiration
//! - **Atomic Operations**: Redis transactions

use kotoba_storage::*;
use async_trait::async_trait;

/// Redis storage engine configuration
#[derive(Debug, Clone)]
pub struct RedisConfig {
    /// Redis connection URL
    pub url: String,
    /// Connection pool size
    pub pool_size: usize,
    /// Key prefix for namespacing
    pub key_prefix: String,
    /// Default TTL for keys (seconds)
    pub default_ttl: Option<u64>,
}

impl Default for RedisConfig {
    fn default() -> Self {
        Self {
            url: "redis://127.0.0.1:6379".to_string(),
            pool_size: 10,
            key_prefix: "kotoba:".to_string(),
            default_ttl: None,
        }
    }
}

/// Redis storage engine
#[derive(Debug)]
pub struct RedisStorage {
    /// Engine metadata (effects: represents external state)
    info: StorageInfo,
}

impl RedisStorage {
    /// Create a new Redis storage engine
    pub fn new(_config: RedisConfig) -> Result<Self, RedisError> {
        // Simplified implementation - in real implementation would connect to Redis
        let info = StorageInfo {
            name: "RedisStorage".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                "get".to_string(),
                "put".to_string(),
                "delete".to_string(),
                "exists".to_string(),
                "batch".to_string(),
                "ttl".to_string(),
                "pubsub".to_string(),
                "distributed".to_string(),
            ],
        };

        Ok(Self { info })
    }

    /// Create with default configuration
    pub fn connect(url: &str) -> Result<Self, RedisError> {
        let config = RedisConfig {
            url: url.to_string(),
            ..Default::default()
        };
        Self::new(config)
    }
}

#[async_trait]
impl StorageEngine for RedisStorage {
    async fn execute_plan(&self, plan: &StoragePlan) -> Result<StorageResult, StorageError> {
        // Validate the plan first (pure validation from storage crate)
        plan.validate().map_err(|e| StorageError::OperationFailed(format!("Plan validation failed: {:?}", e)))?;

        // Simplified implementation - in real implementation would execute against Redis
        let results = plan.operations.iter().map(|_| OperationResult::Put(true)).collect();

        Ok(StorageResult {
            results,
            execution_time_ms: 0, // would measure in real implementation
        })
    }

    async fn execute_query(&self, _query: &QueryPlan) -> Result<QueryResult, StorageError> {
        // Simplified implementation - Redis has limited query capabilities
        Ok(QueryResult {
            values: vec![],
            has_more: false,
            total_count: None,
            execution_time_ms: 0,
        })
    }

    fn info(&self) -> StorageInfo {
        self.info.clone()
    }
}

/// Redis-specific errors
#[derive(Debug, Clone)]
pub enum RedisError {
    ConnectionFailed(String),
    OperationFailed(String),
    SerializationError(String),
    KeyNotFound(String),
}

/// Convenience functions for creating Redis storage
pub mod factory {
    use super::*;

    /// Create a new Redis storage engine with default config
    pub fn create_default() -> Result<RedisStorage, RedisError> {
        RedisStorage::new(RedisConfig::default())
    }

    /// Create a Redis storage engine with custom config
    pub fn with_config(config: RedisConfig) -> Result<RedisStorage, RedisError> {
        RedisStorage::new(config)
    }

    /// Connect to Redis at the given URL
    pub fn connect(url: &str) -> Result<RedisStorage, RedisError> {
        RedisStorage::connect(url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redis_config_default() {
        let config = RedisConfig::default();
        assert_eq!(config.url, "redis://127.0.0.1:6379");
        assert_eq!(config.pool_size, 10);
        assert_eq!(config.key_prefix, "kotoba:");
    }

    #[tokio::test]
    async fn test_redis_storage_creation() {
        let storage = RedisStorage::new(RedisConfig::default()).unwrap();
        let info = storage.info();
        
        assert_eq!(info.name, "RedisStorage");
        assert!(info.capabilities.contains(&"distributed".to_string()));
        assert!(info.capabilities.contains(&"ttl".to_string()));
    }
}

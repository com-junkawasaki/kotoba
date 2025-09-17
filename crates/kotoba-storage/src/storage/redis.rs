//! Redis-based storage backend for Upstash compatibility

use crate::storage::{StorageBackend, StorageConfig, BackendStats};
use async_trait::async_trait;
use kotoba_core::types::Result;
use kotoba_errors::KotobaError;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Redis-based storage backend
#[derive(Clone)]
pub struct RedisBackend {
    client: Client,
    connection_manager: Arc<Mutex<ConnectionManager>>,
    url: String,
}

impl RedisBackend {
    /// Create a new Redis backend
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let url = config.redis_url.as_ref()
            .ok_or_else(|| KotobaError::Storage("Redis URL not configured".to_string()))?
            .clone();

        let client = Client::open(url.clone())
            .map_err(|e| KotobaError::Storage(format!("Failed to create Redis client: {}", e)))?;

        let connection_manager = client.get_tokio_connection_manager()
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to create Redis connection manager: {}", e)))?;

        // Test connection
        let mut conn = connection_manager.clone();
        let _: String = redis::cmd("PING")
            .query_async(&mut conn)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client,
            connection_manager: Arc::new(Mutex::new(connection_manager)),
            url,
        })
    }

    /// Get a connection from the pool
    async fn get_connection(&self) -> Result<redis::aio::ConnectionManager> {
        Ok(self.connection_manager.lock().await.clone())
    }
}

#[async_trait]
impl StorageBackend for RedisBackend {
    async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        let mut conn = self.connection_manager.lock().await;
        conn.set::<_, _, ()>(key, value)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to put data in Redis: {}", e)))?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let mut conn = self.get_connection().await?;
        let result: Option<Vec<u8>> = conn
            .get(key)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to get data from Redis: {}", e)))?;
        Ok(result)
    }

    async fn delete(&self, key: String) -> Result<()> {
        let mut conn = self.get_connection().await?;
        conn.del::<_, ()>(key)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to delete data from Redis: {}", e)))?;
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool> {
        let mut conn = self.get_connection().await?;
        let result: bool = conn
            .exists(key)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to check key existence in Redis: {}", e)))?;
        Ok(result)
    }

    async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}*", prefix);
        let keys: Vec<String> = conn
            .keys(pattern)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to scan keys in Redis: {}", e)))?;
        Ok(keys)
    }

    async fn clear(&self) -> Result<()> {
        let mut conn = self.get_connection().await?;
        redis::cmd("FLUSHDB")
            .query_async(&mut conn)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to clear Redis database: {}", e)))?;
        Ok(())
    }

    async fn stats(&self) -> Result<BackendStats> {
        let mut conn = self.connection_manager.lock().await.clone();
        let info: String = redis::cmd("INFO")
            .query_async(&mut conn)
            .await
            .map_err(|e| KotobaError::Storage(format!("Failed to get Redis info: {}", e)))?;

        // Parse some basic stats from INFO command
        let total_keys = self.parse_redis_info(&info, "db0:keys");
        let memory_usage = self.parse_redis_info(&info, "used_memory");

        Ok(BackendStats {
            backend_type: "Redis".to_string(),
            total_keys,
            memory_usage,
            disk_usage: None, // Redis is in-memory
            connection_count: Some(1), // Connection manager handles this
        })
    }
}

impl RedisBackend {
    /// Parse Redis INFO command output for specific metrics
    fn parse_redis_info(&self, info: &str, key: &str) -> Option<u64> {
        for line in info.lines() {
            if line.starts_with(key) {
                if let Some(value_str) = line.split(':').nth(1) {
                    if let Ok(value) = value_str.trim().parse::<u64>() {
                        return Some(value);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    async fn create_test_redis_backend() -> Option<RedisBackend> {
        // Only run tests if Redis is available
        let redis_url = env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        match RedisBackend::new(&StorageConfig {
            backend_type: crate::storage::BackendType::Redis,
            redis_url: Some(redis_url),
            ..Default::default()
        }).await {
            Ok(backend) => Some(backend),
            Err(_) => {
                println!("Redis not available, skipping Redis tests");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_redis_put_and_get() {
        let backend = match create_test_redis_backend().await {
            Some(b) => b,
            None => return,
        };

        // Clear any existing data
        backend.clear().await.unwrap();

        // Put some data
        backend.put("test_key1".to_string(), b"test_value1".to_vec()).await.unwrap();
        backend.put("test_key2".to_string(), b"test_value2".to_vec()).await.unwrap();

        // Get the data back
        assert_eq!(backend.get("test_key1").await.unwrap(), Some(b"test_value1".to_vec()));
        assert_eq!(backend.get("test_key2").await.unwrap(), Some(b"test_value2".to_vec()));
        assert_eq!(backend.get("test_key3").await.unwrap(), None);

        // Clean up
        backend.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_delete() {
        let backend = match create_test_redis_backend().await {
            Some(b) => b,
            None => return,
        };

        backend.clear().await.unwrap();

        // Put and then delete
        backend.put("delete_key".to_string(), b"delete_value".to_vec()).await.unwrap();
        assert_eq!(backend.get("delete_key").await.unwrap(), Some(b"delete_value".to_vec()));

        backend.delete("delete_key".to_string()).await.unwrap();
        assert_eq!(backend.get("delete_key").await.unwrap(), None);

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_exists() {
        let backend = match create_test_redis_backend().await {
            Some(b) => b,
            None => return,
        };

        backend.clear().await.unwrap();

        assert!(!backend.exists("exists_key").await.unwrap());

        backend.put("exists_key".to_string(), b"exists_value".to_vec()).await.unwrap();
        assert!(backend.exists("exists_key").await.unwrap());

        backend.delete("exists_key".to_string()).await.unwrap();
        assert!(!backend.exists("exists_key").await.unwrap());

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_keys_with_prefix() {
        let backend = match create_test_redis_backend().await {
            Some(b) => b,
            None => return,
        };

        backend.clear().await.unwrap();

        // Add some keys with prefix
        backend.put("prefix_key1".to_string(), b"value1".to_vec()).await.unwrap();
        backend.put("prefix_key2".to_string(), b"value2".to_vec()).await.unwrap();
        backend.put("other_key".to_string(), b"value3".to_vec()).await.unwrap();

        let keys = backend.get_keys_with_prefix("prefix_").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix_key1".to_string()));
        assert!(keys.contains(&"prefix_key2".to_string()));

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_redis_unicode() {
        let backend = match create_test_redis_backend().await {
            Some(b) => b,
            None => return,
        };

        backend.clear().await.unwrap();

        // Test with Unicode keys and values
        let unicode_key = "テストキー🚀";
        let unicode_value = "テスト値🌟".as_bytes().to_vec();

        backend.put(unicode_key.to_string(), unicode_value.clone()).await.unwrap();
        assert_eq!(backend.get(unicode_key).await.unwrap(), Some(unicode_value));

        backend.clear().await.unwrap();
    }
}

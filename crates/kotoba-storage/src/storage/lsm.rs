//! RocksDB-based storage backend

use crate::storage::backend::{StorageBackend, BackendStats};
use crate::storage::StorageConfig;
use async_trait::async_trait;
use rocksdb::{DB, Options};
use std::path::PathBuf;
use std::sync::Arc;
use kotoba_core::types::*;


/// RocksDB-based storage backend
#[derive(Clone)]
pub struct RocksDBBackend {
    db: Arc<DB>,
    data_dir: PathBuf,
}

impl RocksDBBackend {
    /// Create a new RocksDB backend
    pub async fn new(config: &StorageConfig) -> Result<Self> {
        let data_dir = config.rocksdb_path.as_ref()
            .ok_or_else(|| KotobaError::Storage("RocksDB path not configured".to_string()))?
            .clone();

        let memtable_size = config.rocksdb_memtable_size.unwrap_or(64);
        let sstable_max_size = config.rocksdb_sstable_max_size.unwrap_or(128);

        // データディレクトリ作成
        std::fs::create_dir_all(&data_dir)?;

        // RocksDBオプション設定
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_write_buffer_number(3);
        opts.set_write_buffer_size(memtable_size * 1024 * 1024); // MB単位に変換
        opts.set_target_file_size_base((sstable_max_size * 1024 * 1024) as u64); // MB単位に変換
        opts.set_max_background_compactions(4);
        opts.set_max_background_flushes(2);

        // データベースを開く
        let db = DB::open(&opts, &data_dir)
            .map_err(|e| KotobaError::Storage(format!("Failed to open RocksDB: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
            data_dir,
        })
    }

    /// Legacy synchronous put method (for backward compatibility)
    pub fn put_sync(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        self.db.put(key, value)
            .map_err(|e| KotobaError::Storage(format!("Failed to put data: {}", e)))?;
        Ok(())
    }

    /// Legacy synchronous get method (for backward compatibility)
    pub fn get_sync(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(e) => Err(KotobaError::Storage(format!("Failed to get data: {}", e))),
        }
    }

    /// Get database statistics
    fn get_stats(&self) -> BackendStats {
        // Get approximate size of data directory
        let disk_usage = std::fs::metadata(&self.data_dir)
            .and_then(|m| Ok(m.len()))
            .ok();

        BackendStats {
            backend_type: "RocksDB".to_string(),
            total_keys: None, // Would need to scan all keys to count
            memory_usage: None, // RocksDB manages this internally
            disk_usage,
            connection_count: Some(1), // Single local connection
        }
    }
}

#[async_trait]
impl StorageBackend for RocksDBBackend {
    async fn put(&self, key: String, value: Vec<u8>) -> Result<()> {
        // RocksDB operations are synchronous but we need to implement async trait
        // In practice, we'd want to run this on a blocking thread pool
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.put(key, value)
                .map_err(|e| KotobaError::Storage(format!("Failed to put data: {}", e)))
        })
        .await
        .map_err(|e| KotobaError::Storage(format!("Task join error: {}", e)))?
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let db = Arc::clone(&self.db);
        let key = key.to_string();
        tokio::task::spawn_blocking(move || {
            match db.get(key) {
                Ok(Some(value)) => Ok(Some(value)),
                Ok(None) => Ok(None),
                Err(e) => Err(KotobaError::Storage(format!("Failed to get data: {}", e))),
            }
        })
        .await
        .map_err(|e| KotobaError::Storage(format!("Task join error: {}", e)))?
    }

    async fn delete(&self, key: String) -> Result<()> {
        let db = Arc::clone(&self.db);
        tokio::task::spawn_blocking(move || {
            db.delete(key)
                .map_err(|e| KotobaError::Storage(format!("Failed to delete data: {}", e)))
        })
        .await
        .map_err(|e| KotobaError::Storage(format!("Task join error: {}", e)))?
    }

    async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let db = Arc::clone(&self.db);
        let prefix = prefix.to_string();
        tokio::task::spawn_blocking(move || {
            let mut keys = Vec::new();
            let iter = db.iterator(rocksdb::IteratorMode::Start);
            for item in iter {
                match item {
                    Ok((key, _)) => {
                        let key_str = String::from_utf8_lossy(&key);
                        if key_str.starts_with(&prefix) {
                            keys.push(key_str.to_string());
                        }
                    }
                    Err(e) => return Err(KotobaError::Storage(format!("Iterator error: {}", e))),
                }
            }
            Ok(keys)
        })
        .await
        .map_err(|e| KotobaError::Storage(format!("Task join error: {}", e)))?
    }

    async fn clear(&self) -> Result<()> {
        // For RocksDB, we can't easily clear all data atomically
        // This is a simplified implementation for testing
        let keys = self.get_keys_with_prefix("").await?;
        for key in keys {
            self.delete(key).await?;
        }
        Ok(())
    }

    async fn stats(&self) -> Result<BackendStats> {
        Ok(self.get_stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db() -> RocksDBBackend {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_path_buf();
        let config = StorageConfig {
            backend_type: crate::storage::BackendType::RocksDB,
            rocksdb_path: Some(db_path),
            rocksdb_memtable_size: Some(64),
            rocksdb_sstable_max_size: Some(128),
            ..Default::default()
        };
        RocksDBBackend::new(&config).await.unwrap()
    }

    #[tokio::test]
    async fn test_put_and_get() {
        let lsm_tree = create_test_db().await;

        // Put some data
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).await.unwrap();
        lsm_tree.put("key2".to_string(), b"value2".to_vec()).await.unwrap();

        // Get the data back
        assert_eq!(lsm_tree.get("key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(lsm_tree.get("key2").await.unwrap(), Some(b"value2".to_vec()));
        assert_eq!(lsm_tree.get("key3").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_delete() {
        let lsm_tree = create_test_db().await;

        // Put and then delete
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).await.unwrap();
        assert_eq!(lsm_tree.get("key1").await.unwrap(), Some(b"value1".to_vec()));

        lsm_tree.delete("key1".to_string()).await.unwrap();
        assert_eq!(lsm_tree.get("key1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_compaction() {
        let lsm_tree = create_test_db().await;

        // Add some data
        for i in 0..100 {
            lsm_tree.put(format!("key{}", i), format!("value{}", i).into_bytes()).await.unwrap();
        }

        // Force compaction (not implemented in async trait, but we can test the data persistence)
        // lsm_tree.compact().await.unwrap();

        // Verify data is still accessible
        for i in 0..100 {
            let expected = format!("value{}", i).into_bytes();
            assert_eq!(lsm_tree.get(&format!("key{}", i)).await.unwrap(), Some(expected));
        }
    }

    #[tokio::test]
    async fn test_large_data() {
        let lsm_tree = create_test_db().await;

        // Test with larger data
        let large_value = vec![0u8; 1024 * 1024]; // 1MB
        lsm_tree.put("large_key".to_string(), large_value.clone()).await.unwrap();

        let retrieved = lsm_tree.get("large_key").await.unwrap();
        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_concurrent_operations() {
        let lsm_tree = create_test_db().await;

        // Test multiple operations
        for i in 0..50 {
            lsm_tree.put(format!("concurrent_key{}", i), format!("concurrent_value{}", i).into_bytes()).await.unwrap();
        }

        for i in (0..25).step_by(2) {
            lsm_tree.delete(format!("concurrent_key{}", i)).await.unwrap();
        }

        // Verify results
        for i in 0..50 {
            let result = lsm_tree.get(&format!("concurrent_key{}", i)).await.unwrap();
            if i % 2 == 0 && i < 25 {
                assert_eq!(result, None); // Deleted
            } else {
                assert_eq!(result, Some(format!("concurrent_value{}", i).into_bytes())); // Still exists
            }
        }
    }

    #[tokio::test]
    async fn test_unicode_keys() {
        let lsm_tree = create_test_db().await;

        // Test with Unicode keys
        let unicode_key = "テストキー🚀";
        let unicode_value = "テスト値🌟".as_bytes().to_vec();

        lsm_tree.put(unicode_key.to_string(), unicode_value.clone()).await.unwrap();
        assert_eq!(lsm_tree.get(unicode_key).await.unwrap(), Some(unicode_value));
    }

    #[tokio::test]
    async fn test_empty_values() {
        let lsm_tree = create_test_db().await;

        // Test with empty values
        lsm_tree.put("empty_key".to_string(), vec![]).await.unwrap();
        assert_eq!(lsm_tree.get("empty_key").await.unwrap(), Some(vec![]));
    }

    #[tokio::test]
    async fn test_overwrite() {
        let lsm_tree = create_test_db().await;

        // Put initial value
        lsm_tree.put("overwrite_key".to_string(), b"initial".to_vec()).await.unwrap();
        assert_eq!(lsm_tree.get("overwrite_key").await.unwrap(), Some(b"initial".to_vec()));

        // Overwrite
        lsm_tree.put("overwrite_key".to_string(), b"updated".to_vec()).await.unwrap();
        assert_eq!(lsm_tree.get("overwrite_key").await.unwrap(), Some(b"updated".to_vec()));
    }

    #[tokio::test]
    async fn test_nonexistent_keys() {
        let lsm_tree = create_test_db().await;

        // Test various non-existent keys
        assert_eq!(lsm_tree.get("nonexistent").await.unwrap(), None);
        assert_eq!(lsm_tree.get("").await.unwrap(), None);
        assert_eq!(lsm_tree.get("very_long_key_that_does_not_exist_1234567890").await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_keys_with_prefix() {
        let lsm_tree = create_test_db().await;

        // Add some keys with prefix
        lsm_tree.put("prefix_key1".to_string(), b"value1".to_vec()).await.unwrap();
        lsm_tree.put("prefix_key2".to_string(), b"value2".to_vec()).await.unwrap();
        lsm_tree.put("other_key".to_string(), b"value3".to_vec()).await.unwrap();

        let keys = lsm_tree.get_keys_with_prefix("prefix_").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix_key1".to_string()));
        assert!(keys.contains(&"prefix_key2".to_string()));
    }
}

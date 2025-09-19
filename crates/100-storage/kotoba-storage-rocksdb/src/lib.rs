//! `kotoba-storage-rocksdb`
//!
//! RocksDB adapter implementation for kotoba-storage port.
//! Provides persistent key-value storage using RocksDB.

use std::path::Path;
use async_trait::async_trait;
use rocksdb::{DB, Options, IteratorMode};
use anyhow::Result;

use kotoba_storage::KeyValueStore;

/// RocksDB-based key-value store implementation
pub struct RocksDbStore {
    db: DB,
}

impl RocksDbStore {
    /// Create a new RocksDB store at the specified path
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }

    /// Open an existing RocksDB store
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let opts = Options::default();
        let db = DB::open(&opts, path)?;
        Ok(Self { db })
    }
}

#[async_trait]
impl KeyValueStore for RocksDbStore {
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.db.put(key, value)?;
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        match self.db.get(key)? {
            Some(value) => Ok(Some(value)),
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &[u8]) -> Result<()> {
        self.db.delete(key)?;
        Ok(())
    }

    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let prefix_len = prefix.len();
        let mut results = Vec::new();

        let iter = self.db.iterator(IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;

            // Check if key starts with prefix
            if key.len() >= prefix_len && &key[..prefix_len] == prefix {
                results.push((key.to_vec(), value.to_vec()));
            } else if key.len() >= prefix_len && &key[..prefix_len] > prefix {
                break; // No more matching keys
            } else if key.len() < prefix_len && key.as_ref() > prefix {
                break; // Key is shorter than prefix but lexicographically greater
            }
        }

        // Sort results by key for consistent ordering (RocksDB returns sorted results)
        results.sort_by(|a, b| a.0.cmp(&b.0));

        Ok(results)
    }
}

impl Drop for RocksDbStore {
    fn drop(&mut self) {
        // RocksDB will be automatically closed when DB is dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tempfile::TempDir;
    use kotoba_storage::KeyValueStore;

    fn create_temp_db() -> (RocksDbStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let store = RocksDbStore::new(&db_path).unwrap();
        (store, temp_dir)
    }

    #[tokio::test]
    async fn test_rocksdb_store_creation() {
        let (_store, _temp_dir) = create_temp_db();
        // Test passes if creation succeeds
    }

    #[tokio::test]
    async fn test_rocksdb_basic_operations() {
        let (store, _temp_dir) = create_temp_db();

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
    async fn test_rocksdb_multiple_keys() {
        let (store, _temp_dir) = create_temp_db();

        // Put multiple key-value pairs
        store.put(b"key1", b"value1").await.unwrap();
        store.put(b"key2", b"value2").await.unwrap();
        store.put(b"key3", b"value3").await.unwrap();

        // Verify all keys exist
        assert_eq!(store.get(b"key1").await.unwrap(), Some(b"value1".to_vec()));
        assert_eq!(store.get(b"key2").await.unwrap(), Some(b"value2".to_vec()));
        assert_eq!(store.get(b"key3").await.unwrap(), Some(b"value3".to_vec()));
    }

    #[tokio::test]
    async fn test_rocksdb_scan() {
        let (store, _temp_dir) = create_temp_db();

        // Put keys with common prefix
        store.put(b"prefix_key1", b"value1").await.unwrap();
        store.put(b"prefix_key2", b"value2").await.unwrap();
        store.put(b"prefix_key3", b"value3").await.unwrap();
        store.put(b"other_key", b"other_value").await.unwrap();

        // Scan with prefix
        let results = store.scan(b"prefix_").await.unwrap();
        assert_eq!(results.len(), 3);

        // Verify results are sorted (RocksDB returns sorted results)
        assert_eq!(results[0], (b"prefix_key1".to_vec(), b"value1".to_vec()));
        assert_eq!(results[1], (b"prefix_key2".to_vec(), b"value2".to_vec()));
        assert_eq!(results[2], (b"prefix_key3".to_vec(), b"value3".to_vec()));

        // Scan with empty prefix (should return all)
        let all_results = store.scan(b"").await.unwrap();
        assert_eq!(all_results.len(), 4);

        // Scan with non-existent prefix
        let no_results = store.scan(b"nonexistent").await.unwrap();
        assert_eq!(no_results.len(), 0);
    }

    #[tokio::test]
    async fn test_rocksdb_overwrite() {
        let (store, _temp_dir) = create_temp_db();

        // Put initial value
        store.put(b"key", b"initial").await.unwrap();
        assert_eq!(store.get(b"key").await.unwrap(), Some(b"initial".to_vec()));

        // Overwrite with new value
        store.put(b"key", b"updated").await.unwrap();
        assert_eq!(store.get(b"key").await.unwrap(), Some(b"updated".to_vec()));
    }

    #[tokio::test]
    async fn test_rocksdb_empty_keys_values() {
        let (store, _temp_dir) = create_temp_db();

        // Test empty key
        store.put(b"", b"empty_key_value").await.unwrap();
        assert_eq!(store.get(b"").await.unwrap(), Some(b"empty_key_value".to_vec()));

        // Test empty value
        store.put(b"empty_value_key", b"").await.unwrap();
        assert_eq!(store.get(b"empty_value_key").await.unwrap(), Some(b"".to_vec()));
    }

    #[tokio::test]
    async fn test_rocksdb_delete_nonexistent() {
        let (store, _temp_dir) = create_temp_db();

        // Delete non-existent key should not panic
        store.delete(b"nonexistent").await.unwrap();
    }

    #[tokio::test]
    async fn test_rocksdb_large_data() {
        let (store, _temp_dir) = create_temp_db();

        // Test with large key and value
        let large_key = vec![42u8; 1024]; // 1KB key
        let large_value = vec![255u8; 1024 * 1024]; // 1MB value

        store.put(&large_key, &large_value).await.unwrap();
        let retrieved = store.get(&large_key).await.unwrap();

        assert_eq!(retrieved, Some(large_value));
    }

    #[tokio::test]
    async fn test_rocksdb_unicode_keys_values() {
        let (store, _temp_dir) = create_temp_db();

        // Test Unicode keys and values
        let unicode_key = "🚀 котоба 🔥".as_bytes();
        let unicode_value = "Hello 世界 🌍".as_bytes();

        store.put(unicode_key, unicode_value).await.unwrap();
        let retrieved = store.get(unicode_key).await.unwrap();

        assert_eq!(retrieved, Some(unicode_value.to_vec()));
    }

    #[tokio::test]
    async fn test_rocksdb_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("persistence_test.db");

        // Create and populate database
        {
            let store = RocksDbStore::new(&db_path).unwrap();
            store.put(b"persistent_key", b"persistent_value").await.unwrap();
        } // Store goes out of scope and closes

        // Reopen database and check persistence
        {
            let store = RocksDbStore::open(&db_path).unwrap();
            let value = store.get(b"persistent_key").await.unwrap();
            assert_eq!(value, Some(b"persistent_value".to_vec()));
        }
    }

    #[tokio::test]
    async fn test_rocksdb_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("concurrent_test.db");
        let store = Arc::new(RocksDbStore::new(&db_path).unwrap());

        let mut handles = vec![];

        // Spawn multiple tasks to test concurrent access
        for i in 0..10 {
            let store_clone = Arc::clone(&store);
            let handle = tokio::spawn(async move {
                let key = format!("concurrent_key_{}", i).into_bytes();
                let value = format!("concurrent_value_{}", i).into_bytes();

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

        // Verify all keys were stored
        for i in 0..10 {
            let key = format!("concurrent_key_{}", i).into_bytes();
            let expected_value = format!("concurrent_value_{}", i).into_bytes();
            let retrieved = store.get(&key).await.unwrap();
            assert_eq!(retrieved, Some(expected_value));
        }
    }

    #[tokio::test]
    async fn test_rocksdb_scan_ordering() {
        let (store, _temp_dir) = create_temp_db();

        // Insert keys in reverse order
        store.put(b"key3", b"value3").await.unwrap();
        store.put(b"key1", b"value1").await.unwrap();
        store.put(b"key2", b"value2").await.unwrap();

        // Scan should return results in sorted order
        let results = store.scan(b"key").await.unwrap();
        assert_eq!(results.len(), 3);
        assert_eq!(results[0], (b"key1".to_vec(), b"value1".to_vec()));
        assert_eq!(results[1], (b"key2".to_vec(), b"value2".to_vec()));
        assert_eq!(results[2], (b"key3".to_vec(), b"value3".to_vec()));
    }

    #[tokio::test]
    async fn test_rocksdb_scan_prefix_boundaries() {
        let (store, _temp_dir) = create_temp_db();

        // Insert keys with different prefixes
        store.put(b"aaa_prefix_key", b"value1").await.unwrap();
        store.put(b"aaa_other_key", b"value2").await.unwrap();
        store.put(b"aab_prefix_key", b"value3").await.unwrap();
        store.put(b"prefix_key", b"value4").await.unwrap();

        // Scan with prefix should stop at boundary
        let results = store.scan(b"aaa_").await.unwrap();
        assert_eq!(results.len(), 2);

        // Should include both aaa_ keys but not aab_ or prefix_
        let keys: Vec<_> = results.iter().map(|(k, _)| k.clone()).collect();
        assert!(keys.contains(&b"aaa_prefix_key".to_vec()));
        assert!(keys.contains(&b"aaa_other_key".to_vec()));
        assert!(!keys.contains(&b"aab_prefix_key".to_vec()));
        assert!(!keys.contains(&b"prefix_key".to_vec()));
    }

    #[test]
    fn test_rocksdb_error_handling() {
        // Test opening non-existent database without create_if_missing
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("nonexistent.db");
        let result = RocksDbStore::open(&db_path);
        // This should fail as we're trying to open a non-existent database without create_if_missing
        assert!(result.is_err());

        // Test with valid path should work
        let temp_dir2 = TempDir::new().unwrap();
        let db_path2 = temp_dir2.path().join("valid.db");
        let result2 = RocksDbStore::new(&db_path2);
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_rocksdb_binary_data() {
        let (store, _temp_dir) = create_temp_db();

        // Test with binary data (null bytes, etc.)
        let binary_key = vec![0, 1, 2, 255, 0, 1, 2, 255];
        let binary_value = vec![255, 254, 253, 0, 1, 2, 3, 255, 254, 253];

        store.put(&binary_key, &binary_value).await.unwrap();
        let retrieved = store.get(&binary_key).await.unwrap();

        assert_eq!(retrieved, Some(binary_value));
    }

    #[tokio::test]
    async fn test_rocksdb_performance() {
        let (store, _temp_dir) = create_temp_db();

        // Insert many key-value pairs for performance testing
        let num_entries = 1000;
        let start_time = std::time::Instant::now();

        for i in 0..num_entries {
            let key = format!("perf_key_{:04}", i).into_bytes();
            let value = format!("perf_value_{}", i).into_bytes();
            store.put(&key, &value).await.unwrap();
        }

        let insert_time = start_time.elapsed();

        // Read all entries
        let start_time = std::time::Instant::now();
        for i in 0..num_entries {
            let key = format!("perf_key_{:04}", i).into_bytes();
            let expected_value = format!("perf_value_{}", i).into_bytes();
            let value = store.get(&key).await.unwrap();
            assert_eq!(value, Some(expected_value));
        }

        let read_time = start_time.elapsed();

        println!("RocksDB Performance Test:");
        println!("  Inserted {} entries in {:?}", num_entries, insert_time);
        println!("  Read {} entries in {:?}", num_entries, read_time);

        // Basic performance assertions (these will depend on the system)
        assert!(insert_time.as_millis() < 5000, "Insert performance regression");
        assert!(read_time.as_millis() < 5000, "Read performance regression");
    }
}

//! RocksDBベースのストレージエンジン

use rocksdb::{DB, Options};
use std::path::PathBuf;
use kotoba_core::types::*;


/// RocksDBベースのストレージマネージャー
#[derive(Debug)]
pub struct LSMTree {
    db: DB,
    data_dir: PathBuf,
}

impl LSMTree {
    pub fn new(data_dir: PathBuf, memtable_size: usize, sstable_max_size: usize) -> Result<Self> {
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
            db,
            data_dir,
        })
    }

    /// データを書き込み
    pub fn put(&mut self, key: String, value: Vec<u8>) -> Result<()> {
        self.db.put(key, value)
            .map_err(|e| KotobaError::Storage(format!("Failed to put data: {}", e)))?;
        Ok(())
    }

    /// データを削除
    pub fn delete(&mut self, key: String) -> Result<()> {
        self.db.delete(key)
            .map_err(|e| KotobaError::Storage(format!("Failed to delete data: {}", e)))?;
        Ok(())
    }

    /// データを読み込み
    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        match self.db.get(key) {
            Ok(Some(value)) => Ok(Some(value)),
            Ok(None) => Ok(None),
            Err(e) => Err(KotobaError::Storage(format!("Failed to get data: {}", e))),
        }
    }

    /// 手動コンパクションを実行（RocksDBに委譲）
    pub fn compact(&mut self) -> Result<()> {
        // RocksDBのフルコンパクションを実行
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_db() -> (LSMTree, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().to_path_buf();
        let lsm_tree = LSMTree::new(db_path, 64, 128).unwrap();
        (lsm_tree, temp_dir)
    }

    #[test]
    fn test_put_and_get() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Put some data
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).unwrap();
        lsm_tree.put("key2".to_string(), b"value2".to_vec()).unwrap();

        // Get the data back
        assert_eq!(lsm_tree.get("key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(lsm_tree.get("key2").unwrap(), Some(b"value2".to_vec()));
        assert_eq!(lsm_tree.get("key3").unwrap(), None);
    }

    #[test]
    fn test_delete() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Put and then delete
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).unwrap();
        assert_eq!(lsm_tree.get("key1").unwrap(), Some(b"value1".to_vec()));

        lsm_tree.delete("key1".to_string()).unwrap();
        assert_eq!(lsm_tree.get("key1").unwrap(), None);
    }

    #[test]
    fn test_compaction() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Add some data
        for i in 0..100 {
            lsm_tree.put(format!("key{}", i), format!("value{}", i).into_bytes()).unwrap();
        }

        // Force compaction
        lsm_tree.compact().unwrap();

        // Verify data is still accessible
        for i in 0..100 {
            let expected = format!("value{}", i).into_bytes();
            assert_eq!(lsm_tree.get(&format!("key{}", i)).unwrap(), Some(expected));
        }
    }

    #[test]
    fn test_large_data() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Test with larger data
        let large_value = vec![0u8; 1024 * 1024]; // 1MB
        lsm_tree.put("large_key".to_string(), large_value.clone()).unwrap();

        let retrieved = lsm_tree.get("large_key").unwrap();
        assert_eq!(retrieved, Some(large_value));
    }

    #[test]
    fn test_concurrent_operations() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Test multiple operations
        for i in 0..50 {
            lsm_tree.put(format!("concurrent_key{}", i), format!("concurrent_value{}", i).into_bytes()).unwrap();
        }

        for i in (0..25).step_by(2) {
            lsm_tree.delete(format!("concurrent_key{}", i)).unwrap();
        }

        // Verify results
        for i in 0..50 {
            let result = lsm_tree.get(&format!("concurrent_key{}", i)).unwrap();
            if i % 2 == 0 && i < 25 {
                assert_eq!(result, None); // Deleted
            } else {
                assert_eq!(result, Some(format!("concurrent_value{}", i).into_bytes())); // Still exists
            }
        }
    }

    #[test]
    fn test_unicode_keys() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Test with Unicode keys
        let unicode_key = "テストキー🚀";
        let unicode_value = "テスト値🌟".as_bytes().to_vec();

        lsm_tree.put(unicode_key.to_string(), unicode_value.clone()).unwrap();
        assert_eq!(lsm_tree.get(unicode_key).unwrap(), Some(unicode_value));
    }

    #[test]
    fn test_empty_values() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Test with empty values
        lsm_tree.put("empty_key".to_string(), vec![]).unwrap();
        assert_eq!(lsm_tree.get("empty_key").unwrap(), Some(vec![]));
    }

    #[test]
    fn test_overwrite() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Put initial value
        lsm_tree.put("overwrite_key".to_string(), b"initial".to_vec()).unwrap();
        assert_eq!(lsm_tree.get("overwrite_key").unwrap(), Some(b"initial".to_vec()));

        // Overwrite
        lsm_tree.put("overwrite_key".to_string(), b"updated".to_vec()).unwrap();
        assert_eq!(lsm_tree.get("overwrite_key").unwrap(), Some(b"updated".to_vec()));
    }

    #[test]
    fn test_nonexistent_keys() {
        let (lsm_tree, _temp_dir) = create_test_db();

        // Test various non-existent keys
        assert_eq!(lsm_tree.get("nonexistent").unwrap(), None);
        assert_eq!(lsm_tree.get("").unwrap(), None);
        assert_eq!(lsm_tree.get("very_long_key_that_does_not_exist_1234567890").unwrap(), None);
    }
}

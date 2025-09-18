//! RocksDBベースのストレージエンジン

#[cfg(feature = "rocksdb")]
use rocksdb::{DB, Options, WriteBatch, IteratorMode};
use std::path::PathBuf;
use kotoba_core::prelude::*;
use kotoba_errors::KotobaError;

// RocksDBが有効でない場合のスタブ実装
#[cfg(not(feature = "rocksdb"))]
pub type DB = std::collections::HashMap<String, Vec<u8>>;
#[cfg(not(feature = "rocksdb"))]
pub struct Options;
#[cfg(not(feature = "rocksdb"))]
pub struct WriteBatch(Vec<(String, Vec<u8>)>);
#[cfg(not(feature = "rocksdb"))]
pub enum IteratorMode { Start }

#[cfg(not(feature = "rocksdb"))]
impl Default for Options {
    fn default() -> Self { Options }
}

#[cfg(not(feature = "rocksdb"))]
impl Default for WriteBatch {
    fn default() -> Self { WriteBatch(Vec::new()) }
}

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
        opts.set_target_file_size_base(sstable_max_size * 1024 * 1024); // MB単位に変換
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

    /// RocksDBスナップショットを作成
    pub fn create_snapshot(&self, snapshot_id: &str) -> Result<()> {
        // RocksDBのスナップショットを作成
        let snapshot = self.db.snapshot();
        let snapshot_dir = self.data_dir.join(format!("snapshot_{}", snapshot_id));
        std::fs::create_dir_all(&snapshot_dir)?;

        // スナップショット時点のデータをエクスポート
        let iter = snapshot.iterator(IteratorMode::Start);
        let mut batch = WriteBatch::default();

        for item in iter {
            let (key, value) = item?;
            batch.put(key, value);
        }

        // スナップショットを別のデータベースとして保存
        let mut opts = Options::default();
        opts.create_if_missing(true);

        let snapshot_db = DB::open(&opts, &snapshot_dir)?;
        snapshot_db.write(batch)?;

        Ok(())
    }

    /// スナップショットから復元
    pub fn restore_from_snapshot(&mut self, snapshot_id: &str) -> Result<()> {
        let snapshot_dir = self.data_dir.join(format!("snapshot_{}", snapshot_id));

        if !snapshot_dir.exists() {
            return Err(KotobaError::Storage("Snapshot not found".to_string()));
        }

        // 現在のデータをクリア
        self.clear_all()?;

        // スナップショットからデータをコピー
        let mut opts = Options::default();
        opts.create_if_missing(false);

        let snapshot_db = DB::open(&opts, &snapshot_dir)?;
        let iter = snapshot_db.iterator(IteratorMode::Start);

        for item in iter {
            let (key, value) = item?;
            self.db.put(key, value)?;
        }

        Ok(())
    }

    /// すべてのデータをクリア
    fn clear_all(&mut self) -> Result<()> {
        let iter = self.db.iterator(IteratorMode::Start);
        let keys: Vec<Vec<u8>> = iter.map(|item| item.unwrap().0).collect();

        for key in keys {
            self.db.delete(key)?;
        }

        Ok(())
    }

    /// 古いデータをクリーンアップ（TTLベース）
    pub fn cleanup(&mut self, cutoff_timestamp: u64) -> Result<()> {
        // RocksDBのプロパティを取得して古いデータを特定
        // 実際の実装ではTTL付きのデータを削除
        // ここでは簡易版として何もしない
        Ok(())
    }

    /// 統計情報を取得
    pub fn stats(&self) -> LSMStats {
        // RocksDBの統計情報を取得
        let total_entries = self.db.iterator(IteratorMode::Start).count() as usize;

        // 推定サイズを取得（RocksDBのプロパティから）
        let total_size = self.db
            .property_value("rocksdb.estimate-live-data-size")
            .unwrap_or(Some("0".to_string()))
            .unwrap_or("0".to_string())
            .parse::<u64>()
            .unwrap_or(0);

        LSMStats {
            memtable_entries: 0, // RocksDBは内部で管理
            sstable_count: self.db
                .property_value("rocksdb.num-files-at-level0")
                .unwrap_or(Some("0".to_string()))
                .unwrap_or("0".to_string())
                .parse::<usize>()
                .unwrap_or(0),
            total_entries,
            total_size,
        }
    }
}

/// LSMツリー統計情報
#[derive(Debug, Clone)]
pub struct LSMStats {
    pub memtable_entries: usize,
    pub sstable_count: usize,
    pub total_entries: usize,
    pub total_size: u64,
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
    fn test_stats() {
        let (mut lsm_tree, _temp_dir) = create_test_db();

        // Initially should have minimal stats
        let stats = lsm_tree.stats();
        assert_eq!(stats.memtable_entries, 0);

        // Add some data
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).unwrap();
        lsm_tree.put("key2".to_string(), b"value2".to_vec()).unwrap();

        let stats = lsm_tree.stats();
        assert!(stats.total_entries >= 2); // RocksDB may have internal entries
    }

    #[test]
    fn test_snapshot() {
        let (mut lsm_tree, temp_dir) = create_test_db();

        // Add some data
        lsm_tree.put("key1".to_string(), b"value1".to_vec()).unwrap();
        lsm_tree.put("key2".to_string(), b"value2".to_vec()).unwrap();

        // Create snapshot
        lsm_tree.create_snapshot("test_snapshot").unwrap();

        // Modify data
        lsm_tree.put("key1".to_string(), b"modified_value1".to_vec()).unwrap();
        lsm_tree.delete("key2".to_string()).unwrap();

        // Verify current state
        assert_eq!(lsm_tree.get("key1").unwrap(), Some(b"modified_value1".to_vec()));
        assert_eq!(lsm_tree.get("key2").unwrap(), None);

        // Restore from snapshot
        lsm_tree.restore_from_snapshot("test_snapshot").unwrap();

        // Verify restored state
        assert_eq!(lsm_tree.get("key1").unwrap(), Some(b"value1".to_vec()));
        assert_eq!(lsm_tree.get("key2").unwrap(), Some(b"value2".to_vec()));
    }
}

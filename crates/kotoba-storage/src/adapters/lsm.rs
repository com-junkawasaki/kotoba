//! RocksDBベースのストレージエンジン

use crate::domain::kv::KeyValuePort;
use crate::domain::models::BackendStats;
use async_trait::async_trait;

#[cfg(feature = "rocksdb")]
use rocksdb::{DB, Options, WriteBatch, IteratorMode};
#[cfg(not(feature = "rocksdb"))]
use std::collections::HashMap;
use std::path::PathBuf;
// use kotoba_core::prelude::*; // Removed to avoid version conflicts
use anyhow::{anyhow, Error};

// Stub implementations for when rocksdb feature is not enabled
#[cfg(not(feature = "rocksdb"))]
pub type DB = HashMap<String, Vec<u8>>;
#[cfg(not(feature = "rocksdb"))]
#[derive(Default)]
pub struct Options;
#[cfg(not(feature = "rocksdb"))]
#[derive(Default)]
pub struct WriteBatch(Vec<(String, Vec<u8>)>);
#[cfg(not(feature = "rocksdb"))]
pub enum IteratorMode { Start }

/// RocksDB-based storage manager
#[derive(Debug)]
pub struct LSMTree {
    #[cfg(feature = "rocksdb")]
    db: DB,
    #[cfg(not(feature = "rocksdb"))]
    db: std::sync::RwLock<DB>,
    data_dir: PathBuf,
}

#[cfg(feature = "rocksdb")]
impl LSMTree {
    pub fn new(data_dir: PathBuf, memtable_size: usize, sstable_max_size: usize) -> Result<Self> {
        std::fs::create_dir_all(&data_dir)?;

        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.set_max_write_buffer_number(3);
        opts.set_write_buffer_size(memtable_size * 1024 * 1024);
        opts.set_target_file_size_base(sstable_max_size * 1024 * 1024);
        opts.set_max_background_compactions(4);
        opts.set_max_background_flushes(2);

        let db = DB::open(&opts, &data_dir)
            .map_err(|e| anyhow!("Failed to open RocksDB: {}", e))?;

        Ok(Self { db, data_dir })
    }

    pub fn create_snapshot(&self, snapshot_id: &str) -> Result<()> {
        let snapshot = self.db.snapshot();
        let snapshot_dir = self.data_dir.join(snapshot_id);
        
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let snapshot_db = DB::open(&opts, &snapshot_dir)?;

        let iter = snapshot.iterator(IteratorMode::Start);
        let mut batch = WriteBatch::default();
        for (key, value) in iter {
            batch.put(key, value);
        }
        snapshot_db.write(batch).map_err(|e| anyhow!("RocksDB write error: {}", e))?;
        Ok(())
    }

    pub fn restore_from_snapshot(&mut self, snapshot_id: &str) -> Result<()> {
        let snapshot_dir = self.data_dir.join(snapshot_id);
        let mut opts = Options::default();
        opts.create_if_missing(false);
        let snapshot_db = DB::open(&opts, &snapshot_dir)?;
        let iter = snapshot_db.iterator(IteratorMode::Start);
        for (key, value) in iter {
            self.db.put(key, value)?;
        }
        Ok(())
    }
    
    pub fn compact(&mut self) -> Result<()> {
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }

    pub fn cleanup(&mut self, _cutoff_timestamp: u64) -> Result<()> {
        // RocksDB TTL or custom logic would be needed here
        Ok(())
    }

    pub fn stats(&self) -> BackendStats {
        let total_entries = self.db.iterator(IteratorMode::Start).count();
        let total_size = self.db.property_value("rocksdb.estimate-live-data-size").unwrap_or(None).unwrap_or("0").parse().unwrap_or(0);
        let sstable_count = self.db.property_value("rocksdb.num-files-at-level0").unwrap_or(None).unwrap_or("0").parse().unwrap_or(0);

        BackendStats {
            backend_type: "RocksDB".to_string(),
            total_keys: Some(total_entries as u64),
            memory_usage: None, 
            disk_usage: Some(total_size),
            connection_count: Some(sstable_count),
        }
    }
}

#[cfg(not(feature = "rocksdb"))]
impl LSMTree {
    pub fn new(data_dir: PathBuf, _memtable_size: usize, _sstable_max_size: usize) -> Result<Self, Error> {
        Ok(Self { db: std::sync::RwLock::new(HashMap::new()), data_dir })
    }
    pub fn create_snapshot(&self, _snapshot_id: &str) -> Result<(), Error> { Ok(()) }
    pub fn restore_from_snapshot(&mut self, _snapshot_id: &str) -> Result<(), Error> { Ok(()) }
    pub fn compact(&mut self) -> Result<(), Error> { Ok(()) }
    pub fn cleanup(&mut self, _cutoff_timestamp: u64) -> Result<(), Error> { Ok(()) }
    pub fn stats(&self) -> BackendStats { BackendStats::default() }
}

#[async_trait]
impl KeyValuePort for LSMTree {
    async fn put(&self, key: String, value: Vec<u8>) -> Result<(), Error> {
        #[cfg(feature = "rocksdb")]
        {
            self.db.put(key, value).map_err(|e| anyhow!("RocksDB put error: {}", e))
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            self.db.write().unwrap().insert(key, value);
            Ok(())
        }
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, Error> {
        #[cfg(feature = "rocksdb")]
        {
            self.db.get(key).map_err(|e| anyhow!("RocksDB get error: {}", e))
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            Ok(self.db.read().unwrap().get(key).cloned())
        }
    }

    async fn delete(&self, key: String) -> Result<(), Error> {
        #[cfg(feature = "rocksdb")]
        {
            self.db.delete(key).map_err(|e| anyhow!("RocksDB delete error: {}", e))
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            self.db.write().unwrap().remove(&key);
            Ok(())
        }
    }

    async fn scan(&self, prefix: &str) -> Result<Vec<(Vec<u8>, Vec<u8>)>, Error> {
        #[cfg(feature = "rocksdb")]
        {
            let iter = self.db.prefix_iterator(prefix.as_bytes());
            Ok(iter.map(|(k, v)| (k.to_vec(), v.to_vec())).collect())
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            let db = self.db.read().unwrap();
            Ok(db.iter()
                .filter(|(k, _)| k.starts_with(prefix))
                .map(|(k, v)| (k.as_bytes().to_vec(), v.clone()))
                .collect())
        }
    }

    async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>, Error> {
        #[cfg(feature = "rocksdb")]
        {
            let iter = self.db.prefix_iterator(prefix.as_bytes());
            Ok(iter.map(|(k, _v)| String::from_utf8_lossy(&k).to_string()).collect())
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            Ok(self.db.read().unwrap().keys().filter(|k| k.starts_with(prefix)).cloned().collect())
        }
    }

    async fn clear(&self) -> Result<(), Error> {
        #[cfg(feature = "rocksdb")]
        {
             let iter = self.db.iterator(IteratorMode::Start);
             for (key, _) in iter {
                 self.db.delete(key)?;
             }
             Ok(())
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            self.db.write().unwrap().clear();
            Ok(())
        }
    }
    
    async fn stats(&self) -> Result<BackendStats, Error> {
        Ok(self.stats())
    }

    async fn exists(&self, key: &str) -> Result<bool, Error> {
        #[cfg(feature = "rocksdb")]
        {
            Ok(self.db.get(key)?.is_some())
        }
        #[cfg(not(feature = "rocksdb"))]
        {
            Ok(self.db.read().unwrap().contains_key(key))
        }
    }
}

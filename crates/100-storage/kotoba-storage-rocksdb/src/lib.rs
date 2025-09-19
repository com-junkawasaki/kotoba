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
            if key.len() >= prefix_len && &key[..prefix_len] == prefix {
                results.push((key.to_vec(), value.to_vec()));
            } else if &key[..prefix_len] > prefix {
                break; // No more matching keys
            }
        }

        Ok(results)
    }
}

impl Drop for RocksDbStore {
    fn drop(&mut self) {
        // RocksDB will be automatically closed when DB is dropped
    }
}

//! LSM-Tree based storage engine for KotobaDB.
//!
//! This engine implements a Log-Structured Merge-Tree (LSM-Tree) architecture,
//! providing high-performance storage with efficient writes and optimized reads.
//!
//! Features:
//! - Write-Ahead Log (WAL) for durability
//! - MemTable for in-memory buffering
//! - SSTable files for persistent storage
//! - Background compaction for performance optimization

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::io::AsyncWriteExt;
use anyhow::Result;
use kotoba_db_core::engine::StorageEngine;

/// LSM-Tree based storage engine implementation.
pub struct LSMStorageEngine {
    /// Path to the database directory
    db_path: PathBuf,
    /// In-memory buffer (MemTable)
    memtable: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
    /// Write-Ahead Log for durability
    wal: Arc<RwLock<WAL>>,
}

impl LSMStorageEngine {
    /// Creates a new LSM storage engine at the specified path.
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();

        // Create database directory if it doesn't exist
        tokio::fs::create_dir_all(&db_path).await?;

        let memtable = Arc::new(RwLock::new(BTreeMap::new()));
        let wal = Arc::new(RwLock::new(WAL::new(db_path.join("wal"))?));

        Ok(Self {
            db_path,
            memtable,
            wal,
        })
    }
}

#[async_trait::async_trait]
impl StorageEngine for LSMStorageEngine {
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        let mut wal = self.wal.write().await;

        // Write to WAL first for durability
        wal.append(key, value).await?;
        drop(wal); // Release WAL lock

        // Then update memtable
        let mut memtable = self.memtable.write().await;
        memtable.insert(key.to_vec(), value.to_vec());

        // Check if memtable needs to be flushed
        let needs_flush = memtable.len() > 1000; // Simple threshold for now
        drop(memtable); // Release the lock

        if needs_flush {
            self.flush_memtable().await?;
        }

        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        let memtable = self.memtable.read().await;

        // First check memtable
        if let Some(value) = memtable.get(key) {
            return Ok(Some(value.clone()));
        }

        // TODO: Check SSTable files
        // For now, return None if not in memtable
        Ok(None)
    }

    async fn delete(&mut self, key: &[u8]) -> Result<()> {
        let mut memtable = self.memtable.write().await;
        let mut wal = self.wal.write().await;

        // Write tombstone to WAL
        wal.append(key, &[]).await?;

        // Mark as deleted in memtable (empty value = tombstone)
        memtable.insert(key.to_vec(), Vec::new());

        Ok(())
    }

    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let memtable = self.memtable.read().await;
        let mut results = Vec::new();

        // Scan memtable
        for (key, value) in memtable.range(prefix.to_vec()..) {
            if !key.starts_with(prefix) {
                break;
            }
            if !value.is_empty() { // Skip tombstones
                results.push((key.clone(), value.clone()));
            }
        }

        // TODO: Scan SSTable files and merge results

        Ok(results)
    }
}

impl LSMStorageEngine {
    /// Flushes the current memtable to disk as an SSTable file.
    async fn flush_memtable(&mut self) -> Result<()> {
        let mut memtable = self.memtable.write().await;
        if memtable.is_empty() {
            return Ok(());
        }

        // Generate SSTable file name
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let sstable_path = self.db_path.join(format!("sstable_{}.dat", timestamp));

        // Write memtable contents to SSTable file
        let mut file = tokio::fs::File::create(&sstable_path).await?;
        for (key, value) in &*memtable {
            // Simple binary format: [key_len(4bytes)][key][value_len(4bytes)][value]
            let key_len = (key.len() as u32).to_le_bytes();
            let value_len = (value.len() as u32).to_le_bytes();

            tokio::io::AsyncWriteExt::write_all(&mut file, &key_len).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, key).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &value_len).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, value).await?;
        }

        // Clear memtable and reset WAL
        memtable.clear();
        let mut wal = self.wal.write().await;
        wal.reset().await?;

        Ok(())
    }
}

/// Write-Ahead Log for durability
struct WAL {
    path: PathBuf,
    file: Option<tokio::fs::File>,
    sequence: u64,
}

impl WAL {
    fn new(path: PathBuf) -> Result<Self> {
        Ok(Self {
            path,
            file: None,
            sequence: 0,
        })
    }

    async fn append(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        if self.file.is_none() {
            // Open or create WAL file
            self.file = Some(tokio::fs::File::create(&self.path).await?);
        }

        if let Some(file) = &mut self.file {
            // Write entry: [sequence(8)][key_len(4)][key][value_len(4)][value]
            let seq_bytes = self.sequence.to_le_bytes();
            let key_len = (key.len() as u32).to_le_bytes();
            let value_len = (value.len() as u32).to_le_bytes();

            tokio::io::AsyncWriteExt::write_all(file, &seq_bytes).await?;
            tokio::io::AsyncWriteExt::write_all(file, &key_len).await?;
            tokio::io::AsyncWriteExt::write_all(file, key).await?;
            tokio::io::AsyncWriteExt::write_all(file, &value_len).await?;
            tokio::io::AsyncWriteExt::write_all(file, value).await?;

            // Flush to ensure durability
            file.flush().await?;
        }

        self.sequence += 1;
        Ok(())
    }

    async fn reset(&mut self) -> Result<()> {
        if let Some(file) = &mut self.file {
            file.flush().await?;
            // In a real implementation, we'd rotate the WAL file here
        }
        Ok(())
    }
}

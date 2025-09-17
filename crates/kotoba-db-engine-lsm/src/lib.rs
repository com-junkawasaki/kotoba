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

/// Configuration for compaction behavior
#[derive(Clone)]
pub struct CompactionConfig {
    /// Maximum number of SSTable files before triggering compaction
    pub max_sstables: usize,
    /// Minimum number of SSTable files to compact together
    pub min_compaction_files: usize,
}

impl Default for CompactionConfig {
    fn default() -> Self {
        Self {
            max_sstables: 10,
            min_compaction_files: 4,
        }
    }
}

/// LSM-Tree based storage engine implementation.
pub struct LSMStorageEngine {
    /// Path to the database directory
    db_path: PathBuf,
    /// In-memory buffer (MemTable)
    memtable: Arc<RwLock<BTreeMap<Vec<u8>, Vec<u8>>>>,
    /// Write-Ahead Log for durability
    wal: Arc<RwLock<WAL>>,
    /// List of SSTable files (most recent first)
    sstables: Arc<RwLock<Vec<SSTableHandle>>>,
    /// Compaction configuration
    compaction_config: CompactionConfig,
}

impl LSMStorageEngine {
    /// Creates a new LSM storage engine at the specified path with default compaction config.
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::with_config(path, CompactionConfig::default()).await
    }

    /// Creates a new LSM storage engine at the specified path with custom compaction config.
    pub async fn with_config<P: AsRef<Path>>(path: P, config: CompactionConfig) -> Result<Self> {
        let db_path = path.as_ref().to_path_buf();

        // Create database directory if it doesn't exist
        tokio::fs::create_dir_all(&db_path).await?;

        let memtable = Arc::new(RwLock::new(BTreeMap::new()));
        let wal = Arc::new(RwLock::new(WAL::new(db_path.join("wal"))?));

        // Load existing SSTable files
        let sstables = Arc::new(RwLock::new(Self::load_sstables(&db_path).await?));

        Ok(Self {
            db_path,
            memtable,
            wal,
            sstables,
            compaction_config: config,
        })
    }

    /// Load existing SSTable files from disk, sorted by creation time (newest first)
    async fn load_sstables(db_path: &Path) -> Result<Vec<SSTableHandle>> {
        let mut sstables = Vec::new();

        // Read directory and find SSTable files
        let mut entries = tokio::fs::read_dir(db_path).await?;
        let mut sstable_paths = Vec::new();

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                if filename.starts_with("sstable_") && filename.ends_with(".dat") {
                    sstable_paths.push(path);
                }
            }
        }

        // Sort by modification time (newest first)
        sstable_paths.sort_by(|a, b| {
            let a_meta = std::fs::metadata(a).unwrap();
            let b_meta = std::fs::metadata(b).unwrap();
            b_meta.modified().unwrap().cmp(&a_meta.modified().unwrap())
        });

        // Load SSTable handles
        for path in sstable_paths {
            match SSTableHandle::load(&path).await {
                Ok(handle) => sstables.push(handle),
                Err(e) => eprintln!("Warning: Failed to load SSTable {:?}: {}", path, e),
            }
        }

        Ok(sstables)
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

        // First check memtable (most recent data)
        if let Some(value) = memtable.get(key) {
            // Check if it's a tombstone (empty value)
            if value.is_empty() {
                return Ok(None);
            }
            return Ok(Some(value.clone()));
        }

        // Then check SSTable files (newest to oldest)
        let sstables = self.sstables.read().await;
        for sstable in &*sstables {
            if let Some(value) = sstable.search(key).await? {
                // Found in SSTable
                return Ok(Some(value));
            }
        }

        // Not found anywhere
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
        let mut results = Vec::new();
        let mut seen_keys = std::collections::HashSet::new();

        // First scan memtable (most recent data)
        let memtable = self.memtable.read().await;
        for (key, value) in memtable.range(prefix.to_vec()..) {
            if !key.starts_with(prefix) {
                break;
            }
            if !value.is_empty() && seen_keys.insert(key.clone()) { // Skip tombstones and duplicates
                results.push((key.clone(), value.clone()));
            }
        }
        drop(memtable);

        // Then scan SSTable files (newest to oldest)
        let sstables = self.sstables.read().await;
        for sstable in &*sstables {
            // Only scan SSTable if its key range overlaps with our prefix
            if prefix >= sstable.min_key.as_slice() && prefix <= sstable.max_key.as_slice() {
                let sstable_data = tokio::fs::read(&sstable.path).await?;
                let mut pos = 0;

                while pos < sstable_data.len() {
                    // Read key_len (4 bytes, little endian)
                    let key_len = u32::from_le_bytes(sstable_data[pos..pos+4].try_into()?);
                    pos += 4;

                    // Read key
                    let key = sstable_data[pos..pos + key_len as usize].to_vec();
                    pos += key_len as usize;

                    // Read value_len (4 bytes, little endian)
                    let value_len = u32::from_le_bytes(sstable_data[pos..pos+4].try_into()?);
                    pos += 4;

                    // Check if key matches prefix and hasn't been seen in newer data
                    if key.starts_with(prefix) && seen_keys.insert(key.clone()) {
                        let value = sstable_data[pos..pos + value_len as usize].to_vec();
                        results.push((key, value));
                    } else {
                        pos += value_len as usize;
                    }
                }
            }
        }

        // Sort results by key for consistent ordering
        results.sort_by(|a, b| a.0.cmp(&b.0));

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
        file.flush().await?;

        // Create SSTable handle and add to the list (newest first)
        let sstable_handle = SSTableHandle::load(&sstable_path).await?;
        let mut sstables = self.sstables.write().await;
        sstables.insert(0, sstable_handle); // Insert at beginning (newest)

        // Check if compaction is needed
        let needs_compaction = sstables.len() >= self.compaction_config.max_sstables;

        // Clear memtable and reset WAL
        memtable.clear();
        let mut wal = self.wal.write().await;
        wal.reset().await?;
        drop(sstables); // Release lock before compaction

        // Trigger compaction if needed
        if needs_compaction {
            self.compact().await?;
        }

        Ok(())
    }

    /// Compact SSTable files to improve read performance and reclaim space
    async fn compact(&mut self) -> Result<()> {
        let mut sstables = self.sstables.write().await;

        // Need at least min_compaction_files SSTables to compact
        if sstables.len() < self.compaction_config.min_compaction_files {
            return Ok(());
        }

        // Select SSTables to compact (oldest ones)
        let num_to_compact = std::cmp::min(self.compaction_config.min_compaction_files, sstables.len());
        let sstables_to_compact: Vec<_> = sstables.drain(sstables.len() - num_to_compact..).collect();

        // Release the lock temporarily
        drop(sstables);

        // Perform the compaction
        self.perform_compaction(sstables_to_compact).await
    }

    /// Perform the actual compaction by merging SSTables
    async fn perform_compaction(&mut self, old_sstables: Vec<SSTableHandle>) -> Result<()> {
        // Collect all key-value pairs from SSTables to compact
        let mut merged_data = BTreeMap::new();

        for sstable in &old_sstables {
            let data = tokio::fs::read(&sstable.path).await?;
            let mut pos = 0;

            while pos < data.len() {
                // Read key_len (4 bytes, little endian)
                let key_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
                pos += 4;

                // Read key
                let key = data[pos..pos + key_len as usize].to_vec();
                pos += key_len as usize;

                // Read value_len (4 bytes, little endian)
                let value_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
                pos += 4;

                // Read value
                let value = data[pos..pos + value_len as usize].to_vec();
                pos += value_len as usize;

                // Only keep non-tombstone entries and overwrite older values
                if !value.is_empty() {
                    merged_data.insert(key, value);
                } else {
                    merged_data.remove(&key);
                }
            }
        }

        // Create new compacted SSTable
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis();
        let compacted_path = self.db_path.join(format!("sstable_compacted_{}.dat", timestamp));

        let mut file = tokio::fs::File::create(&compacted_path).await?;
        for (key, value) in &merged_data {
            // Simple binary format: [key_len(4bytes)][key][value_len(4bytes)][value]
            let key_len = (key.len() as u32).to_le_bytes();
            let value_len = (value.len() as u32).to_le_bytes();

            tokio::io::AsyncWriteExt::write_all(&mut file, &key_len).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, key).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, &value_len).await?;
            tokio::io::AsyncWriteExt::write_all(&mut file, value).await?;
        }
        file.flush().await?;

        // Load the new compacted SSTable
        let compacted_handle = SSTableHandle::load(&compacted_path).await?;

        // Update SSTable list: remove old ones and add new compacted one
        let mut sstables = self.sstables.write().await;
        // Remove the old SSTables from the list (they should already be removed)
        sstables.retain(|sstable| {
            !old_sstables.iter().any(|old| old.path == sstable.path)
        });
        // Add the new compacted SSTable
        sstables.push(compacted_handle);

        // Delete old SSTable files
        for old_sstable in old_sstables {
            if let Err(e) = tokio::fs::remove_file(&old_sstable.path).await {
                eprintln!("Warning: Failed to remove old SSTable {:?}: {}", old_sstable.path, e);
            }
        }

        Ok(())
    }
}

/// Handle for an SSTable file
struct SSTableHandle {
    path: PathBuf,
    min_key: Vec<u8>,
    max_key: Vec<u8>,
}

impl SSTableHandle {
    async fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let data = tokio::fs::read(&path).await?;

        let mut min_key = Vec::new();
        let mut max_key = Vec::new();
        let mut pos = 0;

        while pos < data.len() {
            // Read key_len (4 bytes, little endian)
            let key_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
            pos += 4;

            // Read key
            let key = data[pos..pos + key_len as usize].to_vec();
            pos += key_len as usize;

            // Read value_len (4 bytes, little endian)
            let value_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
            pos += 4;

            // Skip value
            pos += value_len as usize;

            // Update min/max keys
            if min_key.is_empty() || key < min_key {
                min_key = key.clone();
            }
            if max_key.is_empty() || key > max_key {
                max_key = key.clone();
            }
        }

        Ok(SSTableHandle { path, min_key, max_key })
    }

    async fn search(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // If key is outside this SSTable's range, return None immediately
        if key < self.min_key.as_slice() || key > self.max_key.as_slice() {
            return Ok(None);
        }

        let data = tokio::fs::read(&self.path).await?;
        let mut pos = 0;

        while pos < data.len() {
            // Read key_len (4 bytes, little endian)
            let key_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
            pos += 4;

            // Read key
            let current_key = &data[pos..pos + key_len as usize];
            pos += key_len as usize;

            // Read value_len (4 bytes, little endian)
            let value_len = u32::from_le_bytes(data[pos..pos+4].try_into()?);
            pos += 4;

            // Compare keys
            match current_key.cmp(key) {
                std::cmp::Ordering::Equal => {
                    // Found the key, return the value
                    let value = data[pos..pos + value_len as usize].to_vec();
                    return Ok(Some(value));
                }
                std::cmp::Ordering::Greater => {
                    // Key not found (passed the insertion point)
                    return Ok(None);
                }
                std::cmp::Ordering::Less => {
                    // Continue searching
                    pos += value_len as usize;
                }
            }
        }

        Ok(None)
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

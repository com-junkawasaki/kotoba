//! MVCC+Merkle永続ストレージ

pub mod mvcc;
pub mod merkle;
pub mod lsm;
pub mod redis;
pub mod backend;

/// Storage backend types
#[derive(Debug, Clone, PartialEq)]
pub enum BackendType {
    RocksDB,
    Redis,
}

/// Configuration for storage backends
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub backend_type: BackendType,
    pub rocksdb_path: Option<std::path::PathBuf>,
    pub rocksdb_memtable_size: Option<usize>,
    pub rocksdb_sstable_max_size: Option<usize>,
    pub redis_url: Option<String>,
    pub redis_pool_size: Option<usize>,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend_type: BackendType::RocksDB,
            rocksdb_path: Some(std::path::PathBuf::from("./data")),
            rocksdb_memtable_size: Some(64),
            rocksdb_sstable_max_size: Some(128),
            redis_url: Some("redis://localhost:6379".to_string()),
            redis_pool_size: Some(10),
        }
    }
}

pub use mvcc::*;
pub use merkle::*;
pub use lsm::*;
pub use redis::*;
pub use backend::{StorageBackend, StorageBackendFactory, StorageManager, BackendStats};

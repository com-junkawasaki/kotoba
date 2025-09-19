//! `kotoba-storage`
//!
//! This crate defines the core traits (ports) for storage operations
//! in the Kotoba ecosystem. It provides abstractions for various storage
//! backends like Key-Value stores, Event stores, and Graph stores.

use anyhow::Result;
use async_trait::async_trait;
use std::fmt;

/// Storage backend types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    Memory,
    RocksDB,
    Redis,
    Custom(&'static str),
}

impl fmt::Display for StorageBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageBackend::Memory => write!(f, "Memory"),
            StorageBackend::RocksDB => write!(f, "RocksDB"),
            StorageBackend::Redis => write!(f, "Redis"),
            StorageBackend::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// A generic key-value store trait.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Puts a key-value pair into the store.
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Gets a value for a given key.
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair from the store.
    async fn delete(&self, key: &[u8]) -> Result<()>;

    /// Scans for key-value pairs with a given prefix.
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;

    /// Returns the storage backend type
    fn backend_type(&self) -> StorageBackend {
        StorageBackend::Custom("Unknown")
    }

    /// Returns storage statistics
    async fn stats(&self) -> Result<StorageStats> {
        Ok(StorageStats::default())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    pub total_keys: u64,
    pub total_size_bytes: u64,
    pub operations_count: u64,
    pub hit_ratio: f64,
    pub backend_info: String,
}

// TODO: Define EventStore and GraphStore traits later

/// Re-export common storage implementations
#[cfg(feature = "memory")]
pub use kotoba_memory::MemoryKeyValueStore;

#[cfg(feature = "rocksdb")]
pub use kotoba_storage_rocksdb::RocksDbStore;

#[cfg(feature = "redis")]
pub use kotoba_storage_redis::RedisStore;
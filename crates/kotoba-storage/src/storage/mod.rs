//! The storage module provides a unified interface over various storage backends.

pub mod backend;
pub mod lsm;
pub mod memory;
pub mod merkle;
pub mod mvcc;
pub mod redis;

pub use backend::{StorageBackend, BackendStats, StorageManager, StorageConfig, BackendType};
pub use lsm::LsmStorage;
pub use memory::MemoryStorage;
pub use merkle::MerkleDAG;
pub use mvcc::MVCCManager;
pub use redis::RedisStorage;

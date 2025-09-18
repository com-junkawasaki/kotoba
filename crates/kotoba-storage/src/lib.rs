//! `kotoba-storage`
//!
//! This crate provides the storage layer for Kotoba, offering a unified
//! interface over various database backends like RocksDB (LSM Tree) and Redis.
//! It includes support for MVCC and Merkle DAG persistence.

pub mod adapters;
pub mod domain;
pub mod port;

pub mod prelude {
    pub use crate::port::StoragePort;
    pub use crate::adapters::lsm::LSMTree;
    pub use crate::adapters::memory::MemoryStorage;
    pub use crate::adapters::persistent::{PersistentStorage, PersistentStorageConfig};
    pub use crate::domain::models::{StorageConfig, CidRange};
}
//! `kotoba-storage`
//!
//! This crate provides the storage layer for Kotoba, offering a unified
//! interface over various database backends like RocksDB (LSM Tree) and Redis.
//! It includes support for MVCC and Merkle DAG persistence.

pub mod storage;

pub mod prelude {
    pub use crate::storage::{
        StorageBackend,
        StorageManager,
        MerkleDAG,
        MVCCManager,
        StorageConfig,
        BackendType,
    };
}
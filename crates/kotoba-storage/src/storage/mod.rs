//! The storage module provides a unified interface over various storage backends.

pub mod backend;
pub mod lsm;
pub mod memory;
pub mod merkle;
pub mod mvcc;
pub mod redis;

// Re-export everything from the backend, which is the main public interface.
pub use backend::*;

// Re-export specific types from submodules
pub use merkle::MerkleDAG;
pub use mvcc::MVCCManager;

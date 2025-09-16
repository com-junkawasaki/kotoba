//! kotoba-storage - Kotoba Storage Components

pub mod storage;

pub mod prelude {
    // Re-export commonly used storage items
    pub use crate::storage::mvcc::*;
    pub use crate::storage::merkle::*;
    pub use crate::storage::backend::{StorageBackend, StorageBackendFactory, StorageManager, BackendStats};
}
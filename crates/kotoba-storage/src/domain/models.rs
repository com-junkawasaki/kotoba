//! Shared configuration and model structs for the storage crate.

use serde::{Deserialize, Serialize};

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageStats {
    /// Number of entries in the LSM tree
    pub lsm_entries: usize,
    /// Number of Merkle nodes
    pub merkle_nodes: usize,
    /// Number of active transactions
    pub active_transactions: usize,
    /// Total data size in bytes
    pub data_size: u64,
}

/// A range of CIDs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CidRange {
    pub start: String,
    pub end: String,
}

/// General storage configuration for various backends.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageConfig {
    pub backend_type: BackendType,
    pub path: Option<String>,
    pub redis_url: Option<String>,
    pub s3_bucket: Option<String>,
    pub s3_region: Option<String>,
}

/// Statistics for a specific storage backend.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BackendStats {
    pub backend_type: String,
    pub total_keys: Option<u64>,
    pub memory_usage: Option<u64>,
    pub disk_usage: Option<u64>,
    pub connection_count: Option<u64>,
}

/// Enum for different storage backend types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackendType {
    Memory,
    Lsm,
    Redis,
    S3,
}

impl Default for BackendType {
    fn default() -> Self {
        BackendType::Memory
    }
}

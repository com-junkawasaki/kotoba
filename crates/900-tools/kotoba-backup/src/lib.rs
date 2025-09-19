//! # KotobaDB Backup & Restore
//!
//! Automated backup and restore system for KotobaDB with support for:
//! - Full and incremental backups
//! - Point-in-time recovery
//! - Cloud storage integration
//! - Compression and encryption
//!
//! ## Features
//!
//! - **Full Backups**: Complete database snapshots
//! - **Incremental Backups**: Efficient delta backups
//! - **Point-in-Time Recovery**: Restore to specific timestamps
//! - **Cloud Storage**: AWS S3, Google Cloud Storage support
//! - **Compression**: Built-in compression for storage efficiency
//! - **Encryption**: Optional encryption for secure backups
//! - **Automated Scheduling**: Cron-like backup scheduling

pub mod backup_manager;
pub mod restore_manager;
pub mod point_in_time_recovery;
pub mod storage;
pub mod compression;
pub mod scheduler;

#[cfg(feature = "cloud")]
pub mod cloud;

pub use backup_manager::*;
pub use restore_manager::*;
pub use point_in_time_recovery::*;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tokio::sync::RwLock;

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Backup storage path
    pub backup_path: PathBuf,
    /// Backup retention policy
    pub retention_policy: RetentionPolicy,
    /// Compression enabled
    pub compression_enabled: bool,
    /// Encryption key (optional)
    pub encryption_key: Option<String>,
    /// Cloud storage configuration
    #[cfg(feature = "cloud")]
    pub cloud_config: Option<CloudConfig>,
    /// Maximum concurrent backups
    pub max_concurrent_backups: usize,
}

/// Retention policy for backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetentionPolicy {
    /// Keep daily backups for this many days
    pub daily_retention_days: u32,
    /// Keep weekly backups for this many weeks
    pub weekly_retention_weeks: u32,
    /// Keep monthly backups for this many months
    pub monthly_retention_months: u32,
    /// Maximum number of backups to keep
    pub max_backups: Option<usize>,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            daily_retention_days: 7,
            weekly_retention_weeks: 4,
            monthly_retention_months: 12,
            max_backups: Some(100),
        }
    }
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Unique backup ID
    pub id: String,
    /// Backup type
    pub backup_type: BackupType,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Database version
    pub db_version: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Compression ratio (if compressed)
    pub compression_ratio: Option<f64>,
    /// Checksum for integrity
    pub checksum: String,
    /// Parent backup ID (for incremental backups)
    pub parent_backup_id: Option<String>,
    /// Database state at backup time
    pub db_state: DatabaseState,
}

/// Backup types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackupType {
    /// Full backup (complete database)
    Full,
    /// Incremental backup (changes since last backup)
    Incremental,
    /// Snapshot backup (point-in-time snapshot)
    Snapshot,
}

/// Database state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseState {
    /// Last transaction ID
    pub last_transaction_id: u64,
    /// Last log sequence number
    pub last_lsn: u64,
    /// Active snapshots
    pub snapshots: Vec<String>,
    /// Node information (for distributed backups)
    pub nodes: HashMap<String, NodeInfo>,
}

/// Node information for distributed backups
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub partitions: Vec<String>,
}

/// Cloud storage configuration
#[cfg(feature = "cloud")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudConfig {
    pub provider: CloudProvider,
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
    pub prefix: Option<String>,
}

#[cfg(feature = "cloud")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
}

/// Backup statistics
#[derive(Debug, Clone)]
pub struct BackupStats {
    pub total_backups: usize,
    pub total_size_bytes: u64,
    pub last_backup: Option<DateTime<Utc>>,
    pub oldest_backup: Option<DateTime<Utc>>,
    pub compression_savings: Option<f64>,
}

/// Restore options
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    pub target_path: PathBuf,
    pub point_in_time: Option<DateTime<Utc>>,
    pub verify_integrity: bool,
    pub skip_corrupted: bool,
    pub max_parallelism: usize,
}

/// Backup operation result
#[derive(Debug)]
pub enum BackupResult {
    Success { metadata: BackupMetadata, duration: std::time::Duration },
    Failed { error: BackupError, partial_backup: Option<BackupMetadata> },
}

/// Restore operation result
#[derive(Debug)]
pub enum RestoreResult {
    Success { restored_bytes: u64, duration: std::time::Duration },
    Failed { error: RestoreError, partial_restore: bool },
}

/// Backup-related errors
#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error("Database connection error: {0}")]
    DatabaseConnection(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Cloud storage error: {0}")]
    CloudStorage(String),

    #[error("Integrity check failed: {0}")]
    IntegrityCheck(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Concurrent backup limit exceeded")]
    ConcurrencyLimit,

    #[error("Backup cancelled")]
    Cancelled,
}

/// Restore-related errors
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    #[error("Backup not found: {0}")]
    BackupNotFound(String),

    #[error("Corrupted backup: {0}")]
    CorruptedBackup(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Database restore error: {0}")]
    DatabaseRestore(String),

    #[error("Point-in-time recovery not supported")]
    PitNotSupported,

    #[error("Configuration error: {0}")]
    Config(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retention_policy_default() {
        let policy = RetentionPolicy::default();
        assert_eq!(policy.daily_retention_days, 7);
        assert_eq!(policy.weekly_retention_weeks, 4);
        assert_eq!(policy.monthly_retention_months, 12);
    }

    #[test]
    fn test_backup_metadata_creation() {
        let metadata = BackupMetadata {
            id: "test-backup".to_string(),
            backup_type: BackupType::Full,
            created_at: Utc::now(),
            db_version: "1.0.0".to_string(),
            size_bytes: 1024,
            compression_ratio: Some(0.8),
            checksum: "abc123".to_string(),
            parent_backup_id: None,
            db_state: DatabaseState {
                last_transaction_id: 100,
                last_lsn: 200,
                snapshots: vec![],
                nodes: HashMap::new(),
            },
        };

        assert_eq!(metadata.backup_type, BackupType::Full);
        assert_eq!(metadata.size_bytes, 1024);
    }
}

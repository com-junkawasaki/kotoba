//! # Backup Manager
//!
//! Manages backup creation, scheduling, and maintenance operations.

use crate::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio::time::{self, Duration, Instant};
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Backup manager for creating and managing database backups
pub struct BackupManager {
    /// Database instance to backup
    db: Arc<dyn BackupSource>,
    /// Backup configuration
    config: BackupConfig,
    /// Active backups semaphore
    concurrency_limit: Arc<Semaphore>,
    /// Backup metadata storage
    metadata_store: Arc<RwLock<BackupMetadataStore>>,
    /// Running backup tasks
    running_backups: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(db: Arc<dyn BackupSource>, config: BackupConfig) -> Self {
        let concurrency_limit = Arc::new(Semaphore::new(config.max_concurrent_backups));
        let metadata_store = Arc::new(RwLock::new(BackupMetadataStore::new(&config.backup_path)));

        Self {
            db,
            config,
            concurrency_limit,
            metadata_store,
            running_backups: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a full backup
    pub async fn create_full_backup(&self) -> Result<BackupResult, BackupError> {
        self.create_backup_internal(BackupType::Full, None).await
    }

    /// Create an incremental backup
    pub async fn create_incremental_backup(&self, parent_backup_id: Option<String>) -> Result<BackupResult, BackupError> {
        self.create_backup_internal(BackupType::Incremental, parent_backup_id).await
    }

    /// Create a snapshot backup
    pub async fn create_snapshot_backup(&self) -> Result<BackupResult, BackupError> {
        self.create_backup_internal(BackupType::Snapshot, None).await
    }

    /// Get backup metadata
    pub async fn get_backup_metadata(&self, backup_id: &str) -> Result<Option<BackupMetadata>, BackupError> {
        let store = self.metadata_store.read().await;
        Ok(store.get_backup(backup_id).cloned())
    }

    /// List all backups
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>, BackupError> {
        let store = self.metadata_store.read().await;
        Ok(store.list_backups())
    }

    /// Delete a backup
    pub async fn delete_backup(&self, backup_id: &str) -> Result<(), BackupError> {
        let mut store = self.metadata_store.write().await;
        store.delete_backup(backup_id)?;

        // Remove backup files
        let backup_path = self.config.backup_path.join(backup_id);
        if backup_path.exists() {
            tokio::fs::remove_dir_all(&backup_path).await
                .map_err(|e| BackupError::Storage(format!("Failed to remove backup directory: {}", e)))?;
        }

        Ok(())
    }

    /// Clean up old backups according to retention policy
    pub async fn cleanup_old_backups(&self) -> Result<Vec<String>, BackupError> {
        let mut store = self.metadata_store.write().await;
        let backups_to_delete = store.get_backups_to_cleanup(&self.config.retention_policy);

        let mut deleted_backups = Vec::new();
        for backup_id in &backups_to_delete {
            if let Err(e) = self.delete_backup(backup_id).await {
                eprintln!("Failed to delete backup {}: {:?}", backup_id, e);
            } else {
                deleted_backups.push(backup_id.clone());
            }
        }

        Ok(deleted_backups)
    }

    /// Get backup statistics
    pub async fn get_backup_stats(&self) -> Result<BackupStats, BackupError> {
        let store = self.metadata_store.read().await;
        Ok(store.get_stats())
    }

    /// Schedule automatic backups
    pub async fn schedule_backups(&self, schedule: BackupSchedule) -> Result<String, BackupError> {
        let backup_manager = Arc::new(self.clone());
        let schedule_id = Uuid::new_v4().to_string();

        let handle = tokio::spawn(async move {
            let mut interval = match schedule {
                BackupSchedule::Hourly => time::interval(Duration::from_secs(3600)),
                BackupSchedule::Daily => time::interval(Duration::from_secs(86400)),
                BackupSchedule::Weekly => time::interval(Duration::from_secs(604800)),
            };

            loop {
                interval.tick().await;

                let result = match schedule {
                    BackupSchedule::Hourly => {
                        if Utc::now().hour() == 0 { // Midnight
                            backup_manager.create_incremental_backup(None).await
                        } else {
                            continue;
                        }
                    }
                    BackupSchedule::Daily => {
                        backup_manager.create_full_backup().await
                    }
                    BackupSchedule::Weekly => {
                        if Utc::now().weekday() == chrono::Weekday::Mon { // Monday
                            backup_manager.create_full_backup().await
                        } else {
                            continue;
                        }
                    }
                };

                match result {
                    Ok(BackupResult::Success { .. }) => {
                        println!("Scheduled backup completed successfully");
                        // Clean up old backups
                        if let Err(e) = backup_manager.cleanup_old_backups().await {
                            eprintln!("Failed to cleanup old backups: {:?}", e);
                        }
                    }
                    Ok(BackupResult::Failed { error, .. }) => {
                        eprintln!("Scheduled backup failed: {:?}", error);
                    }
                    Err(e) => {
                        eprintln!("Scheduled backup error: {:?}", e);
                    }
                }
            }
        });

        self.running_backups.write().await.insert(schedule_id.clone(), handle);
        Ok(schedule_id)
    }

    /// Stop scheduled backups
    pub async fn stop_scheduled_backups(&self, schedule_id: &str) -> Result<(), BackupError> {
        let mut running = self.running_backups.write().await;
        if let Some(handle) = running.remove(schedule_id) {
            handle.abort();
        }
        Ok(())
    }

    /// Internal backup creation logic
    async fn create_backup_internal(&self, backup_type: BackupType, parent_backup_id: Option<String>) -> Result<BackupResult, BackupError> {
        // Acquire concurrency permit
        let _permit = self.concurrency_limit.acquire().await
            .map_err(|_| BackupError::ConcurrencyLimit)?;

        let start_time = Instant::now();
        let backup_id = Uuid::new_v4().to_string();

        // Prepare backup metadata
        let db_state = self.db.get_database_state().await
            .map_err(|e| BackupError::DatabaseConnection(format!("Failed to get database state: {}", e)))?;

        let mut metadata = BackupMetadata {
            id: backup_id.clone(),
            backup_type,
            created_at: Utc::now(),
            db_version: self.db.get_version().await
                .unwrap_or_else(|_| "unknown".to_string()),
            size_bytes: 0,
            compression_ratio: None,
            checksum: String::new(),
            parent_backup_id,
            db_state,
        };

        // Create backup directory
        let backup_dir = self.config.backup_path.join(&backup_id);
        tokio::fs::create_dir_all(&backup_dir).await
            .map_err(|e| BackupError::Storage(format!("Failed to create backup directory: {}", e)))?;

        // Perform the backup
        match self.perform_backup(&backup_dir, &mut metadata).await {
            Ok(_) => {
                let duration = start_time.elapsed();

                // Store metadata
                let mut store = self.metadata_store.write().await;
                store.add_backup(metadata.clone())?;

                Ok(BackupResult::Success { metadata, duration })
            }
            Err(e) => {
                // Clean up failed backup
                let _ = tokio::fs::remove_dir_all(&backup_dir).await;
                Ok(BackupResult::Failed { error: e, partial_backup: None })
            }
        }
    }

    /// Perform the actual backup operation
    async fn perform_backup(&self, backup_dir: &PathBuf, metadata: &mut BackupMetadata) -> Result<(), BackupError> {
        match metadata.backup_type {
            BackupType::Full => {
                self.perform_full_backup(backup_dir, metadata).await
            }
            BackupType::Incremental => {
                self.perform_incremental_backup(backup_dir, metadata).await
            }
            BackupType::Snapshot => {
                self.perform_snapshot_backup(backup_dir, metadata).await
            }
        }
    }

    /// Perform full backup
    async fn perform_full_backup(&self, backup_dir: &PathBuf, metadata: &mut BackupMetadata) -> Result<(), BackupError> {
        // Get all database data
        let data_stream = self.db.export_all_data().await
            .map_err(|e| BackupError::DatabaseConnection(format!("Failed to export data: {}", e)))?;

        // Write to backup files
        let backup_file = backup_dir.join("data.full");
        let mut writer = BackupWriter::new(&backup_file, &self.config).await?;

        let mut hasher = blake3::Hasher::new();
        let mut total_bytes = 0u64;

        // Process data stream
        use futures::StreamExt;
        let mut stream = data_stream;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| BackupError::DatabaseConnection(format!("Data stream error: {}", e)))?;
            writer.write_chunk(&chunk).await?;
            hasher.update(&chunk);
            total_bytes += chunk.len() as u64;
        }

        writer.finish().await?;
        metadata.checksum = hasher.finalize().to_hex().to_string();
        metadata.size_bytes = total_bytes;

        Ok(())
    }

    /// Perform incremental backup
    async fn perform_incremental_backup(&self, backup_dir: &PathBuf, metadata: &mut BackupMetadata) -> Result<(), BackupError> {
        // For incremental backups, we need the parent backup to determine changes
        let parent_backup_id = metadata.parent_backup_id.as_ref()
            .ok_or_else(|| BackupError::Config("Parent backup ID required for incremental backup".to_string()))?;

        let parent_metadata = self.get_backup_metadata(parent_backup_id).await?
            .ok_or_else(|| BackupError::Config(format!("Parent backup {} not found", parent_backup_id)))?;

        // Get changes since parent backup
        let changes_stream = self.db.export_changes_since(&parent_metadata.db_state.last_transaction_id).await
            .map_err(|e| BackupError::DatabaseConnection(format!("Failed to export changes: {}", e)))?;

        // Write incremental data
        let backup_file = backup_dir.join("data.incremental");
        let mut writer = BackupWriter::new(&backup_file, &self.config).await?;

        let mut hasher = blake3::Hasher::new();
        let mut total_bytes = 0u64;

        use futures::StreamExt;
        let mut stream = changes_stream;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| BackupError::DatabaseConnection(format!("Change stream error: {}", e)))?;
            writer.write_chunk(&chunk).await?;
            hasher.update(&chunk);
            total_bytes += chunk.len() as u64;
        }

        writer.finish().await?;
        metadata.checksum = hasher.finalize().to_hex().to_string();
        metadata.size_bytes = total_bytes;

        Ok(())
    }

    /// Perform snapshot backup
    async fn perform_snapshot_backup(&self, backup_dir: &PathBuf, metadata: &mut BackupMetadata) -> Result<(), BackupError> {
        // Create a snapshot and export it
        let snapshot_id = self.db.create_snapshot().await
            .map_err(|e| BackupError::DatabaseConnection(format!("Failed to create snapshot: {}", e)))?;

        let snapshot_data = self.db.export_snapshot(&snapshot_id).await
            .map_err(|e| BackupError::DatabaseConnection(format!("Failed to export snapshot: {}", e)))?;

        // Write snapshot data
        let backup_file = backup_dir.join("data.snapshot");
        let mut writer = BackupWriter::new(&backup_file, &self.config).await?;

        let mut hasher = blake3::Hasher::new();
        hasher.update(&snapshot_data);

        writer.write_chunk(&snapshot_data).await?;
        writer.finish().await?;

        metadata.checksum = hasher.finalize().to_hex().to_string();
        metadata.size_bytes = snapshot_data.len() as u64;

        // Clean up snapshot
        let _ = self.db.delete_snapshot(&snapshot_id).await;

        Ok(())
    }
}

impl Clone for BackupManager {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
            config: self.config.clone(),
            concurrency_limit: Arc::clone(&self.concurrency_limit),
            metadata_store: Arc::clone(&self.metadata_store),
            running_backups: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

/// Backup schedule types
#[derive(Debug, Clone)]
pub enum BackupSchedule {
    Hourly,
    Daily,
    Weekly,
}

/// Trait for backup source (database)
#[async_trait::async_trait]
pub trait BackupSource: Send + Sync {
    /// Get database version
    async fn get_version(&self) -> Result<String, BackupError>;

    /// Get current database state
    async fn get_database_state(&self) -> Result<DatabaseState, BackupError>;

    /// Export all database data
    async fn export_all_data(&self) -> Result<Box<dyn futures::Stream<Item = Result<Vec<u8>, BackupError>> + Send + Unpin>, BackupError>;

    /// Export changes since transaction ID
    async fn export_changes_since(&self, transaction_id: u64) -> Result<Box<dyn futures::Stream<Item = Result<Vec<u8>, BackupError>> + Send + Unpin>, BackupError>;

    /// Create a snapshot
    async fn create_snapshot(&self) -> Result<String, BackupError>;

    /// Export snapshot data
    async fn export_snapshot(&self, snapshot_id: &str) -> Result<Vec<u8>, BackupError>;

    /// Delete a snapshot
    async fn delete_snapshot(&self, snapshot_id: &str) -> Result<(), BackupError>;
}

/// Backup writer for handling compression and encryption
struct BackupWriter {
    writer: tokio::fs::File,
    #[cfg(feature = "compression")]
    compressor: Option<flate2::write::GzEncoder<Vec<u8>>>,
}

impl BackupWriter {
    async fn new(path: &PathBuf, config: &BackupConfig) -> Result<Self, BackupError> {
        let file = tokio::fs::File::create(path).await
            .map_err(|e| BackupError::Storage(format!("Failed to create backup file: {}", e)))?;

        #[cfg(feature = "compression")]
        let compressor = if config.compression_enabled {
            Some(flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default()))
        } else {
            None
        };

        #[cfg(not(feature = "compression"))]
        let compressor = None;

        Ok(Self {
            writer: file,
            #[cfg(feature = "compression")]
            compressor,
        })
    }

    async fn write_chunk(&mut self, data: &[u8]) -> Result<(), BackupError> {
        #[cfg(feature = "compression")]
        if let Some(ref mut compressor) = self.compressor {
            use std::io::Write;
            compressor.write_all(data)
                .map_err(|e| BackupError::Compression(format!("Compression failed: {}", e)))?;
            return Ok(());
        }

        self.writer.write_all(data).await
            .map_err(|e| BackupError::Storage(format!("Write failed: {}", e)))?;

        Ok(())
    }

    async fn finish(&mut self) -> Result<(), BackupError> {
        #[cfg(feature = "compression")]
        if let Some(compressor) = self.compressor.take() {
            let compressed_data = compressor.finish()
                .map_err(|e| BackupError::Compression(format!("Compression finish failed: {}", e)))?;
            self.writer.write_all(&compressed_data).await
                .map_err(|e| BackupError::Storage(format!("Write compressed data failed: {}", e)))?;
        }

        self.writer.flush().await
            .map_err(|e| BackupError::Storage(format!("Flush failed: {}", e)))?;

        Ok(())
    }
}

/// Backup metadata storage
pub struct BackupMetadataStore {
    metadata_path: PathBuf,
    backups: HashMap<String, BackupMetadata>,
}

impl BackupMetadataStore {
    fn new(backup_path: &PathBuf) -> Self {
        let metadata_path = backup_path.join("metadata.json");
        let backups = Self::load_metadata(&metadata_path);

        Self {
            metadata_path,
            backups,
        }
    }

    fn load_metadata(path: &PathBuf) -> HashMap<String, BackupMetadata> {
        if !path.exists() {
            return HashMap::new();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => {
                match serde_json::from_str::<Vec<BackupMetadata>>(&content) {
                    Ok(backups) => backups.into_iter().map(|b| (b.id.clone(), b)).collect(),
                    Err(_) => HashMap::new(),
                }
            }
            Err(_) => HashMap::new(),
        }
    }

    fn save_metadata(&self) -> Result<(), BackupError> {
        let backups: Vec<_> = self.backups.values().cloned().collect();
        let content = serde_json::to_string_pretty(&backups)
            .map_err(|e| BackupError::Storage(format!("Serialize metadata failed: {}", e)))?;

        std::fs::write(&self.metadata_path, content)
            .map_err(|e| BackupError::Storage(format!("Write metadata failed: {}", e)))?;

        Ok(())
    }

    fn add_backup(&mut self, metadata: BackupMetadata) -> Result<(), BackupError> {
        self.backups.insert(metadata.id.clone(), metadata);
        self.save_metadata()?;
        Ok(())
    }

    fn get_backup(&self, id: &str) -> Option<&BackupMetadata> {
        self.backups.get(id)
    }

    fn list_backups(&self) -> Vec<BackupMetadata> {
        self.backups.values().cloned().collect()
    }

    fn delete_backup(&mut self, id: &str) -> Result<(), BackupError> {
        self.backups.remove(id);
        self.save_metadata()?;
        Ok(())
    }

    fn get_backups_to_cleanup(&self, policy: &RetentionPolicy) -> Vec<String> {
        let mut backups_by_type: HashMap<BackupType, Vec<&BackupMetadata>> = HashMap::new();

        for backup in self.backups.values() {
            backups_by_type.entry(backup.backup_type).or_default().push(backup);
        }

        let mut to_delete = Vec::new();

        for (backup_type, backups) in backups_by_type {
            let mut sorted_backups: Vec<_> = backups.iter().collect();
            sorted_backups.sort_by(|a, b| b.created_at.cmp(&a.created_at)); // Newest first

            // Apply retention policy
            let keep_count = match backup_type {
                BackupType::Full => policy.monthly_retention_months as usize,
                BackupType::Incremental => policy.daily_retention_days as usize,
                BackupType::Snapshot => policy.weekly_retention_weeks as usize,
            };

            if sorted_backups.len() > keep_count {
                for backup in &sorted_backups[keep_count..] {
                    to_delete.push(backup.id.clone());
                }
            }
        }

        // Apply maximum backups limit
        if let Some(max_backups) = policy.max_backups {
            if self.backups.len() > max_backups {
                let mut all_backups: Vec<_> = self.backups.values().collect();
                all_backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

                for backup in &all_backups[max_backups..] {
                    if !to_delete.contains(&backup.id) {
                        to_delete.push(backup.id.clone());
                    }
                }
            }
        }

        to_delete
    }

    fn get_stats(&self) -> BackupStats {
        let total_backups = self.backups.len();
        let total_size_bytes = self.backups.values().map(|b| b.size_bytes).sum();

        let last_backup = self.backups.values()
            .max_by_key(|b| b.created_at)
            .map(|b| b.created_at);

        let oldest_backup = self.backups.values()
            .min_by_key(|b| b.created_at)
            .map(|b| b.created_at);

        let compression_savings = if self.backups.is_empty() {
            None
        } else {
            let total_original = self.backups.values()
                .filter_map(|b| b.compression_ratio)
                .map(|ratio| b.size_bytes as f64 / ratio)
                .sum::<f64>();

            if total_original > 0.0 {
                Some(1.0 - (total_size_bytes as f64 / total_original))
            } else {
                None
            }
        };

        BackupStats {
            total_backups,
            total_size_bytes,
            last_backup,
            oldest_backup,
            compression_savings,
        }
    }
}

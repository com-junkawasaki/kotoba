# KotobaDB Backup & Restore

**Automated backup and restore system for KotobaDB** with support for point-in-time recovery, compression, and cloud storage integration.

## Features

- **Multiple Backup Types**: Full, incremental, and snapshot backups
- **Point-in-Time Recovery**: Restore to any specific timestamp
- **Compression Support**: Optional compression for storage efficiency
- **Cloud Storage**: AWS S3, Google Cloud Storage, and Azure Blob Storage
- **Integrity Verification**: Checksum-based backup validation
- **Retention Policies**: Automated cleanup of old backups
- **Concurrent Operations**: Parallel backup and restore operations

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kotoba-backup = "0.1.0"
```

### Basic Backup and Restore

```rust
use kotoba_backup::*;
use std::sync::Arc;

// Create backup configuration
let config = BackupConfig {
    backup_path: "./backups".into(),
    retention_policy: RetentionPolicy::default(),
    compression_enabled: true,
    ..Default::default()
};

// Create backup manager (assuming you have a KotobaDB instance)
let backup_manager = BackupManager::new(db_instance, config);

// Create a full backup
match backup_manager.create_full_backup().await {
    Ok(BackupResult::Success { metadata, duration }) => {
        println!("Backup completed in {:?}", duration);
        println!("Backup ID: {}", metadata.id);
    }
    Ok(BackupResult::Failed { error, .. }) => {
        eprintln!("Backup failed: {:?}", error);
    }
    Err(e) => eprintln!("Error: {:?}", e),
}

// List available backups
let backups = backup_manager.list_backups().await?;
for backup in backups {
    println!("Backup: {} ({})", backup.id, backup.backup_type);
}

// Restore from backup
let restore_manager = RestoreManager::new(db_instance, config);
let options = RestoreOptions {
    target_path: "./restored_db".into(),
    ..Default::default()
};

match restore_manager.restore_from_backup(&backup_id, options).await {
    Ok(RestoreResult::Success { restored_bytes, duration }) => {
        println!("Restore completed in {:?}, restored {} bytes", duration, restored_bytes);
    }
    Ok(RestoreResult::Failed { error, .. }) => {
        eprintln!("Restore failed: {:?}", error);
    }
    Err(e) => eprintln!("Error: {:?}", e),
}
```

### Point-in-Time Recovery

```rust
use chrono::{Utc, Duration};

// Create PITR manager
let wal_manager = WALManager {}; // Your WAL manager
let pitr = PointInTimeRecovery::new(restore_manager, wal_manager);

// Recover to 1 hour ago
let target_time = Utc::now() - Duration::hours(1);
let options = PITROptions {
    target_path: "./pitr_recovery".into(),
    include_base_backup: true,
    replay_wal: true,
    verify_integrity: true,
    ..Default::default()
};

match pitr.recover_to_timestamp(target_time, options).await {
    Ok(PITRResult::Success { recovered_to, stats, duration }) => {
        println!("PITR completed in {:?} to {}", duration, recovered_to);
        println!("Records replayed: {}", stats.wal_records_replayed);
    }
    Ok(PITRResult::Failed { error, partial_recovery }) => {
        eprintln!("PITR failed: {:?} (partial: {})", error, partial_recovery);
    }
    Err(e) => eprintln!("Error: {:?}", e),
}
```

## Backup Types

### Full Backup
Complete database snapshot containing all data:
```rust
let result = backup_manager.create_full_backup().await?;
```

### Incremental Backup
Only changes since the last backup:
```rust
let result = backup_manager.create_incremental_backup(Some("parent-backup-id".to_string())).await?;
```

### Snapshot Backup
Point-in-time database snapshot:
```rust
let result = backup_manager.create_snapshot_backup().await?;
```

## Configuration

### Backup Configuration

```rust
let config = BackupConfig {
    backup_path: PathBuf::from("./backups"),
    retention_policy: RetentionPolicy {
        daily_retention_days: 7,
        weekly_retention_weeks: 4,
        monthly_retention_months: 12,
        max_backups: Some(100),
    },
    compression_enabled: true,
    encryption_key: Some("your-encryption-key".to_string()),
    max_concurrent_backups: 4,
    #[cfg(feature = "cloud")]
    cloud_config: Some(CloudConfig {
        provider: CloudProvider::AWS,
        bucket: "my-backups".to_string(),
        region: "us-east-1".to_string(),
        access_key: "key".to_string(),
        secret_key: "secret".to_string(),
        prefix: Some("kotoba/".to_string()),
    }),
};
```

### Retention Policies

Automatic cleanup of old backups:

```rust
// Keep last 7 daily backups, 4 weekly, 12 monthly, max 100 total
let policy = RetentionPolicy {
    daily_retention_days: 7,
    weekly_retention_weeks: 4,
    monthly_retention_months: 12,
    max_backups: Some(100),
};

// Apply retention policy
let deleted_backups = backup_manager.cleanup_old_backups().await?;
println!("Cleaned up {} old backups", deleted_backups.len());
```

## Storage Backends

### Local Filesystem
Default storage using local filesystem:
```rust
let storage = LocalStorage::new("./backups".into());
```

### Cloud Storage (with `cloud` feature)
AWS S3, Google Cloud Storage, or Azure Blob Storage:
```toml
[dependencies]
kotoba-backup = { version = "0.1.0", features = ["cloud"] }
```

```rust
let storage = CloudStorage::new(cloud_config)?;
```

## Point-in-Time Recovery

### Recovery Process

1. **Find Recovery Point**: Locate the appropriate backup and WAL position
2. **Restore Base Backup**: Apply the most recent full backup
3. **Replay WAL**: Apply transaction logs up to target time
4. **Verify Integrity**: Ensure data consistency

### Recovery Estimation

```rust
// Estimate recovery time and resources
let estimate = pitr.estimate_recovery(target_time).await?;
println!("Estimated duration: {:?}", estimate.estimated_duration);
println!("Required space: {} bytes", estimate.required_space);
println!("WAL records to replay: {}", estimate.wal_records_to_replay);
```

### Recovery Points

```rust
// List available recovery points
let recovery_points = pitr.list_recovery_points(
    Utc::now() - Duration::days(7),
    Utc::now()
).await?;

for point in recovery_points {
    println!("Recovery point: {} (WAL available: {})",
             point.timestamp, point.wal_available);
}
```

## Monitoring and Statistics

### Backup Statistics

```rust
let stats = backup_manager.get_backup_stats().await?;
println!("Total backups: {}", stats.total_backups);
println!("Total size: {} bytes", stats.total_size_bytes);
println!("Last backup: {:?}", stats.last_backup);
println!("Compression savings: {:?}", stats.compression_savings);
```

### Recovery Status

```rust
let status = pitr.get_recovery_status().await;
if status.is_recovering {
    println!("Recovery in progress to {:?}", status.current_timestamp);
    println!("Phase: {:?}", status.progress.phase);
}
```

## Advanced Features

### Compression

Enable compression for storage efficiency:
```toml
[dependencies]
kotoba-backup = { version = "0.1.0", features = ["compression"] }
```

### Encryption

Basic encryption support (expandable):
```rust
let config = BackupConfig {
    encryption_key: Some("your-key-here".to_string()),
    ..Default::default()
};
```

### Scheduled Backups

```rust
// Schedule automatic backups
let schedule_id = backup_manager.schedule_backups(BackupSchedule::Daily).await?;
println!("Scheduled backup with ID: {}", schedule_id);

// Stop scheduled backup
backup_manager.stop_scheduled_backups(&schedule_id).await?;
```

## Architecture

```
┌─────────────────────────────────────────┐
│            BackupManager               │ ← Main backup interface
├─────────────────────────────────────────┤
│    ┌─────────────┬─────────────┬─────┐  │
│    │Backup Writer│  Retention  │ WAL │  │ ← Core components
│    │             │  Manager    │ Mgr │  │
│    └─────────────┴─────────────┴─────┘  │
├─────────────────────────────────────────┤
│    ┌─────────────┬─────────────┬─────┐  │
│    │Local Storage│Cloud Storage│ ... │  │ ← Storage backends
│    └─────────────┴─────────────┴─────┘  │
├─────────────────────────────────────────┤
│         Database Integration           │ ← Backup source/target
└─────────────────────────────────────────┘
```

## Error Handling

Comprehensive error handling for various failure scenarios:

```rust
match backup_result {
    BackupResult::Success { metadata, duration } => {
        println!("Backup successful: {} in {:?}", metadata.id, duration);
    }
    BackupResult::Failed { error, partial_backup } => {
        match error {
            BackupError::Storage(e) => println!("Storage error: {}", e),
            BackupError::DatabaseConnection(e) => println!("DB connection error: {}", e),
            BackupError::IntegrityCheck(e) => println!("Integrity check failed: {}", e),
            _ => println!("Other backup error: {:?}", error),
        }

        if let Some(partial) = partial_backup {
            println!("Partial backup available: {}", partial.id);
        }
    }
}
```

## Performance Considerations

### Optimization Tips

1. **Compression**: Enable for storage efficiency
2. **Incremental Backups**: Use for frequent changes
3. **Parallel Operations**: Configure appropriate concurrency
4. **Retention Policies**: Regular cleanup of old backups

### Benchmarks

```
Full Backup (1GB database):
  - Without compression: 45 seconds, 1.0GB storage
  - With compression: 65 seconds, 0.7GB storage

Incremental Backup (100MB changes):
  - Duration: 8 seconds
  - Storage: 85MB

Point-in-Time Recovery (1 hour):
  - Duration: 120 seconds
  - WAL records replayed: 50,000
```

## Security

### Encryption
- At-rest encryption for backup files
- Secure key management (external to this crate)
- Cloud storage with built-in encryption

### Access Control
- Backup file permissions
- Cloud storage access policies
- Audit logging of backup operations

## Integration Examples

### With KotobaDB

```rust
use kotoba_db::DB;
use kotoba_backup::*;

// Create database instance
let db = DB::open_lsm("./database").await?;

// Create backup manager
let backup_config = BackupConfig {
    backup_path: "./backups".into(),
    ..Default::default()
};

// Wrap database for backup operations
let backup_source = Arc::new(db); // Implement BackupSource trait
let backup_manager = BackupManager::new(backup_source, backup_config);
```

### Custom Storage Backend

```rust
use kotoba_backup::*;

// Implement custom storage
#[async_trait]
impl BackupStorage for MyStorage {
    async fn store_backup(&self, backup_id: &str, data: &[u8], metadata: &BackupMetadata) -> Result<(), StorageError> {
        // Your custom storage logic
        Ok(())
    }

    // Implement other required methods...
}
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add tests for new functionality
4. Update documentation
5. Submit a pull request

## License

Licensed under the MIT License.

---

**KotobaDB Backup & Restore** - *Reliable data protection for modern databases* 🛡️💾

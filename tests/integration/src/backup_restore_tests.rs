//! Backup and Restore Integration Tests
//!
//! Tests backup and restore functionality including:
//! - Full backups and restores
//! - Incremental backups
//! - Point-in-time recovery
//! - Backup integrity verification
//! - Cross-platform compatibility

use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use kotoba_graphdb::GraphDB;
use kotoba_core::types::{Value, VertexId, EdgeId};
use std::collections::HashMap;

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
#[ignore] // Temporarily disabled - kotoba_backup crate not available
async fn test_full_backup_restore() -> Result<(), Box<dyn std::error::Error>> {
    // use kotoba_backup::{BackupManager, RestoreManager};

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("backup_test.db");
    let backup_dir = temp_dir.path().join("backups");

    // Create and populate database
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_test_data(&db, 100).await?;
        println!("✓ Created test database with 100 nodes");
    }

    // Create backup
    let backup_manager = BackupManager::new(backup_dir.clone());
    let backup_path = backup_manager.create_full_backup().await?;
    println!("✓ Created full backup: {:?}", backup_path);

    // Verify backup file exists and has content
    assert!(backup_path.exists(), "Backup file should exist");
    let backup_size = std::fs::metadata(&backup_path)?.len();
    assert!(backup_size > 0, "Backup file should not be empty");

    // Create new database and restore
    let restore_dir = temp_dir.path().join("restored");
    std::fs::create_dir_all(&restore_dir)?;
    let restored_db_path = restore_dir.join("restored.db");

    let restore_manager = RestoreManager::new(restore_dir);
    restore_manager.restore_full_backup(&backup_path).await?;
    println!("✓ Restored database from backup");

    // Verify restored data
    {
        let restored_db = GraphDB::new(&restored_db_path.to_string_lossy()).await?;
        let user_nodes = restored_db.find_nodes_by_label("User").await?;
        assert_eq!(user_nodes.len(), 100, "Should have restored all 100 user nodes");

        // Verify specific data integrity
        for i in 1..=10 { // Check first 10 nodes
            let test_nodes = restored_db.find_nodes_by_property(
                "user_id",
                &Value::Int(i as i64)
            ).await?;
            assert_eq!(test_nodes.len(), 1, "Each user ID should be unique");
        }
    }

    println!("✓ Full backup/restore test passed");
    Ok(())
}

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
async fn test_incremental_backup() -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::{BackupManager, RestoreManager};

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("incremental_test.db");
    let backup_dir = temp_dir.path().join("backups");

    // Phase 1: Initial data and backup
    let mut last_backup_path = {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_test_data(&db, 50).await?;

        let backup_manager = BackupManager::new(backup_dir.clone());
        let backup_path = backup_manager.create_full_backup().await?;
        println!("✓ Created initial backup with 50 nodes");
        backup_path
    };

    // Phase 2: Add more data and create incremental backup
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_additional_data(&db, 50, 51).await?;

        let backup_manager = BackupManager::new(backup_dir);
        let incremental_backup = backup_manager.create_incremental_backup().await?;
        println!("✓ Created incremental backup with additional 50 nodes");
        last_backup_path = incremental_backup;
    }

    // Phase 3: Restore and verify all data
    let restore_dir = temp_dir.path().join("incremental_restored");
    std::fs::create_dir_all(&restore_dir)?;
    let restored_db_path = restore_dir.join("restored.db");

    let restore_manager = RestoreManager::new(restore_dir);
    restore_manager.restore_full_backup(&last_backup_path).await?;
    println!("✓ Restored database from incremental backup");

    // Verify all data is present
    {
        let restored_db = GraphDB::new(&restored_db_path.to_string_lossy()).await?;
        let all_users = restored_db.find_nodes_by_label("User").await?;
        assert_eq!(all_users.len(), 100, "Should have all 100 users from full + incremental");

        // Verify incremental data
        let incremental_users = restored_db.find_nodes_by_property_range(
            "user_id",
            &Value::Int(51),
            &Value::Int(100)
        ).await?;
        assert_eq!(incremental_users.len(), 50, "Should have 50 incremental users");
    }

    println!("✓ Incremental backup test passed");
    Ok(())
}

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
async fn test_point_in_time_recovery() -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::{BackupManager, RestoreManager, PointInTimeRecovery};
    use chrono::{DateTime, Utc};

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("pitr_test.db");
    let backup_dir = temp_dir.path().join("backups");

    // Record timestamps for PITR
    let timestamp1 = Utc::now();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Phase 1: Create initial data
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_test_data(&db, 30).await?;
    }

    let timestamp2 = Utc::now();
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

    // Phase 2: Add more data
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_additional_data(&db, 30, 31).await?;
    }

    let timestamp3 = Utc::now();

    // Create backup
    let backup_manager = BackupManager::new(backup_dir);
    let backup_path = backup_manager.create_full_backup().await?;

    // Test PITR to timestamp2 (should only have first 30 users)
    let pitr = PointInTimeRecovery::new(backup_path.clone());
    let restore_dir = temp_dir.path().join("pitr_restored");
    std::fs::create_dir_all(&restore_dir)?;

    pitr.recover_to_timestamp(timestamp2, restore_dir.clone()).await?;
    println!("✓ Performed point-in-time recovery to timestamp2");

    // Verify restored state
    let restored_db_path = restore_dir.join("restored.db");
    let restored_db = GraphDB::new(&restored_db_path.to_string_lossy()).await?;
    let users_at_timestamp2 = restored_db.find_nodes_by_label("User").await?;
    assert_eq!(users_at_timestamp2.len(), 30, "Should only have users from before timestamp2");

    println!("✓ Point-in-time recovery test passed");
    Ok(())
}

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
async fn test_backup_integrity_verification() -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::BackupManager;

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("integrity_test.db");
    let backup_dir = temp_dir.path().join("backups");

    // Create database with known data
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        populate_test_data(&db, 20).await?;
    }

    // Create backup
    let backup_manager = BackupManager::new(backup_dir);
    let backup_path = backup_manager.create_full_backup().await?;

    // Verify backup integrity
    let is_valid = backup_manager.verify_backup(&backup_path).await?;
    assert!(is_valid, "Backup should be valid");

    // Test with corrupted backup (simulate corruption)
    let corrupted_path = temp_dir.path().join("corrupted.backup");
    std::fs::copy(&backup_path, &corrupted_path)?;
    // Corrupt the file by truncating it
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(&corrupted_path)?;
        file.set_len(100)?; // Truncate to 100 bytes
    }

    let is_corrupted_valid = backup_manager.verify_backup(&corrupted_path).await?;
    assert!(!is_corrupted_valid, "Corrupted backup should be detected as invalid");

    println!("✓ Backup integrity verification test passed");
    Ok(())
}

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
async fn test_backup_compression() -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::BackupManager;

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("compression_test.db");
    let backup_dir = temp_dir.path().join("backups");

    // Create database with compressible data (repeated patterns)
    {
        let db = GraphDB::new(&db_path.to_string_lossy()).await?;
        for i in 1..=100 {
            let user = NodeBlock {
                labels: vec!["User".to_string()],
                properties: HashMap::from([
                    ("user_id".to_string(), Value::Int(i)),
                    ("name".to_string(), Value::String(format!("User {}", i))),
                    ("description".to_string(), Value::String("This is a very long description that should compress well when repeated many times in the backup. ".repeat(10))),
                    ("data".to_string(), Value::String("A".repeat(1000))), // Highly compressible
                ]),
            };
            db.put_block(&Block::Node(user)).await?;
        }
    }

    // Create compressed backup
    let backup_manager = BackupManager::new(backup_dir);
    let compressed_backup = backup_manager.create_full_backup().await?;

    // Get sizes
    let db_size = get_directory_size(&db_path.parent().unwrap())?;
    let compressed_size = std::fs::metadata(&compressed_backup)?.len();

    println!("Database size: {} bytes", db_size);
    println!("Compressed backup size: {} bytes", compressed_size);
    println!("Compression ratio: {:.2}", db_size as f64 / compressed_size as f64);

    // Compression should achieve reasonable ratio
    assert!(compressed_size < db_size, "Compressed backup should be smaller than original");

    // Verify restore works
    let restore_dir = temp_dir.path().join("compression_restored");
    std::fs::create_dir_all(&restore_dir)?;

    use kotoba_backup::RestoreManager;
    let restore_manager = RestoreManager::new(restore_dir.clone());
    restore_manager.restore_full_backup(&compressed_backup).await?;

    let restored_db_path = restore_dir.join("restored.db");
    let restored_db = GraphDB::new(&restored_db_path.to_string_lossy()).await?;
    let users = restored_db.find_nodes_by_label("User").await?;
    assert_eq!(users.len(), 100, "All users should be restored from compressed backup");

    println!("✓ Backup compression test passed");
    Ok(())
}

#[ignore] // Temporarily disabled - kotoba_backup crate not available
#[tokio::test]
async fn test_concurrent_backup_and_operations() -> Result<(), Box<dyn std::error::Error>> {
    use kotoba_backup::BackupManager;

    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("concurrent_test.db");
    let backup_dir = temp_dir.path().join("backups");

    let db = Arc::new(Mutex::new(GraphDB::new(&db_path.to_string_lossy()).await?));

    // Start background operations
    let db_clone = Arc::clone(&db);
    let operation_handle = tokio::spawn(async move {
        let db_guard = db_clone.lock().await;
        for i in 1..=50 {
            let user = NodeBlock {
                labels: vec!["ConcurrentUser".to_string()],
                properties: HashMap::from([
                    ("id".to_string(), Value::Int(i)),
                    ("data".to_string(), Value::String(format!("Concurrent data {}", i))),
                ]),
            };
            db_guard.put_block(&Block::Node(user)).await?;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    // Perform backup while operations are running
    let backup_manager = BackupManager::new(backup_dir);
    let backup_path = backup_manager.create_full_backup().await?;
    println!("✓ Created backup while database operations were running");

    // Wait for operations to complete
    operation_handle.await??;

    // Verify backup captured the data
    let restore_dir = temp_dir.path().join("concurrent_restored");
    std::fs::create_dir_all(&restore_dir)?;

    use kotoba_backup::RestoreManager;
    let restore_manager = RestoreManager::new(restore_dir.clone());
    restore_manager.restore_full_backup(&backup_path).await?;

    let restored_db_path = restore_dir.join("restored.db");
    let restored_db = GraphDB::new(&restored_db_path.to_string_lossy()).await?;
    let concurrent_users = restored_db.find_nodes_by_label("ConcurrentUser").await?;
    assert!(!concurrent_users.is_empty(), "Should have captured concurrent operations");

    println!("✓ Concurrent backup test passed");
    Ok(())
}

// Helper functions

async fn populate_test_data(db: &DB, count: usize) -> Result<(), Box<dyn std::error::Error>> {
    for i in 1..=count {
        let user = NodeBlock {
            labels: vec!["User".to_string()],
            properties: HashMap::from([
                ("user_id".to_string(), Value::Int(i as i64)),
                ("name".to_string(), Value::String(format!("User {}", i))),
                ("email".to_string(), Value::String(format!("user{}@example.com", i))),
                ("age".to_string(), Value::Int((20 + (i % 50)) as i64)),
            ]),
        };
        db.put_block(&Block::Node(user)).await?;
    }
    Ok(())
}

async fn populate_additional_data(db: &DB, count: usize, start_id: usize) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..count {
        let user_id = start_id + i;
        let user = NodeBlock {
            labels: vec!["User".to_string()],
            properties: HashMap::from([
                ("user_id".to_string(), Value::Int(user_id as i64)),
                ("name".to_string(), Value::String(format!("Additional User {}", user_id))),
                ("email".to_string(), Value::String(format!("additional{}@example.com", user_id))),
                ("department".to_string(), Value::String("Engineering".to_string())),
            ]),
        };
        db.put_block(&Block::Node(user)).await?;
    }
    Ok(())
}

fn get_directory_size(path: &std::path::Path) -> Result<u64, Box<dyn std::error::Error>> {
    let mut total_size = 0u64;
    for entry in walkdir::WalkDir::new(path) {
        let entry = entry?;
        if entry.file_type().is_file() {
            total_size += entry.metadata()?.len();
        }
    }
    Ok(total_size)
}

//! KotobaDB Integration Tests
//!
//! This crate provides comprehensive integration tests for the entire KotobaDB ecosystem,
//! including database operations, clustering, backup/restore, and performance validation.

pub mod database_lifecycle;
pub mod graph_operations;
pub mod transaction_tests;
pub mod backup_restore_tests;
pub mod cluster_tests;
pub mod performance_tests;
pub mod schema_validation;
pub mod concurrent_access;
pub mod data_integrity;
pub mod error_handling;

// New architecture tests
pub mod ocel_graphdb_tests;

#[cfg(test)]
mod integration_tests {
    use std::sync::Arc;
    use tokio::sync::Mutex;

    /// Global test database instance for shared use across tests
    pub static TEST_DB: once_cell::sync::Lazy<Arc<Mutex<Option<kotoba_db::DB>>>> =
        once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

    /// Setup function to initialize test database
    pub async fn setup_test_db() -> Result<(), Box<dyn std::error::Error>> {
        let mut db_guard = TEST_DB.lock().await;
        if db_guard.is_none() {
            // Create a temporary database for testing
            let temp_dir = tempfile::tempdir()?;
            let db_path = temp_dir.path().join("test_kotoba.db");

            let db = kotoba_db::DB::open_lsm(&db_path).await?;
            *db_guard = Some(db);
        }
        Ok(())
    }

    /// Cleanup function to reset test database
    pub async fn cleanup_test_db() -> Result<(), Box<dyn std::error::Error>> {
        let mut db_guard = TEST_DB.lock().await;
        *db_guard = None;
        Ok(())
    }

    /// Helper to get a reference to the test database
    pub async fn get_test_db() -> Result<Arc<Mutex<kotoba_db::DB>>, Box<dyn std::error::Error>> {
        setup_test_db().await?;
        let db_guard = TEST_DB.lock().await;
        match &*db_guard {
            Some(db) => Ok(Arc::new(Mutex::new(db.clone()))),
            None => Err("Test database not initialized".into()),
        }
    }
}

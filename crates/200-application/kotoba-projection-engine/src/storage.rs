//! Storage Layer
//!
//! Persistent storage for projection states and metadata.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, error, instrument};

use crate::ProjectionState;

/// Storage interface for projection data
#[async_trait]
pub trait ProjectionStorage: Send + Sync {
    /// Save projection state
    async fn save_projection_state(&self, name: &str, state: &ProjectionState) -> Result<()>;

    /// Load projection state
    async fn load_projection_state(&self, name: &str) -> Result<Option<ProjectionState>>;

    /// List all projection names
    async fn list_projections(&self) -> Result<Vec<String>>;

    /// Delete projection state
    async fn delete_projection_state(&self, name: &str) -> Result<()>;

    /// Get storage size
    async fn get_size(&self) -> Result<u64>;
}

/// In-memory storage implementation (for development/testing)
pub struct InMemoryStorage {
    data: std::sync::RwLock<std::collections::HashMap<String, ProjectionState>>,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            data: std::sync::RwLock::new(std::collections::HashMap::new()),
        }
    }
}

#[async_trait]
impl ProjectionStorage for InMemoryStorage {
    async fn save_projection_state(&self, name: &str, state: &ProjectionState) -> Result<()> {
        let mut data = self.data.write().unwrap();
        data.insert(name.to_string(), state.clone());
        Ok(())
    }

    async fn load_projection_state(&self, name: &str) -> Result<Option<ProjectionState>> {
        let data = self.data.read().unwrap();
        Ok(data.get(name).cloned())
    }

    async fn list_projections(&self) -> Result<Vec<String>> {
        let data = self.data.read().unwrap();
        Ok(data.keys().cloned().collect())
    }

    async fn delete_projection_state(&self, name: &str) -> Result<()> {
        let mut data = self.data.write().unwrap();
        data.remove(name);
        Ok(())
    }

    async fn get_size(&self) -> Result<u64> {
        // Estimate size
        let data = self.data.read().unwrap();
        let size = data.len() as u64 * 1024; // Rough estimate
        Ok(size)
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory function to create storage
pub fn create_storage() -> Arc<dyn ProjectionStorage> {
    Arc::new(InMemoryStorage::new())
}

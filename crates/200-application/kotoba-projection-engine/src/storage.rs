//! Storage Layer
//!
//! Persistent storage for projection states and metadata using KeyValueStore interface.

use std::sync::Arc;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use tracing::{info, warn, error, instrument};

use kotoba_storage::KeyValueStore;
use crate::ProjectionState;

/// Generic projection storage using KeyValueStore
pub struct ProjectionStorage<T: KeyValueStore> {
    storage: Arc<T>,
    prefix: String,
}

impl<T: KeyValueStore + 'static> ProjectionStorage<T> {
    /// Create new projection storage
    pub fn new(storage: Arc<T>, prefix: String) -> Self {
        Self { storage, prefix }
    }

    /// Save projection state
    pub async fn save_projection_state(&self, name: &str, state: &ProjectionState) -> Result<()> {
        let key = format!("{}:state:{}", self.prefix, name);
        let value = bincode::serialize(state)?;
        self.storage.put(key.as_bytes(), &value).await?;
        Ok(())
    }

    /// Load projection state
    pub async fn load_projection_state(&self, name: &str) -> Result<Option<ProjectionState>> {
        let key = format!("{}:state:{}", self.prefix, name);
        match self.storage.get(key.as_bytes()).await? {
            Some(data) => {
                let state = bincode::deserialize(&data)?;
                Ok(Some(state))
            }
            None => Ok(None)
        }
    }

    /// List all projection names
    pub async fn list_projections(&self) -> Result<Vec<String>> {
        let prefix = format!("{}:state:", self.prefix);
        let results = self.storage.scan(prefix.as_bytes()).await?;
        let mut projections = Vec::new();

        for (key, _) in results {
            if let Ok(key_str) = std::str::from_utf8(&key) {
                if let Some(name) = key_str.strip_prefix(&prefix) {
                    projections.push(name.to_string());
                }
            }
        }

        Ok(projections)
    }

    /// Delete projection state
    pub async fn delete_projection_state(&self, name: &str) -> Result<()> {
        let key = format!("{}:state:{}", self.prefix, name);
        self.storage.delete(key.as_bytes()).await?;
        Ok(())
    }
}
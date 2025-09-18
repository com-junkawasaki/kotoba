//! Defines the `KeyValuePort` for low-level storage backends.

use async_trait::async_trait;
use kotoba_core::prelude::Result;
use crate::domain::models::BackendStats;

#[async_trait]
pub trait KeyValuePort: Send + Sync {
    /// Puts a key-value pair into the store.
    async fn put(&self, key: String, value: Vec<u8>) -> Result<()>;

    /// Gets a value by key.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair.
    async fn delete(&self, key: String) -> Result<()>;

    /// Gets all keys with a given prefix.
    async fn get_keys_with_prefix(&self, prefix: &str) -> Result<Vec<String>>;

    /// Clears the entire store.
    async fn clear(&self) -> Result<()>;

    /// Gets statistics about the backend.
    async fn stats(&self) -> Result<BackendStats>;

    async fn exists(&self, key: &str) -> Result<bool>;
}

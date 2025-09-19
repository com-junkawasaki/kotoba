//! `kotoba-storage`
//!
//! This crate defines the core traits (ports) for storage operations
//! in the Kotoba ecosystem. It provides abstractions for various storage
//! backends like Key-Value stores, Event stores, and Graph stores.

use anyhow::Result;
use async_trait::async_trait;

/// A generic key-value store trait.
#[async_trait]
pub trait KeyValueStore: Send + Sync {
    /// Puts a key-value pair into the store.
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Gets a value for a given key.
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair from the store.
    async fn delete(&self, key: &[u8]) -> Result<()>;

    /// Scans for key-value pairs with a given prefix.
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

// TODO: Define EventStore and GraphStore traits later
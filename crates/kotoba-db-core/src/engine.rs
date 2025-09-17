use anyhow::Result;

/// A trait for pluggable storage engines.
/// This defines the basic key-value interface that the database core uses.
pub trait StorageEngine {
    /// Puts a key-value pair into the store.
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Gets a value from the store by its key.
    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Deletes a key-value pair from the store.
    fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// Scans a range of keys with a given prefix.
    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}

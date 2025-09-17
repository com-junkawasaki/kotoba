use kotoba_db_core::engine::StorageEngine;
use std::collections::HashMap;
use anyhow::Result;

pub struct MemoryStorageEngine {
    store: HashMap<Vec<u8>, Vec<u8>>,
}

impl MemoryStorageEngine {
    /// Creates a new in-memory storage engine.
    pub fn new() -> Self {
        Self {
            store: HashMap::new(),
        }
    }
}

impl StorageEngine for MemoryStorageEngine {
    fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.store.insert(key.to_vec(), value.to_vec());
        Ok(())
    }

    fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        Ok(self.store.get(key).cloned())
    }

    fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.store.remove(key);
        Ok(())
    }

    fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let mut results = Vec::new();
        for (key, value) in &self.store {
            if key.starts_with(prefix) {
                results.push((key.clone(), value.clone()));
            }
        }
        // Sort by key for consistent ordering
        results.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(results)
    }
}

impl Default for MemoryStorageEngine {
    fn default() -> Self {
        Self::new()
    }
}

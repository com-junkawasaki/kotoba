use kotoba_db_core::engine::StorageEngine;
use std::collections::HashMap;

pub struct MemoryStorageEngine {
    store: HashMap<Vec<u8>, Vec<u8>>,
}

impl StorageEngine for MemoryStorageEngine {
    // ... implementation will go here
}

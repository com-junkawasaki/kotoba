//! AI Memory for conversation context and state management

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub timestamp: u64,
    pub ttl: Option<u64>, // Time to live in seconds
}

/// AI Memory manager
pub struct AiMemory {
    storage: HashMap<String, MemoryEntry>,
}

impl AiMemory {
    /// Create new AI memory
    pub fn new() -> Self {
        Self {
            storage: HashMap::new(),
        }
    }

    /// Store memory entry
    pub fn store(&mut self, key: String, value: serde_json::Value, ttl: Option<u64>) {
        let entry = MemoryEntry {
            key: key.clone(),
            value,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl,
        };
        self.storage.insert(key, entry);
    }

    /// Retrieve memory entry
    pub fn retrieve(&self, key: &str) -> Option<&MemoryEntry> {
        self.storage.get(key)
    }

    /// Delete memory entry
    pub fn delete(&mut self, key: &str) {
        self.storage.remove(key);
    }

    /// Clean expired entries
    pub fn cleanup_expired(&mut self) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.storage.retain(|_, entry| {
            if let Some(ttl) = entry.ttl {
                now - entry.timestamp < ttl
            } else {
                true
            }
        });
    }
}

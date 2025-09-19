//! AI Memory for conversation context and state management

use crate::{KotobaNetError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Memory entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub key: String,
    pub value: serde_json::Value,
    pub timestamp: u64,
    pub ttl: Option<u64>, // Time to live in seconds
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Memory statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStats {
    pub total_entries: usize,
    pub total_messages: usize,
    pub memory_type: String,
    pub max_capacity: Option<usize>,
}

/// Memory type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemoryType {
    KeyValue,
    Conversation,
    Buffer,
}

/// AI Memory manager with conversation support
pub struct AiMemory {
    storage: HashMap<String, MemoryEntry>,
    conversations: VecDeque<ConversationMessage>,
    memory_type: MemoryType,
    max_messages: usize,
}

impl AiMemory {
    /// Create new AI memory with specified type
    pub fn new(memory_type: MemoryType, max_messages: usize) -> Self {
        Self {
            storage: HashMap::new(),
            conversations: VecDeque::new(),
            memory_type,
            max_messages,
        }
    }

    /// Create conversation memory
    pub fn conversation(max_messages: usize) -> Self {
        Self::new(MemoryType::Conversation, max_messages)
    }

    /// Create buffer memory
    pub fn buffer(max_messages: usize) -> Self {
        Self::new(MemoryType::Buffer, max_messages)
    }

    /// Create key-value memory
    pub fn key_value() -> Self {
        Self::new(MemoryType::KeyValue, 0)
    }

    /// Store key-value entry
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

    /// Retrieve key-value entry
    pub fn retrieve(&self, key: &str) -> Option<&MemoryEntry> {
        self.storage.get(key)
    }

    /// Delete key-value entry
    pub fn delete(&mut self, key: &str) {
        self.storage.remove(key);
    }

    /// Store conversation message
    pub async fn store_message(&mut self, role: String, content: String, metadata: Option<HashMap<String, serde_json::Value>>) -> Result<()> {
        let message = ConversationMessage {
            role,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata,
        };

        self.conversations.push_back(message);

        // Maintain size limit
        while self.conversations.len() > self.max_messages {
            self.conversations.pop_front();
        }

        Ok(())
    }

    /// Retrieve conversation messages
    pub async fn retrieve_messages(&self, limit: Option<usize>) -> Result<Vec<ConversationMessage>> {
        let messages: Vec<ConversationMessage> = if let Some(limit) = limit {
            self.conversations.iter().rev().take(limit).cloned().collect()
        } else {
            self.conversations.iter().cloned().collect()
        };

        // Reverse to get chronological order (newest first)
        Ok(messages.into_iter().rev().collect())
    }

    /// Clear all memory
    pub async fn clear(&mut self) -> Result<()> {
        self.storage.clear();
        self.conversations.clear();
        Ok(())
    }

    /// Get memory statistics
    pub async fn stats(&self) -> Result<MemoryStats> {
        let memory_type_str = match self.memory_type {
            MemoryType::KeyValue => "key_value",
            MemoryType::Conversation => "conversation",
            MemoryType::Buffer => "buffer",
        }.to_string();

        Ok(MemoryStats {
            total_entries: self.storage.len(),
            total_messages: self.conversations.len(),
            memory_type: memory_type_str,
            max_capacity: if self.max_messages > 0 { Some(self.max_messages) } else { None },
        })
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

    /// Get all stored keys
    pub fn keys(&self) -> Vec<String> {
        self.storage.keys().cloned().collect()
    }

    /// Check if key exists
    pub fn has_key(&self, key: &str) -> bool {
        self.storage.contains_key(key)
    }

    /// Get memory type
    pub fn memory_type(&self) -> &MemoryType {
        &self.memory_type
    }
}


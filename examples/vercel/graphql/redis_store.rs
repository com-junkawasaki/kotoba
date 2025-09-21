//! Redis backend implementation for Kotoba GraphQL API using kotoba-storage-redis

use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Redis-based graph database store using kotoba-storage-redis
pub struct RedisGraphStore {
    store: Arc<RedisStore>,
    key_prefix: String,
}

impl RedisGraphStore {
    /// Create a new Redis graph store using kotoba-storage-redis
    pub async fn new(redis_url: &str, key_prefix: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = RedisConfig {
            redis_urls: vec![redis_url.to_string()],
            key_prefix: key_prefix.to_string(),
            ..Default::default()
        };

        let store = Arc::new(RedisStore::new(config).await?);
        Ok(Self {
            store,
            key_prefix: key_prefix.to_string(),
        })
    }

    /// Generate key for node
    fn node_key(&self, id: &str) -> String {
        format!("node:{}", id)
    }

    /// Generate key for edge
    fn edge_key(&self, id: &str) -> String {
        format!("edge:{}", id)
    }

    /// Generate key for node index by label
    fn node_label_index_key(&self, label: &str) -> String {
        format!("index:node:label:{}", label)
    }

    /// Generate key for edge index by label
    fn edge_label_index_key(&self, label: &str) -> String {
        format!("index:edge:label:{}", label)
    }

    /// Create a new node
    pub async fn create_node(
        &self,
        id: Option<String>,
        labels: Vec<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisNode, Box<dyn std::error::Error + Send + Sync>> {
        let node_id = id.unwrap_or_else(|| format!("node_{}", uuid::Uuid::new_v4()));

        let node = RedisNode {
            id: node_id.clone(),
            labels: labels.clone(),
            properties,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let key = self.node_key(&node_id);
        let serialized = serde_json::to_string(&node)?;

        // Store node using KeyValueStore
        self.store.put(key.as_bytes(), serialized.as_bytes()).await?;

        // TODO: Add label indexing logic
        // For now, we'll skip the indexing to keep it simple

        Ok(node)
    }

    /// Get node by ID
    pub async fn get_node(&self, id: &str) -> Result<Option<RedisNode>, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.node_key(id);

        match self.store.get(key.as_bytes()).await? {
            Some(data) => {
                let node: RedisNode = serde_json::from_slice(&data)?;
                Ok(Some(node))
            }
            None => Ok(None),
        }
    }

    /// Update node
    pub async fn update_node(
        &self,
        id: &str,
        labels: Option<Vec<String>>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisNode, Box<dyn std::error::Error + Send + Sync>> {
        // Get existing node
        let existing = self.get_node(id).await?
            .ok_or("Node not found")?;

        // Update node
        let updated_labels = labels.unwrap_or(existing.labels);
        let updated_node = RedisNode {
            id: id.to_string(),
            labels: updated_labels,
            properties,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        let key = self.node_key(id);
        let serialized = serde_json::to_string(&updated_node)?;

        // Store updated node using KeyValueStore
        self.store.put(key.as_bytes(), serialized.as_bytes()).await?;

        Ok(updated_node)
    }

    /// Delete node
    pub async fn delete_node(&self, id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.node_key(id);

        // Check if node exists
        let exists = self.store.get(key.as_bytes()).await?.is_some();

        if exists {
            // Delete node using KeyValueStore
            self.store.delete(key.as_bytes()).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Create a new edge
    pub async fn create_edge(
        &self,
        id: Option<String>,
        from_node: String,
        to_node: String,
        label: String,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisEdge, Box<dyn std::error::Error + Send + Sync>> {
        let edge_id = id.unwrap_or_else(|| format!("edge_{}", uuid::Uuid::new_v4()));

        let edge = RedisEdge {
            id: edge_id.clone(),
            from_node,
            to_node,
            label,
            properties,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let key = self.edge_key(&edge_id);
        let serialized = serde_json::to_string(&edge)?;

        // Store edge using KeyValueStore
        self.store.put(key.as_bytes(), serialized.as_bytes()).await?;

        Ok(edge)
    }

    /// Get edge by ID
    pub async fn get_edge(&self, id: &str) -> Result<Option<RedisEdge>, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.edge_key(id);

        match self.store.get(key.as_bytes()).await? {
            Some(data) => {
                let edge: RedisEdge = serde_json::from_slice(&data)?;
                Ok(Some(edge))
            }
            None => Ok(None),
        }
    }

    /// Update edge
    pub async fn update_edge(
        &self,
        id: &str,
        label: Option<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisEdge, Box<dyn std::error::Error + Send + Sync>> {
        // Get existing edge
        let existing = self.get_edge(id).await?
            .ok_or("Edge not found")?;

        // Update edge
        let updated_label = label.unwrap_or(existing.label);
        let updated_edge = RedisEdge {
            id: id.to_string(),
            from_node: existing.from_node,
            to_node: existing.to_node,
            label: updated_label,
            properties,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        let key = self.edge_key(id);
        let serialized = serde_json::to_string(&updated_edge)?;

        // Store updated edge using KeyValueStore
        self.store.put(key.as_bytes(), serialized.as_bytes()).await?;

        Ok(updated_edge)
    }

    /// Delete edge
    pub async fn delete_edge(&self, id: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.edge_key(id);

        // Check if edge exists
        let exists = self.store.get(key.as_bytes()).await?.is_some();

        if exists {
            // Delete edge using KeyValueStore
            self.store.delete(key.as_bytes()).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, Box<dyn std::error::Error + Send + Sync>> {
        // Use KeyValueStore stats
        let store_stats = self.store.stats().await?;

        Ok(DatabaseStats {
            total_keys: store_stats.total_keys as i32,
            connected_clients: 1, // Simplified
            uptime_seconds: 0, // Simplified
        })
    }
}

/// Redis node representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisNode {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Redis edge representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedisEdge {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize, async_graphql::SimpleObject)]
pub struct DatabaseStats {
    pub total_keys: i32,
    pub connected_clients: i32,
    pub uptime_seconds: i32,
}

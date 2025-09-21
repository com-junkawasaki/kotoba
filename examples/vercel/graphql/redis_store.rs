//! Redis backend implementation for Kotoba GraphQL API

use redis::{Client, AsyncCommands, RedisError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Redis-based graph database store
pub struct RedisGraphStore {
    client: Client,
    key_prefix: String,
}

impl RedisGraphStore {
    /// Create a new Redis graph store
    pub fn new(redis_url: &str, key_prefix: &str) -> Result<Self, RedisError> {
        let client = Client::open(redis_url)?;
        Ok(Self {
            client,
            key_prefix: key_prefix.to_string(),
        })
    }

    /// Get connection from pool
    async fn get_connection(&self) -> Result<redis::aio::Connection, RedisError> {
        self.client.get_async_connection().await
    }

    /// Generate key for node
    fn node_key(&self, id: &str) -> String {
        format!("{}:node:{}", self.key_prefix, id)
    }

    /// Generate key for edge
    fn edge_key(&self, id: &str) -> String {
        format!("{}:edge:{}", self.key_prefix, id)
    }

    /// Generate key for node index by label
    fn node_label_index_key(&self, label: &str) -> String {
        format!("{}:index:node:label:{}", self.key_prefix, label)
    }

    /// Generate key for edge index by label
    fn edge_label_index_key(&self, label: &str) -> String {
        format!("{}:index:edge:label:{}", self.key_prefix, label)
    }

    /// Create a new node
    pub async fn create_node(
        &self,
        id: Option<String>,
        labels: Vec<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisNode, RedisError> {
        let node_id = id.unwrap_or_else(|| format!("node_{}", uuid::Uuid::new_v4()));

        let node = RedisNode {
            id: node_id.clone(),
            labels: labels.clone(),
            properties,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut conn = self.get_connection().await?;
        let key = self.node_key(&node_id);
        let serialized = serde_json::to_string(&node)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        // Store node
        conn.set(&key, &serialized).await?;

        // Add to label indexes
        for label in &labels {
            let index_key = self.node_label_index_key(label);
            conn.sadd(&index_key, &node_id).await?;
        }

        Ok(node)
    }

    /// Get node by ID
    pub async fn get_node(&self, id: &str) -> Result<Option<RedisNode>, RedisError> {
        let mut conn = self.get_connection().await?;
        let key = self.node_key(id);

        let serialized: Option<String> = conn.get(&key).await?;
        match serialized {
            Some(data) => {
                let node: RedisNode = serde_json::from_str(&data)
                    .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Deserialization failed", e.to_string())))?;
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
    ) -> Result<RedisNode, RedisError> {
        let mut conn = self.get_connection().await?;

        // Get existing node
        let existing = self.get_node(id).await?
            .ok_or_else(|| RedisError::from((redis::ErrorKind::TypeError, "Node not found")))?;

        // Remove from old label indexes
        for label in &existing.labels {
            let index_key = self.node_label_index_key(label);
            conn.srem(&index_key, id).await?;
        }

        // Update node
        let updated_labels = labels.unwrap_or(existing.labels);
        let mut updated_node = RedisNode {
            id: id.to_string(),
            labels: updated_labels.clone(),
            properties,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        let key = self.node_key(id);
        let serialized = serde_json::to_string(&updated_node)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        conn.set(&key, &serialized).await?;

        // Add to new label indexes
        for label in &updated_labels {
            let index_key = self.node_label_index_key(label);
            conn.sadd(&index_key, id).await?;
        }

        Ok(updated_node)
    }

    /// Delete node
    pub async fn delete_node(&self, id: &str) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;
        let key = self.node_key(id);

        // Get existing node to remove from indexes
        if let Some(node) = self.get_node(id).await? {
            for label in &node.labels {
                let index_key = self.node_label_index_key(label);
                conn.srem(&index_key, id).await?;
            }
        }

        let deleted: i32 = conn.del(&key).await?;
        Ok(deleted > 0)
    }

    /// Create a new edge
    pub async fn create_edge(
        &self,
        id: Option<String>,
        from_node: String,
        to_node: String,
        label: String,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<RedisEdge, RedisError> {
        let edge_id = id.unwrap_or_else(|| format!("edge_{}", uuid::Uuid::new_v4()));

        let edge = RedisEdge {
            id: edge_id.clone(),
            from_node: from_node.clone(),
            to_node: to_node.clone(),
            label: label.clone(),
            properties,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut conn = self.get_connection().await?;
        let key = self.edge_key(&edge_id);
        let serialized = serde_json::to_string(&edge)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        // Store edge
        conn.set(&key, &serialized).await?;

        // Add to label index
        let index_key = self.edge_label_index_key(&label);
        conn.sadd(&index_key, &edge_id).await?;

        // Add to node adjacency lists
        let from_adjacency_key = format!("{}:node:{}:out", self.key_prefix, from_node);
        conn.sadd(&from_adjacency_key, &edge_id).await?;

        let to_adjacency_key = format!("{}:node:{}:in", self.key_prefix, to_node);
        conn.sadd(&to_adjacency_key, &edge_id).await?;

        Ok(edge)
    }

    /// Get edge by ID
    pub async fn get_edge(&self, id: &str) -> Result<Option<RedisEdge>, RedisError> {
        let mut conn = self.get_connection().await?;
        let key = self.edge_key(id);

        let serialized: Option<String> = conn.get(&key).await?;
        match serialized {
            Some(data) => {
                let edge: RedisEdge = serde_json::from_str(&data)
                    .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Deserialization failed", e.to_string())))?;
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
    ) -> Result<RedisEdge, RedisError> {
        let mut conn = self.get_connection().await?;

        // Get existing edge
        let existing = self.get_edge(id).await?
            .ok_or_else(|| RedisError::from((redis::ErrorKind::TypeError, "Edge not found")))?;

        // Remove from old label index
        let old_index_key = self.edge_label_index_key(&existing.label);
        conn.srem(&old_index_key, id).await?;

        // Update edge
        let updated_label = label.unwrap_or(existing.label);
        let mut updated_edge = RedisEdge {
            id: id.to_string(),
            from_node: existing.from_node,
            to_node: existing.to_node,
            label: updated_label.clone(),
            properties,
            created_at: existing.created_at,
            updated_at: Utc::now(),
        };

        let key = self.edge_key(id);
        let serialized = serde_json::to_string(&updated_edge)
            .map_err(|e| RedisError::from((redis::ErrorKind::TypeError, "Serialization failed", e.to_string())))?;

        conn.set(&key, &serialized).await?;

        // Add to new label index
        let new_index_key = self.edge_label_index_key(&updated_label);
        conn.sadd(&new_index_key, id).await?;

        Ok(updated_edge)
    }

    /// Delete edge
    pub async fn delete_edge(&self, id: &str) -> Result<bool, RedisError> {
        let mut conn = self.get_connection().await?;

        // Get existing edge to clean up indexes
        if let Some(edge) = self.get_edge(id).await? {
            let label_index_key = self.edge_label_index_key(&edge.label);
            conn.srem(&label_index_key, id).await?;

            let from_adjacency_key = format!("{}:node:{}:out", self.key_prefix, edge.from_node);
            conn.srem(&from_adjacency_key, id).await?;

            let to_adjacency_key = format!("{}:node:{}:in", self.key_prefix, edge.to_node);
            conn.srem(&to_adjacency_key, id).await?;
        }

        let key = self.edge_key(id);
        let deleted: i32 = conn.del(&key).await?;
        Ok(deleted > 0)
    }

    /// Get database statistics
    pub async fn get_stats(&self) -> Result<DatabaseStats, RedisError> {
        let mut conn = self.get_connection().await?;

        // Count keys with our prefix
        let pattern = format!("{}*", self.key_prefix);
        let keys: Vec<String> = conn.keys(&pattern).await?;
        let total_keys = keys.len() as i32;

        Ok(DatabaseStats {
            total_keys,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseStats {
    pub total_keys: i32,
    pub connected_clients: i32,
    pub uptime_seconds: i32,
}

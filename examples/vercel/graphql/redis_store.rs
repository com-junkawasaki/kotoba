//! Redis backend implementation for Kotoba GraphQL API using kotoba-storage-redis

use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;
use kotoba_graphdb::{Node as GraphDBNode, Edge as GraphDBEdge, PropertyValue};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use chrono::{DateTime, Utc};
use std::sync::Arc;
use uuid::Uuid;

/// Redis-based graph database store using kotoba-storage-redis
pub struct RedisGraphStore {
    store: Arc<RedisStore>,
    key_prefix: String,
}

/// Convert HashMap to BTreeMap
fn hashmap_to_btreemap<K: Ord + Clone, V>(map: HashMap<K, V>) -> BTreeMap<K, V> {
    map.into_iter().collect()
}

/// Convert serde_json::Value to PropertyValue
fn json_value_to_property_value(value: serde_json::Value) -> PropertyValue {
    match value {
        serde_json::Value::String(s) => PropertyValue::String(s),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PropertyValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                PropertyValue::Float(f)
            } else {
                PropertyValue::String(n.to_string())
            }
        }
        serde_json::Value::Bool(b) => PropertyValue::Boolean(b),
        serde_json::Value::Array(arr) => {
            let values = arr.into_iter().map(json_value_to_property_value).collect();
            PropertyValue::List(values)
        }
        serde_json::Value::Object(obj) => {
            let map = obj.into_iter()
                .map(|(k, v)| (k, json_value_to_property_value(v)))
                .collect();
            PropertyValue::Map(map)
        }
        _ => PropertyValue::String(value.to_string()),
    }
}

/// Convert PropertyValue to GraphQL ValueType
fn property_value_to_value_type(value: &PropertyValue) -> super::schema::ValueType {
    match value {
        PropertyValue::String(s) => super::schema::ValueType {
            string_value: Some(s.clone()),
            int_value: None,
            float_value: None,
            bool_value: None,
            array_value: None,
            object_value: None,
        },
        PropertyValue::Integer(i) => super::schema::ValueType {
            string_value: None,
            int_value: Some(*i),
            float_value: None,
            bool_value: None,
            array_value: None,
            object_value: None,
        },
        PropertyValue::Float(f) => super::schema::ValueType {
            string_value: None,
            int_value: None,
            float_value: Some(*f),
            bool_value: None,
            array_value: None,
            object_value: None,
        },
        PropertyValue::Boolean(b) => super::schema::ValueType {
            string_value: None,
            int_value: None,
            float_value: None,
            bool_value: Some(*b),
            array_value: None,
            object_value: None,
        },
        PropertyValue::Date(dt) => super::schema::ValueType {
            string_value: Some(dt.to_rfc3339()),
            int_value: None,
            float_value: None,
            bool_value: None,
            array_value: None,
            object_value: None,
        },
        PropertyValue::List(values) => {
            let graphql_values: Vec<super::schema::Value> = values
                .iter()
                .map(|v| super::schema::Value {
                    value_type: property_value_to_value_type(v),
                })
                .collect();
            super::schema::ValueType {
                string_value: None,
                int_value: None,
                float_value: None,
                bool_value: None,
                array_value: Some(graphql_values),
                object_value: None,
            }
        }
        PropertyValue::Map(map) => {
            let graphql_map: HashMap<String, super::schema::Value> = map
                .iter()
                .map(|(k, v)| (k.clone(), super::schema::Value {
                    value_type: property_value_to_value_type(v),
                }))
                .collect();
            super::schema::ValueType {
                string_value: None,
                int_value: None,
                float_value: None,
                bool_value: None,
                array_value: None,
                object_value: Some(graphql_map),
            }
        }
    }
}

/// Convert PropertyValue to serde_json::Value for GraphQL
fn property_value_to_json_value(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::String(s) => serde_json::Value::String(s.clone()),
        PropertyValue::Integer(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
        PropertyValue::Float(f) => serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap_or(serde_json::Number::from(0))),
        PropertyValue::Boolean(b) => serde_json::Value::Bool(*b),
        PropertyValue::Date(dt) => serde_json::Value::String(dt.to_rfc3339()),
        PropertyValue::List(values) => {
            let arr = values.iter().map(property_value_to_json_value).collect();
            serde_json::Value::Array(arr)
        }
        PropertyValue::Map(map) => {
            let obj = map.iter()
                .map(|(k, v)| (k.clone(), property_value_to_json_value(v)))
                .collect();
            serde_json::Value::Object(obj)
        }
    }
}

/// Convert GraphDB Node to GraphQL Node
pub fn graphdb_node_to_graphql_node(node: &GraphDBNode) -> super::schema::Node {
    let properties: HashMap<String, super::schema::Value> = node.properties
        .iter()
        .map(|(k, v)| (k.clone(), super::schema::Value {
            value_type: property_value_to_value_type(v)
        }))
        .collect();

    super::schema::Node {
        id: node.id.clone(),
        labels: node.labels.clone(),
        properties,
        created_at: node.created_at.to_rfc3339(),
        updated_at: node.updated_at.to_rfc3339(),
    }
}

/// Convert GraphDB Edge to GraphQL Edge
pub fn graphdb_edge_to_graphql_edge(edge: &GraphDBEdge) -> super::schema::Edge {
    let properties: HashMap<String, super::schema::Value> = edge.properties
        .iter()
        .map(|(k, v)| (k.clone(), super::schema::Value {
            value_type: property_value_to_value_type(v)
        }))
        .collect();

    super::schema::Edge {
        id: edge.id.clone(),
        from_node: edge.from_node.clone(),
        to_node: edge.to_node.clone(),
        label: edge.label.clone(),
        properties,
        created_at: edge.created_at.to_rfc3339(),
        updated_at: edge.updated_at.to_rfc3339(),
    }
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

    /// Generate key for node (Merkle DAG path structure)
    fn node_key(&self, id: &str) -> String {
        format!("nodes/{}", id)
    }

    /// Generate key for edge (Merkle DAG path structure)
    fn edge_key(&self, id: &str) -> String {
        format!("edges/{}", id)
    }

    /// Generate key for node index by label (Merkle DAG structure)
    fn node_label_index_key(&self, label: &str) -> String {
        format!("indices/nodes/labels/{}", label)
    }

    /// Generate key for edge index by label (Merkle DAG structure)
    fn edge_label_index_key(&self, label: &str) -> String {
        format!("indices/edges/labels/{}", label)
    }

    /// Create a new node
    pub async fn create_node(
        &self,
        id: Option<String>,
        labels: Vec<String>,
        properties: HashMap<String, serde_json::Value>,
    ) -> Result<GraphDBNode, Box<dyn std::error::Error + Send + Sync>> {
        let node_id = id.unwrap_or_else(|| format!("node_{}", uuid::Uuid::new_v4()));

        // Convert properties from HashMap<serde_json::Value> to BTreeMap<PropertyValue>
        let properties_btree: BTreeMap<String, PropertyValue> = properties
            .into_iter()
            .map(|(k, v)| (k, json_value_to_property_value(v)))
            .collect();

        let node = GraphDBNode {
            id: node_id.clone(),
            labels,
            properties: properties_btree,
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
    pub async fn get_node(&self, id: &str) -> Result<Option<GraphDBNode>, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.node_key(id);

        match self.store.get(key.as_bytes()).await? {
            Some(data) => {
                let node: GraphDBNode = serde_json::from_slice(&data)?;
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
    ) -> Result<GraphDBNode, Box<dyn std::error::Error + Send + Sync>> {
        // Get existing node
        let existing = self.get_node(id).await?
            .ok_or("Node not found")?;

        // Update node
        let updated_labels = labels.unwrap_or(existing.labels);

        // Merge existing properties with new ones
        let mut updated_properties = existing.properties;
        let new_properties_btree: BTreeMap<String, PropertyValue> = properties
            .into_iter()
            .map(|(k, v)| (k, json_value_to_property_value(v)))
            .collect();
        updated_properties.extend(new_properties_btree);

        let updated_node = GraphDBNode {
            id: id.to_string(),
            labels: updated_labels,
            properties: updated_properties,
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
    ) -> Result<GraphDBEdge, Box<dyn std::error::Error + Send + Sync>> {
        let edge_id = id.unwrap_or_else(|| format!("edge_{}", uuid::Uuid::new_v4()));

        // Convert properties from HashMap<serde_json::Value> to BTreeMap<PropertyValue>
        let properties_btree: BTreeMap<String, PropertyValue> = properties
            .into_iter()
            .map(|(k, v)| (k, json_value_to_property_value(v)))
            .collect();

        let edge = GraphDBEdge {
            id: edge_id.clone(),
            from_node,
            to_node,
            label,
            properties: properties_btree,
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
    pub async fn get_edge(&self, id: &str) -> Result<Option<GraphDBEdge>, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.edge_key(id);

        match self.store.get(key.as_bytes()).await? {
            Some(data) => {
                let edge: GraphDBEdge = serde_json::from_slice(&data)?;
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
    ) -> Result<GraphDBEdge, Box<dyn std::error::Error + Send + Sync>> {
        // Get existing edge
        let existing = self.get_edge(id).await?
            .ok_or("Edge not found")?;

        // Update edge
        let updated_label = label.unwrap_or(existing.label);

        // Merge existing properties with new ones
        let mut updated_properties = existing.properties;
        let new_properties_btree: BTreeMap<String, PropertyValue> = properties
            .into_iter()
            .map(|(k, v)| (k, json_value_to_property_value(v)))
            .collect();
        updated_properties.extend(new_properties_btree);

        let updated_edge = GraphDBEdge {
            id: id.to_string(),
            from_node: existing.from_node,
            to_node: existing.to_node,
            label: updated_label,
            properties: updated_properties,
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

// Using kotoba-graphdb Node and Edge structures directly

/// Database statistics
#[derive(Debug, Clone, Serialize, Deserialize, async_graphql::SimpleObject)]
pub struct DatabaseStats {
    pub total_keys: i32,
    pub connected_clients: i32,
    pub uptime_seconds: i32,
}

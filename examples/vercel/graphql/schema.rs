//! GraphQL schema definitions for Kotoba GraphQL API

use async_graphql::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::redis_store::{RedisGraphStore, RedisNode, RedisEdge, DatabaseStats};

/// GraphQL schema for Kotoba Graph Database operations
pub type KotobaSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Query root for GraphQL operations
pub struct QueryRoot {
    pub store: Arc<RedisGraphStore>,
}

impl QueryRoot {
    pub fn new(store: Arc<RedisGraphStore>) -> Self {
        Self { store }
    }
}

/// GraphQL query implementations
#[Object]
impl QueryRoot {
    /// Health check query
    async fn health(&self) -> String {
        "Kotoba GraphQL API is healthy".to_string()
    }

    /// Get database statistics
    async fn stats(&self) -> Result<DatabaseStats> {
        match self.store.get_stats().await {
            Ok(stats) => Ok(stats),
            Err(e) => Err(Error::new(format!("Failed to get stats: {}", e))),
        }
    }

    /// Get node by ID
    async fn node(&self, ctx: &Context<'_>, id: String) -> Result<Option<Node>> {
        match self.store.get_node(&id).await {
            Ok(Some(node)) => Ok(Some(node.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::new(format!("Failed to get node: {}", e))),
        }
    }

    /// Get edge by ID
    async fn edge(&self, ctx: &Context<'_>, id: String) -> Result<Option<Edge>> {
        match self.store.get_edge(&id).await {
            Ok(Some(edge)) => Ok(Some(edge.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(Error::new(format!("Failed to get edge: {}", e))),
        }
    }

    /// Query nodes with filters
    async fn nodes(
        &self,
        ctx: &Context<'_>,
        filter: Option<NodeFilter>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Node>> {
        // For now, return empty vec - full filtering to be implemented
        // TODO: Implement proper node filtering
        Ok(vec![])
    }

    /// Query edges with filters
    async fn edges(
        &self,
        ctx: &Context<'_>,
        filter: Option<EdgeFilter>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<Edge>> {
        // For now, return empty vec - full filtering to be implemented
        // TODO: Implement proper edge filtering
        Ok(vec![])
    }
}

/// Mutation root for GraphQL operations
pub struct MutationRoot {
    pub store: Arc<RedisGraphStore>,
}

impl MutationRoot {
    pub fn new(store: Arc<RedisGraphStore>) -> Self {
        Self { store }
    }
}

/// GraphQL mutation implementations
#[Object]
impl MutationRoot {
    /// Create a new node
    async fn create_node(
        &self,
        ctx: &Context<'_>,
        input: CreateNodeInput,
    ) -> Result<Node> {
        // Convert GraphQL input to properties map
        let properties = input.properties.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        match self.store.create_node(input.id, input.labels, properties).await {
            Ok(node) => Ok(node.into()),
            Err(e) => Err(Error::new(format!("Failed to create node: {}", e))),
        }
    }

    /// Update an existing node
    async fn update_node(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateNodeInput,
    ) -> Result<Node> {
        // Convert GraphQL input to properties map
        let properties = input.properties.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        match self.store.update_node(&id, input.labels, properties).await {
            Ok(node) => Ok(node.into()),
            Err(e) => Err(Error::new(format!("Failed to update node: {}", e))),
        }
    }

    /// Delete a node
    async fn delete_node(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        match self.store.delete_node(&id).await {
            Ok(deleted) => Ok(deleted),
            Err(e) => Err(Error::new(format!("Failed to delete node: {}", e))),
        }
    }

    /// Create a new edge
    async fn create_edge(
        &self,
        ctx: &Context<'_>,
        input: CreateEdgeInput,
    ) -> Result<Edge> {
        // Convert GraphQL input to properties map
        let properties = input.properties.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        match self.store.create_edge(input.id, input.from_node, input.to_node, input.label, properties).await {
            Ok(edge) => Ok(edge.into()),
            Err(e) => Err(Error::new(format!("Failed to create edge: {}", e))),
        }
    }

    /// Update an existing edge
    async fn update_edge(
        &self,
        ctx: &Context<'_>,
        id: String,
        input: UpdateEdgeInput,
    ) -> Result<Edge> {
        // Convert GraphQL input to properties map
        let properties = input.properties.into_iter()
            .map(|(k, v)| (k, v.into()))
            .collect();

        match self.store.update_edge(&id, input.label, properties).await {
            Ok(edge) => Ok(edge.into()),
            Err(e) => Err(Error::new(format!("Failed to update edge: {}", e))),
        }
    }

    /// Delete an edge
    async fn delete_edge(&self, ctx: &Context<'_>, id: String) -> Result<bool> {
        match self.store.delete_edge(&id).await {
            Ok(deleted) => Ok(deleted),
            Err(e) => Err(Error::new(format!("Failed to delete edge: {}", e))),
        }
    }
}

// GraphQL Types

/// Database statistics
#[derive(SimpleObject)]
pub struct DatabaseStats {
    pub total_keys: i32,
    pub connected_clients: i32,
    pub uptime_seconds: i32,
}

/// Node in the graph database
#[derive(SimpleObject, Serialize, Deserialize)]
pub struct Node {
    pub id: String,
    pub labels: Vec<String>,
    pub properties: HashMap<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// Edge in the graph database
#[derive(SimpleObject, Serialize, Deserialize)]
pub struct Edge {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub label: String,
    pub properties: HashMap<String, Value>,
    pub created_at: String,
    pub updated_at: String,
}

/// GraphQL Value type for properties
#[derive(SimpleObject, Serialize, Deserialize)]
#[graphql(input_name = "ValueInput")]
pub struct Value {
    pub value_type: ValueType,
}

/// Different value types supported
#[derive(SimpleObject, Serialize, Deserialize)]
#[graphql(input_name = "ValueTypeInput")]
pub struct ValueType {
    pub string_value: Option<String>,
    pub int_value: Option<i64>,
    pub float_value: Option<f64>,
    pub bool_value: Option<bool>,
    pub array_value: Option<Vec<Value>>,
    pub object_value: Option<HashMap<String, Value>>,
}

// Input types for mutations

/// Input for creating a node
#[derive(InputObject)]
pub struct CreateNodeInput {
    pub id: Option<String>,
    pub labels: Vec<String>,
    pub properties: HashMap<String, Value>,
}

/// Input for updating a node
#[derive(InputObject)]
pub struct UpdateNodeInput {
    pub labels: Option<Vec<String>>,
    pub properties: HashMap<String, Value>,
}

/// Input for creating an edge
#[derive(InputObject)]
pub struct CreateEdgeInput {
    pub id: Option<String>,
    pub from_node: String,
    pub to_node: String,
    pub label: String,
    pub properties: HashMap<String, Value>,
}

/// Input for updating an edge
#[derive(InputObject)]
pub struct UpdateEdgeInput {
    pub label: Option<String>,
    pub properties: HashMap<String, Value>,
}

// Filter types for queries

/// Filter for node queries
#[derive(InputObject)]
pub struct NodeFilter {
    pub labels: Option<Vec<String>>,
    pub property_filters: Option<Vec<PropertyFilter>>,
}

/// Filter for edge queries
#[derive(InputObject)]
pub struct EdgeFilter {
    pub labels: Option<Vec<String>>,
    pub from_node: Option<String>,
    pub to_node: Option<String>,
    pub property_filters: Option<Vec<PropertyFilter>>,
}

/// Property filter for advanced queries
#[derive(InputObject)]
pub struct PropertyFilter {
    pub property: String,
    pub operator: FilterOperator,
    pub value: Value,
}

/// Filter operators
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum FilterOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    StartsWith,
    EndsWith,
}

/// Create the GraphQL schema
pub fn create_schema(store: Arc<RedisGraphStore>) -> KotobaSchema {
    Schema::build(QueryRoot::new(store.clone()), MutationRoot::new(store), EmptySubscription)
        .finish()
}

// Conversion implementations

impl From<RedisNode> for Node {
    fn from(node: RedisNode) -> Self {
        Node {
            id: node.id,
            labels: node.labels,
            properties: node.properties.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            created_at: node.created_at.to_rfc3339(),
            updated_at: node.updated_at.to_rfc3339(),
        }
    }
}

impl From<RedisEdge> for Edge {
    fn from(edge: RedisEdge) -> Self {
        Edge {
            id: edge.id,
            from_node: edge.from_node,
            to_node: edge.to_node,
            label: edge.label,
            properties: edge.properties.into_iter()
                .map(|(k, v)| (k, v.into()))
                .collect(),
            created_at: edge.created_at.to_rfc3339(),
            updated_at: edge.updated_at.to_rfc3339(),
        }
    }
}

impl From<serde_json::Value> for Value {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::String(s) => Value {
                value_type: ValueType {
                    string_value: Some(s),
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: None,
                }
            },
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Value {
                        value_type: ValueType {
                            string_value: None,
                            int_value: Some(i),
                            float_value: None,
                            bool_value: None,
                            array_value: None,
                            object_value: None,
                        }
                    }
                } else if let Some(f) = n.as_f64() {
                    Value {
                        value_type: ValueType {
                            string_value: None,
                            int_value: None,
                            float_value: Some(f),
                            bool_value: None,
                            array_value: None,
                            object_value: None,
                        }
                    }
                } else {
                    Value {
                        value_type: ValueType {
                            string_value: Some(n.to_string()),
                            int_value: None,
                            float_value: None,
                            bool_value: None,
                            array_value: None,
                            object_value: None,
                        }
                    }
                }
            },
            serde_json::Value::Bool(b) => Value {
                value_type: ValueType {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: Some(b),
                    array_value: None,
                    object_value: None,
                }
            },
            serde_json::Value::Array(arr) => Value {
                value_type: ValueType {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: Some(arr.into_iter().map(Into::into).collect()),
                    object_value: None,
                }
            },
            serde_json::Value::Object(obj) => Value {
                value_type: ValueType {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: Some(obj.into_iter().map(|(k, v)| (k, v.into())).collect()),
                }
            },
            serde_json::Value::Null => Value {
                value_type: ValueType {
                    string_value: None,
                    int_value: None,
                    float_value: None,
                    bool_value: None,
                    array_value: None,
                    object_value: None,
                }
            },
        }
    }
}

impl From<Value> for serde_json::Value {
    fn from(value: Value) -> Self {
        if let Some(s) = value.value_type.string_value {
            serde_json::Value::String(s)
        } else if let Some(i) = value.value_type.int_value {
            serde_json::Value::Number(i.into())
        } else if let Some(f) = value.value_type.float_value {
            serde_json::Value::Number(serde_json::Number::from_f64(f).unwrap_or(serde_json::Number::from(0)))
        } else if let Some(b) = value.value_type.bool_value {
            serde_json::Value::Bool(b)
        } else if let Some(arr) = value.value_type.array_value {
            serde_json::Value::Array(arr.into_iter().map(Into::into).collect())
        } else if let Some(obj) = value.value_type.object_value {
            serde_json::Value::Object(obj.into_iter().map(|(k, v)| (k, v.into())).collect())
        } else {
            serde_json::Value::Null
        }
    }
}

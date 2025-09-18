# Database API Reference

This document provides detailed reference for the main KotobaDB database API, including all methods, types, and usage patterns.

## Database Structure

```rust
use kotoba_db::{DB, DBConfig, Result};

/// Main database interface
#[derive(Clone)]
pub struct DB {
    // Internal fields (not part of public API)
}

impl DB {
    /// Open a new LSM-tree based database
    pub async fn open_lsm(path: impl AsRef<std::path::Path>) -> Result<Self> {
        // Implementation details...
    }

    /// Open an in-memory database
    pub async fn open_memory() -> Result<Self> {
        // Implementation details...
    }

    /// Open database with custom configuration
    pub async fn open_with_config(config: DBConfig) -> Result<Self> {
        // Implementation details...
    }
}
```

## Configuration

```rust
#[derive(Debug, Clone)]
pub struct DBConfig {
    /// Storage engine type
    pub engine: EngineType,

    /// Path for persistent storage (if applicable)
    pub path: Option<std::path::PathBuf>,

    /// Cache configuration
    pub cache: CacheConfig,

    /// Transaction settings
    pub transaction: TransactionConfig,

    /// Performance tuning
    pub performance: PerformanceConfig,

    /// Observability settings
    pub observability: ObservabilityConfig,
}

#[derive(Debug, Clone)]
pub enum EngineType {
    LSM,           // Persistent LSM-tree storage
    Memory,        // In-memory storage
    Cluster,       // Distributed cluster
}

#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize,                    // Maximum cache size in bytes
    pub ttl: Option<std::time::Duration>,   // Time-to-live for cache entries
    pub compression: bool,                  // Enable cache compression
}

#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub isolation_level: IsolationLevel,
    pub timeout: std::time::Duration,
    pub max_retries: usize,
}

#[derive(Debug, Clone)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}
```

## Node Operations

### Creating Nodes

```rust
impl DB {
    /// Create a new node with properties
    ///
    /// # Arguments
    /// * `node_type` - The type/schema of the node
    /// * `properties` - Key-value pairs for node properties
    ///
    /// # Returns
    /// The ID of the created node
    ///
    /// # Example
    /// ```rust
    /// use std::collections::HashMap;
    /// use kotoba_core::types::Value;
    ///
    /// let mut properties = HashMap::new();
    /// properties.insert("name".to_string(), Value::String("Alice".to_string()));
    /// properties.insert("age".to_string(), Value::Int(30));
    ///
    /// let node_id = db.create_node("User", properties).await?;
    /// ```
    pub async fn create_node(
        &self,
        node_type: &str,
        properties: impl IntoIterator<Item = (impl Into<String>, Value)>
    ) -> Result<NodeId> {
        // Implementation...
    }

    /// Create multiple nodes in a batch
    ///
    /// # Arguments
    /// * `nodes` - Iterator of (node_type, properties) pairs
    ///
    /// # Returns
    /// Vector of created node IDs in the same order
    ///
    /// # Example
    /// ```rust
    /// let nodes = vec![
    ///     ("User", vec![("name", Value::String("Alice".to_string()))]),
    ///     ("User", vec![("name", Value::String("Bob".to_string()))]),
    /// ];
    ///
    /// let node_ids = db.create_nodes_batch(nodes).await?;
    /// ```
    pub async fn create_nodes_batch(
        &self,
        nodes: impl IntoIterator<Item = (&str, impl IntoIterator<Item = (impl Into<String>, Value)>)>
    ) -> Result<Vec<NodeId>> {
        // Implementation...
    }
}
```

### Reading Nodes

```rust
impl DB {
    /// Get a node by ID
    ///
    /// # Arguments
    /// * `id` - The node ID to retrieve
    ///
    /// # Returns
    /// The node if found, or an error if not found
    ///
    /// # Example
    /// ```rust
    /// let node = db.get_node(node_id).await?;
    /// println!("Node type: {}", node.node_type);
    /// println!("Name: {:?}", node.properties.get("name"));
    /// ```
    pub async fn get_node(&self, id: NodeId) -> Result<Node> {
        // Implementation...
    }

    /// Get multiple nodes by IDs
    ///
    /// # Arguments
    /// * `ids` - Iterator of node IDs to retrieve
    ///
    /// # Returns
    /// Vector of nodes (None for not found IDs)
    ///
    /// # Example
    /// ```rust
    /// let nodes = db.get_nodes_batch(&[id1, id2, id3]).await?;
    /// for node in nodes.into_iter().flatten() {
    ///     println!("Found node: {}", node.node_type);
    /// }
    /// ```
    pub async fn get_nodes_batch(&self, ids: &[NodeId]) -> Result<Vec<Option<Node>>> {
        // Implementation...
    }

    /// Check if a node exists
    ///
    /// # Arguments
    /// * `id` - The node ID to check
    ///
    /// # Returns
    /// true if the node exists, false otherwise
    ///
    /// # Example
    /// ```rust
    /// if db.node_exists(node_id).await? {
    ///     println!("Node exists!");
    /// }
    /// ```
    pub async fn node_exists(&self, id: NodeId) -> Result<bool> {
        // Implementation...
    }
}
```

### Updating Nodes

```rust
impl DB {
    /// Update node properties
    ///
    /// # Arguments
    /// * `id` - The node ID to update
    /// * `properties` - Properties to update (will be merged with existing)
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// use std::collections::HashMap;
    ///
    /// let mut updates = HashMap::new();
    /// updates.insert("age".to_string(), Value::Int(31));
    /// updates.insert("last_login".to_string(), Value::DateTime(Utc::now()));
    ///
    /// db.update_node(node_id, updates).await?;
    /// ```
    pub async fn update_node(
        &self,
        id: NodeId,
        properties: impl IntoIterator<Item = (impl Into<String>, Value)>
    ) -> Result<()> {
        // Implementation...
    }

    /// Replace all node properties
    ///
    /// # Arguments
    /// * `id` - The node ID to replace
    /// * `properties` - New properties (will replace all existing)
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// let new_properties = vec![
    ///     ("name", Value::String("Alice Updated".to_string())),
    ///     ("status", Value::String("active".to_string())),
    /// ];
    ///
    /// db.replace_node_properties(node_id, new_properties).await?;
    /// ```
    pub async fn replace_node_properties(
        &self,
        id: NodeId,
        properties: impl IntoIterator<Item = (impl Into<String>, Value)>
    ) -> Result<()> {
        // Implementation...
    }

    /// Update specific node property
    ///
    /// # Arguments
    /// * `id` - The node ID
    /// * `property` - Property name
    /// * `value` - New value
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.update_node_property(node_id, "last_seen", Value::DateTime(Utc::now())).await?;
    /// ```
    pub async fn update_node_property(&self, id: NodeId, property: &str, value: Value) -> Result<()> {
        // Implementation...
    }
}
```

### Deleting Nodes

```rust
impl DB {
    /// Delete a node
    ///
    /// # Arguments
    /// * `id` - The node ID to delete
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Note
    /// This will also delete all relationships connected to the node.
    /// Use `delete_node_isolated` if you want to ensure no relationships exist.
    ///
    /// # Example
    /// ```rust
    /// db.delete_node(node_id).await?;
    /// ```
    pub async fn delete_node(&self, id: NodeId) -> Result<()> {
        // Implementation...
    }

    /// Delete a node only if it has no relationships
    ///
    /// # Arguments
    /// * `id` - The node ID to delete
    ///
    /// # Returns
    /// Success or error (will error if relationships exist)
    ///
    /// # Example
    /// ```rust
    /// match db.delete_node_isolated(node_id).await {
    ///     Ok(()) => println!("Node deleted"),
    ///     Err(e) if e.is_relationship_exists() => println!("Cannot delete: has relationships"),
    ///     Err(e) => return Err(e),
    /// }
    /// ```
    pub async fn delete_node_isolated(&self, id: NodeId) -> Result<()> {
        // Implementation...
    }

    /// Delete multiple nodes
    ///
    /// # Arguments
    /// * `ids` - Node IDs to delete
    ///
    /// # Returns
    /// Vector of results (one per ID)
    ///
    /// # Example
    /// ```rust
    /// let results = db.delete_nodes_batch(&[id1, id2, id3]).await?;
    /// for result in results {
    ///     match result {
    ///         Ok(()) => println!("Deleted successfully"),
    ///         Err(e) => println!("Failed to delete: {}", e),
    ///     }
    /// }
    /// ```
    pub async fn delete_nodes_batch(&self, ids: &[NodeId]) -> Result<Vec<Result<()>>> {
        // Implementation...
    }
}
```

## Edge Operations

### Creating Edges

```rust
impl DB {
    /// Create a relationship between two nodes
    ///
    /// # Arguments
    /// * `from_node` - Source node ID
    /// * `to_node` - Target node ID
    /// * `edge_type` - Type of relationship
    /// * `properties` - Optional properties for the relationship
    ///
    /// # Returns
    /// The ID of the created edge
    ///
    /// # Example
    /// ```rust
    /// let edge_id = db.create_edge(user_id, post_id, "author", vec![]).await?;
    ///
    /// // With properties
    /// let edge_id = db.create_edge(
    ///     user1_id,
    ///     user2_id,
    ///     "follows",
    ///     vec![("since", Value::DateTime(Utc::now()))]
    /// ).await?;
    /// ```
    pub async fn create_edge(
        &self,
        from_node: NodeId,
        to_node: NodeId,
        edge_type: &str,
        properties: impl IntoIterator<Item = (impl Into<String>, Value)>
    ) -> Result<EdgeId> {
        // Implementation...
    }

    /// Create multiple edges in a batch
    ///
    /// # Arguments
    /// * `edges` - Iterator of (from, to, type, properties) tuples
    ///
    /// # Returns
    /// Vector of created edge IDs
    ///
    /// # Example
    /// ```rust
    /// let edges = vec![
    ///     (user1, post1, "author", vec![]),
    ///     (user2, post2, "author", vec![]),
    ///     (user1, user2, "follows", vec![("mutual", Value::Bool(true))]),
    /// ];
    ///
    /// let edge_ids = db.create_edges_batch(edges).await?;
    /// ```
    pub async fn create_edges_batch(
        &self,
        edges: impl IntoIterator<Item = (NodeId, NodeId, &str, impl IntoIterator<Item = (impl Into<String>, Value)>)>
    ) -> Result<Vec<EdgeId>> {
        // Implementation...
    }
}
```

### Reading Edges

```rust
impl DB {
    /// Get an edge by ID
    ///
    /// # Arguments
    /// * `id` - The edge ID to retrieve
    ///
    /// # Returns
    /// The edge if found
    ///
    /// # Example
    /// ```rust
    /// let edge = db.get_edge(edge_id).await?;
    /// println!("Edge type: {}", edge.edge_type);
    /// println!("From: {}, To: {}", edge.from_node, edge.to_node);
    /// ```
    pub async fn get_edge(&self, id: EdgeId) -> Result<Edge> {
        // Implementation...
    }

    /// Get all edges connected to a node
    ///
    /// # Arguments
    /// * `node_id` - The node ID
    /// * `direction` - Direction filter (optional)
    ///
    /// # Returns
    /// Vector of edges connected to the node
    ///
    /// # Example
    /// ```rust
    /// // Get all relationships for a user
    /// let edges = db.get_node_edges(user_id, None).await?;
    ///
    /// // Get only outgoing relationships
    /// let outgoing = db.get_node_edges(user_id, Some(EdgeDirection::Outgoing)).await?;
    ///
    /// // Get only incoming relationships
    /// let incoming = db.get_node_edges(user_id, Some(EdgeDirection::Incoming)).await?;
    /// ```
    pub async fn get_node_edges(&self, node_id: NodeId, direction: Option<EdgeDirection>) -> Result<Vec<Edge>> {
        // Implementation...
    }

    /// Get edges between two specific nodes
    ///
    /// # Arguments
    /// * `from_node` - Source node ID
    /// * `to_node` - Target node ID
    /// * `edge_type` - Optional edge type filter
    ///
    /// # Returns
    /// Vector of edges between the nodes
    ///
    /// # Example
    /// ```rust
    /// // Get all relationships between two users
    /// let edges = db.get_edges_between(user1_id, user2_id, None).await?;
    ///
    /// // Get only "follows" relationships
    /// let follows = db.get_edges_between(user1_id, user2_id, Some("follows")).await?;
    /// ```
    pub async fn get_edges_between(&self, from_node: NodeId, to_node: NodeId, edge_type: Option<&str>) -> Result<Vec<Edge>> {
        // Implementation...
    }
}
```

### Updating and Deleting Edges

```rust
impl DB {
    /// Update edge properties
    ///
    /// # Arguments
    /// * `id` - Edge ID to update
    /// * `properties` - Properties to update
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// let updates = vec![
    ///     ("strength", Value::String("strong".to_string())),
    ///     ("updated_at", Value::DateTime(Utc::now())),
    /// ];
    ///
    /// db.update_edge(edge_id, updates).await?;
    /// ```
    pub async fn update_edge(
        &self,
        id: EdgeId,
        properties: impl IntoIterator<Item = (impl Into<String>, Value)>
    ) -> Result<()> {
        // Implementation...
    }

    /// Delete an edge
    ///
    /// # Arguments
    /// * `id` - Edge ID to delete
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.delete_edge(edge_id).await?;
    /// ```
    pub async fn delete_edge(&self, id: EdgeId) -> Result<()> {
        // Implementation...
    }

    /// Delete multiple edges
    ///
    /// # Arguments
    /// * `ids` - Edge IDs to delete
    ///
    /// # Returns
    /// Vector of results
    ///
    /// # Example
    /// ```rust
    /// let results = db.delete_edges_batch(&[edge1, edge2, edge3]).await?;
    /// ```
    pub async fn delete_edges_batch(&self, ids: &[EdgeId]) -> Result<Vec<Result<()>>> {
        // Implementation...
    }
}
```

## Query Operations

```rust
impl DB {
    /// Execute a query
    ///
    /// # Arguments
    /// * `query` - Query string in KotobaDB Query Language
    /// * `parameters` - Optional query parameters
    ///
    /// # Returns
    /// Query result set
    ///
    /// # Example
    /// ```rust
    /// // Simple query
    /// let result = db.query("MATCH (u:User {active: true}) RETURN u.name").await?;
    ///
    /// // Query with parameters
    /// let result = db.query_with_params(
    ///     "MATCH (u:User {name: $name}) RETURN u",
    ///     &[("name", Value::String("Alice".to_string()))]
    /// ).await?;
    /// ```
    pub async fn query(&self, query: &str) -> Result<QueryResult> {
        self.query_with_params(query, &[]).await
    }

    pub async fn query_with_params(
        &self,
        query: &str,
        parameters: impl IntoIterator<Item = (&str, Value)>
    ) -> Result<QueryResult> {
        // Implementation...
    }

    /// Execute a query and stream results
    ///
    /// # Arguments
    /// * `query` - Query string
    /// * `parameters` - Query parameters
    ///
    /// # Returns
    /// Stream of result rows
    ///
    /// # Example
    /// ```rust
    /// use futures::stream::StreamExt;
    ///
    /// let mut stream = db.query_stream("MATCH (u:User) RETURN u.name", &[]).await?;
    /// while let Some(row) = stream.next().await {
    ///     let row = row?;
    ///     println!("User: {:?}", row.get("u.name"));
    /// }
    /// ```
    pub async fn query_stream(
        &self,
        query: &str,
        parameters: impl IntoIterator<Item = (&str, Value)>
    ) -> Result<impl Stream<Item = Result<QueryRow>>> {
        // Implementation...
    }

    /// Explain query execution plan
    ///
    /// # Arguments
    /// * `query` - Query to explain
    ///
    /// # Returns
    /// Query execution plan
    ///
    /// # Example
    /// ```rust
    /// let plan = db.explain_query("MATCH (u:User {name: 'Alice'}) RETURN u").await?;
    /// println!("Execution plan:\n{}", plan);
    /// ```
    pub async fn explain_query(&self, query: &str) -> Result<String> {
        // Implementation...
    }

    /// Profile query execution
    ///
    /// # Arguments
    /// * `query` - Query to profile
    /// * `iterations` - Number of times to run the query
    ///
    /// # Returns
    /// Detailed profiling information
    ///
    /// # Example
    /// ```rust
    /// let profile = db.profile_query("MATCH (u:User)-[:FOLLOWS]->(f:User) RETURN count(*)", 10).await?;
    /// println!("Average execution time: {:?}", profile.avg_duration);
    /// println!("Total rows processed: {}", profile.total_rows);
    /// ```
    pub async fn profile_query(&self, query: &str, iterations: usize) -> Result<QueryProfile> {
        // Implementation...
    }
}
```

## Transaction Operations

```rust
impl DB {
    /// Execute operations in a transaction
    ///
    /// # Arguments
    /// * `isolation_level` - Transaction isolation level
    /// * `operations` - Operations to execute atomically
    ///
    /// # Returns
    /// Transaction result
    ///
    /// # Example
    /// ```rust
    /// use kotoba_db::TransactionOperation;
    ///
    /// let operations = vec![
    ///     TransactionOperation::CreateNode {
    ///         node_type: "User".to_string(),
    ///         properties: vec![("name", Value::String("Alice".to_string()))],
    ///     },
    ///     TransactionOperation::CreateNode {
    ///         node_type: "Post".to_string(),
    ///         properties: vec![("title", Value::String("Hello".to_string()))],
    ///     },
    /// ];
    ///
    /// let result = db.execute_transaction(IsolationLevel::Serializable, operations).await?;
    /// ```
    pub async fn execute_transaction(
        &self,
        isolation_level: IsolationLevel,
        operations: Vec<TransactionOperation>
    ) -> Result<TransactionResult> {
        // Implementation...
    }

    /// Begin an interactive transaction
    ///
    /// # Arguments
    /// * `isolation_level` - Transaction isolation level
    ///
    /// # Returns
    /// Transaction handle
    ///
    /// # Example
    /// ```rust
    /// let tx = db.begin_transaction(IsolationLevel::ReadCommitted).await?;
    ///
    /// // Execute operations
    /// let node_id = tx.create_node("User", vec![("name", "Alice")]).await?;
    /// let post_id = tx.create_node("Post", vec![("title", "Hello")]).await?;
    /// tx.create_edge(node_id, post_id, "author", vec![]).await?;
    ///
    /// // Commit or rollback
    /// tx.commit().await?;
    /// ```
    pub async fn begin_transaction(&self, isolation_level: IsolationLevel) -> Result<Transaction> {
        // Implementation...
    }
}
```

## Schema Operations

```rust
impl DB {
    /// Define a node schema
    ///
    /// # Arguments
    /// * `schema` - Node schema definition
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// use kotoba_core::schema::{NodeSchema, PropertySchema, ValueType};
    ///
    /// let user_schema = NodeSchema {
    ///     name: "User".to_string(),
    ///     properties: vec![
    ///         ("name".to_string(), PropertySchema {
    ///             name: "name".to_string(),
    ///             data_type: ValueType::String,
    ///             constraints: vec![PropertyConstraint::Required],
    ///             description: Some("User's full name".to_string()),
    ///         }),
    ///         ("email".to_string(), PropertySchema {
    ///             name: "email".to_string(),
    ///             data_type: ValueType::String,
    ///             constraints: vec![PropertyConstraint::Unique],
    ///             description: Some("User's email address".to_string()),
    ///         }),
    ///     ].into_iter().collect(),
    ///     required_properties: vec!["name".to_string(), "email".to_string()],
    /// };
    ///
    /// db.define_node_schema(user_schema).await?;
    /// ```
    pub async fn define_node_schema(&self, schema: NodeSchema) -> Result<()> {
        // Implementation...
    }

    /// Get node schema by type
    ///
    /// # Arguments
    /// * `node_type` - Node type name
    ///
    /// # Returns
    /// Schema if defined
    ///
    /// # Example
    /// ```rust
    /// let schema = db.get_node_schema("User").await?;
    /// println!("Required properties: {:?}", schema.required_properties);
    /// ```
    pub async fn get_node_schema(&self, node_type: &str) -> Result<NodeSchema> {
        // Implementation...
    }

    /// Validate data against schema
    ///
    /// # Arguments
    /// * `node_type` - Node type to validate against
    /// * `properties` - Properties to validate
    ///
    /// # Returns
    /// Validation result
    ///
    /// # Example
    /// ```rust
    /// let properties = vec![
    ///     ("name", Value::String("Alice".to_string())),
    ///     ("email", Value::String("alice@example.com".to_string())),
    /// ];
    ///
    /// let validation = db.validate_node_data("User", &properties).await?;
    /// if validation.is_valid {
    ///     println!("Data is valid");
    /// } else {
    ///     println!("Validation errors: {:?}", validation.errors);
    /// }
    /// ```
    pub async fn validate_node_data(&self, node_type: &str, properties: &[(impl AsRef<str>, Value)]) -> Result<ValidationResult> {
        // Implementation...
    }
}
```

## Indexing Operations

```rust
impl DB {
    /// Create an index
    ///
    /// # Arguments
    /// * `index_def` - Index definition
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// use kotoba_core::index::{IndexDefinition, IndexTarget, IndexType};
    ///
    /// let index = IndexDefinition {
    ///     name: "user_email_idx".to_string(),
    ///     target: IndexTarget::NodeProperty {
    ///         node_type: "User".to_string(),
    ///         property: "email".to_string(),
    ///     },
    ///     index_type: IndexType::BTree,
    ///     unique: true,
    /// };
    ///
    /// db.create_index(index).await?;
    /// ```
    pub async fn create_index(&self, index_def: IndexDefinition) -> Result<()> {
        // Implementation...
    }

    /// Drop an index
    ///
    /// # Arguments
    /// * `index_name` - Name of index to drop
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.drop_index("user_email_idx").await?;
    /// ```
    pub async fn drop_index(&self, index_name: &str) -> Result<()> {
        // Implementation...
    }

    /// List all indexes
    ///
    /// # Returns
    /// Vector of index definitions
    ///
    /// # Example
    /// ```rust
    /// let indexes = db.list_indexes().await?;
    /// for index in indexes {
    ///     println!("Index: {} on {:?}", index.name, index.target);
    /// }
    /// ```
    pub async fn list_indexes(&self) -> Result<Vec<IndexDefinition>> {
        // Implementation...
    }

    /// Get index statistics
    ///
    /// # Arguments
    /// * `index_name` - Name of index
    ///
    /// # Returns
    /// Index statistics
    ///
    /// # Example
    /// ```rust
    /// let stats = db.get_index_stats("user_email_idx").await?;
    /// println!("Index size: {} bytes", stats.size_bytes);
    /// println!("Entry count: {}", stats.entry_count);
    /// ```
    pub async fn get_index_stats(&self, index_name: &str) -> Result<IndexStats> {
        // Implementation...
    }
}
```

## Administrative Operations

```rust
impl DB {
    /// Get database statistics
    ///
    /// # Returns
    /// Database statistics
    ///
    /// # Example
    /// ```rust
    /// let stats = db.get_stats().await?;
    /// println!("Total nodes: {}", stats.node_count);
    /// println!("Total edges: {}", stats.edge_count);
    /// println!("Database size: {} bytes", stats.size_bytes);
    /// ```
    pub async fn get_stats(&self) -> Result<DatabaseStats> {
        // Implementation...
    }

    /// Compact storage (for LSM engine)
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.compact_storage().await?;
    /// println!("Storage compaction completed");
    /// ```
    pub async fn compact_storage(&self) -> Result<()> {
        // Implementation...
    }

    /// Create a backup
    ///
    /// # Arguments
    /// * `backup_path` - Path to save backup
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.create_backup("backup.tar.gz").await?;
    /// ```
    pub async fn create_backup(&self, backup_path: impl AsRef<std::path::Path>) -> Result<()> {
        // Implementation...
    }

    /// Restore from backup
    ///
    /// # Arguments
    /// * `backup_path` - Path to backup file
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Example
    /// ```rust
    /// db.restore_from_backup("backup.tar.gz").await?;
    /// ```
    pub async fn restore_from_backup(&self, backup_path: impl AsRef<std::path::Path>) -> Result<()> {
        // Implementation...
    }

    /// Close the database
    ///
    /// # Returns
    /// Success or error
    ///
    /// # Note
    /// After closing, the database instance cannot be used anymore.
    ///
    /// # Example
    /// ```rust
    /// db.close().await?;
    /// ```
    pub async fn close(self) -> Result<()> {
        // Implementation...
    }
}
```

This comprehensive API reference covers all the major operations available in the KotobaDB main interface. The API is designed to be intuitive for developers familiar with databases while providing powerful graph database capabilities.

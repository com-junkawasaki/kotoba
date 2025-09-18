# KotobaDB API Reference

## Overview

KotobaDB is a graph-native, version-controlled, embedded database built on top of a Merkle DAG architecture. This API reference provides comprehensive documentation for all KotobaDB components and their usage.

## Architecture

KotobaDB follows a layered architecture:

- **Core Layer**: Fundamental data structures and algorithms
- **Storage Layer**: Persistent storage engines (LSM-Tree, Memory)
- **Graph Layer**: Graph operations and traversal
- **Execution Layer**: Query execution and optimization
- **Network Layer**: Distributed operations and clustering

## Quick Start

```rust
use kotoba_db::{DB, DBConfig};

// Create a new database instance
let config = DBConfig::default();
let db = DB::open_lsm("./my_database").await?;

// Store some data
let node_id = db.create_node("User", &[("name", "Alice"), ("email", "alice@example.com")]).await?;
let post_id = db.create_node("Post", &[("title", "Hello World"), ("content", "My first post")]).await?;

// Create a relationship
db.create_edge(node_id, post_id, "author", &[]).await?;

// Query data
let user = db.get_node(node_id).await?;
println!("User: {:?}", user);
```

## Core Components

### Database Engine

#### KotobaDB

The main database interface providing high-level operations.

```rust
pub struct DB {
    // Main database operations
    pub async fn open_lsm(path: &Path) -> Result<Self>
    pub async fn open_memory() -> Result<Self>

    // Node operations
    pub async fn create_node(&self, node_type: &str, properties: &[(&str, &str)]) -> Result<NodeId>
    pub async fn get_node(&self, id: NodeId) -> Result<Node>
    pub async fn update_node(&self, id: NodeId, properties: &[(&str, &str)]) -> Result<()>
    pub async fn delete_node(&self, id: NodeId) -> Result<()>

    // Edge operations
    pub async fn create_edge(&self, from: NodeId, to: NodeId, edge_type: &str, properties: &[(&str, &str)]) -> Result<EdgeId>
    pub async fn get_edge(&self, id: EdgeId) -> Result<Edge>
    pub async fn delete_edge(&self, id: EdgeId) -> Result<()>

    // Query operations
    pub async fn query(&self, query: &str) -> Result<QueryResult>
    pub async fn execute_query(&self, query: Query) -> Result<QueryResult>
}
```

#### Storage Engines

KotobaDB supports multiple storage engines through a pluggable architecture.

##### LSM-Tree Engine

```rust
// Persistent storage using Log-Structured Merge Trees
let db = DB::open_lsm("./data").await?;
```

##### Memory Engine

```rust
// In-memory storage for testing and development
let db = DB::open_memory().await?;
```

### Data Types

#### NodeId

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);
```

#### EdgeId

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(u64);
```

#### Node

```rust
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub node_type: String,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### Edge

```rust
#[derive(Debug, Clone)]
pub struct Edge {
    pub id: EdgeId,
    pub edge_type: String,
    pub from_node: NodeId,
    pub to_node: NodeId,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
}
```

#### Value

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
}
```

## Query Language

KotobaDB supports a graph-native query language inspired by Cypher and GraphQL.

### Basic Queries

```cypher
// Find all users
MATCH (u:User) RETURN u

// Find users with specific properties
MATCH (u:User {name: "Alice"}) RETURN u

// Find relationships
MATCH (u:User)-[:FOLLOWS]->(u2:User) RETURN u, u2

// Complex patterns
MATCH (u:User)-[:POSTED]->(p:Post)<-[:COMMENTED]-(c:Comment)
WHERE p.created_at > "2024-01-01"
RETURN u.name, p.title, count(c) as comment_count
```

### Aggregation and Functions

```cypher
// Count and group
MATCH (u:User)-[:POSTED]->(p:Post)
RETURN u.name, count(p) as post_count
ORDER BY post_count DESC

// Mathematical functions
MATCH (p:Post)
RETURN avg(p.views) as avg_views, max(p.likes) as max_likes

// Date functions
MATCH (p:Post)
WHERE date(p.created_at) >= date("2024-01-01")
RETURN p
```

## Cluster API

For distributed deployments, KotobaDB provides clustering capabilities.

```rust
use kotoba_db_cluster::KotobaCluster;

// Create a cluster instance
let cluster = KotobaCluster::new(config).await?;

// Distributed operations
let node_id = cluster.create_node_distributed("User", properties).await?;
let result = cluster.query_distributed(query).await?;
```

## Monitoring and Metrics

KotobaDB includes comprehensive monitoring capabilities.

```rust
use kotoba_monitoring::MetricsCollector;

// Collect metrics
let metrics = collector.collect_metrics().await?;

// Health checks
let health = health_checker.check_health().await?;
assert!(health.is_healthy);
```

## Backup and Restore

```rust
use kotoba_backup::BackupManager;

// Create backup
let backup_id = backup_manager.create_backup().await?;

// Restore from backup
backup_manager.restore_from_backup(backup_id).await?;
```

## Configuration

```rust
use kotoba_config::ConfigManager;

// Load configuration
let config = config_manager.load_config().await?;

// Dynamic reconfiguration
config_manager.update_config("max_connections", "1000").await?;
```

## Error Handling

KotobaDB uses a comprehensive error type system:

```rust
use kotoba_errors::{KotobaError, Result};

#[derive(thiserror::Error, Debug)]
pub enum KotobaError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Configuration error: {0}")]
    Config(String),
}
```

## Performance Optimization

### Indexing

```rust
// Create indexes for better performance
db.create_index("User", "email").await?;
db.create_index("Post", "created_at").await?;
```

### Query Optimization

```rust
// Use EXPLAIN to understand query execution
let plan = db.explain_query("MATCH (u:User {name: 'Alice'}) RETURN u").await?;
println!("Execution plan: {:?}", plan);
```

## Advanced Features

### Version Control

KotobaDB provides Git-like versioning for data:

```rust
// Create a branch
db.create_branch("feature/new-feature").await?;

// Commit changes
let commit_id = db.commit("Add new user management").await?;

// Merge branches
db.merge("feature/new-feature", "main").await?;
```

### Time Travel

```rust
// Query historical data
let historical_data = db.query_at_time(commit_id, "MATCH (u:User) RETURN u").await?;
```

### Content Addressing

All data in KotobaDB is content-addressed using BLAKE3 hashes:

```rust
use kotoba_cid::Cid;

// Generate CID for data
let cid = Cid::from_data(&data);
assert_eq!(cid.to_string().len(), 64); // 64-character base64url string
```

## Migration Guide

### From Other Databases

#### From PostgreSQL

```rust
// PostgreSQL-style operations in KotobaDB
// Instead of: INSERT INTO users (name, email) VALUES ('Alice', 'alice@example.com')
db.create_node("User", &[("name", "Alice"), ("email", "alice@example.com")]).await?;

// Instead of: SELECT * FROM users WHERE name = 'Alice'
let result = db.query("MATCH (u:User {name: 'Alice'}) RETURN u").await?;
```

#### From MongoDB

```rust
// MongoDB-style operations in KotobaDB
// Instead of: db.users.insertOne({name: "Alice", email: "alice@example.com"})
db.create_node("User", &[("name", "Alice"), ("email", "alice@example.com")]).await?;

// Instead of: db.users.find({name: "Alice"})
let result = db.query("MATCH (u:User {name: 'Alice'}) RETURN u").await?;
```

#### From Neo4j

```cypher
// Neo4j Cypher queries work directly in KotobaDB
// Neo4j: MATCH (u:User)-[:FOLLOWS]->(u2:User) RETURN u, u2
// KotobaDB: MATCH (u:User)-[:FOLLOWS]->(u2:User) RETURN u, u2  (same syntax!)
```

## Best Practices

### Schema Design

1. **Use descriptive node and edge types**
2. **Keep property names consistent**
3. **Use appropriate data types**
4. **Plan for future scalability**

### Performance

1. **Create indexes on frequently queried properties**
2. **Use specific queries rather than broad scans**
3. **Batch operations when possible**
4. **Monitor query performance regularly**

### Operations

1. **Implement regular backups**
2. **Monitor system metrics**
3. **Plan for scaling requirements**
4. **Keep dependencies updated**

## Troubleshooting

### Common Issues

#### Slow Queries

```rust
// Check if indexes are being used
let plan = db.explain_query(your_slow_query).await?;
if !plan.uses_index() {
    // Add appropriate indexes
    db.create_index("NodeType", "property_name").await?;
}
```

#### Memory Issues

```rust
// Check memory usage
let metrics = collector.get_memory_metrics().await?;
if metrics.heap_used_mb > 1000 {
    // Consider memory optimization
    // - Reduce cache sizes
    // - Implement memory pooling
    // - Check for memory leaks
}
```

#### Connection Issues

```rust
// In cluster mode, check node connectivity
let cluster_health = cluster.check_connectivity().await?;
for (node_id, status) in cluster_health.node_status {
    if !status.connected {
        println!("Node {} is disconnected", node_id);
        // Implement reconnection logic
    }
}
```

## API Stability

KotobaDB follows semantic versioning:

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Breaking Changes Policy

Breaking changes are only introduced in major versions and include:
- API signature changes
- Data format changes
- Behavioral changes

### Deprecation Policy

Deprecated APIs are marked with `#[deprecated]` and supported for at least:
- 2 major versions for stable APIs
- 1 major version for experimental APIs

## Contributing to Documentation

We welcome contributions to improve the documentation!

1. **Report Issues**: Found unclear documentation? [Open an issue](https://github.com/your-org/kotoba/issues)
2. **Suggest Improvements**: Have ideas for better examples? [Create a PR](https://github.com/your-org/kotoba/pulls)
3. **Add Examples**: Share your use cases and examples

## Support

- **Documentation**: [docs.kotoba.dev](https://docs.kotoba.dev)
- **GitHub Issues**: [github.com/your-org/kotoba/issues](https://github.com/your-org/kotoba/issues)
- **Discord**: [discord.gg/kotoba](https://discord.gg/kotoba)
- **Email**: support@kotoba.dev

---

*This documentation is automatically generated from the KotobaDB codebase. Last updated: $(date)*

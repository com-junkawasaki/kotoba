# Core Data Types

This document describes the fundamental data types used throughout KotobaDB.

## Content Identifier (CID)

KotobaDB uses content addressing for all data, providing cryptographic integrity and deduplication.

```rust
use kotoba_cid::Cid;

/// Content Identifier - 32-byte BLAKE3 hash
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cid([u8; 32]);

impl Cid {
    /// Create CID from data
    pub fn from_data(data: &[u8]) -> Self {
        let hash = blake3::hash(data);
        Cid(hash.into())
    }

    /// Create CID from hex string
    pub fn from_hex(hex: &str) -> Result<Self, CidError>

    /// Convert to hex string
    pub fn to_hex(&self) -> String

    /// Convert to base64url string
    pub fn to_string(&self) -> String

    /// Verify data against CID
    pub fn verify(&self, data: &[u8]) -> bool {
        self == &Self::from_data(data)
    }
}
```

### Usage Examples

```rust
// Create CID from data
let data = b"Hello, KotobaDB!";
let cid = Cid::from_data(data);

// Serialize and deserialize
let cid_string = cid.to_string();
let cid_recovered = Cid::from_str(&cid_string)?;

// Verify integrity
assert!(cid.verify(data));
```

## Values and Properties

KotobaDB supports rich data types for node and edge properties.

```rust
use kotoba_core::types::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(HashMap<String, Value>),
    Link(Cid),  // Reference to another object
    DateTime(chrono::DateTime<chrono::Utc>),
}
```

### Property Types

```rust
#[derive(Debug, Clone)]
pub struct Property {
    pub name: String,
    pub value: Value,
    pub indexed: bool,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct PropertySchema {
    pub name: String,
    pub data_type: ValueType,
    pub constraints: Vec<PropertyConstraint>,
    pub description: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ValueType {
    Null,
    Bool,
    Int,
    Float,
    String,
    Bytes,
    Array(Box<ValueType>),
    Object,
    Link,
    DateTime,
}
```

### Property Constraints

```rust
#[derive(Debug, Clone)]
pub enum PropertyConstraint {
    Required,
    Unique,
    MinLength(usize),
    MaxLength(usize),
    MinValue(Value),
    MaxValue(Value),
    Pattern(regex::Regex),
    Enum(Vec<Value>),
    Custom(String, Value), // Custom constraint with parameters
}
```

## Nodes and Edges

The fundamental graph structures in KotobaDB.

### Node Structure

```rust
#[derive(Debug, Clone)]
pub struct Node {
    pub id: NodeId,
    pub cid: Cid,                    // Content identifier
    pub node_type: String,           // Node type/schema
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub version: u64,               // Version number
    pub predecessors: Vec<Cid>,     // Previous versions
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        NodeId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}
```

### Edge Structure

```rust
#[derive(Debug, Clone)]
pub struct Edge {
    pub id: EdgeId,
    pub cid: Cid,                    // Content identifier
    pub edge_type: String,           // Edge type/schema
    pub from_node: NodeId,
    pub to_node: NodeId,
    pub properties: HashMap<String, Value>,
    pub created_at: DateTime<Utc>,
    pub direction: EdgeDirection,    // Directed or undirected
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EdgeId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeDirection {
    Outgoing,    // from → to
    Incoming,    // from ← to
    Bidirectional, // from ↔ to
}
```

## Blocks and Content Addressing

KotobaDB stores all data as content-addressed blocks.

```rust
#[derive(Debug, Clone)]
pub enum Block {
    NodeBlock(NodeBlock),
    EdgeBlock(EdgeBlock),
    IndexBlock(IndexBlock),
    MetadataBlock(MetadataBlock),
}

#[derive(Debug, Clone)]
pub struct NodeBlock {
    pub node_type: String,
    pub properties: HashMap<String, Value>,
    pub links: Vec<Cid>,           // References to other blocks
}

#[derive(Debug, Clone)]
pub struct EdgeBlock {
    pub edge_type: String,
    pub from_cid: Cid,
    pub to_cid: Cid,
    pub properties: HashMap<String, Value>,
}
```

### Block Serialization

```rust
impl Block {
    /// Calculate CID for the block
    pub fn cid(&self) -> Cid {
        let data = self.to_bytes();
        Cid::from_data(&data)
    }

    /// Serialize block to CBOR bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        ciborium::to_vec(self).unwrap()
    }

    /// Deserialize block from CBOR bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, BlockError> {
        ciborium::from_reader(bytes)
    }
}
```

## Schema Definitions

Schemas define the structure and constraints for nodes and edges.

```rust
#[derive(Debug, Clone)]
pub struct Schema {
    pub id: SchemaId,
    pub name: String,
    pub version: semver::Version,
    pub node_schemas: HashMap<String, NodeSchema>,
    pub edge_schemas: HashMap<String, EdgeSchema>,
    pub indexes: Vec<IndexDefinition>,
}

#[derive(Debug, Clone)]
pub struct NodeSchema {
    pub name: String,
    pub properties: HashMap<String, PropertySchema>,
    pub required_properties: Vec<String>,
    pub unique_constraints: Vec<Vec<String>>, // Compound unique constraints
    pub indexes: Vec<String>,                 // Properties to index
}

#[derive(Debug, Clone)]
pub struct EdgeSchema {
    pub name: String,
    pub from_node_types: Vec<String>,         // Allowed source node types
    pub to_node_types: Vec<String>,           // Allowed target node types
    pub properties: HashMap<String, PropertySchema>,
    pub cardinality: Cardinality,
}

#[derive(Debug, Clone)]
pub enum Cardinality {
    OneToOne,
    OneToMany,
    ManyToOne,
    ManyToMany,
}
```

## Queries and Operations

Query structures and operation types.

```rust
#[derive(Debug, Clone)]
pub enum Query {
    Match(MatchQuery),
    Create(CreateQuery),
    Update(UpdateQuery),
    Delete(DeleteQuery),
    Merge(MergeQuery),
}

#[derive(Debug, Clone)]
pub struct MatchQuery {
    pub pattern: GraphPattern,
    pub conditions: Vec<Condition>,
    pub return_items: Vec<ReturnItem>,
    pub order_by: Vec<OrderClause>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct GraphPattern {
    pub nodes: Vec<NodePattern>,
    pub edges: Vec<EdgePattern>,
}

#[derive(Debug, Clone)]
pub struct NodePattern {
    pub variable: String,
    pub node_type: Option<String>,
    pub properties: HashMap<String, PropertyCondition>,
}

#[derive(Debug, Clone)]
pub struct EdgePattern {
    pub variable: String,
    pub edge_type: Option<String>,
    pub from_variable: String,
    pub to_variable: String,
    pub direction: EdgeDirection,
    pub properties: HashMap<String, PropertyCondition>,
}
```

## Transactions

Transaction support for atomic operations.

```rust
#[derive(Debug)]
pub struct Transaction {
    pub id: TransactionId,
    pub operations: Vec<Operation>,
    pub status: TransactionStatus,
    pub isolation_level: IsolationLevel,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum Operation {
    CreateNode { node_type: String, properties: HashMap<String, Value> },
    UpdateNode { node_id: NodeId, properties: HashMap<String, Value> },
    DeleteNode { node_id: NodeId },
    CreateEdge { edge_type: String, from_node: NodeId, to_node: NodeId, properties: HashMap<String, Value> },
    DeleteEdge { edge_id: EdgeId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionStatus {
    Active,
    Committed,
    RolledBack,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}
```

## Indexes and Optimization

Indexing structures for query optimization.

```rust
#[derive(Debug, Clone)]
pub struct IndexDefinition {
    pub name: String,
    pub target: IndexTarget,
    pub index_type: IndexType,
    pub unique: bool,
}

#[derive(Debug, Clone)]
pub enum IndexTarget {
    NodeProperty { node_type: String, property: String },
    EdgeProperty { edge_type: String, property: String },
    FullText { node_type: String, properties: Vec<String> },
    Spatial { node_type: String, property: String },
}

#[derive(Debug, Clone)]
pub enum IndexType {
    BTree,
    Hash,
    FullText,
    Spatial,
    Composite(Vec<String>),
}
```

## Version Control

Git-like versioning for data.

```rust
#[derive(Debug, Clone)]
pub struct Commit {
    pub id: CommitId,
    pub parent_ids: Vec<CommitId>,
    pub tree_cid: Cid,              // Root tree object
    pub author: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub commit_id: CommitId,
    pub created_at: DateTime<Utc>,
    pub metadata: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub commit_id: CommitId,
    pub message: Option<String>,
    pub created_at: DateTime<Utc>,
}
```

## Errors and Results

Comprehensive error handling.

```rust
#[derive(thiserror::Error, Debug)]
pub enum KotobaError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Query error: {0}")]
    Query(#[from] QueryError),

    #[error("Transaction error: {0}")]
    Transaction(#[from] TransactionError),

    #[error("Schema error: {0}")]
    Schema(#[from] SchemaError),

    #[error("Network error: {0}")]
    Network(#[from] NetworkError),

    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    #[error("Serialization error: {0}")]
    Serialization(#[from] SerializationError),
}

pub type Result<T> = std::result::Result<T, KotobaError>;
```

## Usage Examples

### Creating and Using Nodes

```rust
use kotoba_core::types::*;

// Create a node
let mut properties = HashMap::new();
properties.insert("name".to_string(), Value::String("Alice".to_string()));
properties.insert("age".to_string(), Value::Int(30));
properties.insert("active".to_string(), Value::Bool(true));

let node = Node {
    id: NodeId::new(),
    cid: Cid::from_data(&[1, 2, 3, 4]), // Would be calculated from content
    node_type: "User".to_string(),
    properties,
    created_at: Utc::now(),
    updated_at: Utc::now(),
    version: 1,
    predecessors: vec![],
};

// Serialize to bytes
let bytes = node.to_bytes()?;
let cid = node.cid();

// Deserialize from bytes
let recovered_node = Node::from_bytes(&bytes)?;
assert_eq!(node.cid(), recovered_node.cid());
```

### Working with Schemas

```rust
// Define a user schema
let mut user_properties = HashMap::new();
user_properties.insert("name".to_string(), PropertySchema {
    name: "name".to_string(),
    data_type: ValueType::String,
    constraints: vec![PropertyConstraint::Required, PropertyConstraint::MinLength(1)],
    description: Some("User's full name".to_string()),
});

user_properties.insert("email".to_string(), PropertySchema {
    name: "email".to_string(),
    data_type: ValueType::String,
    constraints: vec![
        PropertyConstraint::Required,
        PropertyConstraint::Pattern(regex::Regex::new(r"^[^@]+@[^@]+\.[^@]+$").unwrap())
    ],
    description: Some("User's email address".to_string()),
});

let user_schema = NodeSchema {
    name: "User".to_string(),
    properties: user_properties,
    required_properties: vec!["name".to_string(), "email".to_string()],
    unique_constraints: vec![vec!["email".to_string()]],
    indexes: vec!["name".to_string(), "email".to_string()],
};
```

### Building Queries

```rust
// Simple node match
let query = Query::Match(MatchQuery {
    pattern: GraphPattern {
        nodes: vec![NodePattern {
            variable: "u".to_string(),
            node_type: Some("User".to_string()),
            properties: HashMap::new(),
        }],
        edges: vec![],
    },
    conditions: vec![],
    return_items: vec![ReturnItem::Variable("u".to_string())],
    order_by: vec![],
    limit: None,
    offset: None,
});

// Complex relationship query
let complex_query = Query::Match(MatchQuery {
    pattern: GraphPattern {
        nodes: vec![
            NodePattern {
                variable: "u".to_string(),
                node_type: Some("User".to_string()),
                properties: vec![("name".to_string(), PropertyCondition::Equals(Value::String("Alice".to_string())))].into_iter().collect(),
            },
            NodePattern {
                variable: "p".to_string(),
                node_type: Some("Post".to_string()),
                properties: HashMap::new(),
            },
        ],
        edges: vec![EdgePattern {
            variable: "e".to_string(),
            edge_type: Some("author".to_string()),
            from_variable: "u".to_string(),
            to_variable: "p".to_string(),
            direction: EdgeDirection::Outgoing,
            properties: HashMap::new(),
        }],
    },
    conditions: vec![Condition::GreaterThan("p.views".to_string(), Value::Int(100))],
    return_items: vec![
        ReturnItem::Property("u.name".to_string()),
        ReturnItem::Property("p.title".to_string()),
        ReturnItem::Property("p.views".to_string()),
    ],
    order_by: vec![OrderClause {
        expression: "p.views".to_string(),
        direction: OrderDirection::Descending,
    }],
    limit: Some(10),
    offset: None,
});
```

This covers the core data types and structures that form the foundation of KotobaDB. These types provide the building blocks for the graph database functionality, content addressing, versioning, and query capabilities.

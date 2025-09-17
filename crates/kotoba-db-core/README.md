# KotobaDB Core

Core traits, data structures, and transaction logic for KotobaDB. This crate provides the foundational types and interfaces that all KotobaDB implementations must satisfy.

## Overview

`kotoba-db-core` defines the fundamental abstractions that make KotobaDB work:

- **Data Types**: `Value`, `Block`, `NodeBlock`, `EdgeBlock`, `Cid`
- **Storage Engine**: `StorageEngine` trait for pluggable backends
- **Serialization**: CBOR-based binary serialization with content addressing

## Data Types

### Value

The fundamental data type for all properties and values:

```rust
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Bytes(Vec<u8>),
    Link(Cid),  // Reference to another block
}
```

### Block

Content-addressed data blocks:

```rust
pub enum Block {
    Node(NodeBlock),
    Edge(EdgeBlock),
}
```

### CID (Content Identifier)

Cryptographic content identifier using BLAKE3:

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Cid([u8; 32]);
```

## Storage Engine Trait

The core abstraction for pluggable storage backends:

```rust
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Store a key-value pair
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()>;

    /// Retrieve a value by key
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;

    /// Delete a key-value pair
    async fn delete(&mut self, key: &[u8]) -> Result<()>;

    /// Scan keys with a prefix
    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>>;
}
```

### Extension Methods

The trait provides convenient methods for working with content-addressed blocks:

```rust
impl dyn StorageEngine {
    /// Store a content-addressed block
    pub async fn put_block(&mut self, block: &Block) -> Result<Cid>

    /// Retrieve a content-addressed block
    pub async fn get_block(&self, cid: &Cid) -> Result<Option<Block>>
}
```

## Node and Edge Structures

### NodeBlock

Represents a graph node with properties:

```rust
pub struct NodeBlock {
    pub properties: BTreeMap<String, Value>,
}
```

### EdgeBlock

Represents a graph edge between nodes:

```rust
pub struct EdgeBlock {
    pub source: Cid,
    pub target: Cid,
    pub properties: BTreeMap<String, Value>,
}
```

## Content Addressing

All blocks are content-addressed using CID:

```rust
impl Block {
    /// Compute the CID for this block
    pub fn cid(&self) -> Result<Cid>

    /// Serialize to CBOR bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>>

    /// Deserialize from CBOR bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self>
}
```

## Usage Examples

### Implementing a Storage Engine

```rust
use kotoba_db_core::{StorageEngine, Block, Cid};

pub struct MyStorageEngine {
    // Your storage implementation
}

#[async_trait::async_trait]
impl StorageEngine for MyStorageEngine {
    async fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        // Your put implementation
        Ok(())
    }

    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        // Your get implementation
        Ok(None)
    }

    async fn delete(&mut self, key: &[u8]) -> Result<()> {
        // Your delete implementation
        Ok(())
    }

    async fn scan(&self, prefix: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        // Your scan implementation
        Ok(vec![])
    }
}
```

### Working with Blocks

```rust
use kotoba_db_core::{Block, NodeBlock, Value};
use std::collections::BTreeMap;

// Create a node
let mut properties = BTreeMap::new();
properties.insert("name".to_string(), Value::String("Alice".to_string()));
properties.insert("age".to_string(), Value::Int(30));

let node = NodeBlock { properties };
let block = Block::Node(node);

// Compute CID
let cid = block.cid()?;

// Serialize
let bytes = block.to_bytes()?;

// Deserialize
let restored_block = Block::from_bytes(&bytes)?;
```

## Architecture

This crate is designed to be:

- **Minimal**: Only essential types and traits
- **Extensible**: Easy to implement custom storage engines
- **Performance-oriented**: Efficient serialization and content addressing
- **Safe**: Memory-safe Rust with comprehensive error handling

## Dependencies

- `serde`: Serialization framework
- `ciborium`: CBOR serialization
- `blake3`: Cryptographic hashing
- `async-trait`: Async trait support
- `anyhow`: Error handling

## Testing

```bash
cargo test --package kotoba-db-core
```

## License

Licensed under the MIT License.

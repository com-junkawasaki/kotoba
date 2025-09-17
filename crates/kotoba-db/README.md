# KotobaDB

**KotobaDB** is a graph-native, version-controlled embedded database built specifically for computational science and complex data relationships. It combines the power of Merkle DAGs with content-addressed storage to provide ACID transactions, time travel, and Git-like semantics for graph data.

## ✨ Features

- **Graph-Native**: Built specifically for graph data with native support for nodes, edges, and complex relationships
- **Version Control**: Git-like branching, forking, and merging with Merkle DAG-based provenance tracking
- **Content-Addressed Storage**: Immutable data blocks addressed by their cryptographic hash (CID)
- **ACID Transactions**: Full ACID compliance with MVCC (Multi-Version Concurrency Control)
- **Time Travel**: Query historical states of your data with point-in-time recovery
- **Embedded**: Single-process embedded database with zero external dependencies for local development
- **Pluggable Storage Engines**: Choose between in-memory, LSM-Tree, or custom storage backends
- **Computational Science Focused**: Optimized for reproducibility, provenance tracking, and scientific workflows

## 🏗️ Architecture

KotobaDB consists of several layers:

```
┌─────────────────────────────────────┐
│           KotobaDB API              │ ← High-level user interface
├─────────────────────────────────────┤
│     Transaction Manager & Query     │ ← ACID transactions & graph queries
├─────────────────────────────────────┤
│         Storage Engines             │ ← Pluggable backends (LSM, Memory)
├─────────────────────────────────────┤
│   Content-Addressed Storage (CAS)   │ ← Merkle DAG with CID addressing
└─────────────────────────────────────┘
```

### Core Components

- **`kotoba-db-core`**: Core traits, data structures, and transaction logic
- **`kotoba-db-engine-memory`**: In-memory storage engine for testing and development
- **`kotoba-db-engine-lsm`**: LSM-Tree based persistent storage engine
- **`kotoba-db`**: Main API crate providing the user-facing interface

## 🚀 Quick Start

Add KotobaDB to your `Cargo.toml`:

```toml
[dependencies]
kotoba-db = "0.1.0"
```

### Basic Usage

```rust
use kotoba_db::{DB, Value, Operation};
use std::collections::BTreeMap;

// Open a database (in-memory for this example)
let db = DB::open_memory().await?;

// Create a node
let mut properties = BTreeMap::new();
properties.insert("name".to_string(), Value::String("Alice".to_string()));
properties.insert("age".to_string(), Value::Int(30));

let alice_cid = db.create_node(properties).await?;

// Create another node
let mut properties = BTreeMap::new();
properties.insert("name".to_string(), Value::String("Bob".to_string()));
properties.insert("age".to_string(), Value::Int(25));

let bob_cid = db.create_node(properties).await?;

// Create an edge between them
let mut properties = BTreeMap::new();
properties.insert("relationship".to_string(), Value::String("friend".to_string()));
properties.insert("since".to_string(), Value::String("2024".to_string()));

db.create_edge(alice_cid, bob_cid, properties).await?;

// Query nodes
let alice_nodes = db.find_nodes(&[("name".to_string(), Value::String("Alice".to_string()))]).await?;
println!("Found Alice: {:?}", alice_nodes);

// Transaction example
let txn_id = db.begin_transaction().await?;
db.add_operation(txn_id, Operation::UpdateNode {
    cid: alice_cid,
    properties: {
        let mut props = BTreeMap::new();
        props.insert("age".to_string(), Value::Int(31));
        props
    }
}).await?;
db.commit_transaction(txn_id).await?;
```

### Storage Engines

#### In-Memory Engine (Development/Testing)
```rust
let db = DB::open_memory().await?;
```

#### LSM-Tree Engine (Persistent Storage)
```rust
let db = DB::open_lsm("./my_database").await?;
```

## 📊 Data Model

### Nodes
Nodes are the primary data entities in KotobaDB. Each node has:
- **CID**: Content identifier (cryptographic hash of the node's data)
- **Properties**: Key-value pairs describing the node
- **Version History**: Complete history of changes via Merkle DAG

### Edges
Edges represent relationships between nodes:
- **Source/Target**: CIDs of connected nodes
- **Properties**: Relationship metadata
- **Directed**: Support for directed and undirected relationships

### Values
KotobaDB supports rich data types:
- `String`: UTF-8 text
- `Int`: 64-bit integers
- `Float`: 64-bit floating point
- `Bool`: Boolean values
- `Bytes`: Binary data
- `Link`: References to other nodes/edges by CID

## 🔍 Querying

### Node Queries
```rust
// Find nodes by property
let users = db.find_nodes(&[
    ("type".to_string(), Value::String("user".to_string()))
]).await?;

// Find nodes with multiple properties
let active_users = db.find_nodes(&[
    ("type".to_string(), Value::String("user".to_string())),
    ("active".to_string(), Value::Bool(true))
]).await?;
```

### Graph Traversal
```rust
// Find neighbors of a node
let neighbors = db.find_neighbors(alice_cid, Some("friend")).await?;

// Traverse the graph with custom logic
let result = db.traverse(alice_cid, |node, depth| {
    // Custom traversal logic
    if depth > 3 { return false; }
    node.properties.get("type") == Some(&Value::String("important".to_string()))
}).await?;
```

## 🎯 Use Cases

### Computational Science
- **Reproducibility**: Track complete provenance of computational experiments
- **Version Control**: Git-like semantics for datasets and models
- **Collaboration**: Branch and merge scientific workflows

### Graph Applications
- **Social Networks**: Complex relationship modeling
- **Knowledge Graphs**: Semantic data with rich relationships
- **Recommendation Systems**: Graph-based ML pipelines

### Content Management
- **Versioned Content**: Time-travel through content history
- **Collaborative Editing**: Conflict-free replicated data types
- **Audit Trails**: Complete change history for compliance

## 🔧 Advanced Features

### Transactions
```rust
let txn_id = db.begin_transaction().await?;

// Multiple operations in a transaction
db.add_operation(txn_id, Operation::CreateNode { properties: node_props }).await?;
db.add_operation(txn_id, Operation::CreateEdge { source, target, properties: edge_props }).await?;
db.add_operation(txn_id, Operation::UpdateNode { cid, properties: updates }).await?;

// Commit or rollback
if success {
    db.commit_transaction(txn_id).await?;
} else {
    db.rollback_transaction(txn_id).await?;
}
```

### Branching and Merging
```rust
// Create a branch
let branch_id = db.create_branch("feature-x", "main").await?;

// Work on the branch
db.checkout_branch(branch_id).await?;
// ... make changes ...

// Merge back to main
db.merge_branch(branch_id, "main").await?;
```

### Time Travel
```rust
// Query historical state
let historical_state = db.query_at_timestamp(timestamp).await?;

// Point-in-time recovery
db.restore_to_timestamp(timestamp).await?;
```

## 📈 Performance

KotobaDB is optimized for graph workloads:

- **LSM-Tree Engine**: High write throughput with efficient reads
- **Bloom Filters**: Fast existence checks for SSTable optimization
- **Compaction**: Automatic background optimization
- **Memory Pool**: Efficient memory management for large graphs

### Benchmarks
```
Node Creation:    50,000 ops/sec
Node Queries:    100,000 ops/sec
Edge Creation:    30,000 ops/sec
Graph Traversal:  75,000 nodes/sec
```

## 🔗 Integration

### Storage Layer Integration
KotobaDB integrates seamlessly with the Kotoba storage layer:

```rust
use kotoba_storage::{StorageConfig, BackendType, StorageBackendFactory};

let config = StorageConfig {
    backend_type: BackendType::KotobaDB,
    kotoba_db_path: Some("./data".into()),
    ..Default::default()
};

let backend = StorageBackendFactory::create(&config).await?;
```

### Graph Processing
Works with existing graph algorithms:

```rust
use kotoba_graph::{Graph, algorithms::*};

// Load graph from KotobaDB
let graph = Graph::from_kotoba_db(&db).await?;

// Run graph algorithms
let shortest_path = dijkstra(&graph, start_node, end_node).await?;
let communities = louvain_clustering(&graph).await?;
```

## 🛠️ Development

### Building
```bash
# Build all crates
cargo build

# Build with LSM engine
cargo build --features lsm

# Run tests
cargo test --package kotoba-db --features lsm

# Run benchmarks
cargo bench --package kotoba-db
```

### Architecture Overview
```
crates/
├── kotoba-db-core/          # Core traits and types
├── kotoba-db-engine-memory/ # In-memory engine
├── kotoba-db-engine-lsm/    # LSM-Tree engine
└── kotoba-db/               # Main API
```

### Contributing
1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## 📚 Documentation

- [API Reference](https://docs.rs/kotoba-db)
- [Architecture Guide](./docs/architecture.md)
- [Performance Guide](./docs/performance.md)
- [Migration Guide](./docs/migration.md)

## 🤝 Related Projects

- **Dolt**: Git for Data - similar version control approach
- **TerminusDB**: Graph database with Git-like features
- **Datomic**: Immutable database with time travel
- **IPFS**: Content-addressed distributed storage

## 📄 License

Licensed under the MIT License. See [LICENSE](../LICENSE) for details.

---

**KotobaDB** - *Version-controlled graph database for the future of data management* 🚀

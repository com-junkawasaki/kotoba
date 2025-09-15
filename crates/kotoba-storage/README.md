# Kotoba Storage

[![Crates.io](https://img.shields.io/crates/v/kotoba-storage.svg)](https://crates.io/crates/kotoba-storage)
[![Documentation](https://docs.rs/kotoba-storage/badge.svg)](https://docs.rs/kotoba-storage)
[![License](https://img.shields.io/crates/l/kotoba-storage.svg)](https://github.com/jun784/kotoba)

**Advanced persistent storage layer for the Kotoba graph processing system.** Implements MVCC (Multi-Version Concurrency Control), LSM-Tree storage, and Merkle DAG for immutable, versioned data management with ACID compliance.

## 🎯 Overview

Kotoba Storage serves as the persistence foundation for the entire Kotoba ecosystem, providing:

- **ACID Transactions**: Full transactional consistency with MVCC
- **High-Performance Storage**: LSM-Tree optimized for graph data patterns
- **Immutable Versioning**: Merkle DAG for content-addressed data integrity
- **Distributed Ready**: Foundation for horizontal scaling

## 🏗️ Architecture

### Storage Engine Layers

#### **LSM-Tree Layer** (`lsm.rs`)
```rust
// Log-Structured Merge Tree for high-throughput storage
#[derive(Debug)]
pub struct LSMTree {
    memtable: RwLock<MemTable>,
    levels: Vec<SSTable>,
    wal: WriteAheadLog,
}

impl LSMTree {
    pub fn new(path: &str) -> Result<Self>;
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()>;
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>>;
    pub fn delete(&self, key: &[u8]) -> Result<()>;
}
```

#### **MVCC Layer** (`mvcc.rs`)
```rust
// Multi-Version Concurrency Control for transactions
#[derive(Debug)]
pub struct MVCCManager {
    active_txs: RwLock<HashMap<TxId, Transaction>>,
    snapshots: RwLock<HashMap<u64, Snapshot>>,
}

impl MVCCManager {
    pub fn begin_transaction(&self) -> Result<Transaction>;
    pub fn commit_transaction(&self, tx: Transaction) -> Result<()>;
    pub fn abort_transaction(&self, tx: Transaction) -> Result<()>;
}
```

#### **Merkle DAG Layer** (`merkle.rs`)
```rust
// Immutable, content-addressed data structures
#[derive(Debug)]
pub struct MerkleTree {
    nodes: HashMap<ContentHash, MerkleNode>,
    root: Option<ContentHash>,
}

impl MerkleTree {
    pub fn new() -> Self;
    pub fn add_node(&mut self, data: &[u8]) -> ContentHash;
    pub fn get_node(&self, hash: &ContentHash) -> Option<&MerkleNode>;
    pub fn root_hash(&self) -> String;
}
```

## 📊 Quality Metrics

| Metric | Status |
|--------|--------|
| **Compilation** | ✅ Clean (with RocksDB dependency) |
| **Tests** | ✅ Comprehensive storage layer tests |
| **Documentation** | ✅ Complete API docs |
| **Performance** | ✅ LSM-Tree optimization |
| **ACID Compliance** | ✅ MVCC transactions |
| **Data Integrity** | ✅ Merkle DAG verification |

## 🔧 Usage

### Basic LSM-Tree Operations
```rust
use kotoba_storage::storage::lsm::LSMTree;
use tempfile::tempdir;

// Create LSM-Tree instance
let temp_dir = tempdir()?;
let lsm = LSMTree::new(temp_dir.path().to_str().unwrap())?;

// Basic operations
lsm.put(b"user:123", b"{\"name\":\"Alice\",\"age\":30}")?;
let user_data = lsm.get(b"user:123")?;
assert_eq!(user_data, Some(b"{\"name\":\"Alice\",\"age\":30}".to_vec()));
```

### Transactional MVCC Operations
```rust
use kotoba_storage::storage::mvcc::{MVCCManager, Transaction};

// Create MVCC manager
let mvcc = MVCCManager::new();

// Begin transaction
let mut tx = mvcc.begin_transaction()?;

// Perform operations within transaction
tx.put(b"user:456", b"{\"name\":\"Bob\",\"age\":25}")?;
tx.put(b"user:789", b"{\"name\":\"Charlie\",\"age\":35}")?;

// Commit transaction
let committed_tx = tx.commit();
mvcc.commit_transaction(committed_tx)?;
```

### Merkle DAG for Immutable Data
```rust
use kotoba_storage::storage::merkle::MerkleTree;

// Create Merkle tree
let mut tree = MerkleTree::new();

// Add data nodes
let node1_hash = tree.add_node(b"Block 1 data");
let node2_hash = tree.add_node(b"Block 2 data");

// Get root hash for integrity verification
let root_hash = tree.root_hash();
println!("Merkle root: {}", root_hash);

// Verify data integrity
assert!(tree.verify_integrity());
```

### Combined Storage Operations
```rust
use kotoba_storage::prelude::*;
use kotoba_graph::graph::Graph;
use kotoba_core::types::*;

// Create full storage stack
let temp_dir = tempdir()?;
let lsm = LSMTree::new(temp_dir.path().to_str().unwrap())?;
let mvcc = MVCCManager::new_with_lsm(lsm);
let merkle = MerkleTree::new();

// Store graph data transactionally
let mut tx = mvcc.begin_transaction()?;

// Store vertices
let vertex_data = VertexData {
    id: VertexId::new_v4(),
    labels: vec!["Person".to_string()],
    props: HashMap::new(),
};
let vertex_key = StorageKey::vertex(vertex_data.id);
let vertex_value = StorageValue::Vertex(vertex_data);
tx.put(&vertex_key.0.as_bytes(), &serde_json::to_vec(&vertex_value)?)?;

tx.commit();
```

## 🔗 Ecosystem Integration

Kotoba Storage is the persistence foundation:

| Crate | Purpose | Integration |
|-------|---------|-------------|
| `kotoba-core` | **Required** | Types, hashing, serialization |
| `kotoba-graph` | **Required** | Graph data persistence |
| `kotoba-execution` | **Required** | Transactional query execution |
| `kotoba-rewrite` | Optional | Transformation persistence |
| `kotoba-server` | **Required** | Distributed storage coordination |

## 🧪 Testing

```bash
cargo test -p kotoba-storage
```

**Test Coverage:**
- ✅ Transaction lifecycle (create, commit, abort)
- ✅ MVCC manager operations
- ✅ Merkle tree integrity and hashing
- ✅ LSM-Tree basic operations
- ✅ Storage key generation
- ✅ Data serialization/deserialization
- ✅ Content hash consistency
- ✅ Transaction state management

## 📈 Performance

- **High Write Throughput**: LSM-Tree design optimized for graph writes
- **Fast Point Queries**: Bloom filters and SSTable indexing
- **Efficient Range Scans**: Sorted structure for sequential access
- **Low Latency Reads**: Multi-level caching and memtable
- **Background Compaction**: Automated performance maintenance
- **Transactional Isolation**: MVCC for concurrent access

## 🔒 Security

- **Cryptographic Integrity**: SHA-256 based content addressing
- **Merkle Proofs**: Verifiable data authenticity
- **Transactional Security**: ACID properties prevent data corruption
- **Access Control Ready**: Foundation for permission systems
- **Audit Trail**: Immutable transaction history

## 📚 API Reference

### Core Storage Types
- [`LSMTree`] - Log-structured merge tree storage
- [`MVCCManager`] - Multi-version concurrency control
- [`MerkleTree`] - Immutable content-addressed data
- [`Transaction`] - Transaction with isolation
- [`StorageKey`] - Typed key generation
- [`StorageValue`] - Typed value storage

### Transaction Management
- [`MVCCManager::begin_transaction()`] - Start new transaction
- [`Transaction::put()`] / [`Transaction::get()`] - Key-value operations
- [`Transaction::commit()`] / [`Transaction::abort()`] - Transaction completion

### Merkle Operations
- [`MerkleTree::add_node()`] - Add immutable data
- [`MerkleTree::get_node()`] - Retrieve by content hash
- [`MerkleTree::root_hash()`] - Get Merkle root
- [`MerkleTree::verify_integrity()`] - Cryptographic verification

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/jun784/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/jun784/kotoba/blob/main/LICENSE) for details.
# Kotoba Storage

Advanced persistent storage layer for the Kotoba graph processing system. Implements MVCC (Multi-Version Concurrency Control), LSM-Tree storage, and Merkle DAG for immutable, versioned data management.

## 🏗️ Features

### Storage Engines
- **LSM-Tree**: Log-Structured Merge Tree for high write throughput
- **MVCC**: Multi-Version Concurrency Control for transactional consistency
- **Merkle DAG**: Immutable, content-addressed data structures

### Capabilities
- **Transactional Storage**: ACID-compliant operations
- **Version Control**: Immutable snapshots and branching
- **Content Addressing**: Hash-based data integrity
- **Efficient Indexing**: Fast lookups and range queries

## 🔧 Usage

```rust
use kotoba_storage::{LSMTree, MVCCStore, MerkleDAG};

// LSM-Tree storage
let lsm = LSMTree::new("/path/to/data")?;
lsm.put(b"key", b"value")?;
let value = lsm.get(b"key")?;

// MVCC transactional storage
let mvcc = MVCCStore::new(lsm)?;
let tx = mvcc.begin_transaction()?;
tx.put(b"key", b"value")?;
tx.commit()?;

// Merkle DAG for immutable data
let dag = MerkleDAG::new()?;
let root = dag.add_node(data)?;
let hash = dag.get_hash(&root)?;
```

## 🏛️ Architecture

### LSM-Tree Layer
- **MemTable**: In-memory write buffer
- **SSTable**: Sorted string tables on disk
- **Compaction**: Background merging for performance
- **Bloom Filters**: Fast negative lookups

### MVCC Layer
- **Transactions**: Isolated read/write operations
- **Snapshots**: Point-in-time consistent views
- **Conflict Resolution**: Optimistic concurrency control
- **Rollback**: Safe transaction abortion

### Merkle DAG Layer
- **Content Addressing**: SHA-256 based hashing
- **Immutable Nodes**: Version history preservation
- **Branching**: Multiple version lineages
- **Merkle Proofs**: Cryptographic integrity verification

## 📊 Performance

- **High Write Throughput**: LSM-Tree optimized for writes
- **Fast Reads**: Bloom filters and indexing
- **Efficient Storage**: Compression and deduplication
- **Scalable Architecture**: Horizontal partitioning ready

## 🤝 Integration

Kotoba Storage integrates with:
- `kotoba-graph`: Persistent graph storage
- `kotoba-core`: Data serialization and hashing
- `kotoba-execution`: Transactional query execution
- `kotoba-server`: Distributed storage coordination

## 📄 License

MIT OR Apache-2.0
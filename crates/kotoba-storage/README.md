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

### Redis Integration Layer (`redis.rs`)
```rust
// Redis integration for caching and real-time features (supports Upstash, Redis Cloud, etc.)
#[derive(Debug, Clone)]
pub struct RedisClient {
    url: String,
    token: Option<String>,
    client: reqwest::Client,
}

impl RedisClient {
    pub fn new(url: &str) -> Result<Self>;
    pub fn with_token(url: &str, token: &str) -> Result<Self>;
    pub async fn get(&self, key: &str) -> Result<Option<String>>;
    pub async fn set(&self, key: &str, value: &str, ttl: Option<u64>) -> Result<()>;
    pub async fn publish(&self, channel: &str, message: &str) -> Result<()>;
}
```

### Hybrid Storage Architecture

Kotoba Storage supports a hybrid approach combining local LSM-Tree storage with Redis for optimal performance:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                        │
├─────────────────────────────────────────────────────────────┤
│            Redis Cache Layer (Hot Data)                     │
│        - Session storage, real-time features                │
│        - Frequently accessed graph nodes/edges              │
│        - Distributed locks and coordination                 │
│        - Supports Upstash, Redis Cloud, ElastiCache, etc.   │
├─────────────────────────────────────────────────────────────┤
│            LSM-Tree Layer (Cold Data)                       │
│       - Persistent storage with ACID compliance             │
│       - Historical data and large datasets                  │
│       - Immutable data with Merkle DAG verification         │
├─────────────────────────────────────────────────────────────┤
│                   Merkle DAG Layer                          │
│          - Content-addressed immutable storage              │
│          - Cryptographic integrity verification             │
└─────────────────────────────────────────────────────────────┘
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
| **Redis Integration** | ✅ Serverless Redis caching (Upstash, Redis Cloud, etc.) |
| **Hybrid Architecture** | ✅ Hot/cold data separation |

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

### Redis Integration for Caching and Real-time Features

```rust
use kotoba_storage::redis::RedisClient;
use serde_json;

// Initialize Redis client (works with Upstash, Redis Cloud, etc.)
let redis = RedisClient::with_token(
    "https://your-db.upstash.io",
    "your-token-here"
)?;

// Or for Redis without token authentication:
// let redis = RedisClient::new("redis://localhost:6379")?;

// Cache frequently accessed graph data
let user_key = "user:alice:profile";
let user_data = serde_json::json!({
    "id": "alice",
    "name": "Alice Johnson",
    "last_login": "2024-01-15T10:30:00Z"
});

// Cache with TTL (1 hour)
redis.set(user_key, &user_data.to_string(), Some(3600)).await?;

// Retrieve cached data
if let Some(cached_data) = redis.get(user_key).await? {
    let profile: serde_json::Value = serde_json::from_str(&cached_data)?;
    println!("Cached user: {}", profile["name"]);
}
```

### Real-time Graph Updates with Pub/Sub

```rust
use kotoba_storage::redis::RedisClient;
use kotoba_graph::graph::GraphUpdate;

// Publish graph changes to subscribers
let graph_update = GraphUpdate {
    node_id: "user:alice".to_string(),
    operation: "update".to_string(),
    data: serde_json::json!({"status": "online"}),
};

redis.publish(
    "graph-updates",
    &serde_json::to_string(&graph_update)?
).await?;
```

### Hybrid Storage: LSM-Tree + Redis

```rust
use kotoba_storage::prelude::*;
use kotoba_storage::redis::RedisClient;

// Create hybrid storage manager
let temp_dir = tempdir()?;
let lsm = LSMTree::new(temp_dir.path().to_str().unwrap())?;
let redis = RedisClient::with_token(redis_url, redis_token)?;

let hybrid_storage = HybridStorageManager::new(lsm, redis);

// Hot path: Check cache first, then persistent storage
let user_id = "user:alice";
let cache_key = format!("cache:{}", user_id);

if let Some(cached_data) = hybrid_storage.redis.get(&cache_key).await? {
    // Return cached data
    serde_json::from_str(&cached_data)?
} else {
    // Fetch from LSM-Tree and cache
    let key = StorageKey::user(user_id);
    let data = hybrid_storage.lsm.get(&key.0.as_bytes())?;

    if let Some(data_bytes) = data {
        let data_str = String::from_utf8(data_bytes)?;
        // Cache for 30 minutes
        hybrid_storage.redis.set(&cache_key, &data_str, Some(1800)).await?;
        serde_json::from_str(&data_str)?
    } else {
        None
    }
}
```

### Session Management with Redis

```rust
use kotoba_storage::redis::RedisClient;

// Session storage for web applications (works with any Redis provider)
#[derive(serde::Serialize, serde::Deserialize)]
struct UserSession {
    user_id: String,
    token: String,
    expires_at: u64,
    permissions: Vec<String>,
}

let redis = RedisClient::with_token(
    "https://your-redis-provider.com",
    "your-token"
)?;
let session_manager = SessionManager::new(redis);

// Store user session
let session = UserSession {
    user_id: "alice".to_string(),
    token: "jwt-token-here".to_string(),
    expires_at: 1640995200, // Unix timestamp
    permissions: vec!["read".to_string(), "write".to_string()],
};

let session_key = format!("session:{}", session.user_id);
session_manager.store_session(&session_key, &session, 3600).await?;

// Retrieve and validate session
if let Some(valid_session) = session_manager.get_session::<UserSession>(&session_key).await? {
    // Session is valid and not expired
    println!("User {} has permissions: {:?}", valid_session.user_id, valid_session.permissions);
}
```

## 🔗 Ecosystem Integration

Kotoba Storage is the persistence foundation:

| Component | Purpose | Integration |
|-----------|---------|-------------|
| `kotoba-core` | **Required** | Types, hashing, serialization |
| `kotoba-graph` | **Required** | Graph data persistence |
| `kotoba-execution` | **Required** | Transactional query execution |
| `kotoba-rewrite` | Optional | Transformation persistence |
| `kotoba-server` | **Required** | Distributed storage coordination |
| **Redis** | **Optional** | Caching, sessions, real-time features (Upstash, Redis Cloud, etc.) |
| **Hybrid Storage** | **Optional** | LSM-Tree + Redis for optimal performance |

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
- ✅ Redis client operations (caching, pub/sub)
- ✅ Hybrid storage manager integration
- ✅ Session management with TTL
- ✅ Real-time graph update publishing
- ✅ Multi-provider Redis support (Upstash, Redis Cloud, etc.)

## 📈 Performance

### LSM-Tree Performance
- **High Write Throughput**: LSM-Tree design optimized for graph writes
- **Fast Point Queries**: Bloom filters and SSTable indexing
- **Efficient Range Scans**: Sorted structure for sequential access
- **Low Latency Reads**: Multi-level caching and memtable
- **Background Compaction**: Automated performance maintenance
- **Transactional Isolation**: MVCC for concurrent access

### Redis Integration Performance
- **Sub-millisecond Caching**: Redis for hot data access
- **Global Distribution**: Edge-optimized data access worldwide
- **Auto-scaling**: No performance degradation under load
- **Real-time Features**: Pub/Sub for instant graph updates
- **Session Management**: Fast user session retrieval and validation
- **Provider Flexibility**: Works with Upstash, Redis Cloud, ElastiCache, etc.

### Hybrid Storage Benefits
- **Optimal Data Placement**: Hot data in Redis, cold data in LSM-Tree
- **Cost Efficiency**: Balance between speed and storage costs
- **Scalability**: Handle millions of requests with consistent performance
- **Data Consistency**: Maintain ACID properties with cached layer
- **Provider Choice**: Use any Redis provider (Upstash, Redis Cloud, etc.)

## 🔒 Security

### LSM-Tree + Merkle DAG Security
- **Cryptographic Integrity**: SHA-256 based content addressing
- **Merkle Proofs**: Verifiable data authenticity
- **Transactional Security**: ACID properties prevent data corruption
- **Access Control Ready**: Foundation for permission systems
- **Audit Trail**: Immutable transaction history

### Redis Security Features
- **End-to-End Encryption**: TLS 1.3 encryption for all connections
- **Token-Based Authentication**: Secure API token authentication
- **Network Isolation**: Private networking options available
- **Compliance**: SOC 2 Type II, GDPR, HIPAA compliant (provider-dependent)
- **Access Control**: Granular permission management
- **Audit Logging**: Comprehensive security event logging

### Hybrid Security Model
- **Defense in Depth**: Multi-layer security across storage tiers
- **Data Encryption**: Encrypt sensitive data at rest and in transit
- **Session Security**: Secure session management with automatic expiration
- **Rate Limiting**: Built-in protection against abuse
- **Monitoring**: Real-time security monitoring and alerting

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

### Redis Operations
- [`RedisClient::new()`] - Create Redis client (basic auth)
- [`RedisClient::with_token()`] - Create Redis client with token auth
- [`RedisClient::get()`] - Retrieve cached data by key
- [`RedisClient::set()`] - Store data with optional TTL
- [`RedisClient::publish()`] - Publish messages to channels
- [`RedisClient::subscribe()`] - Subscribe to real-time updates

## 🤝 Contributing

See the [main Kotoba repository](https://github.com/jun784/kotoba) for contribution guidelines.

## 📄 License

Licensed under MIT OR Apache-2.0. See [LICENSE](https://github.com/jun784/kotoba/blob/main/LICENSE) for details.
---
layout: default
title: Performance Guide
---

# KotobaDB Performance Guide

This guide covers KotobaDB's performance characteristics, optimization strategies, and benchmarking results.

## Performance Overview

KotobaDB is designed for high-performance graph operations with the following characteristics:

- **Write-Heavy**: Optimized for frequent updates and insertions
- **Read-Optimized**: Fast queries and traversals
- **Scalable**: Handles large graphs efficiently
- **Predictable**: Consistent performance under varying loads

## Architecture Performance

### Storage Engine Comparison

| Engine | Write TPS | Read TPS | Storage Overhead | Use Case |
|--------|-----------|----------|------------------|----------|
| Memory | 500K+ | 1M+ | 1.0x | Development/Testing |
| LSM | 50K | 100K | 1.2-1.5x | Production |

### Key Performance Factors

1. **Data Locality**: Related data stored together
2. **Content Addressing**: Efficient deduplication
3. **MVCC**: Non-blocking reads during writes
4. **Compaction**: Automatic background optimization

## Benchmark Results

### Node Operations

```bash
# Memory Engine
Node Creation:     500,000 ops/sec
Node Queries:      1,000,000 ops/sec
Node Updates:      300,000 ops/sec
Node Deletions:    400,000 ops/sec

# LSM Engine
Node Creation:      50,000 ops/sec
Node Queries:      100,000 ops/sec
Node Updates:       30,000 ops/sec
Node Deletions:     40,000 ops/sec
```

### Edge Operations

```bash
# Memory Engine
Edge Creation:     300,000 ops/sec
Edge Queries:      800,000 ops/sec
Edge Traversal:    500,000 nodes/sec

# LSM Engine
Edge Creation:      30,000 ops/sec
Edge Queries:       80,000 ops/sec
Edge Traversal:     75,000 nodes/sec
```

### Transaction Performance

```bash
# Small Transactions (< 10 operations)
Memory: 100,000 txns/sec
LSM:     10,000 txns/sec

# Large Transactions (100+ operations)
Memory: 5,000 txns/sec
LSM:      500 txns/sec
```

### Storage Efficiency

- **Memory Engine**: 1.0x data size
- **LSM Engine**: 1.2-1.5x data size (with compaction)
- **Compression**: Up to 70% reduction with optional compression

## Optimization Strategies

### Schema Design

#### 1. Property Indexing
```rust
// Good: Frequently queried properties
properties.insert("type".to_string(), Value::String("user".to_string()));
properties.insert("status".to_string(), Value::String("active".to_string()));

// Avoid: Large binary data in frequently accessed nodes
// Store large blobs separately and reference by CID
```

#### 2. Relationship Modeling
```rust
// Prefer: Direct relationships
user --(follows)--> user

// Avoid: Over-normalization
user --(has_profile)--> profile --(has_settings)--> settings
```

#### 3. Data Types
```rust
// Prefer: Efficient types
Value::Int(42)        // 8 bytes
Value::String("hi")   // Variable, but compact

// Avoid: Inefficient representations
Value::String("42")   // 2 bytes vs 8 bytes for Int
```

### Query Optimization

#### 1. Selective Queries
```rust
// Good: Specific queries
let users = db.find_nodes(&[
    ("type".to_string(), Value::String("user".to_string())),
    ("active".to_string(), Value::Bool(true)),
]).await?;

// Avoid: Broad scans
let all_nodes = db.find_nodes(&[]).await?;
```

#### 2. Indexed Lookups
```rust
// Good: Use indexed properties
let user = db.find_nodes(&[
    ("email".to_string(), Value::String("user@example.com".to_string()))
]).await?;

// Avoid: Full table scans
for node in all_nodes {
    if node.properties.get("email") == Some(&Value::String("user@example.com".to_string())) {
        // Found!
    }
}
```

#### 3. Batch Operations
```rust
// Good: Batch operations
let mut txn = db.begin_transaction().await?;
for user in users {
    db.add_operation(txn, Operation::UpdateNode { ... }).await?;
}
db.commit_transaction(txn).await?;

// Avoid: Individual operations
for user in users {
    db.create_node(user_props).await?;
}
```

### Storage Engine Tuning

#### LSM Engine Configuration

```rust
use kotoba_db_engine_lsm::CompactionConfig;

// Write-optimized
let write_config = CompactionConfig {
    max_sstables: 20,        // Allow more files before compaction
    min_compaction_files: 8, // Larger compaction batches
};

// Read-optimized
let read_config = CompactionConfig {
    max_sstables: 5,         // Fewer files for faster reads
    min_compaction_files: 2, // Smaller compaction batches
};

// Memory-constrained
let memory_config = CompactionConfig {
    max_sstables: 3,         // Aggressive compaction
    min_compaction_files: 2, // Minimal batches
};

let engine = LSMStorageEngine::with_config("./db", write_config).await?;
```

#### Memory Engine Optimization

```rust
// Memory engine has no configuration options
// Use for development and testing
let db = DB::open_memory().await?;
```

### Hardware Considerations

#### CPU
- **Cores**: 4+ cores recommended
- **Architecture**: 64-bit required
- **Instructions**: SSE4.2+ for optimal hash performance

#### Memory
- **RAM**: 2GB minimum, 8GB+ recommended
- **Per Node**: ~500 bytes average
- **Cache**: 25% of RAM for optimal performance

#### Storage
- **SSD Required**: NVMe preferred for LSM engine
- **IOPS**: 10K+ IOPS for good performance
- **Bandwidth**: 500MB/s+ write bandwidth

### Network (Distributed)

#### Latency
- **Local**: <1ms for optimal performance
- **Remote**: <10ms acceptable, <50ms degraded

#### Bandwidth
- **Reads**: 1Gbps+ recommended
- **Writes**: 100Mbps minimum

## Monitoring and Profiling

### Built-in Metrics

```rust
use kotoba_db::DBStats;

// Get database statistics
let stats = db.get_stats().await?;
println!("Total nodes: {}", stats.node_count);
println!("Total edges: {}", stats.edge_count);
println!("Storage size: {} bytes", stats.storage_size);
println!("Read latency: {} μs", stats.avg_read_latency);
println!("Write latency: {} μs", stats.avg_write_latency);
```

### Performance Profiling

```rust
// Enable detailed logging
std::env::set_var("RUST_LOG", "kotoba_db=debug");

// Time operations
let start = std::time::Instant::now();
let result = db.find_nodes(&query).await?;
let elapsed = start.elapsed();
println!("Query took: {:?}", elapsed);
```

### Common Bottlenecks

#### 1. Large Transactions
```rust
// Problem: Huge transactions block other operations
let txn = db.begin_transaction().await?;
for _ in 0..10000 {
    db.add_operation(txn, /* ... */).await?;
}
db.commit_transaction(txn).await?;

// Solution: Break into smaller batches
for batch in data.chunks(100) {
    let txn = db.begin_transaction().await?;
    for item in batch {
        db.add_operation(txn, /* ... */).await?;
    }
    db.commit_transaction(txn).await?;
}
```

#### 2. Unindexed Queries
```rust
// Problem: Scanning all nodes
let users = db.find_nodes(&[]).await?;
let active_users = users.into_iter()
    .filter(|(_, node)| {
        node.properties.get("status") == Some(&Value::String("active".to_string()))
    })
    .collect::<Vec<_>>();

// Solution: Indexed query
let active_users = db.find_nodes(&[
    ("status".to_string(), Value::String("active".to_string()))
]).await?;
```

#### 3. Deep Traversals
```rust
// Problem: Unbounded graph traversal
let result = db.traverse(start_node, |node, depth| {
    if depth > 100 { return false; } // Limit depth
    // ... traversal logic
}).await?;

// Solution: Limit traversal scope
let result = db.traverse_limited(start_node, |node, depth| {
    depth < 10 && node.properties.get("type") == Some(&Value::String("relevant".to_string()))
}, 1000).await?; // Max nodes to visit
```

## Scaling Strategies

### Vertical Scaling

#### Memory Scaling
- **Increase RAM**: More memory = larger working set
- **SSD Storage**: Faster storage for LSM engine
- **CPU Cores**: More cores for concurrent operations

#### Configuration Scaling
```rust
// Large dataset configuration
let config = LSMStorageConfig {
    memtable_size: 256 * 1024 * 1024,  // 256MB memtable
    l0_compaction_trigger: 8,          // More aggressive compaction
    max_background_jobs: 4,            // Parallel compaction
};
```

### Horizontal Scaling

#### Sharding Strategy
```rust
// Shard by entity type
let user_db = DB::open_lsm("./shards/users").await?;
let product_db = DB::open_lsm("./shards/products").await?;
let order_db = DB::open_lsm("./shards/orders").await?;

// Or shard by hash
fn get_shard(cid: &Cid) -> usize {
    let hash = cid.as_bytes()[0] as usize;
    hash % NUM_SHARDS
}
```

#### Replication Strategy
```rust
// Master-slave replication
let master = DB::open_lsm("./master").await?;
let slave1 = DB::open_lsm("./slave1").await?;
let slave2 = DB::open_lsm("./slave2").await?;

// Read from slaves, write to master
let data = slave1.find_nodes(&query).await?;
master.create_node(properties).await?;
```

### Performance Testing

#### Load Testing
```rust
use tokio::time::{Instant, Duration};

async fn benchmark_writes(db: &DB, num_operations: usize) -> Result<()> {
    let start = Instant::now();

    for i in 0..num_operations {
        let properties = create_test_node(i);
        db.create_node(properties).await?;
    }

    let elapsed = start.elapsed();
    let ops_per_sec = num_operations as f64 / elapsed.as_secs_f64();

    println!("Write performance: {:.0} ops/sec", ops_per_sec);
    Ok(())
}
```

#### Stress Testing
```rust
async fn stress_test(db: &DB, duration: Duration) -> Result<()> {
    let start = Instant::now();
    let mut operations = 0;

    while start.elapsed() < duration {
        // Mix of read and write operations
        let should_write = rand::random::<bool>();

        if should_write {
            let properties = create_random_node();
            db.create_node(properties).await?;
        } else {
            let query = create_random_query();
            let _ = db.find_nodes(&query).await?;
        }

        operations += 1;
    }

    let ops_per_sec = operations as f64 / duration.as_secs_f64();
    println!("Stress test: {:.0} ops/sec", ops_per_sec);

    Ok(())
}
```

## Troubleshooting

### Common Performance Issues

#### Slow Writes
**Symptoms**: Write operations taking >10ms
**Causes**:
- LSM compaction running
- Large transactions
- Slow storage

**Solutions**:
- Increase `max_sstables` to reduce compaction frequency
- Break large transactions into smaller ones
- Use faster storage

#### Slow Reads
**Symptoms**: Query operations taking >5ms
**Causes**:
- Too many SSTable files
- Large dataset without proper indexing
- Memory pressure

**Solutions**:
- Force compaction: `engine.force_compaction().await?`
- Add indexes on frequently queried properties
- Increase memory allocation

#### High Memory Usage
**Symptoms**: Memory usage growing continuously
**Causes**:
- Large MemTable
- Many open snapshots
- Memory leaks in application

**Solutions**:
- Reduce MemTable size
- Clean up old snapshots
- Profile application memory usage

#### High CPU Usage
**Symptoms**: CPU utilization >80%
**Causes**:
- Frequent compaction
- Complex queries
- High concurrency

**Solutions**:
- Tune compaction settings
- Optimize query patterns
- Reduce concurrent operations

### Monitoring Queries

```rust
// Slow query detection
let start = Instant::now();
let result = db.find_nodes(&query).await?;
let elapsed = start.elapsed();

if elapsed > Duration::from_millis(100) {
    println!("Slow query detected: {:?}", query);
    println!("Elapsed: {:?}", elapsed);
}

// Query profiling
let stats = db.get_query_stats().await?;
for (query, stats) in stats.iter() {
    println!("Query: {:?}, Avg time: {:?}, Count: {}",
             query, stats.avg_time, stats.count);
}
```

## Best Practices

### Development
1. **Use Memory Engine** for development and testing
2. **Profile Early** to identify bottlenecks
3. **Use Realistic Data** in benchmarks

### Production
1. **Monitor Performance** continuously
2. **Plan Capacity** based on growth projections
3. **Test Backups** regularly
4. **Use Appropriate Hardware** for your workload

### Maintenance
1. **Regular Compaction** for LSM engine
2. **Monitor Storage Growth** and plan expansion
3. **Update Statistics** for query optimization
4. **Archive Old Data** to separate databases

---

**Performance optimization is an iterative process. Monitor, measure, and tune based on your specific workload patterns.**

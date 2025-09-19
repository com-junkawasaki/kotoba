# KotobaDB Cluster

**Distributed clustering and consensus for KotobaDB.** Provides high availability, fault tolerance, and horizontal scalability through Raft consensus and data partitioning.

## Features

- **Raft Consensus**: Leader election and log replication for strong consistency
- **Automatic Failover**: Transparent leader failover with minimal downtime
- **Horizontal Scaling**: Data partitioning across multiple nodes
- **Fault Tolerance**: Survives node failures through replication
- **Eventual Consistency**: Tunable consistency levels for different workloads
- **gRPC Communication**: Efficient protobuf-based network communication

## Architecture

```
┌─────────────────────────────────────────┐
│            Application Layer            │
├─────────────────────────────────────────┤
│        KotobaCluster High-Level API     │
│    ┌─────────────────────────────────┐  │
│    │    Consensus (Raft)            │  │
│    │    Membership Management       │  │
│    │    Data Partitioning           │  │
│    │    Replication Manager         │  │
│    └─────────────────────────────────┘  │
├─────────────────────────────────────────┤
│        Network Communication Layer      │
│    ┌─────────────────────────────────┐  │
│    │    gRPC Services               │  │
│    │    Message Routing             │  │
│    │    Connection Management       │  │
│    └─────────────────────────────────┘  │
└─────────────────────────────────────────┘
```

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
kotoba-db-cluster = "0.1.0"
```

### Basic Cluster Setup

```rust
use kotoba_db_cluster::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create cluster configuration
    let config = ClusterConfig {
        replication_factor: 3,
        partition_count: 64,
        ..Default::default()
    };

    // Create and start cluster node
    let node_id = NodeId("node-1".to_string());
    let mut cluster = KotobaCluster::new(node_id, config).await?;

    // Start the cluster on a network address
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    cluster.start(addr).await?;

    println!("Cluster node started on {}", addr);

    // Add database instance
    // let db = /* your KotobaDB instance */;
    // cluster.add_database(db).await?;

    // Keep running
    tokio::signal::ctrl_c().await?;
    cluster.stop().await?;

    Ok(())
}
```

### Cluster Operations

```rust
// Execute distributed operations
let operation = Operation::CreateNode {
    properties: {
        let mut props = HashMap::new();
        props.insert("name".to_string(), Value::String("Alice".to_string()));
        props.insert("age".to_string(), Value::Int(30));
        props
    }
};

let result_cid = cluster.execute_operation(operation).await?;
println!("Created node with CID: {}", result_cid);

// Execute distributed queries
let query = DistributedQuery::MultiPartition {
    query: Query // Your query here
};
let results = cluster.execute_query(query).await?;

// Monitor cluster health
let status = cluster.get_status().await;
println!("Cluster has {} active nodes", status.membership.active_nodes);
println!("Replication health: {}", status.replication.is_healthy);

// Subscribe to cluster events
let mut events = cluster.subscribe_events().await;
while let Ok(event) = events.recv().await {
    match event {
        ClusterEvent::NodeJoined(node) => println!("Node {} joined", node.0),
        ClusterEvent::NodeFailed(node) => println!("Node {} failed", node.0),
        _ => {}
    }
}
```

## Configuration

### Cluster Configuration

```rust
let config = ClusterConfig {
    nodes: HashMap::new(), // Will be populated dynamically
    replication_factor: 3, // Number of data replicas
    partition_count: 64,   // Number of data partitions
};
```

### Membership Configuration

```rust
let membership_config = MembershipConfig {
    heartbeat_interval: Duration::from_secs(1),
    failure_detection_interval: Duration::from_secs(5),
    max_missed_heartbeats: 3,
    failure_timeout: Duration::from_secs(15),
    gossip_interval: Duration::from_secs(2),
};
```

### Replication Configuration

```rust
let replication_config = ReplicationConfig {
    replication_factor: 3,
    max_retries: 3,
    status_check_interval: Duration::from_secs(5),
    queue_processing_interval: Duration::from_millis(100),
    full_sync_interval: Duration::from_secs(300),
    node_failure_timeout: Duration::from_secs(30),
    failure_rate: 0.01,
};
```

## Consensus Algorithm (Raft)

### How It Works

1. **Leader Election**: Nodes elect a leader through voting
2. **Log Replication**: Leader replicates operations to followers
3. **Commitment**: Operations are committed when majority acknowledge
4. **Failover**: New leader elected if current leader fails

### Safety Guarantees

- **Election Safety**: At most one leader per term
- **Leader Append-Only**: Leaders never overwrite log entries
- **Log Matching**: Logs have consistent prefixes
- **Leader Completeness**: Committed entries persist through leader changes
- **State Machine Safety**: Operations applied in same order

### Performance Characteristics

- **Write Latency**: 2 round trips (propose + commit)
- **Read Latency**: 1 round trip (from leader)
- **Throughput**: Limited by network and storage I/O
- **Scalability**: Linear with cluster size (for reads)

## Data Partitioning

### Consistent Hashing

Data is partitioned using consistent hashing with virtual nodes:

```rust
// Each physical node gets multiple virtual nodes on the hash ring
// This ensures even data distribution
partitioning.add_node(node_id, 100).await?; // 100 virtual nodes
```

### Replication Strategy

Data is replicated to N nodes based on proximity on the hash ring:

```rust
// For replication_factor = 3
let nodes = partitioning.get_nodes_for_key(&key, 3);
// Returns 3 closest nodes on the ring
```

### Partition Management

```rust
// Check partition ownership
let is_owner = partitioning.is_node_responsible(&node_id, &key).await;

// Get partition statistics
let stats = partitioning.get_distribution_stats().await;
println!("Partition variance: {}", stats.variance());
```

## Replication & Fault Tolerance

### Replication Queue

Operations are queued and replicated asynchronously:

```rust
// Queue operation for replication
replication.replicate_operation(operation, &primary_node).await?;

// Check replication health
let health = replication.check_health().await;
if !health.is_healthy {
    println!("Warning: High replication lag");
}
```

### Failure Handling

Automatic failure detection and recovery:

```rust
// Node failure detected
replication.handle_node_failure(&failed_node).await?;

// Partitions redistributed
partitioning.rebalance().await?;
```

### Consistency Levels

Choose appropriate consistency for your use case:

- **Strong Consistency**: Wait for majority acknowledgment
- **Eventual Consistency**: Asynchronous replication
- **Read-Your-Writes**: Read from primary replica

## Network Communication

### gRPC Protocol

All communication uses efficient protobuf messages:

```protobuf
service ClusterService {
  rpc RequestVote(VoteRequest) returns (VoteResponse);
  rpc AppendEntries(AppendEntriesRequest) returns (AppendEntriesResponse);
  rpc Heartbeat(HeartbeatRequest) returns (HeartbeatResponse);
  rpc ExecuteOperation(ClientRequest) returns (ClientResponse);
}
```

### Connection Management

Automatic connection handling with reconnection:

```rust
// Connect to cluster node
network.connect_to_node(node_id, "127.0.0.1:8080".to_string()).await?;

// Send Raft message
network.send_raft_message(&target_node, message).await?;
```

## Monitoring & Observability

### Cluster Metrics

```rust
let status = cluster.get_status().await;

println!("Cluster Status:");
println!("  Leader: {:?}", status.leader.map(|n| n.0));
println!("  Active Nodes: {}", status.membership.active_nodes);
println!("  Failed Nodes: {}", status.membership.failed_nodes);
println!("  Replication Lag: {:?}", status.replication.replication_lag);
```

### Health Checks

```rust
// Check cluster health
let health = cluster.check_health().await;
if !health.is_healthy {
    // Alert or take corrective action
    println!("Cluster unhealthy: {} failed nodes", health.failed_nodes_count);
}
```

### Event Subscription

```rust
// Subscribe to cluster events
let mut events = cluster.subscribe_events().await;
while let Ok(event) = events.recv().await {
    match event {
        ClusterEvent::NodeJoined(node) => log::info!("Node joined: {}", node.0),
        ClusterEvent::NodeFailed(node) => log::error!("Node failed: {}", node.0),
        ClusterEvent::LeaderElected(node) => log::info!("New leader: {}", node.0),
        _ => {}
    }
}
```

## Deployment Patterns

### Single Region Cluster

```
┌─────────┐    ┌─────────┐    ┌─────────┐
│ Node 1  │◄──►│ Node 2  │◄──►│ Node 3  │
│ Leader  │    │ Follower│    │ Follower│
└─────────┘    └─────────┘    └─────────┘
```

### Multi-Region Cluster

```
Region A                    Region B
┌─────────┐    ┌─────────┐    ┌─────────┐
│ Node 1  │◄──►│ Node 2  │◄──►│ Node 4  │
│ Leader  │    │ Follower│    │ Follower│
└─────────┘    └─────────┘    └─────────┘
                              │
                              ▼
                       ┌─────────┐
                       │ Node 5  │
                       │ Follower│
                       └─────────┘
```

### Development Setup

```bash
# Start 3-node cluster for development
./kotoba-cluster --node-id=node1 --address=127.0.0.1:8080 --peers=127.0.0.1:8081,127.0.0.1:8082 &
./kotoba-cluster --node-id=node2 --address=127.0.0.1:8081 --peers=127.0.0.1:8080,127.0.0.1:8082 &
./kotoba-cluster --node-id=node3 --address=127.0.0.1:8082 --peers=127.0.0.1:8080,127.0.0.1:8081 &
```

## Performance Tuning

### Network Optimization

```rust
// Increase connection pool size
// Configure keep-alive settings
// Use connection multiplexing
```

### Storage Optimization

```rust
// Tune LSM compaction settings
// Configure bloom filter sizes
// Optimize WAL sync intervals
```

### Consensus Tuning

```rust
// Adjust election timeouts
// Configure heartbeat intervals
// Tune batch sizes
```

## Error Handling

### Common Errors

```rust
match cluster.execute_operation(operation).await {
    Ok(cid) => println!("Success: {}", cid),
    Err(ClusterError::NotLeader(leader)) => {
        // Redirect to leader
        println!("Redirect to leader: {}", leader);
    }
    Err(ClusterError::NoLeader) => {
        // Wait for leader election
        println!("Waiting for leader election...");
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    Err(ClusterError::NetworkError(e)) => {
        // Retry with backoff
        println!("Network error, retrying: {}", e);
    }
    _ => println!("Other error occurred"),
}
```

## Future Enhancements

- **Multi-Raft**: Multiple independent Raft groups
- **Witness Nodes**: Non-voting nodes for read scaling
- **Dynamic Membership**: Add/remove nodes without restart
- **Cross-DC Replication**: Geographic replication
- **Query Optimization**: Distributed query planning
- **Backup/Restore**: Cluster-wide backup utilities

## Contributing

1. Fork the repository
2. Create a feature branch
3. Add comprehensive tests
4. Update documentation
5. Submit a pull request

## License

Licensed under the MIT License.

---

**KotobaDB Cluster** - *Distributed graph database with strong consistency and high availability* 🚀

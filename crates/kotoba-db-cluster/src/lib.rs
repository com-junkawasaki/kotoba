//! # KotobaDB Cluster
//!
//! Distributed clustering and consensus implementation for KotobaDB.
//! Provides high availability, fault tolerance, and horizontal scalability.
//!
//! ## Features
//!
//! - **Raft Consensus**: Leader election and log replication
//! - **Automatic Failover**: Transparent leader failover
//! - **Horizontal Scaling**: Data partitioning across nodes
//! - **Fault Tolerance**: Survives node failures
//! - **Eventual Consistency**: Tunable consistency levels

pub mod consensus;
pub mod membership;
pub mod networking;
pub mod partitioning;
pub mod replication;

#[cfg(feature = "full")]
pub mod cluster;

pub use consensus::*;
pub use membership::*;
pub use networking::*;
pub use partitioning::*;
pub use replication::*;

#[cfg(feature = "full")]
pub use cluster::*;

// Include generated protobuf types directly
include!("cluster.rs");

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

// Use protobuf-generated types directly
pub use cluster::*;

/// Partition identifier for data sharding
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PartitionId(pub u32);

/// Cluster state
#[derive(Debug)]
pub struct ClusterState {
    pub config: Arc<RwLock<cluster::ClusterConfig>>,
    pub local_node: NodeId,
    pub consensus_state: Arc<RwLock<ConsensusState>>,
    pub partition_table: Arc<RwLock<PartitionTable>>,
}

impl ClusterState {
    /// Create a new cluster state
    pub fn new(local_node: NodeId) -> Self {
        Self {
            config: Arc::new(RwLock::new(cluster::ClusterConfig {
                nodes: Vec::new(),
                version: 0,
            })),
            local_node,
            consensus_state: Arc::new(RwLock::new(ConsensusState::new())),
            partition_table: Arc::new(RwLock::new(PartitionTable::new())),
        }
    }

    /// Get the current leader node
    pub async fn get_leader(&self) -> Option<NodeId> {
        let consensus = self.consensus_state.read().await;
        consensus.current_leader.clone()
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self) -> bool {
        let consensus = self.consensus_state.read().await;
        consensus.current_leader.as_ref() == Some(&self.local_node)
    }

    /// Get partition assignment for a key
    pub async fn get_partition_for_key(&self, key: &[u8]) -> PartitionId {
        let table = self.partition_table.read().await;
        table.get_partition(key)
    }

    /// Get nodes responsible for a partition
    pub async fn get_nodes_for_partition(&self, _partition: &PartitionId) -> Vec<NodeId> {
        // TODO: Implement partition-based node lookup
        // For now, return all nodes
        let config = self.config.read().await;
        config.nodes.iter()
            .map(|node| node.id.clone())
            .collect()
    }
}

/// Consensus algorithm state (simplified Raft)
#[derive(Debug)]
pub struct ConsensusState {
    pub current_term: u64,
    pub voted_for: Option<NodeId>,
    pub current_leader: Option<NodeId>,
    pub log: Vec<cluster::LogEntry>,
    pub commit_index: u64,
    pub last_applied: u64,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            current_leader: None,
            log: Vec::new(),
            commit_index: 0,
            last_applied: 0,
        }
    }
}


/// Partition table for data distribution
#[derive(Debug)]
pub struct PartitionTable {
    partition_count: usize,
}

impl PartitionTable {
    pub fn new() -> Self {
        Self {
            partition_count: 64, // Default partition count
        }
    }

    /// Get partition for a key using consistent hashing
    pub fn get_partition(&self, key: &[u8]) -> PartitionId {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let hash = hasher.finish();

        PartitionId((hash % self.partition_count as u64) as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_assignment() {
        let table = PartitionTable::new();

        let key1 = b"alice";
        let key2 = b"bob";
        let key3 = b"alice"; // Same key should get same partition

        let partition1 = table.get_partition(key1);
        let partition2 = table.get_partition(key2);
        let partition3 = table.get_partition(key3);

        assert_eq!(partition1, partition3); // Same key, same partition
        assert_ne!(partition1, partition2); // Different keys, likely different partitions
    }

    #[test]
    fn test_cluster_state_creation() {
        let node_id = NodeId("node-1".to_string());
        let state = ClusterState::new(node_id.clone());

        assert_eq!(state.local_node, node_id);
    }
}

//! # Data Partitioning
//!
//! Data distribution and partitioning strategies for KotobaDB cluster.
//! Provides consistent hashing and partition management.

use crate::*;
use std::collections::{HashMap, BTreeMap};
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Partition manager for data distribution
pub struct PartitionManager {
    /// Partition table mapping partitions to nodes
    partition_table: Arc<RwLock<PartitionTable>>,
    /// Replication factor for data redundancy
    replication_factor: usize,
    /// Virtual nodes per physical node for better distribution
    virtual_nodes: usize,
}

impl PartitionManager {
    /// Create a new partition manager
    pub fn new(replication_factor: usize) -> Self {
        Self {
            partition_table: Arc::new(RwLock::new(PartitionTable::new())),
            replication_factor,
            virtual_nodes: 100, // Default virtual nodes
        }
    }

    /// Add a node to the partition ring
    pub async fn add_node(&self, node_id: NodeId) -> Result<(), PartitionError> {
        let mut table = self.partition_table.write().await;
        table.add_node(node_id, self.virtual_nodes);
        Ok(())
    }

    /// Remove a node from the partition ring
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<(), PartitionError> {
        let mut table = self.partition_table.write().await;
        table.remove_node(node_id);
        Ok(())
    }

    /// Get nodes responsible for a key
    pub async fn get_nodes_for_key(&self, key: &[u8]) -> Vec<NodeId> {
        let table = self.partition_table.read().await;
        table.get_nodes_for_key(key, self.replication_factor)
    }

    /// Get partition for a key
    pub async fn get_partition_for_key(&self, key: &[u8]) -> PartitionId {
        let table = self.partition_table.read().await;
        table.get_partition(key)
    }

    /// Get all partitions assigned to a node
    pub async fn get_partitions_for_node(&self, node_id: &NodeId) -> Vec<PartitionId> {
        let table = self.partition_table.read().await;
        table.get_partitions_for_node(node_id)
    }

    /// Check if a node is responsible for a key
    pub async fn is_node_responsible(&self, node_id: &NodeId, key: &[u8]) -> bool {
        let nodes = self.get_nodes_for_key(key).await;
        nodes.contains(node_id)
    }

    /// Get partition distribution statistics
    pub async fn get_distribution_stats(&self) -> PartitionStats {
        let table = self.partition_table.read().await;
        table.get_distribution_stats()
    }

    /// Rebalance partitions after node changes
    pub async fn rebalance(&self) -> Result<(), PartitionError> {
        let mut table = self.partition_table.write().await;
        table.rebalance();
        Ok(())
    }
}

/// Consistent hashing ring for partition distribution
pub struct PartitionTable {
    /// Hash ring: hash -> node
    ring: BTreeMap<u64, NodeId>,
    /// Node to partitions mapping
    node_partitions: HashMap<NodeId, Vec<PartitionId>>,
    /// Partition to nodes mapping
    partition_nodes: HashMap<PartitionId, Vec<NodeId>>,
    /// Total number of partitions
    partition_count: usize,
}

impl PartitionTable {
    /// Create a new partition table
    pub fn new() -> Self {
        Self {
            ring: BTreeMap::new(),
            node_partitions: HashMap::new(),
            partition_nodes: HashMap::new(),
            partition_count: 64, // Default partitions
        }
    }

    /// Add a node with virtual nodes to the ring
    pub fn add_node(&mut self, node_id: NodeId, virtual_nodes: usize) {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Add virtual nodes to the ring
        for i in 0..virtual_nodes {
            hasher.write(node_id.0.as_bytes());
            hasher.write(&i.to_le_bytes());
            let hash = hasher.finish();

            self.ring.insert(hash, node_id.clone());
        }

        // Initialize node partitions
        self.node_partitions.insert(node_id, Vec::new());

        // Rebalance partitions
        self.rebalance();
    }

    /// Remove a node from the ring
    pub fn remove_node(&mut self, node_id: &NodeId) {
        // Remove all virtual nodes for this physical node
        self.ring.retain(|_, node| node != node_id);

        // Remove from node mappings
        self.node_partitions.remove(node_id);

        // Remove from partition mappings
        for nodes in self.partition_nodes.values_mut() {
            nodes.retain(|node| node != node_id);
        }

        // Rebalance partitions
        self.rebalance();
    }

    /// Get nodes responsible for a key (with replication)
    pub fn get_nodes_for_key(&self, key: &[u8], replication_factor: usize) -> Vec<NodeId> {
        if self.ring.is_empty() {
            return Vec::new();
        }

        let key_hash = self.hash_key(key);
        let mut nodes = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Find the first node clockwise from the key hash
        let mut current_hash = *self.ring.range(key_hash..).next().unwrap_or_else(|| self.ring.iter().next().unwrap()).0;

        // Walk the ring to find replication_factor unique nodes
        for _ in 0..(self.ring.len().min(replication_factor * 2)) {
            if let Some(node) = self.ring.get(&current_hash) {
                if seen.insert(node.clone()) {
                    nodes.push(node.clone());
                    if nodes.len() >= replication_factor {
                        break;
                    }
                }
            }

            // Move to next hash
            let next_hash = self.ring.range(current_hash + 1..).next()
                .unwrap_or_else(|| self.ring.iter().next().unwrap()).0;
            current_hash = *next_hash;
        }

        nodes
    }

    /// Get partition for a key
    pub fn get_partition(&self, key: &[u8]) -> PartitionId {
        let key_hash = self.hash_key(key);
        PartitionId((key_hash % self.partition_count as u64) as u32)
    }

    /// Get partitions assigned to a node
    pub fn get_partitions_for_node(&self, node_id: &NodeId) -> Vec<PartitionId> {
        self.node_partitions.get(node_id).cloned().unwrap_or_default()
    }

    /// Rebalance partitions across nodes
    pub fn rebalance(&mut self) {
        if self.ring.is_empty() {
            return;
        }

        // Clear existing assignments
        for partitions in self.node_partitions.values_mut() {
            partitions.clear();
        }
        self.partition_nodes.clear();

        // Assign each partition to nodes
        for partition in 0..self.partition_count {
            let partition_id = PartitionId(partition as u32);
            let key = format!("partition-{}", partition).into_bytes();
            let nodes = self.get_nodes_for_key(&key, 3); // 3 replicas

            // Update mappings
            for node in &nodes {
                self.node_partitions.get_mut(node).unwrap().push(partition_id);
            }
            self.partition_nodes.insert(partition_id, nodes);
        }
    }

    /// Get distribution statistics
    pub fn get_distribution_stats(&self) -> PartitionStats {
        let mut node_counts = HashMap::new();
        let mut partition_counts = HashMap::new();

        for (node, partitions) in &self.node_partitions {
            node_counts.insert(node.clone(), partitions.len());
        }

        for (partition, nodes) in &self.partition_nodes {
            partition_counts.insert(*partition, nodes.len());
        }

        PartitionStats {
            total_partitions: self.partition_count,
            total_nodes: self.node_partitions.len(),
            node_partition_counts: node_counts,
            partition_replica_counts: partition_counts,
        }
    }

    /// Hash a key for consistent hashing
    fn hash_key(&self, key: &[u8]) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }
}

/// Statistics about partition distribution
#[derive(Debug, Clone)]
pub struct PartitionStats {
    pub total_partitions: usize,
    pub total_nodes: usize,
    pub node_partition_counts: HashMap<NodeId, usize>,
    pub partition_replica_counts: HashMap<PartitionId, usize>,
}

impl PartitionStats {
    /// Get the most loaded node
    pub fn most_loaded_node(&self) -> Option<(&NodeId, usize)> {
        self.node_partition_counts.iter()
            .max_by_key(|(_, count)| *count)
            .map(|(node, count)| (node, *count))
    }

    /// Get the least loaded node
    pub fn least_loaded_node(&self) -> Option<(&NodeId, usize)> {
        self.node_partition_counts.iter()
            .min_by_key(|(_, count)| *count)
            .map(|(node, count)| (node, *count))
    }

    /// Calculate distribution variance
    pub fn variance(&self) -> f64 {
        if self.total_nodes == 0 {
            return 0.0;
        }

        let mean = self.total_partitions as f64 / self.total_nodes as f64;
        let variance = self.node_partition_counts.values()
            .map(|count| {
                let diff = *count as f64 - mean;
                diff * diff
            })
            .sum::<f64>() / self.total_nodes as f64;

        variance
    }
}

/// Partition-related errors
#[derive(Debug, thiserror::Error)]
pub enum PartitionError {
    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Partition not found: {0}")]
    PartitionNotFound(u32),

    #[error("Rebalancing failed: {0}")]
    RebalanceFailed(String),

    #[error("Invalid replication factor: {0}")]
    InvalidReplicationFactor(usize),
}

/// Key range for partition management
#[derive(Debug, Clone)]
pub struct KeyRange {
    pub start: Vec<u8>,
    pub end: Vec<u8>,
}

impl KeyRange {
    /// Check if a key falls within this range
    pub fn contains(&self, key: &[u8]) -> bool {
        key >= &self.start && key < &self.end
    }

    /// Split range into two halves
    pub fn split(&self) -> (KeyRange, KeyRange) {
        let mid = (self.start.len() + self.end.len()) / 2;
        let mid_key = if mid < self.start.len() {
            self.start[mid..].to_vec()
        } else {
            self.end[..mid - self.start.len()].to_vec()
        };

        let left = KeyRange {
            start: self.start.clone(),
            end: mid_key.clone(),
        };

        let right = KeyRange {
            start: mid_key,
            end: self.end.clone(),
        };

        (left, right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partition_assignment() {
        let manager = PartitionManager::new(3);

        // Add nodes
        let node1 = NodeId("node-1".to_string());
        let node2 = NodeId("node-2".to_string());
        let node3 = NodeId("node-3".to_string());

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            manager.add_node(node1.clone()).await.unwrap();
            manager.add_node(node2.clone()).await.unwrap();
            manager.add_node(node3.clone()).await.unwrap();

            // Test key assignment
            let key1 = b"alice";
            let key2 = b"bob";

            let nodes1 = manager.get_nodes_for_key(key1).await;
            let nodes2 = manager.get_nodes_for_key(key2).await;

            assert_eq!(nodes1.len(), 3); // Replication factor
            assert_eq!(nodes2.len(), 3);

            // Different keys should have some overlap but not be identical
            assert_ne!(nodes1, nodes2);
        });
    }

    #[test]
    fn test_partition_stats() {
        let manager = PartitionManager::new(2);

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            // Add nodes
            manager.add_node(NodeId("node-1".to_string())).await.unwrap();
            manager.add_node(NodeId("node-2".to_string())).await.unwrap();

            let stats = manager.get_distribution_stats().await;

            assert_eq!(stats.total_nodes, 2);
            assert!(stats.total_partitions > 0);
            assert!(stats.variance() >= 0.0);
        });
    }

    #[test]
    fn test_key_range() {
        let range = KeyRange {
            start: b"alice".to_vec(),
            end: b"zebra".to_vec(),
        };

        assert!(range.contains(b"bob"));
        assert!(range.contains(b"alice"));
        assert!(!range.contains(b"zebra"));
        assert!(!range.contains(b"aaa"));
    }
}

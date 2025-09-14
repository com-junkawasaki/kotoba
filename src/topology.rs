//! Topology management for distributed Kotoba clusters
//!
//! This module handles cluster topology, node discovery, and network configuration.

/// Cluster topology representation
pub struct ClusterTopology {
    /// List of cluster nodes
    pub nodes: Vec<ClusterNode>,
    /// Network configuration
    pub network_config: NetworkConfig,
}

/// Individual cluster node
pub struct ClusterNode {
    /// Node ID
    pub id: String,
    /// Node address
    pub address: String,
    /// Node capabilities
    pub capabilities: NodeCapabilities,
}

/// Node capabilities
pub struct NodeCapabilities {
    /// CPU cores
    pub cpu_cores: usize,
    /// Memory in MB
    pub memory_mb: usize,
    /// Storage in GB
    pub storage_gb: usize,
}

/// Network configuration
pub struct NetworkConfig {
    /// Cluster subnet
    pub subnet: String,
    /// Communication ports
    pub ports: Vec<u16>,
}

impl Default for ClusterTopology {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            network_config: NetworkConfig::default(),
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            subnet: "10.0.0.0/16".to_string(),
            ports: vec![8080, 8443],
        }
    }
}

impl ClusterTopology {
    /// Create a new empty cluster topology
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a node to the cluster
    pub fn add_node(&mut self, node: ClusterNode) {
        self.nodes.push(node);
    }

    /// Remove a node from the cluster
    pub fn remove_node(&mut self, node_id: &str) {
        self.nodes.retain(|node| node.id != node_id);
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&ClusterNode> {
        self.nodes.iter().find(|node| node.id == node_id)
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> &[ClusterNode] {
        &self.nodes
    }

    /// Get node count
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
}
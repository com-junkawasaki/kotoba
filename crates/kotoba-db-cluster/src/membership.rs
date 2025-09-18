//! # Cluster Membership Management
//!
//! Manages cluster membership, node discovery, and cluster configuration.
//! Handles node joins, leaves, failures, and configuration updates.

use crate::*;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc, broadcast};
use tokio::time::{self, MissedTickBehavior};

/// Cluster membership manager
pub struct MembershipManager {
    /// Cluster state
    cluster_state: Arc<ClusterState>,
    /// Membership configuration
    config: MembershipConfig,
    /// Node heartbeat tracking
    heartbeats: Arc<RwLock<HashMap<NodeId, HeartbeatInfo>>>,
    /// Membership change notifications
    membership_tx: broadcast::Sender<MembershipEvent>,
    membership_rx: broadcast::Receiver<MembershipEvent>,
    /// Command channel
    command_tx: mpsc::Sender<MembershipCommand>,
    command_rx: mpsc::Receiver<MembershipCommand>,
}

impl MembershipManager {
    /// Create a new membership manager
    pub fn new(cluster_state: Arc<ClusterState>, config: MembershipConfig) -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);
        let (membership_tx, membership_rx) = broadcast::channel(100);

        Self {
            cluster_state,
            config,
            heartbeats: Arc::new(RwLock::new(HashMap::new())),
            membership_tx,
            membership_rx,
            command_tx,
            command_rx,
        }
    }

    /// Start membership management processes
    pub async fn start(&mut self) -> Result<(), MembershipError> {
        // Clone state for async tasks
        let cluster_state = Arc::clone(&self.cluster_state);
        let membership_tx = self.membership_tx.clone();
        let command_tx = self.command_tx.clone();

        // Start heartbeat monitoring
        let heartbeat_handle = self.start_heartbeat_monitor(cluster_state.clone(), membership_tx.clone());

        // Start failure detection
        let failure_handle = self.start_failure_detector(cluster_state.clone(), membership_tx.clone());

        // Start command processor
        let command_handle = self.start_command_processor(cluster_state, command_tx, &mut self.command_rx);

        // Wait for all tasks
        tokio::try_join!(heartbeat_handle, failure_handle, command_handle)?;

        Ok(())
    }

    /// Add a node to the cluster
    pub async fn add_node(&self, node_id: NodeId, node_info: NodeInfo) -> Result<(), MembershipError> {
        let mut config = self.cluster_state.config.write().await;

        // Check if node already exists
        if config.nodes.contains_key(&node_id) {
            return Err(MembershipError::NodeAlreadyExists(node_id.0));
        }

        // Add node to configuration
        config.nodes.insert(node_id.clone(), node_info);

        // Initialize heartbeat tracking
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.insert(node_id.clone(), HeartbeatInfo::new());

        // Notify listeners
        let _ = self.membership_tx.send(MembershipEvent::NodeJoined(node_id));

        println!("Node {} joined the cluster", node_id.0.clone());
        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: &NodeId) -> Result<(), MembershipError> {
        let mut config = self.cluster_state.config.write().await;

        // Check if node exists
        if !config.nodes.contains_key(node_id) {
            return Err(MembershipError::NodeNotFound(node_id.0.clone()));
        }

        // Remove node from configuration
        config.nodes.remove(node_id);

        // Remove heartbeat tracking
        let mut heartbeats = self.heartbeats.write().await;
        heartbeats.remove(node_id);

        // Notify listeners
        let _ = self.membership_tx.send(MembershipEvent::NodeLeft(node_id.clone()));

        println!("Node {} left the cluster", node_id.0);
        Ok(())
    }

    /// Update node information
    pub async fn update_node(&self, node_id: &NodeId, node_info: NodeInfo) -> Result<(), MembershipError> {
        let mut config = self.cluster_state.config.write().await;

        // Check if node exists
        if !config.nodes.contains_key(node_id) {
            return Err(MembershipError::NodeNotFound(node_id.0.clone()));
        }

        // Update node information
        config.nodes.insert(node_id.clone(), node_info);

        println!("Node {} information updated", node_id.0);
        Ok(())
    }

    /// Record a heartbeat from a node
    pub async fn record_heartbeat(&self, node_id: &NodeId) -> Result<(), MembershipError> {
        let mut heartbeats = self.heartbeats.write().await;

        if let Some(info) = heartbeats.get_mut(node_id) {
            info.last_heartbeat = Instant::now();
            info.missed_heartbeats = 0;
            Ok(())
        } else {
            Err(MembershipError::NodeNotFound(node_id.0.clone()))
        }
    }

    /// Get current cluster configuration
    pub async fn get_cluster_config(&self) -> ClusterConfig {
        self.cluster_state.config.read().await.clone()
    }

    /// Get list of active nodes
    pub async fn get_active_nodes(&self) -> Vec<NodeId> {
        let heartbeats = self.heartbeats.read().await;
        let config = self.cluster_state.config.read().await;

        config.nodes.keys()
            .filter(|node_id| {
                heartbeats.get(node_id)
                    .map(|info| info.is_alive())
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Check if a node is active
    pub async fn is_node_active(&self, node_id: &NodeId) -> bool {
        let heartbeats = self.heartbeats.read().await;
        heartbeats.get(node_id)
            .map(|info| info.is_alive())
            .unwrap_or(false)
    }

    /// Get cluster statistics
    pub async fn get_cluster_stats(&self) -> ClusterStats {
        let config = self.cluster_state.config.read().await;
        let heartbeats = self.heartbeats.read().await;

        let total_nodes = config.nodes.len();
        let active_nodes = heartbeats.values()
            .filter(|info| info.is_alive())
            .count();

        let suspected_nodes = heartbeats.values()
            .filter(|info| info.is_suspected())
            .count();

        let failed_nodes = total_nodes - active_nodes;

        ClusterStats {
            total_nodes,
            active_nodes,
            suspected_nodes,
            failed_nodes,
            replication_factor: config.replication_factor,
            partition_count: config.partition_count,
        }
    }

    /// Subscribe to membership events
    pub fn subscribe_events(&self) -> broadcast::Receiver<MembershipEvent> {
        self.membership_tx.subscribe()
    }

    // Internal methods

    async fn start_heartbeat_monitor(&self, cluster_state: Arc<ClusterState>, membership_tx: tokio::sync::mpsc::Sender<MembershipEvent>) -> Result<(), MembershipError> {
        let heartbeats = Arc::clone(&self.heartbeats);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(config.heartbeat_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;

                let mut heartbeats = heartbeats.write().await;
                let mut suspected_nodes = Vec::new();

                // Check for missed heartbeats
                for (node_id, info) in heartbeats.iter_mut() {
                    info.missed_heartbeats += 1;

                    if info.is_suspected() && !info.was_suspected {
                        info.was_suspected = true;
                        suspected_nodes.push(node_id.clone());
                    }
                }

                // Notify about suspected nodes
                for node_id in suspected_nodes {
                    let _ = membership_tx.send(MembershipEvent::NodeSuspected(node_id));
                }
            }
        });

        Ok(())
    }

    async fn start_failure_detector(&self, cluster_state: Arc<ClusterState>, membership_tx: tokio::sync::mpsc::Sender<MembershipEvent>) -> Result<(), MembershipError> {
        let heartbeats = Arc::clone(&self.heartbeats);
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(config.failure_detection_interval);
            interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

            loop {
                interval.tick().await;

                let heartbeats = heartbeats.read().await;
                let mut failed_nodes = Vec::new();

                // Check for failed nodes
                for (node_id, info) in heartbeats.iter() {
                    if info.is_failed() {
                        failed_nodes.push(node_id.clone());
                    }
                }

                // Notify about failed nodes
                for node_id in failed_nodes {
                    let _ = membership_tx.send(MembershipEvent::NodeFailed(node_id));
                }

                // Clean up old failed nodes after some time
                // TODO: Implement cleanup logic
            }
        });

        Ok(())
    }

    async fn start_command_processor(&self, cluster_state: Arc<ClusterState>, command_tx: tokio::sync::mpsc::Sender<MembershipCommand>, command_rx: &mut tokio::sync::mpsc::Receiver<MembershipCommand>) -> Result<(), MembershipError> {
        let membership_tx = tokio::sync::mpsc::Sender::clone(&command_tx);

        tokio::spawn(async move {
            while let Some(command) = command_rx.recv().await {
                match command {
                    MembershipCommand::AddNode { node_id, node_info, response_tx } => {
                        // TODO: Implement add_node logic
                        let _ = response_tx.send(Ok(()));
                    }
                    MembershipCommand::RemoveNode { node_id, response_tx } => {
                        // TODO: Implement remove_node logic
                        let _ = response_tx.send(Ok(()));
                    }
                }
            }
            Ok(())
        });

        Ok(())
    }

    /// Get command sender for external commands
    pub fn command_sender(&self) -> mpsc::Sender<MembershipCommand> {
        self.command_tx.clone()
    }
}

/// Membership configuration
#[derive(Debug, Clone)]
pub struct MembershipConfig {
    /// Heartbeat interval
    pub heartbeat_interval: Duration,
    /// Failure detection interval
    pub failure_detection_interval: Duration,
    /// Maximum missed heartbeats before suspecting failure
    pub max_missed_heartbeats: u32,
    /// Maximum suspected time before marking as failed
    pub failure_timeout: Duration,
    /// Gossip interval for membership propagation
    pub gossip_interval: Duration,
}

impl Default for MembershipConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval: Duration::from_secs(1),
            failure_detection_interval: Duration::from_secs(5),
            max_missed_heartbeats: 3,
            failure_timeout: Duration::from_secs(15),
            gossip_interval: Duration::from_secs(2),
        }
    }
}

/// Heartbeat information for a node
#[derive(Debug, Clone)]
pub struct HeartbeatInfo {
    pub last_heartbeat: Instant,
    pub missed_heartbeats: u32,
    pub was_suspected: bool,
}

impl HeartbeatInfo {
    pub fn new() -> Self {
        Self {
            last_heartbeat: Instant::now(),
            missed_heartbeats: 0,
            was_suspected: false,
        }
    }

    /// Check if node is considered alive
    pub fn is_alive(&self) -> bool {
        !self.is_suspected() && !self.is_failed()
    }

    /// Check if node is suspected of failure
    pub fn is_suspected(&self) -> bool {
        self.missed_heartbeats >= 1 // Simplified: any missed heartbeat
    }

    /// Check if node has failed
    pub fn is_failed(&self) -> bool {
        self.last_heartbeat.elapsed() > Duration::from_secs(30) // Simplified timeout
    }
}

/// Membership events
#[derive(Debug, Clone)]
pub enum MembershipEvent {
    NodeJoined(NodeId),
    NodeLeft(NodeId),
    NodeSuspected(NodeId),
    NodeFailed(NodeId),
    NodeRecovered(NodeId),
    ConfigChanged,
}

/// Membership commands
#[derive(Debug)]
pub enum MembershipCommand {
    AddNode {
        node_id: NodeId,
        node_info: NodeInfo,
        response_tx: tokio::sync::oneshot::Sender<Result<(), MembershipError>>,
    },
    RemoveNode {
        node_id: NodeId,
        response_tx: tokio::sync::oneshot::Sender<Result<(), MembershipError>>,
    },
    UpdateNode {
        node_id: NodeId,
        node_info: NodeInfo,
        response_tx: tokio::sync::oneshot::Sender<Result<(), MembershipError>>,
    },
}

/// Cluster statistics
#[derive(Debug, Clone)]
pub struct ClusterStats {
    pub total_nodes: usize,
    pub active_nodes: usize,
    pub suspected_nodes: usize,
    pub failed_nodes: usize,
    pub replication_factor: usize,
    pub partition_count: usize,
}

impl ClusterStats {
    /// Calculate cluster health score (0.0 to 1.0)
    pub fn health_score(&self) -> f64 {
        if self.total_nodes == 0 {
            return 1.0; // Empty cluster is "healthy"
        }

        let healthy_nodes = self.active_nodes as f64;
        let total_nodes = self.total_nodes as f64;

        healthy_nodes / total_nodes
    }

    /// Check if cluster has quorum
    pub fn has_quorum(&self) -> bool {
        let quorum_size = (self.total_nodes / 2) + 1;
        self.active_nodes >= quorum_size
    }

    /// Check if cluster can tolerate failures
    pub fn can_tolerate_failures(&self, max_failures: usize) -> bool {
        self.active_nodes > max_failures
    }
}

/// Membership-related errors
#[derive(Debug, thiserror::Error)]
pub enum MembershipError {
    #[error("Node already exists: {0}")]
    NodeAlreadyExists(String),

    #[error("Node not found: {0}")]
    NodeNotFound(String),

    #[error("Invalid node configuration: {0}")]
    InvalidNodeConfig(String),

    #[error("Membership operation timed out")]
    Timeout,

    #[error("Network communication error: {0}")]
    NetworkError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_info() {
        let info = HeartbeatInfo::new();

        assert!(info.is_alive());
        assert!(!info.is_suspected());
        assert!(!info.is_failed());
    }

    #[test]
    fn test_cluster_stats() {
        let stats = ClusterStats {
            total_nodes: 5,
            active_nodes: 4,
            suspected_nodes: 1,
            failed_nodes: 0,
            replication_factor: 3,
            partition_count: 64,
        };

        assert_eq!(stats.health_score(), 0.8);
        assert!(stats.has_quorum()); // 4 >= 3
        assert!(stats.can_tolerate_failures(1)); // 4 > 1
        assert!(!stats.can_tolerate_failures(4)); // 4 > 4 is false
    }

    #[tokio::test]
    async fn test_membership_manager_creation() {
        let cluster_state = Arc::new(ClusterState::new(NodeId("test".to_string())));
        let config = MembershipConfig::default();

        let manager = MembershipManager::new(cluster_state, config);

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 0); // No nodes added yet
        assert_eq!(stats.active_nodes, 0);
    }

    #[tokio::test]
    async fn test_add_remove_node() {
        let cluster_state = Arc::new(ClusterState::new(NodeId("test".to_string())));
        let config = MembershipConfig::default();

        let manager = MembershipManager::new(cluster_state, config);

        let node_id = NodeId("node-1".to_string());
        let node_info = NodeInfo {
            id: node_id.clone(),
            address: "127.0.0.1".to_string(),
            port: 8080,
            role: NodeRole::Follower,
            partitions: vec![],
        };

        // Add node
        manager.add_node(node_id.clone(), node_info.clone()).await.unwrap();

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 1);

        // Remove node
        manager.remove_node(&node_id).await.unwrap();

        let stats = manager.get_cluster_stats().await;
        assert_eq!(stats.total_nodes, 0);
    }
}

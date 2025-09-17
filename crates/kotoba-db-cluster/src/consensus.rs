//! # Raft Consensus Algorithm Implementation
//!
//! This module implements the Raft consensus algorithm for KotobaDB cluster.
//! Raft provides strong consistency guarantees and fault tolerance.

use crate::*;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, mpsc};
use tokio::time::{self, MissedTickBehavior};

/// Raft consensus implementation
pub struct RaftConsensus {
    /// Cluster state
    state: Arc<RwLock<ConsensusState>>,
    /// Election timeout range
    election_timeout: (Duration, Duration),
    /// Heartbeat interval
    heartbeat_interval: Duration,
    /// Node communication channels
    node_channels: HashMap<NodeId, mpsc::Sender<RaftMessage>>,
    /// Command channel for client requests
    command_tx: mpsc::Sender<ConsensusCommand>,
    command_rx: mpsc::Receiver<ConsensusCommand>,
}

impl RaftConsensus {
    /// Create a new Raft consensus instance
    pub fn new() -> Self {
        let (command_tx, command_rx) = mpsc::channel(100);

        Self {
            state: Arc::new(RwLock::new(ConsensusState::new())),
            election_timeout: (Duration::from_millis(150), Duration::from_millis(300)),
            heartbeat_interval: Duration::from_millis(50),
            node_channels: HashMap::new(),
            command_tx,
            command_rx,
        }
    }

    /// Start the consensus algorithm
    pub async fn start(&mut self, cluster_state: Arc<ClusterState>) -> Result<(), ConsensusError> {
        // Start election timer
        let election_handle = self.start_election_timer(cluster_state.clone());

        // Start heartbeat timer (only for leader)
        let heartbeat_handle = self.start_heartbeat_timer(cluster_state.clone());

        // Start command processor
        let command_handle = self.start_command_processor(cluster_state);

        // Wait for all tasks
        tokio::try_join!(election_handle, heartbeat_handle, command_handle)?;

        Ok(())
    }

    /// Start election timer task
    async fn start_election_timer(&self, cluster_state: Arc<ClusterState>) -> Result<(), ConsensusError> {
        let mut interval = time::interval(self.random_election_timeout());
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let mut state = self.state.write().await;

            // Only start election if we're not leader and haven't voted in current term
            if state.current_leader.is_none() && state.voted_for.is_none() {
                self.start_election(&mut state, &cluster_state).await?;
            }
        }
    }

    /// Start heartbeat timer task
    async fn start_heartbeat_timer(&self, cluster_state: Arc<ClusterState>) -> Result<(), ConsensusError> {
        let mut interval = time::interval(self.heartbeat_interval);
        interval.set_missed_tick_behavior(MissedTickBehavior::Delay);

        loop {
            interval.tick().await;

            let state = self.state.read().await;

            // Only send heartbeats if we're the leader
            if let Some(ref leader) = state.current_leader {
                if leader == &cluster_state.local_node {
                    self.send_heartbeats(&cluster_state).await?;
                }
            }
        }
    }

    /// Start command processor task
    async fn start_command_processor(&mut self, cluster_state: Arc<ClusterState>) -> Result<(), ConsensusError> {
        while let Some(command) = self.command_rx.recv().await {
            match command {
                ConsensusCommand::ProposeOperation { operation, response_tx } => {
                    let result = self.propose_operation(operation, &cluster_state).await;
                    let _ = response_tx.send(result);
                }
                ConsensusCommand::AddNode { node_id, response_tx } => {
                    let result = self.add_node(node_id, &cluster_state).await;
                    let _ = response_tx.send(result);
                }
                ConsensusCommand::RemoveNode { node_id, response_tx } => {
                    let result = self.remove_node(node_id, &cluster_state).await;
                    let _ = response_tx.send(result);
                }
            }
        }

        Ok(())
    }

    /// Generate random election timeout
    fn random_election_timeout(&self) -> Duration {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let millis = rng.gen_range(self.election_timeout.0.as_millis()..=self.election_timeout.1.as_millis());
        Duration::from_millis(millis as u64)
    }

    /// Start leader election
    async fn start_election(&self, state: &mut ConsensusState, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        state.current_term += 1;
        state.voted_for = Some(cluster_state.local_node.clone());
        state.current_leader = None;

        let last_log_index = state.log.len() as u64;
        let last_log_term = state.log.last().map(|entry| entry.term).unwrap_or(0);

        let request = VoteRequest {
            term: state.current_term,
            candidate_id: cluster_state.local_node.clone(),
            last_log_index,
            last_log_term,
        };

        // Send vote requests to all other nodes
        let mut votes = 1; // Vote for ourselves
        let mut responses = Vec::new();

        for (node_id, sender) in &self.node_channels {
            if node_id != &cluster_state.local_node {
                let message = RaftMessage::VoteRequest(request.clone());
                if sender.send(message).await.is_ok() {
                    responses.push(node_id.clone());
                }
            }
        }

        // TODO: Collect votes and become leader if majority
        // For now, assume we become leader immediately (single node cluster)

        if votes > responses.len() / 2 {
            state.current_leader = Some(cluster_state.local_node.clone());
            println!("Node {} became leader for term {}", cluster_state.local_node.0, state.current_term);
        }

        Ok(())
    }

    /// Send heartbeat to all followers
    async fn send_heartbeats(&self, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        let state = self.state.read().await;

        for (node_id, sender) in &self.node_channels {
            if node_id != &cluster_state.local_node {
                let request = AppendEntriesRequest {
                    term: state.current_term,
                    leader_id: cluster_state.local_node.clone(),
                    prev_log_index: state.log.len() as u64,
                    prev_log_term: state.log.last().map(|entry| entry.term).unwrap_or(0),
                    entries: Vec::new(), // Empty for heartbeat
                    leader_commit: state.commit_index,
                };

                let message = RaftMessage::AppendEntriesRequest(request);
                let _ = sender.send(message).await; // Ignore send errors for now
            }
        }

        Ok(())
    }

    /// Propose a new operation to be replicated
    async fn propose_operation(&self, operation: Operation, cluster_state: &ClusterState) -> Result<String, ConsensusError> {
        let mut state = self.state.write().await;

        // Only leader can accept new operations
        if state.current_leader.as_ref() != Some(&cluster_state.local_node) {
            return Err(ConsensusError::NotLeader);
        }

        // Create log entry
        let entry = LogEntry {
            term: state.current_term,
            index: state.log.len() as u64 + 1,
            operation,
        };

        // Append to local log
        state.log.push(entry.clone());

        // TODO: Replicate to followers and wait for majority

        // For now, immediately commit (single node)
        state.commit_index = entry.index;
        state.last_applied = entry.index;

        // Generate CID for the operation result
        // This is a simplified version - in reality, we'd compute the actual CID
        let cid = format!("cid-{}-{}", entry.term, entry.index);

        Ok(cid)
    }

    /// Add a new node to the cluster
    async fn add_node(&self, node_id: NodeId, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        // TODO: Implement node addition with joint consensus
        println!("Adding node {} to cluster", node_id.0);
        Ok(())
    }

    /// Remove a node from the cluster
    async fn remove_node(&self, node_id: NodeId, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        // TODO: Implement node removal
        println!("Removing node {} from cluster", node_id.0);
        Ok(())
    }

    /// Handle incoming Raft messages
    pub async fn handle_message(&self, message: RaftMessage, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        match message {
            RaftMessage::VoteRequest(request) => {
                self.handle_vote_request(request, cluster_state).await
            }
            RaftMessage::VoteResponse(response) => {
                self.handle_vote_response(response, cluster_state).await
            }
            RaftMessage::AppendEntriesRequest(request) => {
                self.handle_append_entries_request(request, cluster_state).await
            }
            RaftMessage::AppendEntriesResponse(response) => {
                self.handle_append_entries_response(response, cluster_state).await
            }
        }
    }

    /// Handle vote request from candidate
    async fn handle_vote_request(&self, request: VoteRequest, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        let mut state = self.state.write().await;

        let grant_vote = request.term > state.current_term &&
            (state.voted_for.is_none() || state.voted_for == Some(request.candidate_id.clone()));

        if grant_vote {
            state.current_term = request.term;
            state.voted_for = Some(request.candidate_id.clone());
        }

        let response = VoteResponse {
            term: state.current_term,
            vote_granted: grant_vote,
        };

        // Send response back
        if let Some(sender) = self.node_channels.get(&request.candidate_id) {
            let message = RaftMessage::VoteResponse(response);
            let _ = sender.send(message).await;
        }

        Ok(())
    }

    /// Handle vote response
    async fn handle_vote_response(&self, response: VoteResponse, _cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        let mut state = self.state.write().await;

        if response.term > state.current_term {
            state.current_term = response.term;
            state.voted_for = None;
            state.current_leader = None;
        }

        // TODO: Count votes and become leader if majority

        Ok(())
    }

    /// Handle append entries request from leader
    async fn handle_append_entries_request(&self, request: AppendEntriesRequest, cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        let mut state = self.state.write().await;

        // Reject if term is outdated
        if request.term < state.current_term {
            let response = AppendEntriesResponse {
                term: state.current_term,
                success: false,
                match_index: 0,
            };
            self.send_append_response(request.leader_id, response).await?;
            return Ok(());
        }

        // Update term and become follower
        if request.term > state.current_term {
            state.current_term = request.term;
            state.voted_for = None;
        }

        state.current_leader = Some(request.leader_id.clone());

        // TODO: Log replication logic

        let response = AppendEntriesResponse {
            term: state.current_term,
            success: true,
            match_index: request.prev_log_index + request.entries.len() as u64,
        };

        self.send_append_response(request.leader_id, response).await?;

        Ok(())
    }

    /// Handle append entries response
    async fn handle_append_entries_response(&self, response: AppendEntriesResponse, _cluster_state: &ClusterState) -> Result<(), ConsensusError> {
        let mut state = self.state.write().await;

        if response.term > state.current_term {
            state.current_term = response.term;
            state.voted_for = None;
            state.current_leader = None;
        }

        // TODO: Update match indices and commit logic

        Ok(())
    }

    /// Send append entries response
    async fn send_append_response(&self, target: NodeId, response: AppendEntriesResponse) -> Result<(), ConsensusError> {
        if let Some(sender) = self.node_channels.get(&target) {
            let message = RaftMessage::AppendEntriesResponse(response);
            let _ = sender.send(message).await;
        }
        Ok(())
    }

    /// Get command sender for client requests
    pub fn command_sender(&self) -> mpsc::Sender<ConsensusCommand> {
        self.command_tx.clone()
    }

    /// Add communication channel for a node
    pub fn add_node_channel(&mut self, node_id: NodeId, sender: mpsc::Sender<RaftMessage>) {
        self.node_channels.insert(node_id, sender);
    }
}

/// Messages exchanged between Raft nodes
#[derive(Debug, Clone)]
pub enum RaftMessage {
    VoteRequest(VoteRequest),
    VoteResponse(VoteResponse),
    AppendEntriesRequest(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),
}

/// Consensus commands from clients
#[derive(Debug)]
pub enum ConsensusCommand {
    ProposeOperation {
        operation: Operation,
        response_tx: tokio::sync::oneshot::Sender<Result<String, ConsensusError>>,
    },
    AddNode {
        node_id: NodeId,
        response_tx: tokio::sync::oneshot::Sender<Result<(), ConsensusError>>,
    },
    RemoveNode {
        node_id: NodeId,
        response_tx: tokio::sync::oneshot::Sender<Result<(), ConsensusError>>,
    },
}

/// Consensus errors
#[derive(Debug, thiserror::Error)]
pub enum ConsensusError {
    #[error("Not the current leader")]
    NotLeader,

    #[error("Operation timed out")]
    Timeout,

    #[error("Network communication error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Consensus protocol error: {0}")]
    Protocol(String),
}

/// Vote request message
#[derive(Debug, Clone)]
pub struct VoteRequest {
    pub term: u64,
    pub candidate_id: NodeId,
    pub last_log_index: u64,
    pub last_log_term: u64,
}

/// Vote response message
#[derive(Debug, Clone)]
pub struct VoteResponse {
    pub term: u64,
    pub vote_granted: bool,
}

/// Append entries request (log replication and heartbeat)
#[derive(Debug, Clone)]
pub struct AppendEntriesRequest {
    pub term: u64,
    pub leader_id: NodeId,
    pub prev_log_index: u64,
    pub prev_log_term: u64,
    pub entries: Vec<LogEntry>,
    pub leader_commit: u64,
}

/// Append entries response
#[derive(Debug, Clone)]
pub struct AppendEntriesResponse {
    pub term: u64,
    pub success: bool,
    pub match_index: u64,
}

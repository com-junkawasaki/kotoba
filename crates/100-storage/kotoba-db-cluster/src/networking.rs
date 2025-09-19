//! # Cluster Networking
//!
//! Network communication layer for KotobaDB cluster.
//! Provides gRPC-based communication between cluster nodes.

use crate::*;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tonic::{transport::Server, Request, Response, Status};
use futures::future;

/// Network service for cluster communication
pub struct ClusterNetwork {
    /// Local node information
    local_node: NodeId,
    /// Cluster state
    cluster_state: Arc<ClusterState>,
    /// Consensus engine
    consensus: Arc<RwLock<RaftConsensus>>,
    /// Active connections to other nodes
    connections: Arc<RwLock<HashMap<NodeId, ClusterClient>>>,
    /// Server handle
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl ClusterNetwork {
    /// Create a new cluster network service
    pub fn new(local_node: NodeId, cluster_state: Arc<ClusterState>) -> Self {
        let consensus = Arc::new(RwLock::new(RaftConsensus::new()));

        Self {
            local_node,
            cluster_state,
            consensus,
            connections: Arc::new(RwLock::new(HashMap::new())),
            server_handle: None,
        }
    }

    /// Start the network service
    pub async fn start(&mut self, address: SocketAddr) -> Result<(), NetworkError> {
        // Start gRPC server
        let service = ClusterServiceImpl::new(
            self.cluster_state.clone(),
            self.consensus.clone(),
        );

        let server = Server::builder()
            .add_service(cluster_proto::cluster_service_server::ClusterServiceServer::new(service))
            .serve(address);

        self.server_handle = Some(tokio::spawn(async move {
            if let Err(e) = server.await {
                eprintln!("gRPC server error: {}", e);
            }
        }));

        println!("Cluster network started on {}", address);
        Ok(())
    }

    /// Stop the network service
    pub async fn stop(&mut self) -> Result<(), NetworkError> {
        if let Some(handle) = self.server_handle.take() {
            handle.abort();
        }
        Ok(())
    }

    /// Connect to another cluster node
    pub async fn connect_to_node(&self, node_id: NodeId, address: String) -> Result<(), NetworkError> {
        let addr = format!("http://{}", address);
        let client = cluster_proto::cluster_service_client::ClusterServiceClient::connect(addr).await?;

        let mut connections = self.connections.write().await;
        connections.insert(node_id, ClusterClient { client });

        println!("Connected to node {} at {}", node_id.0, address);
        Ok(())
    }

    /// Send a Raft message to another node
    pub async fn send_raft_message(&self, target: &NodeId, message: RaftMessage) -> Result<(), NetworkError> {
        let connections = self.connections.read().await;

        if let Some(client) = connections.get(target) {
            match message {
                RaftMessage::VoteRequest(req) => {
                    let proto_req = self.convert_vote_request(req)?;
                    let _ = client.client.request_vote(proto_req).await?;
                }
                RaftMessage::AppendEntriesRequest(req) => {
                    let proto_req = self.convert_append_entries_request(req)?;
                    let _ = client.client.append_entries(proto_req).await?;
                }
                _ => {} // Handle other message types
            }
        }

        Ok(())
    }

    /// Get consensus command sender
    pub async fn consensus_sender(&self) -> mpsc::Sender<ConsensusCommand> {
        let consensus = self.consensus.read().await;
        consensus.command_sender()
    }

    // Conversion helpers for protobuf messages
    fn convert_vote_request(&self, req: VoteRequest) -> Result<cluster_proto::VoteRequest, NetworkError> {
        Ok(cluster_proto::VoteRequest {
            term: req.term,
            candidate_id: req.candidate_id.0,
            last_log_index: req.last_log_index,
            last_log_term: req.last_log_term,
        })
    }

    fn convert_append_entries_request(&self, req: AppendEntriesRequest) -> Result<cluster_proto::AppendEntriesRequest, NetworkError> {
        let entries = req.entries.into_iter()
            .map(|entry| self.convert_log_entry(entry))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(cluster_proto::AppendEntriesRequest {
            term: req.term,
            leader_id: req.leader_id.0,
            prev_log_index: req.prev_log_index,
            prev_log_term: req.prev_log_term,
            entries,
            leader_commit: req.leader_commit,
        })
    }

    fn convert_log_entry(&self, entry: LogEntry) -> Result<cluster_proto::LogEntry, NetworkError> {
        let operation = self.convert_operation(entry.operation)?;

        Ok(cluster_proto::LogEntry {
            index: entry.index,
            term: entry.term,
            operation: Some(operation),
            client_id: "".to_string(), // TODO: Add client tracking
            request_id: 0,
        })
    }

    fn convert_operation(&self, op: Operation) -> Result<cluster_proto::Operation, NetworkError> {
        let operation = match op {
            Operation::CreateNode { properties } => {
                let props = self.convert_properties(properties)?;
                cluster_proto::operation::Operation::CreateNode(cluster_proto::CreateNode {
                    properties: props,
                })
            }
            Operation::UpdateNode { cid, properties } => {
                let props = self.convert_properties(properties)?;
                cluster_proto::operation::Operation::UpdateNode(cluster_proto::UpdateNode {
                    cid,
                    properties: props,
                })
            }
            Operation::DeleteNode { cid } => {
                cluster_proto::operation::Operation::DeleteNode(cluster_proto::DeleteNode {
                    cid,
                })
            }
            Operation::CreateEdge { source_cid, target_cid, properties } => {
                let props = self.convert_properties(properties)?;
                cluster_proto::operation::Operation::CreateEdge(cluster_proto::CreateEdge {
                    source_cid,
                    target_cid,
                    properties: props,
                })
            }
            Operation::UpdateEdge { cid, properties } => {
                let props = self.convert_properties(properties)?;
                cluster_proto::operation::Operation::UpdateEdge(cluster_proto::UpdateEdge {
                    cid,
                    properties: props,
                })
            }
            Operation::DeleteEdge { cid } => {
                cluster_proto::operation::Operation::DeleteEdge(cluster_proto::DeleteEdge {
                    cid,
                })
            }
        };

        Ok(cluster_proto::Operation {
            operation: Some(operation),
        })
    }

    fn convert_properties(&self, props: HashMap<String, Value>) -> Result<HashMap<String, cluster_proto::Value>, NetworkError> {
        let mut result = HashMap::new();

        for (key, value) in props {
            let proto_value = self.convert_value(value)?;
            result.insert(key, proto_value);
        }

        Ok(result)
    }

    fn convert_value(&self, value: Value) -> Result<cluster_proto::Value, NetworkError> {
        let value = match value {
            Value::String(s) => cluster_proto::value::Value::StringValue(s),
            Value::Int(i) => cluster_proto::value::Value::IntValue(i),
            Value::Float(f) => cluster_proto::value::Value::FloatValue(f),
            Value::Bool(b) => cluster_proto::value::Value::BoolValue(b),
            Value::Bytes(b) => cluster_proto::value::value::Value::BytesValue(b),
            Value::Link(l) => cluster_proto::value::Value::LinkValue(l),
        };

        Ok(cluster_proto::Value {
            value: Some(value),
        })
    }
}

/// Wrapper for gRPC client
struct ClusterClient {
    client: cluster_proto::cluster_service_client::ClusterServiceClient<tonic::transport::Channel>,
}

/// gRPC service implementation
pub struct ClusterServiceImpl {
    cluster_state: Arc<ClusterState>,
    consensus: Arc<RwLock<RaftConsensus>>,
}

impl ClusterServiceImpl {
    pub fn new(cluster_state: Arc<ClusterState>, consensus: Arc<RwLock<RaftConsensus>>) -> Self {
        Self {
            cluster_state,
            consensus,
        }
    }
}

#[tonic::async_trait]
impl cluster_proto::cluster_service_server::ClusterService for ClusterServiceImpl {
    async fn request_vote(
        &self,
        request: Request<cluster_proto::VoteRequest>,
    ) -> Result<Response<cluster_proto::VoteResponse>, Status> {
        let req = request.into_inner();

        let vote_req = VoteRequest {
            term: req.term,
            candidate_id: NodeId(req.candidate_id),
            last_log_index: req.last_log_index,
            last_log_term: req.last_log_term,
        };

        let consensus = self.consensus.read().await;
        consensus.handle_vote_request(vote_req, &self.cluster_state).await
            .map_err(|e| Status::internal(format!("Vote request failed: {}", e)))?;

        // Return response (simplified)
        let response = cluster_proto::VoteResponse {
            term: 0, // TODO: Get from state
            vote_granted: true,
        };

        Ok(Response::new(response))
    }

    async fn append_entries(
        &self,
        request: Request<cluster_proto::AppendEntriesRequest>,
    ) -> Result<Response<cluster_proto::AppendEntriesResponse>, Status> {
        let req = request.into_inner();

        let append_req = AppendEntriesRequest {
            term: req.term,
            leader_id: NodeId(req.leader_id),
            prev_log_index: req.prev_log_index,
            prev_log_term: req.prev_log_term,
            entries: Vec::new(), // TODO: Convert entries
            leader_commit: req.leader_commit,
        };

        let consensus = self.consensus.read().await;
        consensus.handle_append_entries_request(append_req, &self.cluster_state).await
            .map_err(|e| Status::internal(format!("Append entries failed: {}", e)))?;

        // Return response (simplified)
        let response = cluster_proto::AppendEntriesResponse {
            term: 0, // TODO: Get from state
            success: true,
            match_index: 0,
        };

        Ok(Response::new(response))
    }

    async fn heartbeat(
        &self,
        _request: Request<cluster_proto::HeartbeatRequest>,
    ) -> Result<Response<cluster_proto::HeartbeatResponse>, Status> {
        // TODO: Implement heartbeat logic
        let response = cluster_proto::HeartbeatResponse {
            node_id: self.cluster_state.local_node.0.clone(),
            role: cluster_proto::NodeRole::Leader as i32, // TODO: Get actual role
            last_log_index: 0,
            commit_index: 0,
        };

        Ok(Response::new(response))
    }

    async fn execute_operation(
        &self,
        _request: Request<cluster_proto::ClientRequest>,
    ) -> Result<Response<cluster_proto::ClientResponse>, Status> {
        // TODO: Implement client operation execution
        let response = cluster_proto::ClientResponse {
            client_id: "".to_string(),
            request_id: 0,
            success: true,
            error_message: "".to_string(),
            result_cid: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn join_cluster(
        &self,
        _request: Request<cluster_proto::NodeId>,
    ) -> Result<Response<cluster_proto::ClusterConfig>, Status> {
        // TODO: Implement cluster join logic
        let response = cluster_proto::ClusterConfig {
            nodes: Vec::new(),
            version: 0,
        };

        Ok(Response::new(response))
    }

    async fn leave_cluster(
        &self,
        _request: Request<cluster_proto::NodeId>,
    ) -> Result<Response<cluster_proto::ClusterConfig>, Status> {
        // TODO: Implement cluster leave logic
        let response = cluster_proto::ClusterConfig {
            nodes: Vec::new(),
            version: 0,
        };

        Ok(Response::new(response))
    }

    async fn get_cluster_config(
        &self,
        _request: Request<()>,
    ) -> Result<Response<cluster_proto::ClusterConfig>, Status> {
        // TODO: Implement cluster config retrieval
        let response = cluster_proto::ClusterConfig {
            nodes: Vec::new(),
            version: 0,
        };

        Ok(Response::new(response))
    }
}

/// Network-related errors
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("gRPC transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    #[error("gRPC status error: {0}")]
    Status(#[from] tonic::Status),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Protocol error: {0}")]
    Protocol(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_network_creation() {
        let node_id = NodeId("test-node".to_string());
        let cluster_state = Arc::new(ClusterState::new(node_id.clone()));
        let network = ClusterNetwork::new(node_id, cluster_state);

        assert_eq!(network.local_node.0, "test-node");
    }

    #[tokio::test]
    async fn test_value_conversion() {
        let network = ClusterNetwork::new(
            NodeId("test".to_string()),
            Arc::new(ClusterState::new(NodeId("test".to_string())))
        );

        // Test string conversion
        let value = Value::String("hello".to_string());
        let proto_value = network.convert_value(value).unwrap();
        assert!(proto_value.value.is_some());

        // Test int conversion
        let value = Value::Int(42);
        let proto_value = network.convert_value(value).unwrap();
        assert!(proto_value.value.is_some());
    }
}

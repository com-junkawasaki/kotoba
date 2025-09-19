//! NetworkMessage と関連構造体の実装

use super::*;
use sha2::{Sha256, Digest};

/// CIDを生成するヘルパー関数
fn generate_cid(data: &str) -> Cid {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result[..32]);
    Cid(bytes)
}

/// ネットワークマネージャー
#[derive(Debug)]
pub struct NetworkManager {
    /// ノードID
    node_id: NodeId,
    /// 通信チャネル
    message_sender: mpsc::UnboundedSender<NetworkMessage>,
    message_receiver: mpsc::UnboundedReceiver<NetworkMessage>,
    /// 接続中のピア
    peers: HashMap<NodeId, PeerConnection>,
    /// 待機中のレスポンス
    pending_responses: HashMap<TaskId, oneshot::Sender<TaskResult>>,
}

/// ピア接続情報
#[derive(Debug)]
pub struct PeerConnection {
    /// ピアノードID
    peer_id: NodeId,
    /// ネットワークアドレス
    address: String,
    /// 接続状態
    status: ConnectionStatus,
    /// 最後の通信時刻
    last_seen: std::time::Instant,
    /// 通信チャネル
    sender: Option<mpsc::UnboundedSender<NetworkMessage>>,
}

/// 接続状態
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    /// 接続中
    Connected,
    /// 接続試行中
    Connecting,
    /// 切断中
    Disconnected,
    /// エラー状態
    Error(String),
}

/// メッセージハンドラー
#[derive(Debug)]
pub struct MessageHandler {
    /// ネットワークマネージャーへの参照
    network_manager: std::sync::Arc<tokio::sync::Mutex<NetworkManager>>,
    /// 分散実行エンジン
    distributed_engine: std::sync::Arc<DistributedEngine>,
}

impl MessageHandler {
    /// ネットワークマネージャーを取得
    pub fn network_manager(&self) -> std::sync::Arc<tokio::sync::Mutex<NetworkManager>> {
        self.network_manager.clone()
    }
}

impl NetworkManager {
    /// 新しいネットワークマネージャーを作成
    pub fn new(node_id: NodeId) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            node_id,
            message_sender: tx,
            message_receiver: rx,
            peers: HashMap::new(),
            pending_responses: HashMap::new(),
        }
    }

    /// ピアを追加
    pub fn add_peer(&mut self, peer_id: NodeId, address: String) {
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            address,
            status: ConnectionStatus::Connecting,
            last_seen: std::time::Instant::now(),
            sender: None,
        };

        self.peers.insert(peer_id, connection);
    }

    /// ピアを削除
    pub fn remove_peer(&mut self, peer_id: &NodeId) {
        self.peers.remove(peer_id);
    }

    /// メッセージを送信
    pub fn send_message(&self, peer_id: &NodeId, message: NetworkMessage) -> kotoba_core::types::Result<()> {
        if let Some(peer) = self.peers.get(peer_id) {
            if let Some(sender) = &peer.sender {
                sender.send(message)
                    .map_err(|_| KotobaError::Execution("Failed to send message".to_string()))?;
                Ok(())
            } else {
                Err(KotobaError::Execution("Peer not connected".to_string()))
            }
        } else {
            Err(KotobaError::Execution("Peer not found".to_string()))
        }
    }

    /// ブロードキャスト
    pub fn broadcast(&self, message: NetworkMessage) -> kotoba_core::types::Result<()> {
        for peer in self.peers.values() {
            if let Some(sender) = &peer.sender {
                let _ = sender.send(message.clone()); // エラーは無視
            }
        }
        Ok(())
    }
}

impl MessageHandler {
    /// 新しいメッセージハンドラーを作成
    pub fn new(
        network_manager: std::sync::Arc<tokio::sync::Mutex<NetworkManager>>,
        distributed_engine: std::sync::Arc<DistributedEngine>,
    ) -> Self {
        Self {
            network_manager,
            distributed_engine,
        }
    }

    /// メッセージを処理
    pub async fn handle_message(&self, message: NetworkMessage) -> kotoba_core::types::Result<()> {
        match message {
            NetworkMessage::TaskRequest { task, requester_id } => {
                self.handle_task_request(task, requester_id).await
            }
            NetworkMessage::TaskResponse { task_id, result, executor_id: _ } => {
                self.handle_task_response(task_id, result).await
            }
            NetworkMessage::Heartbeat { node_id, status, load } => {
                self.handle_heartbeat(node_id, status, load).await
            }
            NetworkMessage::JoinRequest { node_id, address, capabilities } => {
                self.handle_join_request(node_id, address, capabilities).await
            }
            NetworkMessage::CacheSync { entries, sender_id } => {
                self.handle_cache_sync(entries, sender_id).await
            }
            NetworkMessage::GraphTransfer { graph_cid, data, compression: _ } => {
                self.handle_graph_transfer(graph_cid, data).await
            }
            _ => Ok(()), // その他のメッセージは無視
        }
    }

    /// タスクリクエストを処理
    async fn handle_task_request(&self, task: DistributedTask, requester_id: NodeId) -> kotoba_core::types::Result<()> {
        // 分散実行エンジンでタスクを実行
        let result = match task.task_type {
            TaskType::RuleApplication { rule_cid, host_graph_cid } => {
                // ルールとグラフを取得して実行（簡易版）
                TaskResult::Success(GraphInstance {
                    core: GraphCore {
                        nodes: vec![],
                        edges: vec![],
                        boundary: None,
                        attrs: None,
                    },
                    kind: GraphKind::Graph,
                    cid: generate_cid("result"),
                    typing: None,
                })
            }
            TaskType::QueryExecution { query_cid: _, target_graph_cid: _ } => {
                TaskResult::Success(GraphInstance {
                    core: GraphCore {
                        nodes: vec![],
                        edges: vec![],
                        boundary: None,
                        attrs: None,
                    },
                    kind: GraphKind::Graph,
                    cid: generate_cid("query_result"),
                    typing: None,
                })
            }
            _ => TaskResult::Failure {
                error: "Unsupported task type".to_string(),
                retryable: false,
            },
        };

        // 結果を送信
        let response = NetworkMessage::TaskResponse {
            task_id: task.id,
            result,
            executor_id: self.network_manager.lock().await.node_id.clone(),
        };

        self.network_manager.lock().await.send_message(&requester_id, response)?;
        Ok(())
    }

    /// タスクレスポンスを処理
    async fn handle_task_response(&self, task_id: TaskId, result: TaskResult) -> kotoba_core::types::Result<()> {
        let mut manager = self.network_manager.lock().await;
        if let Some(sender) = manager.pending_responses.remove(&task_id) {
            let _ = sender.send(result); // エラーは無視
        }
        Ok(())
    }

    /// ハートビートを処理
    async fn handle_heartbeat(&self, node_id: NodeId, status: NodeStatus, load: f64) -> kotoba_core::types::Result<()> {
        let mut manager = self.network_manager.lock().await;
        if let Some(peer) = manager.peers.get_mut(&node_id) {
            peer.status = match status {
                NodeStatus::Active => ConnectionStatus::Connected,
                NodeStatus::Unreachable => ConnectionStatus::Disconnected,
                _ => ConnectionStatus::Error("Node status error".to_string()),
            };
            peer.last_seen = std::time::Instant::now();
        }
        Ok(())
    }

    /// 参加リクエストを処理
    async fn handle_join_request(&self, node_id: NodeId, address: String, _capabilities: NodeCapabilities) -> kotoba_core::types::Result<()> {
        let mut manager = self.network_manager.lock().await;
        manager.add_peer(node_id.clone(), address);

        let response = NetworkMessage::JoinResponse {
            accepted: true,
            cluster_info: Some(ClusterInfo {
                cluster_id: "default".to_string(),
                total_nodes: manager.peers.len(),
                active_nodes: manager.peers.len(),
                master_node: manager.node_id.clone(),
                config: ClusterConfig {
                    heartbeat_interval_secs: 30,
                    timeout_secs: 60,
                    max_retries: 3,
                    load_threshold: 0.8,
                },
            }),
            reason: None,
        };

        manager.send_message(&node_id, response)?;
        Ok(())
    }

    /// キャッシュ同期を処理
    async fn handle_cache_sync(&self, _entries: Vec<CacheSyncEntry>, _sender_id: NodeId) -> kotoba_core::types::Result<()> {
        // キャッシュ同期処理（実装予定）
        Ok(())
    }

    /// グラフ転送を処理
    async fn handle_graph_transfer(&self, _graph_cid: Cid, _data: GraphInstance) -> kotoba_core::types::Result<()> {
        // グラフ転送処理（実装予定）
        Ok(())
    }
}

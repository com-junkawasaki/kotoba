//! ネットワーク通信プロトコル - 分散実行のための通信層
//!
//! このモジュールは、分散実行におけるノード間通信を担当します。

use crate::schema::*;
use crate::distributed::*;
use kotoba_core::types::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

/// ネットワークプロトコルメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// タスク実行リクエスト
    TaskRequest {
        task: DistributedTask,
        requester_id: NodeId,
    },
    /// タスク実行レスポンス
    TaskResponse {
        task_id: TaskId,
        result: TaskResult,
        executor_id: NodeId,
    },
    /// ハートビート
    Heartbeat {
        node_id: NodeId,
        status: NodeStatus,
        load: f64,
    },
    /// クラスタ参加リクエスト
    JoinRequest {
        node_id: NodeId,
        address: String,
        capabilities: NodeCapabilities,
    },
    /// クラスタ参加レスポンス
    JoinResponse {
        accepted: bool,
        cluster_info: Option<ClusterInfo>,
        reason: Option<String>,
    },
    /// CIDキャッシュ同期
    CacheSync {
        entries: Vec<CacheSyncEntry>,
        sender_id: NodeId,
    },
    /// グラフデータ転送
    GraphTransfer {
        graph_cid: Cid,
        data: GraphInstance,
        compression: CompressionType,
    },
}

/// タスク実行結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskResult {
    /// 成功
    Success(GraphInstance),
    /// 失敗
    Failure {
        error: String,
        retryable: bool,
    },
    /// 部分成功
    Partial(Vec<PartialTaskResult>),
}

/// 部分タスク結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialTaskResult {
    /// サブタスク識別子
    subtask_id: String,
    /// 結果
    result: Box<TaskResult>,
    /// 実行時間
    execution_time_ms: u64,
}

/// ノード能力情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// CPUコア数
    cpu_cores: usize,
    /// メモリ容量（MB）
    memory_mb: usize,
    /// サポートするCID範囲
    supported_cid_ranges: Vec<CidRange>,
    /// 特殊機能
    features: Vec<NodeFeature>,
}

/// ノード機能
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NodeFeature {
    /// GPUアクセラレーション
    GpuAcceleration,
    /// 高メモリ容量
    HighMemory,
    /// 高速ストレージ
    FastStorage,
    /// ネットワーク最適化
    NetworkOptimized,
}

/// クラスタ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// クラスタ内の全ノード
    nodes: Vec<ClusterNode>,
    /// リーダーノード
    leader: NodeId,
    /// クラスタ設定
    config: ClusterConfig,
}

/// クラスタ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// ハートビート間隔（秒）
    heartbeat_interval_secs: u64,
    /// タスクタイムアウト（秒）
    task_timeout_secs: u64,
    /// 最大リトライ回数
    max_retries: usize,
    /// 負荷バランス閾値
    load_balance_threshold: f64,
}

/// キャッシュ同期エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSyncEntry {
    /// CID
    cid: Cid,
    /// エントリタイプ
    entry_type: CacheEntryType,
    /// バージョン
    version: u64,
    /// 最終更新時刻
    last_updated: u64,
}

/// キャッシュエントリタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheEntryType {
    /// グラフデータ
    GraphData,
    /// ルール結果
    RuleResult,
    /// クエリ結果
    QueryResult,
}

/// 圧縮タイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// 無圧縮
    None,
    /// GZIP圧縮
    Gzip,
    /// LZ4圧縮
    Lz4,
    /// Zstandard圧縮
    Zstd,
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
    Error,
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

    /// タスクをリモートノードに送信
    pub async fn send_task(
        &mut self,
        task: DistributedTask,
        target_node: &NodeId,
    ) -> Result<oneshot::Receiver<TaskResult>> {
        // レスポンス待機用のチャネル作成
        let (tx, rx) = oneshot::channel();
        self.pending_responses.insert(task.id.clone(), tx);

        let message = NetworkMessage::TaskRequest {
            task,
            requester_id: self.node_id.clone(),
        };

        self.send_message(target_node, message).await?;

        Ok(rx)
    }

    /// メッセージを送信
    pub async fn send_message(
        &mut self,
        target_node: &NodeId,
        message: NetworkMessage,
    ) -> Result<()> {
        if let Some(peer) = self.peers.get_mut(target_node) {
            if let Some(sender) = &peer.sender {
                sender.send(message)
                    .map_err(|_| KotobaError::Execution("Failed to send message".to_string()))?;
                return Ok(());
            }
        }

        Err(KotobaError::Execution(format!("Peer {} not connected", target_node.0)))
    }

    /// メッセージを受信して処理
    pub async fn process_messages(&mut self) -> Result<()> {
        while let Ok(message) = self.message_receiver.try_recv() {
            self.handle_message(message).await?;
        }

        Ok(())
    }

    /// メッセージを処理
    async fn handle_message(&mut self, message: NetworkMessage) -> Result<()> {
        match message {
            NetworkMessage::TaskRequest { task, requester_id } => {
                // タスク実行リクエストの処理
                self.handle_task_request(task, requester_id).await?;
            }
            NetworkMessage::TaskResponse { task_id, result, executor_id } => {
                // タスク実行レスポンスの処理
                self.handle_task_response(task_id, result, executor_id).await?;
            }
            NetworkMessage::Heartbeat { node_id, status, load } => {
                // ハートビートの処理
                self.handle_heartbeat(node_id, status, load).await?;
            }
            NetworkMessage::JoinRequest { node_id, address, capabilities } => {
                // クラスタ参加リクエストの処理
                self.handle_join_request(node_id, address, capabilities).await?;
            }
            NetworkMessage::CacheSync { entries, sender_id } => {
                // キャッシュ同期の処理
                self.handle_cache_sync(entries, sender_id).await?;
            }
            _ => {
                // その他のメッセージはログに記録
                println!("Received unhandled message type");
            }
        }

        Ok(())
    }

    /// タスク実行リクエストを処理
    async fn handle_task_request(
        &mut self,
        task: DistributedTask,
        requester_id: NodeId,
    ) -> Result<()> {
        // 実際の実装ではタスクを実行して結果を返す
        println!("Received task request from {}", requester_id.0);

        // 仮の成功レスポンス
        let response = NetworkMessage::TaskResponse {
            task_id: task.id,
            result: TaskResult::Success(GraphInstance {
                        core: GraphCore {
                            nodes: vec![],
                            edges: vec![],
                            boundary: None,
                            attrs: None,
                        },
                        kind: GraphKind::Instance,
                cid: Cid::new("dummy_result"),
                        typing: None,
            }),
            executor_id: self.node_id.clone(),
        };

        self.send_message(&requester_id, response).await?;

        Ok(())
    }

    /// タスク実行レスポンスを処理
    async fn handle_task_response(
        &mut self,
        task_id: TaskId,
        result: TaskResult,
        executor_id: NodeId,
    ) -> Result<()> {
        if let Some(sender) = self.pending_responses.remove(&task_id) {
            let _ = sender.send(result);
        }

        println!("Received task response from {}", executor_id.0);
        Ok(())
    }

    /// ハートビートを処理
    async fn handle_heartbeat(
        &mut self,
        node_id: NodeId,
        status: NodeStatus,
        load: f64,
    ) -> Result<()> {
        if let Some(peer) = self.peers.get_mut(&node_id) {
            peer.status = match status {
                NodeStatus::Active => ConnectionStatus::Connected,
                NodeStatus::Overloaded => ConnectionStatus::Connected,
                NodeStatus::Maintenance => ConnectionStatus::Disconnected,
                NodeStatus::Unreachable => ConnectionStatus::Error,
            };
            peer.last_seen = std::time::Instant::now();
        }

        println!("Heartbeat from {}: status={:?}, load={}", node_id.0, status, load);
        Ok(())
    }

    /// クラスタ参加リクエストを処理
    async fn handle_join_request(
        &mut self,
        node_id: NodeId,
        address: String,
        capabilities: NodeCapabilities,
    ) -> Result<()> {
        // 新しいノードを追加
        let peer = PeerConnection {
            peer_id: node_id.clone(),
            address: address.clone(),
            status: ConnectionStatus::Connected,
            last_seen: std::time::Instant::now(),
            sender: None, // 実際の実装では接続を確立
        };

        self.peers.insert(node_id.clone(), peer);

        // 参加レスポンスを送信
        let response = NetworkMessage::JoinResponse {
            accepted: true,
            cluster_info: Some(ClusterInfo {
                nodes: vec![], // 簡易版
                leader: self.node_id.clone(),
                config: ClusterConfig {
                    heartbeat_interval_secs: 30,
                    task_timeout_secs: 300,
                    max_retries: 3,
                    load_balance_threshold: 0.8,
                },
            }),
            reason: None,
        };

        self.send_message(&node_id, response).await?;

        println!("Node {} joined the cluster", node_id.0);
        Ok(())
    }

    /// キャッシュ同期を処理
    async fn handle_cache_sync(
        &mut self,
        entries: Vec<CacheSyncEntry>,
        sender_id: NodeId,
    ) -> Result<()> {
        println!("Received cache sync from {} with {} entries", sender_id.0, entries.len());

        // 実際の実装ではキャッシュを更新
        for entry in entries {
            println!("Syncing CID: {}", entry.cid.as_str());
        }

        Ok(())
    }

    /// ピア接続を確立
    pub async fn connect_to_peer(&mut self, peer_id: NodeId, address: String) -> Result<()> {
        let peer = PeerConnection {
            peer_id: peer_id.clone(),
            address: address.clone(),
            status: ConnectionStatus::Connecting,
            last_seen: std::time::Instant::now(),
            sender: Some(self.message_sender.clone()), // 簡易版
        };

        self.peers.insert(peer_id.clone(), peer);

        // 実際の実装ではTCP/WebSocket接続を確立
        println!("Connected to peer {} at {}", peer_id.0, address);

        Ok(())
    }

    /// ピア接続を切断
    pub async fn disconnect_peer(&mut self, peer_id: &NodeId) -> Result<()> {
        if let Some(mut peer) = self.peers.remove(peer_id) {
            peer.status = ConnectionStatus::Disconnected;
            println!("Disconnected from peer {}", peer_id.0);
        }

        Ok(())
    }

    /// ネットワーク統計を取得
    pub fn get_stats(&self) -> NetworkStats {
        NetworkStats {
            connected_peers: self.peers.values()
                .filter(|p| p.status == ConnectionStatus::Connected)
                .count(),
            total_peers: self.peers.len(),
            pending_responses: self.pending_responses.len(),
        }
    }
}

/// ネットワーク統計
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// 接続中のピア数
    pub connected_peers: usize,
    /// 総ピア数
    pub total_peers: usize,
    /// 待機中のレスポンス数
    pub pending_responses: usize,
}

/// ネットワーククライアント
#[derive(Debug)]
pub struct NetworkClient {
    /// ネットワークマネージャーへの参照
    manager: std::sync::Arc<tokio::sync::RwLock<NetworkManager>>,
}

impl NetworkClient {
    /// 新しいネットワーククライアントを作成
    pub fn new(manager: std::sync::Arc<tokio::sync::RwLock<NetworkManager>>) -> Self {
        Self { manager }
    }

    /// リモートノードにタスクを送信
    pub async fn send_task(
        &self,
        task: DistributedTask,
        target_node: &NodeId,
    ) -> Result<TaskResult> {
        let mut manager = self.manager.write().await;
        let receiver = manager.send_task(task, target_node).await?;

        // レスポンスを待機
        match tokio::time::timeout(std::time::Duration::from_secs(300), receiver).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(_)) => Err(KotobaError::Execution("Response channel closed".to_string())),
            Err(_) => Err(KotobaError::Execution("Task timeout".to_string())),
        }
    }

    /// クラスタに参加
    pub async fn join_cluster(&self, coordinator_address: &str) -> Result<()> {
        // 実際の実装ではcoordinator_addressに接続
        println!("Joining cluster at {}", coordinator_address);
        Ok(())
    }

    /// ハートビートを送信
    pub async fn send_heartbeat(&self, target_node: &NodeId, status: NodeStatus, load: f64) -> Result<()> {
        let manager = self.manager.read().await;
        let message = NetworkMessage::Heartbeat {
            node_id: manager.node_id.clone(),
            status,
            load,
        };

        // 実際の実装ではメッセージを送信
        println!("Sent heartbeat to {}", target_node.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_message_serialization() {
        let message = NetworkMessage::Heartbeat {
            node_id: NodeId("test_node".to_string()),
            status: NodeStatus::Active,
            load: 0.5,
        };

        let serialized = serde_json::to_string(&message).unwrap();
        let deserialized: NetworkMessage = serde_json::from_str(&serialized).unwrap();

        match deserialized {
            NetworkMessage::Heartbeat { node_id, status, load } => {
                assert_eq!(node_id.0, "test_node");
                assert_eq!(status, NodeStatus::Active);
                assert_eq!(load, 0.5);
            }
            _ => panic!("Wrong message type"),
        }
    }

    #[test]
    fn test_task_result() {
        let result = TaskResult::Success(GraphInstance {
            core: GraphCore {
                nodes: vec![],
                edges: vec![],
                boundary: None,
                attrs: None,
            },
            kind: GraphKind::Instance,
            cid: Cid::new("test_cid"),
            typing: None,
        });

        match result {
            TaskResult::Success(graph) => {
                assert_eq!(graph.cid.as_str(), "test_cid");
            }
            _ => panic!("Wrong result type"),
        }
    }

    #[tokio::test]
    async fn test_network_manager() {
        let node_id = NodeId("test_node".to_string());
        let manager = NetworkManager::new(node_id.clone());

        assert_eq!(manager.node_id, node_id);
        assert_eq!(manager.get_stats().connected_peers, 0);
    }

    #[test]
    fn test_node_capabilities() {
        let capabilities = NodeCapabilities {
            cpu_cores: 8,
            memory_mb: 16384,
            supported_cid_ranges: vec![],
            features: vec![NodeFeature::GpuAcceleration],
        };

        assert_eq!(capabilities.cpu_cores, 8);
        assert_eq!(capabilities.memory_mb, 16384);
        assert_eq!(capabilities.features.len(), 1);
    }
}
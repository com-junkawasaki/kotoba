//! ネットワーク通信プロトコル - 分散実行のための通信層
//!
//! このモジュールは、分散実行におけるノード間通信を担当します。

use kotoba_core::types::*;
use kotoba_distributed::*;
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
    /// ストレージ最適化
    StorageOptimized,
    /// ネットワーク最適化
    NetworkOptimized,
}

/// クラスタ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// クラスタID
    cluster_id: String,
    /// 全ノード数
    total_nodes: usize,
    /// アクティブノード数
    active_nodes: usize,
    /// マスターノードID
    master_node: NodeId,
    /// クラスタ設定
    config: ClusterConfig,
}

/// クラスタ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// ハートビート間隔（秒）
    heartbeat_interval_secs: u64,
    /// タイムアウト時間（秒）
    timeout_secs: u64,
    /// 最大リトライ回数
    max_retries: usize,
    /// 負荷閾値
    load_threshold: f64,
}

/// CID範囲（シャーディング用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CidRange {
    /// 開始CID（ハッシュ値）
    start: u64,
    /// 終了CID（ハッシュ値）
    end: u64,
}

/// キャッシュ同期エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheSyncEntry {
    /// CID
    cid: Cid,
    /// 最終アクセス時刻
    last_accessed: u64,
    /// アクセス回数
    access_count: u64,
    /// データサイズ
    size_bytes: usize,
}

/// 圧縮タイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// 無圧縮
    None,
    /// Gzip圧縮
    Gzip,
    /// LZ4圧縮
    Lz4,
    /// Snappy圧縮
    Snappy,
}

// 実装は別ファイルに分離
mod protocol_impl;
mod connection_impl;
mod server_impl;

// 再エクスポート
pub use protocol_impl::*;
pub use connection_impl::*;
pub use server_impl::*;

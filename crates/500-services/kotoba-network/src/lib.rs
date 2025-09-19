//! ネットワーク通信プロトコル - 分散実行のための通信層
//!
//! このモジュールは、分散実行におけるノード間通信を担当します。

use kotoba_core::prelude::*;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_network_message_task_request() {
        let node_id = NodeId("test-node".to_string());
        let task = DistributedTask {
            id: TaskId("test-task".to_string()),
            task_type: TaskType::Query,
            payload: serde_json::json!({"query": "SELECT * FROM test"}),
            priority: 1,
            timeout_ms: 5000,
            dependencies: vec![],
        };

        let message = NetworkMessage::TaskRequest {
            task,
            requester_id: node_id.clone(),
        };

        match message {
            NetworkMessage::TaskRequest { task: t, requester_id: id } => {
                assert_eq!(id, node_id);
                assert_eq!(t.id.0, "test-task");
                assert!(matches!(t.task_type, TaskType::Query));
            }
            _ => panic!("Expected TaskRequest"),
        }
    }

    #[test]
    fn test_network_message_task_response() {
        let task_id = TaskId("test-task".to_string());
        let node_id = NodeId("executor-node".to_string());
        let result = TaskResult::Success(GraphInstance::default());

        let message = NetworkMessage::TaskResponse {
            task_id: task_id.clone(),
            result,
            executor_id: node_id.clone(),
        };

        match message {
            NetworkMessage::TaskResponse { task_id: id, result: r, executor_id: eid } => {
                assert_eq!(id, task_id);
                assert_eq!(eid, node_id);
                assert!(matches!(r, TaskResult::Success(_)));
            }
            _ => panic!("Expected TaskResponse"),
        }
    }

    #[test]
    fn test_network_message_heartbeat() {
        let node_id = NodeId("test-node".to_string());
        let status = NodeStatus::Active;
        let load = 0.75;

        let message = NetworkMessage::Heartbeat {
            node_id: node_id.clone(),
            status,
            load,
        };

        match message {
            NetworkMessage::Heartbeat { node_id: id, status: s, load: l } => {
                assert_eq!(id, node_id);
                assert_eq!(s, status);
                assert_eq!(l, load);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }

    #[test]
    fn test_network_message_join_request() {
        let node_id = NodeId("new-node".to_string());
        let address = "127.0.0.1:8080".to_string();
        let capabilities = NodeCapabilities {
            cpu_cores: 8,
            memory_mb: 16384,
            supported_cid_ranges: vec![],
            features: vec![NodeFeature::GpuAcceleration],
        };

        let message = NetworkMessage::JoinRequest {
            node_id: node_id.clone(),
            address: address.clone(),
            capabilities: capabilities.clone(),
        };

        match message {
            NetworkMessage::JoinRequest { node_id: id, address: addr, capabilities: caps } => {
                assert_eq!(id, node_id);
                assert_eq!(addr, address);
                assert_eq!(caps.cpu_cores, 8);
                assert_eq!(caps.memory_mb, 16384);
                assert_eq!(caps.features.len(), 1);
            }
            _ => panic!("Expected JoinRequest"),
        }
    }

    #[test]
    fn test_network_message_join_response() {
        let cluster_info = ClusterInfo {
            cluster_id: "test-cluster".to_string(),
            total_nodes: 5,
            active_nodes: 4,
            master_node: NodeId("master-node".to_string()),
            config: ClusterConfig::default(),
        };

        let message = NetworkMessage::JoinResponse {
            accepted: true,
            cluster_info: Some(cluster_info.clone()),
            reason: None,
        };

        match message {
            NetworkMessage::JoinResponse { accepted, cluster_info: info, reason } => {
                assert!(accepted);
                assert!(info.is_some());
                assert_eq!(info.unwrap().cluster_id, "test-cluster");
                assert!(reason.is_none());
            }
            _ => panic!("Expected JoinResponse"),
        }
    }

    #[test]
    fn test_network_message_cache_sync() {
        let entries = vec![
            CacheSyncEntry {
                cid: Cid("test-cid-1".to_string()),
                last_accessed: 1234567890,
                access_count: 42,
                size_bytes: 1024,
            },
            CacheSyncEntry {
                cid: Cid("test-cid-2".to_string()),
                last_accessed: 1234567891,
                access_count: 15,
                size_bytes: 2048,
            },
        ];

        let sender_id = NodeId("sender-node".to_string());

        let message = NetworkMessage::CacheSync {
            entries: entries.clone(),
            sender_id: sender_id.clone(),
        };

        match message {
            NetworkMessage::CacheSync { entries: e, sender_id: id } => {
                assert_eq!(e.len(), 2);
                assert_eq!(e[0].cid.0, "test-cid-1");
                assert_eq!(e[1].access_count, 15);
                assert_eq!(id, sender_id);
            }
            _ => panic!("Expected CacheSync"),
        }
    }

    #[test]
    fn test_network_message_graph_transfer() {
        let graph_cid = Cid("graph-cid-123".to_string());
        let data = GraphInstance::default();
        let compression = CompressionType::Gzip;

        let message = NetworkMessage::GraphTransfer {
            graph_cid: graph_cid.clone(),
            data,
            compression,
        };

        match message {
            NetworkMessage::GraphTransfer { graph_cid: cid, data: _, compression: comp } => {
                assert_eq!(cid, graph_cid);
                assert!(matches!(comp, CompressionType::Gzip));
            }
            _ => panic!("Expected GraphTransfer"),
        }
    }

    #[test]
    fn test_task_result_success() {
        let graph_instance = GraphInstance::default();
        let result = TaskResult::Success(graph_instance);

        match result {
            TaskResult::Success(_) => assert!(true),
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn test_task_result_failure() {
        let error = "Task execution failed".to_string();
        let retryable = true;
        let result = TaskResult::Failure { error: error.clone(), retryable };

        match result {
            TaskResult::Failure { error: e, retryable: r } => {
                assert_eq!(e, error);
                assert!(r);
            }
            _ => panic!("Expected Failure"),
        }
    }

    #[test]
    fn test_task_result_partial() {
        let partial_results = vec![
            PartialTaskResult {
                subtask_id: "subtask-1".to_string(),
                result: Box::new(TaskResult::Success(GraphInstance::default())),
                execution_time_ms: 150,
            },
            PartialTaskResult {
                subtask_id: "subtask-2".to_string(),
                result: Box::new(TaskResult::Failure {
                    error: "Subtask failed".to_string(),
                    retryable: false,
                }),
                execution_time_ms: 200,
            },
        ];

        let result = TaskResult::Partial(partial_results.clone());

        match result {
            TaskResult::Partial(results) => {
                assert_eq!(results.len(), 2);
                assert_eq!(results[0].subtask_id, "subtask-1");
                assert_eq!(results[1].execution_time_ms, 200);
            }
            _ => panic!("Expected Partial"),
        }
    }

    #[test]
    fn test_partial_task_result_creation() {
        let subtask_id = "test-subtask".to_string();
        let task_result = TaskResult::Success(GraphInstance::default());
        let execution_time_ms = 300;

        let partial = PartialTaskResult {
            subtask_id: subtask_id.clone(),
            result: Box::new(task_result),
            execution_time_ms,
        };

        assert_eq!(partial.subtask_id, subtask_id);
        assert_eq!(partial.execution_time_ms, execution_time_ms);
        assert!(matches!(*partial.result, TaskResult::Success(_)));
    }

    #[test]
    fn test_node_capabilities_creation() {
        let cid_ranges = vec![
            CidRange { start: 0, end: 1000 },
            CidRange { start: 1001, end: 2000 },
        ];

        let features = vec![
            NodeFeature::GpuAcceleration,
            NodeFeature::HighMemory,
            NodeFeature::NetworkOptimized,
        ];

        let capabilities = NodeCapabilities {
            cpu_cores: 16,
            memory_mb: 65536,
            supported_cid_ranges: cid_ranges.clone(),
            features: features.clone(),
        };

        assert_eq!(capabilities.cpu_cores, 16);
        assert_eq!(capabilities.memory_mb, 65536);
        assert_eq!(capabilities.supported_cid_ranges.len(), 2);
        assert_eq!(capabilities.features.len(), 3);
        assert!(capabilities.features.contains(&NodeFeature::GpuAcceleration));
    }

    #[test]
    fn test_node_feature_enum() {
        let gpu = NodeFeature::GpuAcceleration;
        let memory = NodeFeature::HighMemory;
        let storage = NodeFeature::StorageOptimized;
        let network = NodeFeature::NetworkOptimized;

        assert!(matches!(gpu, NodeFeature::GpuAcceleration));
        assert!(matches!(memory, NodeFeature::HighMemory));
        assert!(matches!(storage, NodeFeature::StorageOptimized));
        assert!(matches!(network, NodeFeature::NetworkOptimized));

        let gpu_debug = format!("{:?}", gpu);
        assert!(gpu_debug.contains("GpuAcceleration"));
    }

    #[test]
    fn test_cluster_info_creation() {
        let cluster_config = ClusterConfig {
            heartbeat_interval_secs: 30,
            timeout_secs: 60,
            max_retries: 3,
            load_threshold: 0.8,
        };

        let cluster_info = ClusterInfo {
            cluster_id: "production-cluster".to_string(),
            total_nodes: 10,
            active_nodes: 8,
            master_node: NodeId("master-node-01".to_string()),
            config: cluster_config.clone(),
        };

        assert_eq!(cluster_info.cluster_id, "production-cluster");
        assert_eq!(cluster_info.total_nodes, 10);
        assert_eq!(cluster_info.active_nodes, 8);
        assert_eq!(cluster_info.master_node.0, "master-node-01");
        assert_eq!(cluster_info.config.heartbeat_interval_secs, 30);
    }

    #[test]
    fn test_cluster_config_creation() {
        let config = ClusterConfig {
            heartbeat_interval_secs: 45,
            timeout_secs: 90,
            max_retries: 5,
            load_threshold: 0.9,
        };

        assert_eq!(config.heartbeat_interval_secs, 45);
        assert_eq!(config.timeout_secs, 90);
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.load_threshold, 0.9);
    }

    #[test]
    fn test_cid_range_creation() {
        let cid_range = CidRange { start: 1000, end: 5000 };

        assert_eq!(cid_range.start, 1000);
        assert_eq!(cid_range.end, 5000);
    }

    #[test]
    fn test_cid_range_overlap() {
        let range1 = CidRange { start: 0, end: 100 };
        let range2 = CidRange { start: 50, end: 150 };

        // Check if ranges overlap
        assert!(range1.end >= range2.start || range2.end >= range1.start);
    }

    #[test]
    fn test_cache_sync_entry_creation() {
        let cache_entry = CacheSyncEntry {
            cid: Cid("test-cid".to_string()),
            last_accessed: 1234567890,
            access_count: 100,
            size_bytes: 4096,
        };

        assert_eq!(cache_entry.cid.0, "test-cid");
        assert_eq!(cache_entry.last_accessed, 1234567890);
        assert_eq!(cache_entry.access_count, 100);
        assert_eq!(cache_entry.size_bytes, 4096);
    }

    #[test]
    fn test_compression_type_enum() {
        let none = CompressionType::None;
        let gzip = CompressionType::Gzip;
        let lz4 = CompressionType::Lz4;
        let snappy = CompressionType::Snappy;

        assert!(matches!(none, CompressionType::None));
        assert!(matches!(gzip, CompressionType::Gzip));
        assert!(matches!(lz4, CompressionType::Lz4));
        assert!(matches!(snappy, CompressionType::Snappy));

        let gzip_debug = format!("{:?}", gzip);
        assert!(gzip_debug.contains("Gzip"));
    }

    #[test]
    fn test_network_message_serialization() {
        let node_id = NodeId("test-node".to_string());
        let message = NetworkMessage::Heartbeat {
            node_id: node_id.clone(),
            status: NodeStatus::Active,
            load: 0.5,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&message);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("Heartbeat"));
        assert!(json_str.contains("test-node"));
        assert!(json_str.contains("0.5"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<NetworkMessage> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        match deserialized {
            NetworkMessage::Heartbeat { node_id: id, status: s, load: l } => {
                assert_eq!(id, node_id);
                assert_eq!(s, NodeStatus::Active);
                assert_eq!(l, 0.5);
            }
            _ => panic!("Expected Heartbeat after deserialization"),
        }
    }

    #[test]
    fn test_task_result_serialization() {
        let result = TaskResult::Failure {
            error: "Test error".to_string(),
            retryable: true,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&result);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("Failure"));
        assert!(json_str.contains("Test error"));
        assert!(json_str.contains("true"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<TaskResult> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        match deserialized {
            TaskResult::Failure { error: e, retryable: r } => {
                assert_eq!(e, "Test error");
                assert!(r);
            }
            _ => panic!("Expected Failure after deserialization"),
        }
    }

    #[test]
    fn test_node_capabilities_serialization() {
        let capabilities = NodeCapabilities {
            cpu_cores: 4,
            memory_mb: 8192,
            supported_cid_ranges: vec![
                CidRange { start: 0, end: 1000 },
            ],
            features: vec![NodeFeature::HighMemory],
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&capabilities);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("4"));
        assert!(json_str.contains("8192"));
        assert!(json_str.contains("HighMemory"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<NodeCapabilities> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.cpu_cores, 4);
        assert_eq!(deserialized.memory_mb, 8192);
        assert_eq!(deserialized.features.len(), 1);
    }

    #[test]
    fn test_cluster_info_serialization() {
        let cluster_info = ClusterInfo {
            cluster_id: "test-cluster".to_string(),
            total_nodes: 5,
            active_nodes: 4,
            master_node: NodeId("master".to_string()),
            config: ClusterConfig::default(),
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&cluster_info);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("test-cluster"));
        assert!(json_str.contains("5"));
        assert!(json_str.contains("4"));
        assert!(json_str.contains("master"));

        // Test JSON deserialization
        let deserialized_result: serde_json::Result<ClusterInfo> = serde_json::from_str(&json_str);
        assert!(deserialized_result.is_ok());

        let deserialized = deserialized_result.unwrap();
        assert_eq!(deserialized.cluster_id, "test-cluster");
        assert_eq!(deserialized.total_nodes, 5);
        assert_eq!(deserialized.active_nodes, 4);
    }

    #[test]
    fn test_compression_type_serialization() {
        let compression = CompressionType::Snappy;

        // Test JSON serialization
        let json_result = serde_json::to_string(&compression);
        assert!(json_result.is_ok());
        assert_eq!(json_result.unwrap(), "\"Snappy\"");

        // Test JSON deserialization
        let deserialized: CompressionType = serde_json::from_str("\"Snappy\"").unwrap();
        assert!(matches!(deserialized, CompressionType::Snappy));
    }

    #[test]
    fn test_network_message_debug() {
        let message = NetworkMessage::Heartbeat {
            node_id: NodeId("debug-node".to_string()),
            status: NodeStatus::Active,
            load: 0.75,
        };

        let debug_str = format!("{:?}", message);
        assert!(debug_str.contains("NetworkMessage"));
        assert!(debug_str.contains("Heartbeat"));
        assert!(debug_str.contains("debug-node"));
    }

    #[test]
    fn test_task_result_debug() {
        let result = TaskResult::Partial(vec![]);

        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("TaskResult"));
        assert!(debug_str.contains("Partial"));
    }

    #[test]
    fn test_node_capabilities_debug() {
        let capabilities = NodeCapabilities {
            cpu_cores: 8,
            memory_mb: 16384,
            supported_cid_ranges: vec![],
            features: vec![],
        };

        let debug_str = format!("{:?}", capabilities);
        assert!(debug_str.contains("NodeCapabilities"));
        assert!(debug_str.contains("8"));
        assert!(debug_str.contains("16384"));
    }

    #[test]
    fn test_cluster_info_debug() {
        let cluster_info = ClusterInfo {
            cluster_id: "debug-cluster".to_string(),
            total_nodes: 3,
            active_nodes: 2,
            master_node: NodeId("debug-master".to_string()),
            config: ClusterConfig::default(),
        };

        let debug_str = format!("{:?}", cluster_info);
        assert!(debug_str.contains("ClusterInfo"));
        assert!(debug_str.contains("debug-cluster"));
        assert!(debug_str.contains("3"));
        assert!(debug_str.contains("2"));
    }

    #[test]
    fn test_network_message_clone() {
        let original = NetworkMessage::TaskRequest {
            task: DistributedTask {
                id: TaskId("test".to_string()),
                task_type: TaskType::Query,
                payload: serde_json::json!({"test": "data"}),
                priority: 1,
                timeout_ms: 1000,
                dependencies: vec![],
            },
            requester_id: NodeId("requester".to_string()),
        };

        let cloned = original.clone();

        match cloned {
            NetworkMessage::TaskRequest { task, requester_id } => {
                assert_eq!(task.id.0, "test");
                assert_eq!(requester_id.0, "requester");
            }
            _ => panic!("Expected TaskRequest after clone"),
        }
    }

    #[test]
    fn test_task_result_clone() {
        let original = TaskResult::Failure {
            error: "Clone test".to_string(),
            retryable: true,
        };

        let cloned = original.clone();

        match cloned {
            TaskResult::Failure { error, retryable } => {
                assert_eq!(error, "Clone test");
                assert!(retryable);
            }
            _ => panic!("Expected Failure after clone"),
        }
    }

    #[test]
    fn test_node_capabilities_clone() {
        let original = NodeCapabilities {
            cpu_cores: 12,
            memory_mb: 32768,
            supported_cid_ranges: vec![CidRange { start: 0, end: 100 }],
            features: vec![NodeFeature::StorageOptimized],
        };

        let cloned = original.clone();

        assert_eq!(cloned.cpu_cores, 12);
        assert_eq!(cloned.memory_mb, 32768);
        assert_eq!(cloned.supported_cid_ranges.len(), 1);
        assert_eq!(cloned.features.len(), 1);
    }

    #[test]
    fn test_cluster_info_clone() {
        let original = ClusterInfo {
            cluster_id: "original-cluster".to_string(),
            total_nodes: 7,
            active_nodes: 6,
            master_node: NodeId("original-master".to_string()),
            config: ClusterConfig {
                heartbeat_interval_secs: 20,
                timeout_secs: 40,
                max_retries: 2,
                load_threshold: 0.7,
            },
        };

        let cloned = original.clone();

        assert_eq!(cloned.cluster_id, "original-cluster");
        assert_eq!(cloned.total_nodes, 7);
        assert_eq!(cloned.active_nodes, 6);
        assert_eq!(cloned.config.heartbeat_interval_secs, 20);
    }

    #[test]
    fn test_network_message_large_task_request() {
        // Test with a large payload
        let large_payload = serde_json::json!({
            "query": "SELECT * FROM large_table",
            "parameters": (0..1000).map(|i| format!("param_{}", i)).collect::<Vec<_>>(),
            "metadata": {
                "user_id": "test-user",
                "session_id": "test-session",
                "timestamp": 1234567890
            }
        });

        let message = NetworkMessage::TaskRequest {
            task: DistributedTask {
                id: TaskId("large-task".to_string()),
                task_type: TaskType::Query,
                payload: large_payload,
                priority: 5,
                timeout_ms: 30000,
                dependencies: vec![TaskId("dep1".to_string()), TaskId("dep2".to_string())],
            },
            requester_id: NodeId("large-requester".to_string()),
        };

        match message {
            NetworkMessage::TaskRequest { task, requester_id } => {
                assert_eq!(task.id.0, "large-task");
                assert_eq!(task.priority, 5);
                assert_eq!(task.timeout_ms, 30000);
                assert_eq!(task.dependencies.len(), 2);
                assert_eq!(requester_id.0, "large-requester");
                assert!(task.payload.get("parameters").unwrap().as_array().unwrap().len() == 1000);
            }
            _ => panic!("Expected TaskRequest"),
        }
    }

    #[test]
    fn test_task_result_nested_partial() {
        // Test deeply nested partial results
        let nested_result = TaskResult::Partial(vec![
            PartialTaskResult {
                subtask_id: "level1".to_string(),
                result: Box::new(TaskResult::Partial(vec![
                    PartialTaskResult {
                        subtask_id: "level2".to_string(),
                        result: Box::new(TaskResult::Success(GraphInstance::default())),
                        execution_time_ms: 50,
                    }
                ])),
                execution_time_ms: 100,
            }
        ]);

        match nested_result {
            TaskResult::Partial(results) => {
                assert_eq!(results.len(), 1);
                assert_eq!(results[0].subtask_id, "level1");
                assert_eq!(results[0].execution_time_ms, 100);

                match *results[0].result {
                    TaskResult::Partial(sub_results) => {
                        assert_eq!(sub_results.len(), 1);
                        assert_eq!(sub_results[0].subtask_id, "level2");
                        assert_eq!(sub_results[0].execution_time_ms, 50);
                        assert!(matches!(*sub_results[0].result, TaskResult::Success(_)));
                    }
                    _ => panic!("Expected nested Partial"),
                }
            }
            _ => panic!("Expected Partial"),
        }
    }

    #[test]
    fn test_node_capabilities_extreme_values() {
        // Test with extreme values
        let capabilities = NodeCapabilities {
            cpu_cores: usize::MAX,
            memory_mb: usize::MAX,
            supported_cid_ranges: vec![CidRange { start: u64::MIN, end: u64::MAX }],
            features: vec![
                NodeFeature::GpuAcceleration,
                NodeFeature::HighMemory,
                NodeFeature::StorageOptimized,
                NodeFeature::NetworkOptimized,
            ],
        };

        assert_eq!(capabilities.cpu_cores, usize::MAX);
        assert_eq!(capabilities.memory_mb, usize::MAX);
        assert_eq!(capabilities.supported_cid_ranges[0].start, u64::MIN);
        assert_eq!(capabilities.supported_cid_ranges[0].end, u64::MAX);
        assert_eq!(capabilities.features.len(), 4);
    }

    #[test]
    fn test_cluster_config_edge_cases() {
        // Test with zero values
        let config1 = ClusterConfig {
            heartbeat_interval_secs: 0,
            timeout_secs: 0,
            max_retries: 0,
            load_threshold: 0.0,
        };

        assert_eq!(config1.heartbeat_interval_secs, 0);
        assert_eq!(config1.timeout_secs, 0);
        assert_eq!(config1.max_retries, 0);
        assert_eq!(config1.load_threshold, 0.0);

        // Test with maximum values
        let config2 = ClusterConfig {
            heartbeat_interval_secs: u64::MAX,
            timeout_secs: u64::MAX,
            max_retries: usize::MAX,
            load_threshold: 1.0,
        };

        assert_eq!(config2.heartbeat_interval_secs, u64::MAX);
        assert_eq!(config2.timeout_secs, u64::MAX);
        assert_eq!(config2.max_retries, usize::MAX);
        assert_eq!(config2.load_threshold, 1.0);
    }

    #[test]
    fn test_cache_sync_entry_large_values() {
        let cache_entry = CacheSyncEntry {
            cid: Cid("large-cid".to_string()),
            last_accessed: u64::MAX,
            access_count: u64::MAX,
            size_bytes: usize::MAX,
        };

        assert_eq!(cache_entry.last_accessed, u64::MAX);
        assert_eq!(cache_entry.access_count, u64::MAX);
        assert_eq!(cache_entry.size_bytes, usize::MAX);
    }

    #[test]
    fn test_network_message_empty_join_response() {
        let message = NetworkMessage::JoinResponse {
            accepted: false,
            cluster_info: None,
            reason: Some("Node not compatible".to_string()),
        };

        match message {
            NetworkMessage::JoinResponse { accepted, cluster_info, reason } => {
                assert!(!accepted);
                assert!(cluster_info.is_none());
                assert_eq!(reason, Some("Node not compatible".to_string()));
            }
            _ => panic!("Expected JoinResponse"),
        }
    }

    #[test]
    fn test_task_result_empty_partial() {
        let result = TaskResult::Partial(vec![]);

        match result {
            TaskResult::Partial(results) => {
                assert!(results.is_empty());
            }
            _ => panic!("Expected Partial"),
        }
    }

    #[test]
    fn test_node_capabilities_empty() {
        let capabilities = NodeCapabilities {
            cpu_cores: 0,
            memory_mb: 0,
            supported_cid_ranges: vec![],
            features: vec![],
        };

        assert_eq!(capabilities.cpu_cores, 0);
        assert_eq!(capabilities.memory_mb, 0);
        assert!(capabilities.supported_cid_ranges.is_empty());
        assert!(capabilities.features.is_empty());
    }

    #[test]
    fn test_cid_range_edge_cases() {
        // Test with same start and end
        let range1 = CidRange { start: 100, end: 100 };
        assert_eq!(range1.start, range1.end);

        // Test with end before start (invalid but allowed by struct)
        let range2 = CidRange { start: 200, end: 100 };
        assert!(range2.start > range2.end);
    }

    #[test]
    fn test_network_message_complex_graph_transfer() {
        let graph_cid = Cid("complex-graph-cid".to_string());
        let data = GraphInstance::default();
        let compression = CompressionType::Lz4;

        let message = NetworkMessage::GraphTransfer {
            graph_cid: graph_cid.clone(),
            data,
            compression,
        };

        match message {
            NetworkMessage::GraphTransfer { graph_cid: cid, data: _, compression: comp } => {
                assert_eq!(cid, graph_cid);
                assert!(matches!(comp, CompressionType::Lz4));
            }
            _ => panic!("Expected GraphTransfer"),
        }
    }

    #[test]
    fn test_partial_task_result_with_failure() {
        let failure_result = TaskResult::Failure {
            error: "Subtask execution failed".to_string(),
            retryable: false,
        };

        let partial = PartialTaskResult {
            subtask_id: "failed-subtask".to_string(),
            result: Box::new(failure_result),
            execution_time_ms: 500,
        };

        assert_eq!(partial.subtask_id, "failed-subtask");
        assert_eq!(partial.execution_time_ms, 500);

        match *partial.result {
            TaskResult::Failure { error, retryable } => {
                assert_eq!(error, "Subtask execution failed");
                assert!(!retryable);
            }
            _ => panic!("Expected Failure in partial result"),
        }
    }

    #[test]
    fn test_cluster_info_with_max_nodes() {
        let cluster_info = ClusterInfo {
            cluster_id: "max-cluster".to_string(),
            total_nodes: usize::MAX,
            active_nodes: usize::MAX,
            master_node: NodeId("max-master".to_string()),
            config: ClusterConfig {
                heartbeat_interval_secs: u64::MAX,
                timeout_secs: u64::MAX,
                max_retries: usize::MAX,
                load_threshold: 1.0,
            },
        };

        assert_eq!(cluster_info.total_nodes, usize::MAX);
        assert_eq!(cluster_info.active_nodes, usize::MAX);
        assert_eq!(cluster_info.config.heartbeat_interval_secs, u64::MAX);
        assert_eq!(cluster_info.config.max_retries, usize::MAX);
    }

    #[test]
    fn test_network_message_heartbeat_edge_cases() {
        // Test with extreme load values
        let message1 = NetworkMessage::Heartbeat {
            node_id: NodeId("test-node".to_string()),
            status: NodeStatus::Active,
            load: 0.0,
        };

        let message2 = NetworkMessage::Heartbeat {
            node_id: NodeId("test-node".to_string()),
            status: NodeStatus::Inactive,
            load: 1.0,
        };

        match message1 {
            NetworkMessage::Heartbeat { load: l, .. } => assert_eq!(l, 0.0),
            _ => panic!("Expected Heartbeat"),
        }

        match message2 {
            NetworkMessage::Heartbeat { load: l, status: s, .. } => {
                assert_eq!(l, 1.0);
                assert_eq!(s, NodeStatus::Inactive);
            }
            _ => panic!("Expected Heartbeat"),
        }
    }
}

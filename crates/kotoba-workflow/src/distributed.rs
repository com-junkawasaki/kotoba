//! Distributed Workflow Execution - Phase 2
//!
//! 分散環境でのワークフロー実行をサポートするコンポーネント。
//! ノード間でのタスク分散、負荷分散、フェイルオーバーを実現。

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::ir::{WorkflowExecutionId, WorkflowExecution, ExecutionStatus};
use kotoba_errors::WorkflowError;

/// ノード情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub address: String,
    pub capacity: usize, // 同時実行可能なワークフロー数
    pub active_workflows: usize,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
}

/// 分散実行タスク
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedTask {
    pub task_id: String,
    pub execution_id: WorkflowExecutionId,
    pub node_id: Option<String>, // 割り当てられたノード
    pub status: TaskStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub assigned_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// タスク実行状態
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    Pending,
    Assigned,
    Running,
    Completed,
    Failed,
}

/// 分散コーディネーター
pub struct DistributedCoordinator {
    nodes: RwLock<HashMap<String, NodeInfo>>,
    tasks: RwLock<HashMap<String, DistributedTask>>,
    /// ノード選択戦略
    load_balancer: Arc<dyn LoadBalancer>,
}

/// 負荷分散インターフェース
#[async_trait::async_trait]
pub trait LoadBalancer: Send + Sync {
    /// 最適なノードを選択
    async fn select_node(&self, nodes: &HashMap<String, NodeInfo>) -> Option<String>;
}

/// ラウンドロビン負荷分散
pub struct RoundRobinBalancer {
    current_index: std::sync::atomic::AtomicUsize,
}

impl RoundRobinBalancer {
    pub fn new() -> Self {
        Self {
            current_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl LoadBalancer for RoundRobinBalancer {
    async fn select_node(&self, nodes: &HashMap<String, NodeInfo>) -> Option<String> {
        let available_nodes: Vec<_> = nodes.values()
            .filter(|node| node.active_workflows < node.capacity)
            .collect();

        if available_nodes.is_empty() {
            return None;
        }

        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let selected = available_nodes[index % available_nodes.len()];
        Some(selected.node_id.clone())
    }
}

/// 最小負荷優先の負荷分散
pub struct LeastLoadedBalancer;

impl LeastLoadedBalancer {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl LoadBalancer for LeastLoadedBalancer {
    async fn select_node(&self, nodes: &HashMap<String, NodeInfo>) -> Option<String> {
        nodes.values()
            .filter(|node| node.active_workflows < node.capacity)
            .min_by_key(|node| node.active_workflows)
            .map(|node| node.node_id.clone())
    }
}

impl DistributedCoordinator {
    pub fn new(load_balancer: Arc<dyn LoadBalancer>) -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            tasks: RwLock::new(HashMap::new()),
            load_balancer,
        }
    }

    /// ノードを登録
    pub async fn register_node(&self, node: NodeInfo) {
        let mut nodes = self.nodes.write().await;
        nodes.insert(node.node_id.clone(), node);
    }

    /// ノードを削除
    pub async fn unregister_node(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        nodes.remove(node_id);
    }

    /// ノードのハートビートを更新
    pub async fn update_heartbeat(&self, node_id: &str) {
        let mut nodes = self.nodes.write().await;
        if let Some(node) = nodes.get_mut(node_id) {
            node.last_heartbeat = chrono::Utc::now();
        }
    }

    /// ワークフロー実行を分散実行キューに追加
    pub async fn submit_workflow(&self, execution_id: WorkflowExecutionId) -> Result<String, WorkflowError> {
        let task = DistributedTask {
            task_id: uuid::Uuid::new_v4().to_string(),
            execution_id,
            node_id: None,
            status: TaskStatus::Pending,
            created_at: chrono::Utc::now(),
            assigned_at: None,
            completed_at: None,
        };

        let mut tasks = self.tasks.write().await;
        let task_id = task.task_id.clone();
        tasks.insert(task_id.clone(), task);

        Ok(task_id)
    }

    /// 利用可能なノードにタスクを割り当て
    pub async fn assign_task(&self, task_id: &str) -> Result<Option<String>, WorkflowError> {
        let mut tasks = self.tasks.write().await;
        let nodes = self.nodes.read().await;

        if let Some(task) = tasks.get_mut(task_id) {
            if task.status != TaskStatus::Pending {
                return Ok(None);
            }

            if let Some(node_id) = self.load_balancer.select_node(&nodes).await {
                task.node_id = Some(node_id.clone());
                task.status = TaskStatus::Running;
                task.assigned_at = Some(chrono::Utc::now());

                // ノードのアクティブワークフロー数を更新
                drop(tasks);
                let mut nodes = self.nodes.write().await;
                if let Some(node) = nodes.get_mut(&node_id) {
                    node.active_workflows += 1;
                }

                return Ok(Some(node_id));
            }
        }

        Ok(None)
    }

    /// タスク完了を報告
    pub async fn complete_task(&self, task_id: &str, success: bool) -> Result<(), WorkflowError> {
        let mut tasks = self.tasks.write().await;

        if let Some(task) = tasks.get_mut(task_id) {
            task.status = if success { TaskStatus::Completed } else { TaskStatus::Failed };
            task.completed_at = Some(chrono::Utc::now());

            // ノードのアクティブワークフロー数を減らす
            if let Some(node_id) = &task.node_id {
                drop(tasks);
                let mut nodes = self.nodes.write().await;
                if let Some(node) = nodes.get_mut(node_id) {
                    node.active_workflows = node.active_workflows.saturating_sub(1);
                }
            }
        }

        Ok(())
    }

    /// 実行中のタスクを取得
    pub async fn get_running_tasks(&self) -> Vec<DistributedTask> {
        let tasks = self.tasks.read().await;
        tasks.values()
            .filter(|task| matches!(task.status, TaskStatus::Running | TaskStatus::Assigned))
            .cloned()
            .collect()
    }

    /// ノードの負荷情報を取得
    pub async fn get_node_load(&self, node_id: &str) -> Option<f64> {
        let nodes = self.nodes.read().await;
        nodes.get(node_id).map(|node| {
            if node.capacity == 0 {
                0.0
            } else {
                node.active_workflows as f64 / node.capacity as f64
            }
        })
    }

    /// クラスター全体の負荷を取得
    pub async fn get_cluster_load(&self) -> f64 {
        let nodes = self.nodes.read().await;
        if nodes.is_empty() {
            return 0.0;
        }

        let total_active: usize = nodes.values().map(|n| n.active_workflows).sum();
        let total_capacity: usize = nodes.values().map(|n| n.capacity).sum();

        if total_capacity == 0 {
            0.0
        } else {
            total_active as f64 / total_capacity as f64
        }
    }

    /// デッドノードの検出とクリーンアップ
    pub async fn cleanup_dead_nodes(&self, timeout: std::time::Duration) {
        let mut nodes = self.nodes.write().await;
        let now = chrono::Utc::now();

        let dead_nodes: Vec<String> = nodes.values()
            .filter(|node| {
                let duration = now.signed_duration_since(node.last_heartbeat);
                duration.to_std().unwrap_or(std::time::Duration::from_secs(0)) > timeout
            })
            .map(|node| node.node_id.clone())
            .collect();

        for node_id in dead_nodes {
            println!("Removing dead node: {}", node_id);
            nodes.remove(&node_id);
        }
    }

    /// フェイルオーバー: 失敗したタスクを別のノードに再割り当て
    pub async fn failover_task(&self, task_id: &str) -> Result<Option<String>, WorkflowError> {
        // まずタスクの状態を確認
        let task_info = {
            let tasks = self.tasks.read().await;
            tasks.get(task_id).map(|t| (t.status.clone(), t.node_id.clone()))
        };

        if let Some((TaskStatus::Failed, Some(old_node_id))) = task_info {
            // 以前のノードのカウンターを減らす
            {
                let mut nodes = self.nodes.write().await;
                if let Some(node) = nodes.get_mut(&old_node_id) {
                    node.active_workflows = node.active_workflows.saturating_sub(1);
                }
            }

            // 新しいノードを探す
            let nodes = self.nodes.read().await;
            if let Some(new_node_id) = self.load_balancer.select_node(&nodes).await {
                let mut tasks = self.tasks.write().await;
                if let Some(task) = tasks.get_mut(task_id) {
                    task.node_id = Some(new_node_id.clone());
                    task.status = TaskStatus::Running;
                    task.assigned_at = Some(chrono::Utc::now());

                    // 新しいノードのカウンターを増やす
                        drop(tasks);
                        let mut nodes = self.nodes.write().await;
                        if let Some(node) = nodes.get_mut(&new_node_id) {
                            node.active_workflows += 1;
                        }

                        return Ok(Some(new_node_id));
                    }
                }
            }
        }

        Ok(None)
    }

/// 分散実行マネージャー
pub struct DistributedExecutionManager {
    coordinator: Arc<DistributedCoordinator>,
    /// ローカルノードID
    local_node_id: String,
}

impl DistributedExecutionManager {
    pub fn new(local_node_id: String, load_balancer: Arc<dyn LoadBalancer>) -> Self {
        Self {
            coordinator: Arc::new(DistributedCoordinator::new(load_balancer)),
            local_node_id,
        }
    }

    /// 分散コーディネーターを取得
    pub fn coordinator(&self) -> &Arc<DistributedCoordinator> {
        &self.coordinator
    }

    /// ワークフロー実行を分散キューに投入
    pub async fn submit_execution(&self, execution_id: WorkflowExecutionId) -> Result<String, WorkflowError> {
        self.coordinator.submit_workflow(execution_id).await
    }

    /// ローカルノードの情報を登録
    pub async fn register_local_node(&self, capacity: usize) {
        let node = NodeInfo {
            node_id: self.local_node_id.clone(),
            address: "localhost:8080".to_string(), // TODO: 設定から取得
            capacity,
            active_workflows: 0,
            last_heartbeat: chrono::Utc::now(),
        };

        self.coordinator.register_node(node).await;
    }

    /// バックグラウンドでデッドノードクリーンアップを実行
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let coordinator = Arc::clone(&self.coordinator);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));

            loop {
                interval.tick().await;
                coordinator.cleanup_dead_nodes(std::time::Duration::from_secs(60)).await;
            }
        })
    }
}

/// 分散ワークフロー実行器
pub struct DistributedWorkflowExecutor {
    pub execution_manager: Arc<DistributedExecutionManager>,
    /// ノードごとの実行統計
    execution_stats: RwLock<HashMap<String, NodeExecutionStats>>,
}

#[derive(Debug, Clone)]
pub struct NodeExecutionStats {
    pub total_tasks: usize,
    pub successful_tasks: usize,
    pub failed_tasks: usize,
    pub avg_execution_time: std::time::Duration,
}

impl DistributedWorkflowExecutor {
    pub fn new(execution_manager: Arc<DistributedExecutionManager>) -> Self {
        Self {
            execution_manager,
            execution_stats: RwLock::new(HashMap::new()),
        }
    }

    /// 分散実行の統計情報を取得
    pub async fn get_execution_stats(&self) -> HashMap<String, NodeExecutionStats> {
        let stats = self.execution_stats.read().await;
        stats.clone()
    }

    /// クラスター全体のヘルスチェック
    pub async fn cluster_health_check(&self) -> ClusterHealth {
        let cluster_load = self.execution_manager.coordinator.get_cluster_load().await;
        let running_tasks = self.execution_manager.coordinator.get_running_tasks().await;

        ClusterHealth {
            cluster_load,
            active_tasks: running_tasks.len(),
            healthy_nodes: 0, // TODO: ヘルスチェック実装
            unhealthy_nodes: 0,
        }
    }
}

#[derive(Debug)]
pub struct ClusterHealth {
    pub cluster_load: f64,
    pub active_tasks: usize,
    pub healthy_nodes: usize,
    pub unhealthy_nodes: usize,
}

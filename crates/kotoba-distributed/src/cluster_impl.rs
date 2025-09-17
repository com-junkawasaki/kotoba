//! ClusterManager と LoadBalancer の実装

use super::*;
use kotoba_errors::KotobaError;
use std::collections::HashMap;

impl ClusterManager {
    /// 新しいクラスタマネージャーを作成
    pub fn new(local_node_id: NodeId) -> Self {
        Self {
            nodes: HashMap::new(),
            local_node_id,
            load_balancer: LoadBalancer::new(),
        }
    }

    /// 利用可能なノードを取得
    pub fn get_available_nodes(&self) -> Vec<&ClusterNode> {
        self.nodes.values()
            .filter(|node| node.status == NodeStatus::Active)
            .collect()
    }

    /// ノードを追加
    pub fn add_node(&mut self, node: ClusterNode) {
        self.nodes.insert(node.id.clone(), node);
    }

    /// ノードを削除
    pub fn remove_node(&mut self, node_id: &NodeId) {
        self.nodes.remove(node_id);
    }

    /// 完了時間の見積もり
    fn estimate_completion_time(&self, assignments: &HashMap<NodeId, Vec<DistributedTask>>) -> std::time::Duration {
        let mut max_time = std::time::Duration::from_millis(0);

        for (node_id, tasks) in assignments {
            if let Some(node) = self.nodes.get(node_id) {
                let node_time = std::time::Duration::from_millis((tasks.len() as u64) * 1000 / (node.load as u64 + 1));
                if node_time > max_time {
                    max_time = node_time;
                }
            }
        }

        max_time
    }
}

impl LoadBalancer {
    /// 新しい負荷分散器を作成
    pub fn new() -> Self {
        Self {
            node_loads: HashMap::new(),
            sharding_strategy: ShardingStrategy::LoadBased,
        }
    }

    /// タスクをノードに割り当て
    pub fn assign_tasks(
        &self,
        tasks: &[DistributedTask],
        nodes: &[&ClusterNode],
    ) -> kotoba_core::types::Result<HashMap<NodeId, Vec<DistributedTask>>> {
        let mut assignments: HashMap<NodeId, Vec<DistributedTask>> = HashMap::new();

        for task in tasks {
            let best_node = self.select_best_node(task, nodes)?;
            assignments.entry(best_node.id.clone())
                .or_insert_with(Vec::new)
                .push(task.clone());
        }

        Ok(assignments)
    }

    /// 最適なノードを選択
    fn select_best_node<'a>(&self, task: &DistributedTask, nodes: &[&'a ClusterNode]) -> kotoba_core::types::Result<&'a ClusterNode> {
        if nodes.is_empty() {
            return Err(KotobaError::Execution("No available nodes".to_string()));
        }

        // 簡易版: 最も負荷の低いノードを選択
        let best_node = nodes.iter()
            .min_by(|a, b| a.load.partial_cmp(&b.load).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap();

        Ok(best_node)
    }
}

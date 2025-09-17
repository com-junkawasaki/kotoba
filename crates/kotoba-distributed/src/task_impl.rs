//! タスク関連の実装とテスト

use super::*;


#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cid_cache() {
        let cache = CidCache::new();
        assert_eq!(cache.get_stats().entries, 0);
    }

    #[test]
    fn test_cluster_manager() {
        let node_id = NodeId("test_node".to_string());
        let manager = ClusterManager::new(node_id.clone());
        assert_eq!(manager.local_node_id, node_id);
    }

    #[test]
    fn test_load_balancer() {
        let balancer = LoadBalancer::new();
        assert!(matches!(balancer.sharding_strategy, ShardingStrategy::LoadBased));
    }

    #[test]
    fn test_distributed_task_creation() {
        let task = DistributedTask {
            id: TaskId("test_task".to_string()),
            task_type: TaskType::RuleApplication {
            rule_cid: Cid::new("rule_cid"),
                host_graph_cid: Cid::new("host_cid"),
            },
            input: TaskInput::CidReference(Cid::new("input_cid")),
            priority: TaskPriority::Normal,
            timeout: Some(std::time::Duration::from_secs(60)),
        };

        assert_eq!(task.id.0, "test_task");
        assert!(matches!(task.task_type, TaskType::RuleApplication { .. }));
    }
}

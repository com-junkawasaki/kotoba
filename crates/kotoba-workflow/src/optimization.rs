//! Workflow Optimization - Phase 3
//!
//! ワークフロー実行の最適化を提供します。
//! コストベース最適化、並列実行最適化、クエリ最適化などをサポート。

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use serde::{Deserialize, Serialize};

use crate::ir::{WorkflowIR, WorkflowStrategyOp, ActivityIR};

/// 最適化戦略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizationStrategy {
    /// コストベース最適化
    CostBased {
        cost_model: CostModel,
        budget_limit: Option<f64>,
    },
    /// パフォーマンスベース最適化
    PerformanceBased {
        target_throughput: f64,
        target_latency: std::time::Duration,
    },
    /// リソースベース最適化
    ResourceBased {
        max_parallelism: usize,
        resource_limits: HashMap<String, f64>,
    },
    /// 品質ベース最適化
    QualityBased {
        reliability_target: f64,
        quality_metrics: HashMap<String, f64>,
    },
}

/// コストモデル
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostModel {
    /// CPUコスト係数
    pub cpu_cost_factor: f64,
    /// メモリコスト係数
    pub memory_cost_factor: f64,
    /// IOコスト係数
    pub io_cost_factor: f64,
    /// ネットワークコスト係数
    pub network_cost_factor: f64,
    /// Activityごとのコスト
    pub activity_costs: HashMap<String, ActivityCost>,
}

/// Activityコスト情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityCost {
    pub base_cost: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub io_operations: f64,
    pub network_usage: f64,
    pub estimated_duration: std::time::Duration,
}

/// 最適化結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub original_workflow: WorkflowIR,
    pub optimized_workflow: WorkflowIR,
    pub estimated_cost: f64,
    pub estimated_duration: std::time::Duration,
    pub improvements: Vec<OptimizationImprovement>,
    pub applied_optimizations: Vec<String>,
}

/// 最適化改善点
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationImprovement {
    pub improvement_type: String,
    pub description: String,
    pub cost_savings: f64,
    pub duration_reduction: std::time::Duration,
    pub confidence: f64, // 0.0 to 1.0
}

/// 並列実行計画
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelExecutionPlan {
    pub stages: Vec<ExecutionStage>,
    pub dependencies: HashMap<String, Vec<String>>,
    pub resource_requirements: HashMap<String, ResourceRequirement>,
    pub estimated_parallelism: usize,
}

/// 実行ステージ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStage {
    pub stage_id: String,
    pub activities: Vec<String>,
    pub execution_mode: ExecutionMode,
    pub estimated_duration: std::time::Duration,
    pub resource_usage: ResourceUsage,
}

/// 実行モード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionMode {
    Sequential,
    Parallel,
    Conditional,
    Loop,
}

/// リソース要件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub cpu_cores: usize,
    pub memory_mb: usize,
    pub io_bandwidth: f64,
    pub network_bandwidth: f64,
}

/// リソース使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub io_usage: f64,
    pub network_usage: f64,
}

/// ワークフロー最適化エンジン
pub struct WorkflowOptimizer {
    cost_model: CostModel,
    resource_manager: ResourceManager,
    optimization_rules: Vec<Box<dyn OptimizationRule>>,
}

#[async_trait::async_trait]
pub trait OptimizationRule: Send + Sync {
    async fn apply(&self, workflow: &WorkflowIR, context: &OptimizationContext) -> Option<OptimizationResult>;
    fn name(&self) -> &str;
    fn priority(&self) -> i32; // 低いほど優先度が高い
}

/// 最適化コンテキスト
#[derive(Debug, Clone)]
pub struct OptimizationContext {
    pub available_resources: HashMap<String, f64>,
    pub historical_performance: HashMap<String, ActivityPerformance>,
    pub target_constraints: OptimizationStrategy,
    pub execution_history: Vec<WorkflowExecution>,
}

/// Activityパフォーマンス情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityPerformance {
    pub activity_name: String,
    pub avg_execution_time: std::time::Duration,
    pub success_rate: f64,
    pub resource_usage: ResourceUsage,
    pub cost_per_execution: f64,
    pub sample_size: usize,
}

/// ワークフロー実行履歴
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub workflow_id: String,
    pub execution_time: std::time::Duration,
    pub cost: f64,
    pub success: bool,
    pub resource_usage: ResourceUsage,
}

/// リソースマネージャー
pub struct ResourceManager {
    available_resources: HashMap<String, f64>,
    allocated_resources: HashMap<String, f64>,
}

impl ResourceManager {
    pub fn new(available_resources: HashMap<String, f64>) -> Self {
        Self {
            available_resources,
            allocated_resources: HashMap::new(),
        }
    }

    /// リソースが利用可能かチェック
    pub fn check_resource_availability(&self, requirements: &ResourceRequirement) -> bool {
        let available_cpu = self.available_resources.get("cpu").unwrap_or(&0.0) - self.allocated_resources.get("cpu").unwrap_or(&0.0);
        let available_memory = self.available_resources.get("memory").unwrap_or(&0.0) - self.allocated_resources.get("memory").unwrap_or(&0.0);

        available_cpu >= requirements.cpu_cores as f64 && available_memory >= requirements.memory_mb as f64
    }

    /// リソースを割り当て
    pub fn allocate_resources(&mut self, requirements: &ResourceRequirement) -> bool {
        if !self.check_resource_availability(requirements) {
            return false;
        }

        *self.allocated_resources.entry("cpu".to_string()).or_insert(0.0) += requirements.cpu_cores as f64;
        *self.allocated_resources.entry("memory".to_string()).or_insert(0.0) += requirements.memory_mb as f64;

        true
    }

    /// リソースを解放
    pub fn release_resources(&mut self, requirements: &ResourceRequirement) {
        if let Some(cpu) = self.allocated_resources.get_mut("cpu") {
            *cpu = (*cpu - requirements.cpu_cores as f64).max(0.0);
        }
        if let Some(memory) = self.allocated_resources.get_mut("memory") {
            *memory = (*memory - requirements.memory_mb as f64).max(0.0);
        }
    }
}

impl WorkflowOptimizer {
    pub fn new(cost_model: CostModel, resource_manager: ResourceManager) -> Self {
        Self {
            cost_model,
            resource_manager,
            optimization_rules: Vec::new(),
        }
    }

    /// 最適化ルールを追加
    pub fn add_rule(&mut self, rule: Box<dyn OptimizationRule>) {
        self.optimization_rules.push(rule);
        self.optimization_rules.sort_by_key(|r| r.priority());
    }

    /// ワークフローを最適化
    pub async fn optimize_workflow(
        &self,
        workflow: &WorkflowIR,
        context: &OptimizationContext,
    ) -> Result<OptimizationResult, OptimizationError> {
        let mut best_result = None;
        let mut best_score = f64::INFINITY;

        // 各最適化ルールを適用
        for rule in &self.optimization_rules {
            if let Some(result) = rule.apply(workflow, context).await {
                let score = self.calculate_optimization_score(&result);
                if score < best_score {
                    best_score = score;
                    best_result = Some(result);
                }
            }
        }

        best_result.ok_or(OptimizationError::NoOptimizationPossible)
    }

    /// 並列実行計画を生成
    pub async fn generate_parallel_plan(
        &self,
        workflow: &WorkflowIR,
        max_parallelism: usize,
    ) -> Result<ParallelExecutionPlan, OptimizationError> {
        let mut stages = Vec::new();
        let mut dependencies = HashMap::new();
        let mut processed = HashSet::new();
        let mut queue = VecDeque::new();

        // 依存関係のないActivityから開始
        queue.push_back(workflow.strategy.clone());

        while !queue.is_empty() {
            let mut current_stage_activities = Vec::new();
            let mut current_dependencies = HashMap::new();

            // 現在のステージで実行可能なActivityを収集
            let batch_size = std::cmp::min(queue.len(), max_parallelism);
            for _ in 0..batch_size {
                if let Some(strategy) = queue.pop_front() {
                    match strategy {
                        WorkflowStrategyOp::Activity { activity_ref, .. } => {
                            if !processed.contains(&activity_ref) {
                                current_stage_activities.push(activity_ref.clone());
                                processed.insert(activity_ref);
                            }
                        }
                        WorkflowStrategyOp::Seq { strategies } => {
                            for strategy in strategies {
                                queue.push_back(*strategy);
                            }
                        }
                        WorkflowStrategyOp::Parallel { branches, .. } => {
                            for branch in branches {
                                queue.push_back(*branch);
                            }
                        }
                        _ => {
                            // その他の戦略は後で処理
                            queue.push_back(strategy);
                        }
                    }
                }
            }

            if !current_stage_activities.is_empty() {
                let stage = ExecutionStage {
                    stage_id: format!("stage_{}", stages.len()),
                    activities: current_stage_activities,
                    execution_mode: ExecutionMode::Parallel,
                    estimated_duration: std::time::Duration::from_secs(1), // TODO: 実際の推定時間を計算
                    resource_usage: ResourceUsage {
                        cpu_usage: 1.0,
                        memory_usage: 100.0,
                        io_usage: 0.0,
                        network_usage: 0.0,
                    },
                };
                stages.push(stage);
            }
        }

        Ok(ParallelExecutionPlan {
            stages,
            dependencies,
            resource_requirements: HashMap::new(), // TODO: リソース要件を計算
            estimated_parallelism: max_parallelism,
        })
    }

    /// コストを見積もり
    pub fn estimate_cost(&self, workflow: &WorkflowIR) -> f64 {
        let mut total_cost = 0.0;

        self.traverse_workflow(&workflow.strategy, &mut |activity_ref| {
            if let Some(activity_cost) = self.cost_model.activity_costs.get(activity_ref) {
                total_cost += activity_cost.base_cost
                    + activity_cost.cpu_usage * self.cost_model.cpu_cost_factor
                    + activity_cost.memory_usage * self.cost_model.memory_cost_factor
                    + activity_cost.io_operations * self.cost_model.io_cost_factor
                    + activity_cost.network_usage * self.cost_model.network_cost_factor;
            }
        });

        total_cost
    }

    /// 実行時間を推定
    pub fn estimate_duration(&self, workflow: &WorkflowIR) -> std::time::Duration {
        let mut total_duration = std::time::Duration::from_secs(0);

        self.traverse_workflow(&workflow.strategy, &mut |activity_ref| {
            if let Some(activity_cost) = self.cost_model.activity_costs.get(activity_ref) {
                total_duration += activity_cost.estimated_duration;
            }
        });

        total_duration
    }

    /// ワークフローをトラバースしてActivityを処理
    fn traverse_workflow<F>(&self, strategy: &WorkflowStrategyOp, processor: &mut F)
    where
        F: FnMut(&str),
    {
        match strategy {
            WorkflowStrategyOp::Activity { activity_ref, .. } => {
                processor(activity_ref);
            }
            WorkflowStrategyOp::Seq { strategies } => {
                for strategy in strategies {
                    self.traverse_workflow(strategy, processor);
                }
            }
            WorkflowStrategyOp::Parallel { branches, .. } => {
                for branch in branches {
                    self.traverse_workflow(branch, processor);
                }
            }
            WorkflowStrategyOp::Decision { conditions, default_branch } => {
                for branch in conditions {
                    self.traverse_workflow(&branch.branch, processor);
                }
                if let Some(default_branch) = default_branch {
                    self.traverse_workflow(default_branch, processor);
                }
            }
            WorkflowStrategyOp::Wait { .. } => {
                // Waitはコストなし
            }
            WorkflowStrategyOp::Saga { main_flow, compensation } => {
                self.traverse_workflow(main_flow, processor);
                self.traverse_workflow(compensation, processor);
            }
            WorkflowStrategyOp::SubWorkflow { .. } => {
                // TODO: サブワークフローのコストを計算
            }
            _ => {}
        }
    }

    /// 最適化結果のスコアを計算（低いほど良い）
    fn calculate_optimization_score(&self, result: &OptimizationResult) -> f64 {
        // コスト削減と時間削減のバランスを考慮したスコア
        let cost_weight = 0.6;
        let time_weight = 0.4;

        let cost_score = result.improvements.iter()
            .map(|imp| imp.cost_savings * imp.confidence)
            .sum::<f64>();

        let time_score = result.improvements.iter()
            .map(|imp| imp.duration_reduction.as_secs_f64() * imp.confidence)
            .sum::<f64>();

        -(cost_score * cost_weight + time_score * time_weight) // 負の値にして最大化問題を最小化問題に変換
    }
}

/// 並列実行最適化ルール
pub struct ParallelExecutionRule {
    max_parallelism: usize,
}

impl ParallelExecutionRule {
    pub fn new(max_parallelism: usize) -> Self {
        Self { max_parallelism }
    }
}

#[async_trait::async_trait]
impl OptimizationRule for ParallelExecutionRule {
    async fn apply(&self, workflow: &WorkflowIR, context: &OptimizationContext) -> Option<OptimizationResult> {
        // 並列実行可能な部分を特定
        let mut parallelizable_activities = Vec::new();

        Self::find_parallelizable_activities(&workflow.strategy, &mut parallelizable_activities);

        if parallelizable_activities.len() < 2 {
            return None;
        }

        // 並列実行を最適化
        let optimized_strategy = Self::optimize_parallel_execution(&workflow.strategy, self.max_parallelism);

        let improvements = vec![
            OptimizationImprovement {
                improvement_type: "parallel_execution".to_string(),
                description: format!("Parallelized {} activities", parallelizable_activities.len()),
                cost_savings: 0.0, // TODO: コスト削減を計算
                duration_reduction: std::time::Duration::from_secs(10), // TODO: 実際の時間削減を計算
                confidence: 0.8,
            }
        ];

        let mut optimized_workflow = workflow.clone();
        optimized_workflow.strategy = optimized_strategy;

        Some(OptimizationResult {
            original_workflow: workflow.clone(),
            optimized_workflow,
            estimated_cost: 0.0, // TODO: コストを計算
            estimated_duration: std::time::Duration::from_secs(30), // TODO: 時間を計算
            improvements,
            applied_optimizations: vec!["parallel_execution".to_string()],
        })
    }

    fn name(&self) -> &str {
        "parallel_execution"
    }

    fn priority(&self) -> i32 {
        1
    }
}

impl ParallelExecutionRule {
    fn find_parallelizable_activities(strategy: &WorkflowStrategyOp, activities: &mut Vec<String>) {
        match strategy {
            WorkflowStrategyOp::Activity { activity_ref, .. } => {
                activities.push(activity_ref.clone());
            }
            WorkflowStrategyOp::Seq { strategies } => {
                for strategy in strategies {
                    Self::find_parallelizable_activities(strategy, activities);
                }
            }
            _ => {
                // 他の戦略は並列化が難しい
            }
        }
    }

    fn optimize_parallel_execution(strategy: &WorkflowStrategyOp, max_parallelism: usize) -> WorkflowStrategyOp {
        match strategy {
            WorkflowStrategyOp::Seq { strategies } => {
                let mut optimized_strategies = Vec::new();

                // 並列実行可能なActivityをグループ化
                let mut parallel_group = Vec::new();
                let mut sequential_part = Vec::new();

                for strategy in strategies {
                    if Self::is_parallelizable(strategy) {
                        parallel_group.push(*strategy.clone());
                    } else {
                        if !parallel_group.is_empty() {
                            if parallel_group.len() > 1 {
                                optimized_strategies.push(WorkflowStrategyOp::Parallel {
                                    branches: parallel_group.into_iter().map(Box::new).collect(),
                                    completion_condition: crate::ir::CompletionCondition::All,
                                });
                            } else {
                                optimized_strategies.extend(parallel_group);
                            }
                            parallel_group = Vec::new();
                        }
                        sequential_part.push(*strategy.clone());
                    }
                }

                // 残りの並列グループを処理
                if !parallel_group.is_empty() {
                    if parallel_group.len() > 1 {
                        optimized_strategies.push(WorkflowStrategyOp::Parallel {
                            branches: parallel_group.into_iter().map(Box::new).collect(),
                            completion_condition: crate::ir::CompletionCondition::All,
                        });
                    } else {
                        optimized_strategies.extend(parallel_group.into_iter().map(Box::new));
                    }
                }

                WorkflowStrategyOp::Seq {
                    strategies: optimized_strategies.into_iter().map(Box::new).collect(),
                }
            }
            _ => strategy.clone(),
        }
    }

    fn is_parallelizable(strategy: &WorkflowStrategyOp) -> bool {
        matches!(strategy, WorkflowStrategyOp::Activity { .. })
    }
}

/// コストベース最適化ルール
pub struct CostBasedOptimizationRule {
    cost_threshold: f64,
}

impl CostBasedOptimizationRule {
    pub fn new(cost_threshold: f64) -> Self {
        Self { cost_threshold }
    }
}

#[async_trait::async_trait]
impl OptimizationRule for CostBasedOptimizationRule {
    async fn apply(&self, workflow: &WorkflowIR, context: &OptimizationContext) -> Option<OptimizationResult> {
        // TODO: コストベースの最適化を実装
        // 例: 高コストのActivityをより効率的な代替手段に置き換え

        Some(OptimizationResult {
            original_workflow: workflow.clone(),
            optimized_workflow: workflow.clone(), // TODO: 最適化されたワークフロー
            estimated_cost: 0.0,
            estimated_duration: std::time::Duration::from_secs(30),
            improvements: vec![],
            applied_optimizations: vec!["cost_based".to_string()],
        })
    }

    fn name(&self) -> &str {
        "cost_based"
    }

    fn priority(&self) -> i32 {
        2
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OptimizationError {
    #[error("No optimization possible")]
    NoOptimizationPossible,
    #[error("Resource constraints violated")]
    ResourceConstraintsViolated,
    #[error("Invalid optimization strategy")]
    InvalidStrategy,
}

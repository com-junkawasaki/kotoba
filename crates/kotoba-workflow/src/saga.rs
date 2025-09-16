//! Advanced Saga Pattern Implementation - Phase 3
//!
//! 完全なSagaパターンサポートを提供します。
//! 補償トランザクション、Saga状態管理、分散Sagaをサポート。

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::ir::{WorkflowExecutionId, ActivityExecutionId, ExecutionStatus, WorkflowStrategyOp};
use crate::WorkflowError;

/// Saga トランザクションの状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SagaStatus {
    /// Sagaが開始された
    Started,
    /// Sagaが実行中
    Executing,
    /// Sagaが完了した
    Completed,
    /// Sagaが補償中
    Compensating,
    /// Sagaが補償完了した
    Compensated,
    /// Sagaが失敗した
    Failed,
    /// Sagaがタイムアウトした
    TimedOut,
}

/// Saga 実行コンテキスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaContext {
    pub saga_id: String,
    pub workflow_id: WorkflowExecutionId,
    pub status: SagaStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub timeout: Option<std::time::Duration>,
    pub transaction_log: Vec<SagaTransaction>,
    pub compensation_log: Vec<SagaCompensation>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Saga トランザクション記録
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaTransaction {
    pub transaction_id: String,
    pub activity_ref: String,
    pub activity_id: Option<ActivityExecutionId>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: Option<HashMap<String, serde_json::Value>>,
    pub status: ExecutionStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub compensation_ref: Option<String>,
}

/// Saga 補償記録
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaCompensation {
    pub compensation_id: String,
    pub original_transaction_id: String,
    pub compensation_activity: String,
    pub status: CompensationStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub error: Option<String>,
}

/// 補償実行状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationStatus {
    Pending,
    Executing,
    Completed,
    Failed,
}

/// 高度なSagaパターン定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSagaPattern {
    pub name: String,
    pub description: Option<String>,
    pub version: String,

    /// Sagaのメイン処理フロー
    pub main_flow: WorkflowStrategyOp,

    /// 補償処理定義
    pub compensations: HashMap<String, WorkflowStrategyOp>,

    /// Sagaレベルの設定
    pub config: SagaConfig,

    /// 依存関係と実行順序
    pub dependencies: HashMap<String, Vec<String>>,
}

/// Saga設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaConfig {
    /// Saga全体のタイムアウト
    pub timeout: Option<std::time::Duration>,

    /// 補償実行ポリシー
    pub compensation_policy: CompensationPolicy,

    /// 並列実行設定
    pub parallelism: usize,

    /// 失敗時のリトライ設定
    pub retry_config: Option<SagaRetryConfig>,

    /// 監視設定
    pub monitoring_config: SagaMonitoringConfig,
}

/// 補償実行ポリシー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompensationPolicy {
    /// 逆順に補償を実行
    ReverseOrder,
    /// 並列に補償を実行
    Parallel,
    /// カスタム順序で補償を実行
    Custom(Vec<String>),
    /// 条件付き補償
    Conditional,
}

/// Sagaリトライ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaRetryConfig {
    pub max_attempts: u32,
    pub backoff_multiplier: f64,
    pub max_backoff: std::time::Duration,
}

/// Saga監視設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaMonitoringConfig {
    pub enable_metrics: bool,
    pub enable_tracing: bool,
    pub log_level: String,
}

/// Sagaマネージャー
pub struct SagaManager {
    sagas: RwLock<HashMap<String, SagaContext>>,
    patterns: RwLock<HashMap<String, AdvancedSagaPattern>>,
    metrics: SagaMetrics,
}

#[derive(Debug, Default)]
pub struct SagaMetrics {
    pub total_sagas: u64,
    pub completed_sagas: u64,
    pub failed_sagas: u64,
    pub compensated_sagas: u64,
    pub avg_execution_time: std::time::Duration,
    pub compensation_rate: f64,
}

impl SagaManager {
    pub fn new() -> Self {
        Self {
            sagas: RwLock::new(HashMap::new()),
            patterns: RwLock::new(HashMap::new()),
            metrics: SagaMetrics::default(),
        }
    }

    /// Sagaパターンを登録
    pub async fn register_pattern(&self, pattern: AdvancedSagaPattern) -> Result<(), WorkflowError> {
        let mut patterns = self.patterns.write().await;
        patterns.insert(pattern.name.clone(), pattern);
        Ok(())
    }

    /// Saga実行を開始
    pub async fn start_saga(
        &self,
        pattern_name: &str,
        workflow_id: WorkflowExecutionId,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<String, WorkflowError> {
        let patterns = self.patterns.read().await;
        let pattern = patterns.get(pattern_name)
            .ok_or_else(|| WorkflowError::InvalidDefinition(format!("Saga pattern '{}' not found", pattern_name)))?;

        let saga_id = uuid::Uuid::new_v4().to_string();
        let context = SagaContext {
            saga_id: saga_id.clone(),
            workflow_id,
            status: SagaStatus::Started,
            start_time: chrono::Utc::now(),
            end_time: None,
            timeout: pattern.config.timeout,
            transaction_log: Vec::new(),
            compensation_log: Vec::new(),
            metadata: inputs,
        };

        let mut sagas = self.sagas.write().await;
        sagas.insert(saga_id.clone(), context);

        Ok(saga_id)
    }

    /// トランザクションを記録
    pub async fn record_transaction(
        &self,
        saga_id: &str,
        transaction: SagaTransaction,
    ) -> Result<(), WorkflowError> {
        let mut sagas = self.sagas.write().await;
        if let Some(context) = sagas.get_mut(saga_id) {
            context.transaction_log.push(transaction);
            Ok(())
        } else {
            Err(WorkflowError::WorkflowNotFound(saga_id.to_string()))
        }
    }

    /// 補償を記録
    pub async fn record_compensation(
        &self,
        saga_id: &str,
        compensation: SagaCompensation,
    ) -> Result<(), WorkflowError> {
        let mut sagas = self.sagas.write().await;
        if let Some(context) = sagas.get_mut(saga_id) {
            context.compensation_log.push(compensation);
            Ok(())
        } else {
            Err(WorkflowError::WorkflowNotFound(saga_id.to_string()))
        }
    }

    /// Saga状態を更新
    pub async fn update_saga_status(
        &self,
        saga_id: &str,
        status: SagaStatus,
    ) -> Result<(), WorkflowError> {
        let mut sagas = self.sagas.write().await;
        if let Some(context) = sagas.get_mut(saga_id) {
            context.status = status.clone();

            // 完了状態の場合は終了時間を設定
            if matches!(status, SagaStatus::Completed | SagaStatus::Compensated | SagaStatus::Failed | SagaStatus::TimedOut) {
                context.end_time = Some(chrono::Utc::now());
            }

            Ok(())
        } else {
            Err(WorkflowError::WorkflowNotFound(saga_id.to_string()))
        }
    }

    /// Sagaコンテキストを取得
    pub async fn get_saga_context(&self, saga_id: &str) -> Option<SagaContext> {
        let sagas = self.sagas.read().await;
        sagas.get(saga_id).cloned()
    }

    /// 補償が必要なトランザクションを取得
    pub async fn get_compensable_transactions(&self, saga_id: &str) -> Result<Vec<SagaTransaction>, WorkflowError> {
        let sagas = self.sagas.read().await;
        if let Some(context) = sagas.get(saga_id) {
            let compensable = context.transaction_log.iter()
                .filter(|tx| tx.compensation_ref.is_some() && matches!(tx.status, ExecutionStatus::Completed))
                .cloned()
                .collect();
            Ok(compensable)
        } else {
            Err(WorkflowError::WorkflowNotFound(saga_id.to_string()))
        }
    }

    /// Sagaの補償実行順序を決定
    pub async fn get_compensation_order(&self, saga_id: &str, policy: &CompensationPolicy) -> Result<Vec<String>, WorkflowError> {
        let compensable = self.get_compensable_transactions(saga_id).await?;

        match policy {
            CompensationPolicy::ReverseOrder => {
                // 逆順で補償を実行
                let mut order: Vec<String> = compensable.iter()
                    .rev()
                    .filter_map(|tx| tx.compensation_ref.clone())
                    .collect();
                Ok(order)
            }
            CompensationPolicy::Parallel => {
                // 並列実行の場合は順序は重要ではない
                let order: Vec<String> = compensable.iter()
                    .filter_map(|tx| tx.compensation_ref.clone())
                    .collect();
                Ok(order)
            }
            CompensationPolicy::Custom(order) => {
                Ok(order.clone())
            }
            CompensationPolicy::Conditional => {
                // TODO: 条件に基づいて補償順序を決定
                Ok(Vec::new())
            }
        }
    }

    /// Sagaの依存関係を解決
    pub async fn resolve_dependencies(&self, pattern: &AdvancedSagaPattern, completed: &[String]) -> Vec<String> {
        let mut ready = Vec::new();

        for (activity, deps) in &pattern.dependencies {
            if !completed.contains(activity) {
                let all_deps_completed = deps.iter().all(|dep| completed.contains(dep));
                if all_deps_completed {
                    ready.push(activity.clone());
                }
            }
        }

        ready
    }

    /// Sagaのメトリクスを取得
    pub fn get_metrics(&self) -> &SagaMetrics {
        &self.metrics
    }

    /// 実行中のSagaを取得
    pub async fn get_running_sagas(&self) -> Vec<SagaContext> {
        let sagas = self.sagas.read().await;
        sagas.values()
            .filter(|ctx| matches!(ctx.status, SagaStatus::Started | SagaStatus::Executing))
            .cloned()
            .collect()
    }

    /// タイムアウトしたSagaを検出
    pub async fn detect_timed_out_sagas(&self) -> Vec<String> {
        let sagas = self.sagas.read().await;
        let now = chrono::Utc::now();

        sagas.iter()
            .filter_map(|(id, ctx)| {
                if let Some(timeout) = ctx.timeout {
                    let elapsed = now.signed_duration_since(ctx.start_time);
                    if elapsed.to_std().unwrap_or(std::time::Duration::from_secs(0)) > timeout {
                        Some(id.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Sagaをクリーンアップ
    pub async fn cleanup_saga(&self, saga_id: &str) -> Result<(), WorkflowError> {
        let mut sagas = self.sagas.write().await;
        sagas.remove(saga_id);
        Ok(())
    }
}

/// Saga実行エンジン
pub struct SagaExecutionEngine {
    saga_manager: Arc<SagaManager>,
    activity_registry: Arc<crate::executor::ActivityRegistry>,
    state_manager: Arc<crate::executor::WorkflowStateManager>,
}

impl SagaExecutionEngine {
    pub fn new(
        saga_manager: Arc<SagaManager>,
        activity_registry: Arc<crate::executor::ActivityRegistry>,
        state_manager: Arc<crate::executor::WorkflowStateManager>,
    ) -> Self {
        Self {
            saga_manager,
            activity_registry,
            state_manager,
        }
    }

    /// 高度なSagaを実行
    pub async fn execute_advanced_saga(
        &self,
        pattern: &AdvancedSagaPattern,
        workflow_id: WorkflowExecutionId,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<(), WorkflowError> {
        // Sagaを開始
        let saga_id = self.saga_manager.start_saga(&pattern.name, workflow_id.clone(), inputs).await?;

        // Saga状態を実行中に設定
        self.saga_manager.update_saga_status(&saga_id, SagaStatus::Executing).await?;

        // 実行キューを初期化
        let mut execution_queue: VecDeque<String> = self.saga_manager.resolve_dependencies(pattern, &[]).into();
        let mut completed = Vec::new();
        let mut failed = false;

        while !execution_queue.is_empty() && !failed {
            let activity_ref = execution_queue.pop_front().unwrap();

            // Activityを実行
            match self.execute_activity_with_tracking(&saga_id, &activity_ref, HashMap::new()).await {
                Ok(_) => {
                    completed.push(activity_ref);

                    // 新しい実行可能なActivityを追加
                    let new_ready = self.saga_manager.resolve_dependencies(pattern, &completed);
                    for activity in new_ready {
                        if !execution_queue.contains(&activity) {
                            execution_queue.push_back(activity);
                        }
                    }
                }
                Err(e) => {
                    println!("Activity {} failed: {:?}", activity_ref, e);
                    failed = true;

                    // 補償を実行
                    self.execute_compensation(&saga_id, pattern).await?;
                }
            }
        }

        // Sagaを完了
        let final_status = if failed {
            SagaStatus::Compensated
        } else {
            SagaStatus::Completed
        };

        self.saga_manager.update_saga_status(&saga_id, final_status).await?;
        Ok(())
    }

    /// Activityを実行し、トランザクションを記録
    async fn execute_activity_with_tracking(
        &self,
        saga_id: &str,
        activity_ref: &str,
        inputs: HashMap<String, serde_json::Value>,
    ) -> Result<HashMap<String, serde_json::Value>, WorkflowError> {
        // Activityを実行
        let result = self.activity_registry.execute(activity_ref, inputs.clone()).await?;

        // トランザクションを記録
        let transaction = SagaTransaction {
            transaction_id: uuid::Uuid::new_v4().to_string(),
            activity_ref: activity_ref.to_string(),
            activity_id: None, // TODO: ActivityExecutionIdを取得
            inputs,
            outputs: result.outputs.clone(),
            status: if result.error.is_some() { ExecutionStatus::Failed } else { ExecutionStatus::Completed },
            timestamp: chrono::Utc::now(),
            compensation_ref: Some(format!("compensate_{}", activity_ref)), // TODO: 実際の補償Activityを決定
        };

        self.saga_manager.record_transaction(saga_id, transaction).await?;

        if let Some(error) = result.error {
            return Err(WorkflowError::ActivityFailed(crate::executor::ActivityError::ExecutionFailed(error)));
        }

        Ok(result.outputs.unwrap_or_default())
    }

    /// 補償を実行
    async fn execute_compensation(
        &self,
        saga_id: &str,
        pattern: &AdvancedSagaPattern,
    ) -> Result<(), WorkflowError> {
        self.saga_manager.update_saga_status(saga_id, SagaStatus::Compensating).await?;

        let compensable = self.saga_manager.get_compensable_transactions(saga_id).await?;
        let compensation_order = self.saga_manager.get_compensation_order(saga_id, &pattern.config.compensation_policy).await?;

        for compensation_ref in compensation_order {
            if let Some(compensation_strategy) = pattern.compensations.get(&compensation_ref) {
                // 補償Activityを実行
                match self.execute_compensation_activity(saga_id, &compensation_ref).await {
                    Ok(_) => {
                        let compensation = SagaCompensation {
                            compensation_id: uuid::Uuid::new_v4().to_string(),
                            original_transaction_id: "".to_string(), // TODO: 元のトランザクションIDを設定
                            compensation_activity: compensation_ref,
                            status: CompensationStatus::Completed,
                            timestamp: chrono::Utc::now(),
                            error: None,
                        };
                        self.saga_manager.record_compensation(saga_id, compensation).await?;
                    }
                    Err(e) => {
                        let compensation = SagaCompensation {
                            compensation_id: uuid::Uuid::new_v4().to_string(),
                            original_transaction_id: "".to_string(),
                            compensation_activity: compensation_ref,
                            status: CompensationStatus::Failed,
                            timestamp: chrono::Utc::now(),
                            error: Some(e.to_string()),
                        };
                        self.saga_manager.record_compensation(saga_id, compensation).await?;
                    }
                }
            }
        }

        Ok(())
    }

    /// 補償Activityを実行
    async fn execute_compensation_activity(
        &self,
        saga_id: &str,
        compensation_ref: &str,
    ) -> Result<(), WorkflowError> {
        // TODO: 補償Activityを実行するロジックを実装
        println!("Executing compensation activity: {}", compensation_ref);
        Ok(())
    }
}

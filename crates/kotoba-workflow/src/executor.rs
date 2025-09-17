//! WorkflowExecutor - Temporalベースワークフロー実行器
//!
//! 拡張されたStrategyIRを解釈し、Temporal風のワークフロー実行を
//! 実現します。Activity実行、並列実行、Sagaパターンなどをサポート。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

// Import the new routing schema
use kotoba_routing::schema::{WorkflowStep, WorkflowStepType};
// These will be needed once we integrate the DB handler
// use kotoba_jsonnet::runtime::DbHandler;
// use kotoba_core::execution::QueryExecutor;
// use kotoba_core::rewrite::RewriteEngine;

use kotoba_core::types::{GraphRef_ as GraphRef};
use kotoba_core::prelude::StrategyOp;
use crate::ir::*;

/// Activity実行インターフェース
#[async_trait]
pub trait Activity: Send + Sync {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> std::result::Result<HashMap<String, serde_json::Value>, ActivityError>;
    fn name(&self) -> &str;
    fn timeout(&self) -> Option<Duration> { None }
    fn retry_policy(&self) -> Option<RetryPolicy> { None }
}

/// Activity実行エラー
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity not found: {0}")]
    NotFound(String),
    #[error("Activity execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Activity timeout")]
    Timeout,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// リトライポリシー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub initial_interval: Duration,
    pub backoff_coefficient: f64,
    pub maximum_interval: Option<Duration>,
    pub maximum_attempts: u32,
    pub non_retryable_errors: Vec<String>,
}

/// Activity実行結果
#[derive(Debug, Clone)]
pub struct ActivityResult {
    pub activity_name: String,
    pub status: ActivityStatus,
    pub outputs: Option<HashMap<String, serde_json::Value>>,
    pub error: Option<String>,
    pub execution_time: Duration,
    pub attempt_count: u32,
}

/// Activity実行状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStatus {
    Scheduled,
    Started,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Activityレジストリ
pub struct ActivityRegistry {
    activities: tokio::sync::RwLock<HashMap<String, Arc<dyn Activity>>>,
}

impl ActivityRegistry {
    pub fn new() -> Self {
        Self {
            activities: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Activityを登録
    pub async fn register(&self, activity: Arc<dyn Activity>) {
        let mut activities = self.activities.write().await;
        activities.insert(activity.name().to_string(), activity);
    }

    /// Activityを取得
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Activity>> {
        let activities = self.activities.read().await;
        activities.get(name).cloned()
    }

    /// Activityを実行
    pub async fn execute(
        &self,
        name: &str,
        inputs: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        let start_time = std::time::Instant::now();
        let activity = self.get(name).await
            .ok_or(ActivityError::NotFound(name.to_string()))?;

        let mut attempt_count = 0;
        let retry_policy = activity.retry_policy();

        // リトライロジック
        if let Some(retry_policy) = retry_policy {
            self.execute_with_retry(&*activity, inputs, retry_policy, start_time).await
        } else {
            self.execute_once(&*activity, inputs, start_time, 1).await
        }
    }

    async fn execute_with_retry(
        &self,
        activity: &dyn Activity,
        inputs: HashMap<String, serde_json::Value>,
        retry_policy: RetryPolicy,
        start_time: std::time::Instant,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        let mut attempt_count = 0;
        let mut current_interval = retry_policy.initial_interval;

        loop {
            attempt_count += 1;

            match self.execute_once(activity, inputs.clone(), start_time, attempt_count).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // リトライ不可エラーのチェック
                    if retry_policy.non_retryable_errors.iter().any(|err| e.to_string().contains(err)) {
                        return Err(e);
                    }

                    // 最大試行回数チェック
                    if attempt_count >= retry_policy.maximum_attempts {
                        return Err(e);
                    }

                    // リトライ待機
                    tokio::time::sleep(current_interval).await;

                    // インターバル更新
                    current_interval = std::cmp::min(
                        current_interval.mul_f64(retry_policy.backoff_coefficient),
                        retry_policy.maximum_interval.unwrap_or(Duration::from_secs(300)),
                    );
                }
            }
        }
    }

    async fn execute_once(
        &self,
        activity: &dyn Activity,
        inputs: HashMap<String, serde_json::Value>,
        start_time: std::time::Instant,
        attempt_count: u32,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        // タイムアウト設定を考慮した実行
        let result = if let Some(timeout_duration) = activity.timeout() {
            match timeout(timeout_duration, activity.execute(inputs)).await {
                Ok(result) => result,
                Err(_) => return Err(ActivityError::Timeout),
            }
        } else {
            activity.execute(inputs).await
        };

        let execution_time = start_time.elapsed();

        match result {
            Ok(outputs) => Ok(ActivityResult {
                activity_name: activity.name().to_string(),
                status: ActivityStatus::Completed,
                outputs: Some(outputs),
                error: None,
                execution_time,
                attempt_count,
            }),
            Err(e) => Ok(ActivityResult {
                activity_name: activity.name().to_string(),
                status: ActivityStatus::Failed,
                outputs: None,
                error: Some(e.to_string()),
                execution_time,
                attempt_count,
            }),
        }
    }

    /// 登録されているActivity一覧を取得
    pub async fn list_activities(&self) -> Vec<String> {
        let activities = self.activities.read().await;
        activities.keys().cloned().collect()
    }
}

/// ワークフロー実行エラー
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Activity execution failed: {0}")]
    ActivityFailed(#[from] ActivityError),
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),
    #[error("Invalid workflow definition: {0}")]
    InvalidDefinition(String),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),
    #[error("Graph operation failed: {0}")]
    GraphError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("Invalid step type for this executor: {0:?}")]
    InvalidStepType(WorkflowStepType),
    #[error("Context variable not found: {0}")]
    ContextVariableNotFound(String),
}

/// A context object for a single workflow execution.
#[derive(Debug, Clone, Default)]
pub struct ExecutionContext {
    /// Holds the initial request data and results from each step.
    pub data: HashMap<String, serde_json::Value>,
}

impl ExecutionContext {
    /// Creates a new context, usually from an initial request object.
    pub fn new(initial_data: serde_json::Value) -> Self {
        let mut data = HashMap::new();
        data.insert("request".to_string(), initial_data);
        Self { data }
    }

    /// Resolves a variable path (e.g., "context.step1.result") from the context.
    pub fn resolve(&self, path: &str) -> Option<&serde_json::Value> {
        let mut parts = path.split('.');
        if parts.next()? != "context" {
            return None;
        }
        let step_id = parts.next()?;
        let mut current = self.data.get(step_id)?;
        for part in parts {
            current = current.get(part)?;
        }
        Some(current)
    }
}


/// WorkflowExecutor - Temporalベースワークフロー実行エンジン
pub struct WorkflowExecutor {
    activity_registry: Arc<ActivityRegistry>,
    state_manager: Arc<WorkflowStateManager>,
    // This will hold the db_handler
    // db_handler: Arc<DbHandler>,
}

impl WorkflowExecutor {
    pub fn new(
        activity_registry: Arc<ActivityRegistry>,
        state_manager: Arc<WorkflowStateManager>,
        // db_handler: Arc<DbHandler>,
    ) -> Self {
        Self {
            activity_registry,
            state_manager,
            // db_handler,
        }
    }

    /// Executes a declarative workflow, like one from an HTTP route.
    pub async fn execute_declarative_workflow(
        &self,
        steps: &[WorkflowStep],
        initial_context: ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        let mut context = initial_context;

        for step in steps {
            let result = self.execute_step(step, &context).await?;
            context.data.insert(step.id.clone(), result);

            // If the step was a 'return' step, terminate the workflow.
            if step.step_type == WorkflowStepType::Return {
                // The body of the return step is the final result.
                return Ok(context.data.get(&step.id).cloned().unwrap_or_default());
            }
        }

        Err(WorkflowError::InvalidDefinition("Workflow did not end with a 'return' step.".to_string()))
    }

    /// Executes a single step from a declarative workflow.
    async fn execute_step(
        &self,
        step: &WorkflowStep,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        match step.step_type {
            WorkflowStepType::DbQuery => {
                // Mock implementation
                println!("Executing DB Query: {}", step.query);
                Ok(serde_json::json!({ "result": "mock_db_query_result" }))
                // Real implementation:
                // let params = self.materialize_params(&step.params, context)?;
                // let result = self.db_handler.query(&step.query, params).await?;
                // Ok(serde_json::to_value(result)?)
            }
            WorkflowStepType::DbRewrite => {
                // Mock implementation
                println!("Executing DB Rewrite Rule: {}", step.rule);
                Ok(serde_json::json!({ "result": "mock_db_rewrite_result" }))
                // Real implementation:
                // let params = self.materialize_params(&step.params, context)?;
                // let result = self.db_handler.rewrite(&step.rule, params).await?;
                // Ok(serde_json::to_value(result)?)
            }
            WorkflowStepType::Return => {
                // The body of the return step becomes its result.
                let body = self.materialize_params(&step.body, context)?;
                Ok(body)
            }
            // Other step types would be handled here...
            _ => Err(WorkflowError::InvalidStepType(step.step_type.clone())),
        }
    }

    /// Resolves parameters that might be context references.
    fn materialize_params(
        &self,
        params: &serde_json::Value,
        context: &ExecutionContext,
    ) -> Result<serde_json::Value, WorkflowError> {
        match params {
            serde_json::Value::String(s) if s.starts_with("context.") => {
                context.resolve(s)
                    .cloned()
                    .ok_or_else(|| WorkflowError::ContextVariableNotFound(s.clone()))
            }
            serde_json::Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (k, v) in map {
                    new_map.insert(k.clone(), self.materialize_params(v, context)?);
                }
                Ok(serde_json::Value::Object(new_map))
            }
            serde_json::Value::Array(arr) => {
                let mut new_arr = Vec::new();
                for v in arr {
                    new_arr.push(self.materialize_params(v, context)?);
                }
                Ok(serde_json::Value::Array(new_arr))
            }
            // If it's not a context reference or a container, return as is.
            _ => Ok(params.clone()),
        }
    }


    /// ワークフロー実行開始
    pub async fn start_workflow(
        &self,
        workflow_ir: &WorkflowIR,
        inputs: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<WorkflowExecutionId, WorkflowError> {
        // ワークフロー実行インスタンス作成
        let execution_id = self.state_manager.create_execution(workflow_ir, inputs.clone()).await?;

        // ワークフロー実行を開始（バックグラウンドで実行）
        let executor = Arc::new(Self::new(
            Arc::clone(&self.activity_registry),
            Arc::clone(&self.state_manager),
            // Arc::clone(&self.db_handler),
        ));

        let workflow_ir = workflow_ir.clone();
        let execution_id_clone = execution_id.clone();

        tokio::spawn(async move {
            if let Err(e) = executor.execute_workflow(workflow_ir, execution_id_clone).await {
                eprintln!("Workflow execution failed: {:?}", e);
            }
        });

        Ok(execution_id)
    }

    /// ワークフロー実行メイン処理
    async fn execute_workflow(
        &self,
        workflow_ir: WorkflowIR,
        execution_id: WorkflowExecutionId,
    ) -> std::result::Result<(), WorkflowError> {
        // 初期グラフ状態を作成（TODO: 実際のグラフ作成ロジックを実装）
        let initial_graph = GraphRef("initial".to_string());

        // 戦略実行
        let result = self.execute_strategy(workflow_ir.strategy, initial_graph, &execution_id).await;

        // 実行結果に基づいて最終状態を更新（MVCC対応）
        let mut execution = self.state_manager.get_execution(&execution_id).await
            .ok_or(WorkflowError::WorkflowNotFound(execution_id.0.clone()))?;

        match result {
            Ok(final_graph) => {
                execution.status = ExecutionStatus::Completed;
                execution.end_time = Some(chrono::Utc::now());
                execution.current_graph = final_graph;
                // outputs は最終グラフから抽出（TODO）

                // 完了イベントを追加
                let event = ExecutionEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: ExecutionEventType::WorkflowCompleted,
                    payload: HashMap::new(),
                };
                self.state_manager.add_execution_event(&execution_id, event).await?;
            }
            Err(e) => {
                execution.status = ExecutionStatus::Failed;
                execution.end_time = Some(chrono::Utc::now());

                // 失敗イベントを追加
                let event = ExecutionEvent {
                    id: uuid::Uuid::new_v4().to_string(),
                    timestamp: chrono::Utc::now(),
                    event_type: ExecutionEventType::WorkflowFailed,
                    payload: [("error".to_string(), serde_json::json!(e.to_string()))].into_iter().collect(),
                };
                self.state_manager.add_execution_event(&execution_id, event).await?;
            }
        }

        // 最終状態を保存
        self.state_manager.update_execution(execution).await?;
        Ok(())
    }

    /// 戦略実行（再帰的）
    fn execute_strategy<'a>(
        &'a self,
        strategy: WorkflowStrategyOp,
        graph: GraphRef,
        execution_id: &'a WorkflowExecutionId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<GraphRef, WorkflowError>> + Send + 'a>> {
        Box::pin(async move {
            match strategy {
                WorkflowStrategyOp::Basic { strategy } => {
                    self.execute_basic_strategy(strategy, graph, execution_id).await
                }
                WorkflowStrategyOp::Seq { strategies } => {
                    self.execute_seq(strategies, graph, execution_id).await
                }
                WorkflowStrategyOp::Parallel { branches, completion_condition } => {
                    self.execute_parallel(branches, completion_condition, graph, execution_id).await
                }
                WorkflowStrategyOp::Decision { conditions, default_branch } => {
                    self.execute_decision(conditions, default_branch, graph, execution_id).await
                }
                WorkflowStrategyOp::Wait { condition, timeout } => {
                    self.execute_wait(condition, timeout, graph, execution_id).await
                }
                WorkflowStrategyOp::Saga { main_flow, compensation } => {
                    self.execute_saga(*main_flow, *compensation, graph, execution_id).await
                }
                WorkflowStrategyOp::Activity { activity_ref, input_mapping, retry_policy } => {
                    self.execute_activity(activity_ref, input_mapping, retry_policy, graph, execution_id).await
                }
                WorkflowStrategyOp::SubWorkflow { workflow_ref, input_mapping } => {
                    self.execute_subworkflow(workflow_ref, input_mapping, graph, execution_id).await
                }
            }
        })
    }

    /// 基本戦略実行
    fn execute_basic_strategy<'a>(
        &'a self,
        strategy: StrategyOp,
        graph: GraphRef,
        execution_id: &'a WorkflowExecutionId,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = std::result::Result<GraphRef, WorkflowError>> + Send + 'a>> {
        Box::pin(async move {
            match strategy {
                StrategyOp::Seq { strategies } => {
                    let mut current_graph = graph;
                    for strategy in strategies {
                        current_graph = self.execute_basic_strategy(*strategy, current_graph, execution_id).await?;
                    }
                    Ok(current_graph)
                }
                StrategyOp::Once { rule } => {
                    // ルール適用（簡易実装）
                    println!("Executing rule: {}", rule);
                    Ok(graph) // TODO: 実際のルール適用を実装
                }
                StrategyOp::Exhaust { rule, order: _, measure: _ } => {
                    // ルール適用（簡易実装）
                    println!("Executing rule exhaustively: {}", rule);
                    Ok(graph)
                }
                StrategyOp::While { rule, pred: _, order: _ } => {
                    // 条件付きルール適用
                    println!("Executing rule while predicate: {}", rule);
                    Ok(graph)
                }
                StrategyOp::Choice { strategies } => {
                    // 選択実行（最初の成功したものを返す）
                    for strategy in strategies {
                        match self.execute_basic_strategy(*strategy.clone(), graph.clone(), execution_id).await {
                            Ok(result_graph) => return Ok(result_graph),
                            Err(_) => continue,
                        }
                    }
                    Err(WorkflowError::InvalidStrategy("All strategies in choice failed".to_string()))
                }
                StrategyOp::Priority { strategies } => {
                    // 優先度付き実行
                    println!("Executing with priority");
                    // TODO: 優先度に基づいて実行順序を決定
                    // 簡易実装として最初の戦略を実行
                    if let Some(first_strategy) = strategies.first() {
                        self.execute_basic_strategy((*first_strategy.strategy).clone(), graph, execution_id).await
                    } else {
                        Ok(graph)
                    }
                }
            }
        })
    }

    /// 順次実行
    async fn execute_seq(
        &self,
        strategies: Vec<Box<WorkflowStrategyOp>>,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        let mut current_graph = graph;
        for strategy in strategies {
            current_graph = self.execute_strategy(*strategy, current_graph, execution_id).await?;
        }
        Ok(current_graph)
    }

    /// 並列実行
    async fn execute_parallel(
        &self,
        branches: Vec<Box<WorkflowStrategyOp>>,
        completion_condition: CompletionCondition,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        let mut handles = vec![];

        for branch in branches {
            let executor = Arc::new(Self::new(
                Arc::clone(&self.activity_registry),
                Arc::clone(&self.state_manager),
                // Arc::clone(&self.db_handler),
            ));
            let graph_clone = graph.clone();
            let execution_id_clone = execution_id.clone();

            let handle = tokio::spawn(async move {
                executor.execute_strategy(*branch, graph_clone, &execution_id_clone).await
            });
            handles.push(handle);
        }

        match completion_condition {
            CompletionCondition::All => {
                // 全てのブランチが完了するまで待つ
                let mut results = vec![];
                for handle in handles {
                    results.push(handle.await.map_err(|_| WorkflowError::InvalidStrategy("Task panicked".to_string()))?);
                }
                // 最初の成功したグラフを返す
                results.into_iter().next().unwrap_or(Ok(graph.clone()))
            }
            CompletionCondition::Any => {
                // いずれかのブランチが完了したら進む
                // TODO: select! マクロを使って実装
                Err(WorkflowError::InvalidStrategy("Any completion not implemented".to_string()))
            }
            CompletionCondition::AtLeast(count) => {
                // 指定数のブランチが完了したら進む
                Err(WorkflowError::InvalidStrategy("AtLeast completion not implemented".to_string()))
            }
        }
    }

    /// 条件分岐実行
    async fn execute_decision(
        &self,
        conditions: Vec<DecisionBranch>,
        default_branch: Option<Box<WorkflowStrategyOp>>,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        // 実行コンテキストを取得
        let execution = self.state_manager.get_execution(execution_id).await
            .ok_or(WorkflowError::WorkflowNotFound(execution_id.0.clone()))?;

        let context = execution.inputs.clone();

        // 条件を順番に評価
        for branch in conditions {
            if self.evaluate_condition(&branch.condition, &context) {
                return self.execute_strategy(*branch.branch, graph, execution_id).await;
            }
        }

        // デフォルトブランチを実行
        if let Some(default_branch) = default_branch {
            self.execute_strategy(*default_branch, graph, execution_id).await
        } else {
            Ok(graph) // 何も実行しない
        }
    }

    /// 待機実行
    async fn execute_wait(
        &self,
        condition: WaitCondition,
        timeout: Option<Duration>,
        graph: GraphRef,
        _execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        match condition {
            WaitCondition::Timer { duration } => {
                tokio::time::sleep(duration).await;
                Ok(graph)
            }
            WaitCondition::Event { event_type, filter } => {
                // イベント待機は別途実装が必要
                println!("Waiting for event: {}", event_type);
                if let Some(timeout) = timeout {
                    tokio::time::sleep(timeout).await;
                }
                Ok(graph)
            }
            WaitCondition::Signal { signal_name } => {
                // シグナル待機は別途実装が必要
                println!("Waiting for signal: {}", signal_name);
                if let Some(timeout) = timeout {
                    tokio::time::sleep(timeout).await;
                }
                Ok(graph)
            }
        }
    }

    /// Sagaパターン実行
    async fn execute_saga(
        &self,
        main_flow: WorkflowStrategyOp,
        compensation: WorkflowStrategyOp,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        // メイン処理を実行
        match self.execute_strategy(main_flow, graph.clone(), execution_id).await {
            Ok(result_graph) => Ok(result_graph),
            Err(e) => {
                // 失敗したら補償処理を実行
                println!("Main flow failed, executing compensation");
                match self.execute_strategy(compensation, graph, execution_id).await {
                    Ok(_) => Err(WorkflowError::CompensationFailed("Main flow failed, compensation executed".to_string())),
                    Err(compensation_error) => Err(WorkflowError::CompensationFailed(
                        format!("Main flow failed and compensation also failed: {:?}", compensation_error)
                    )),
                }
            }
        }
    }

    /// Activity実行
    async fn execute_activity(
        &self,
        activity_ref: String,
        input_mapping: HashMap<String, String>,
        retry_policy: Option<crate::ir::RetryPolicy>,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        // 実行コンテキストから入力値をマッピング
        let execution = self.state_manager.get_execution(execution_id).await
            .ok_or(WorkflowError::WorkflowNotFound(execution_id.0.clone()))?;

        let inputs = self.map_inputs(&input_mapping, &execution.inputs)?;

        // Activity実行（リトライ対応）
        let result = if let Some(retry_policy) = retry_policy {
            self.execute_with_retry(&activity_ref, inputs, retry_policy).await
        } else {
            self.activity_registry.execute(&activity_ref, inputs).await
                .map(|result| result.outputs.unwrap_or_default())
        };

        match result {
            Ok(outputs) => {
                // 実行結果をグラフに反映（TODO）
                println!("Activity {} completed successfully", activity_ref);
                Ok(graph)
            }
            Err(e) => {
                println!("Activity {} failed: {:?}", activity_ref, e);
                Err(WorkflowError::ActivityFailed(e))
            }
        }
    }

    /// 子ワークフロー実行
    async fn execute_subworkflow(
        &self,
        workflow_ref: String,
        input_mapping: HashMap<String, String>,
        graph: GraphRef,
        execution_id: &WorkflowExecutionId,
    ) -> std::result::Result<GraphRef, WorkflowError> {
        // 親ワークフローから入力値をマッピング
        let execution = self.state_manager.get_execution(execution_id).await
            .ok_or(WorkflowError::WorkflowNotFound(execution_id.0.clone()))?;

        let inputs = self.map_inputs(&input_mapping, &execution.inputs)?;

        // 子ワークフロー定義を取得（実際の実装ではレジストリから取得）
        // TODO: 子ワークフロー定義の取得を実装
        println!("Subworkflow {} execution not yet implemented", workflow_ref);
        Ok(graph)
    }

    /// リトライ付きActivity実行
    async fn execute_with_retry(
        &self,
        activity_ref: &str,
        inputs: HashMap<String, serde_json::Value>,
        retry_policy: crate::ir::RetryPolicy,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, ActivityError> {
        let mut attempts = 0;
        let mut current_interval = retry_policy.initial_interval;

        loop {
            attempts += 1;

            match self.activity_registry.execute(activity_ref, inputs.clone()).await {
                Ok(result) => return Ok(result.outputs.unwrap_or_default()),
                Err(e) => {
                    // リトライ不可エラーのチェック
                    if retry_policy.non_retryable_errors.iter().any(|err| e.to_string().contains(err)) {
                        return Err(e);
                    }

                    // 最大試行回数チェック
                    if attempts >= retry_policy.maximum_attempts {
                        return Err(e);
                    }

                    // リトライ待機
                    tokio::time::sleep(current_interval).await;

                    // インターバル更新
                    current_interval = std::cmp::min(
                        current_interval.mul_f64(retry_policy.backoff_coefficient),
                        retry_policy.maximum_interval.unwrap_or(Duration::from_secs(300)),
                    );
                }
            }
        }
    }

    /// 入力値マッピング
    fn map_inputs(
        &self,
        mapping: &HashMap<String, String>,
        context: &HashMap<String, serde_json::Value>,
    ) -> std::result::Result<HashMap<String, serde_json::Value>, WorkflowError> {
        let mut inputs = HashMap::new();

        for (key, expr) in mapping {
            // 簡易的な式評価（実際の実装ではもっと複雑）
            if expr.starts_with("$.inputs.") {
                let field = &expr[9..]; // "$.inputs." を除去
                if let Some(value) = context.get(field) {
                    inputs.insert(key.clone(), value.clone());
                }
            }
        }

        Ok(inputs)
    }

    /// 条件式評価
    fn evaluate_condition(
        &self,
        condition: &str,
        context: &HashMap<String, serde_json::Value>,
    ) -> bool {
        // 簡易的な条件評価（実際の実装では式パーサーを使用）
        // TODO: より複雑な条件式の評価を実装
        if condition.contains("==") {
            // 例: "$.inputs.status == 'active'"
            // 簡易実装なので常にtrueを返す
            true
        } else {
            false
        }
    }
}

/// ワークフロー状態マネージャー - MVCCベースの実装
pub struct WorkflowStateManager {
    /// 実行状態の管理（TxIdベースのバージョン管理）
    executions: RwLock<HashMap<String, Vec<(TxId, WorkflowExecution)>>>,
    /// 現在のTxIdカウンター
    current_tx_id: std::sync::atomic::AtomicU64,
}

impl WorkflowStateManager {
    pub fn new() -> Self {
        Self {
            executions: RwLock::new(HashMap::new()),
            current_tx_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 新しいTxIdを生成
    fn next_tx_id(&self) -> TxId {
        let tx_id = self.current_tx_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        TxId(tx_id)
    }

    /// MVCCベースのワークフロー実行作成
    pub async fn create_execution(
        &self,
        workflow_ir: &WorkflowIR,
        inputs: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<WorkflowExecutionId, WorkflowError> {
        let tx_id = self.next_tx_id();
        let execution_id = WorkflowExecutionId(uuid::Uuid::new_v4().to_string());

        let execution = WorkflowExecution {
            id: execution_id.clone(),
            workflow_id: workflow_ir.id.clone(),
            status: ExecutionStatus::Running,
            start_time: chrono::Utc::now(),
            end_time: None,
            inputs,
            outputs: None,
            current_graph: GraphRef("initial".to_string()),
            execution_history: vec![ExecutionEvent {
                id: uuid::Uuid::new_v4().to_string(),
                timestamp: chrono::Utc::now(),
                event_type: ExecutionEventType::Started,
                payload: HashMap::new(),
            }],
            retry_count: 0,
            timeout_at: workflow_ir.timeout.map(|t| chrono::Utc::now() + chrono::Duration::from_std(t).unwrap()),
        };

        let mut executions = self.executions.write().await;
        let versions = executions.entry(execution_id.0.clone()).or_insert_with(Vec::new);
        versions.push((tx_id, execution));

        Ok(execution_id)
    }

    /// 指定されたTxId時点での実行状態を取得（MVCC対応）
    pub async fn get_execution_at(&self, id: &WorkflowExecutionId, tx_id: Option<TxId>) -> Option<WorkflowExecution> {
        let executions = self.executions.read().await;
        let versions = executions.get(&id.0)?;

        match tx_id {
            Some(tx_id) => {
                // 指定TxId以前の最新バージョンを取得
                versions.iter()
                    .filter(|(v_tx_id, _)| *v_tx_id <= tx_id)
                    .max_by_key(|(v_tx_id, _)| v_tx_id)
                    .map(|(_, execution)| execution.clone())
            }
            None => {
                // 最新バージョンを取得
                versions.last().map(|(_, execution)| execution.clone())
            }
        }
    }

    /// 最新バージョンの実行状態を取得
    pub async fn get_execution(&self, id: &WorkflowExecutionId) -> Option<WorkflowExecution> {
        self.get_execution_at(id, None).await
    }

    /// MVCCベースの実行状態更新
    pub async fn update_execution(&self, execution: WorkflowExecution) -> std::result::Result<TxId, WorkflowError> {
        let tx_id = self.next_tx_id();

        let mut executions = self.executions.write().await;
        let versions = executions.entry(execution.id.0.clone()).or_insert_with(Vec::new);
        versions.push((tx_id, execution));

        Ok(tx_id)
    }

    /// 実行のバージョン履歴を取得
    pub async fn get_execution_history(&self, id: &WorkflowExecutionId) -> Vec<(TxId, WorkflowExecution)> {
        let executions = self.executions.read().await;
        executions.get(&id.0).cloned().unwrap_or_default()
    }

    /// スナップショット作成（古いバージョンのクリーンアップ）
    pub async fn create_snapshot(&self, execution_id: &WorkflowExecutionId, max_versions: usize) -> std::result::Result<(), WorkflowError> {
        let mut executions = self.executions.write().await;
        if let Some(versions) = executions.get_mut(&execution_id.0) {
            if versions.len() > max_versions {
                // 最新のmax_versions個を保持
                let keep_count = versions.len().saturating_sub(max_versions);
                versions.drain(0..keep_count);
            }
        }
        Ok(())
    }

    /// 実行中のワークフロー一覧を取得
    pub async fn get_running_executions(&self) -> Vec<WorkflowExecution> {
        let executions = self.executions.read().await;
        executions.values()
            .filter_map(|versions| {
                versions.last().map(|(_, execution)| execution.clone())
                    .filter(|execution| matches!(execution.status, ExecutionStatus::Running))
            })
            .collect()
    }

    /// 実行イベントを追加
    pub async fn add_execution_event(&self, execution_id: &WorkflowExecutionId, event: ExecutionEvent) -> std::result::Result<TxId, WorkflowError> {
        let mut execution = self.get_execution(execution_id).await
            .ok_or(WorkflowError::WorkflowNotFound(execution_id.0.clone()))?;

        execution.execution_history.push(event);
        self.update_execution(execution).await
    }
}

// TODO: Implement workflow execution engine
// For now, this module provides basic activity execution framework


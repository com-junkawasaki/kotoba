//! WorkflowIR - TemporalベースワークフローIR定義
//!
//! Kotobaのプロセスネットワークグラフモデル上に、Temporal風のワークフロー実行を
//! 実現するためのIRを定義します。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use kotoba_core::prelude::*;
use kotoba_core::types::{GraphRef_ as GraphRef, Value};

/// ワークフロー実行ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowExecutionId(pub String);

/// Activity実行ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActivityExecutionId(pub String);

/// ワークフロー定義IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowIR {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub version: String,

    /// ワークフロー入力パラメータ
    pub inputs: Vec<WorkflowParam>,

    /// ワークフロー出力パラメータ
    pub outputs: Vec<WorkflowParam>,

    /// 実行戦略（Temporalパターンをサポート）
    pub strategy: WorkflowStrategyOp,

    /// Serverless Workflow互換のアクティビティリスト
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub activities: Vec<ActivityIR>,

    /// タイムアウト設定
    pub timeout: Option<Duration>,

    /// リトライポリシー
    pub retry_policy: Option<RetryPolicy>,

    /// メタデータ
    pub metadata: HashMap<String, Value>,
}

/// ワークフローパラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub default_value: Option<Value>,
}

/// Temporal拡張ワークフロー戦略
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum WorkflowStrategyOp {
    /// 既存のStrategyOpを継承
    Basic {
        strategy: StrategyOp,
    },

    /// 順次実行
    Seq {
        strategies: Vec<Box<WorkflowStrategyOp>>,
    },

    /// 並列実行
    Parallel {
        branches: Vec<Box<WorkflowStrategyOp>>,
        #[serde(default)]
        completion_condition: CompletionCondition,
    },

    /// 条件分岐
    Decision {
        conditions: Vec<DecisionBranch>,
        default_branch: Option<Box<WorkflowStrategyOp>>,
    },

    /// タイマー/イベント待ち
    Wait {
        condition: WaitCondition,
        timeout: Option<Duration>,
    },

    /// Sagaパターン（補償トランザクション）
    Saga {
        main_flow: Box<WorkflowStrategyOp>,
        compensation: Box<WorkflowStrategyOp>,
    },

    /// Activity実行
    Activity {
        activity_ref: String,  // extern 関数参照
        input_mapping: HashMap<String, String>,
        retry_policy: Option<RetryPolicy>,
    },

    /// 子ワークフロー実行
    SubWorkflow {
        workflow_ref: String,
        input_mapping: HashMap<String, String>,
    },
}

/// 並列完了条件
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum CompletionCondition {
    #[default]
    /// 全てのブランチが完了するまで待つ
    All,
    /// いずれかのブランチが完了したら進む
    Any,
    /// 指定数のブランチが完了したら進む
    AtLeast(u32),
}

/// 条件分岐ブランチ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionBranch {
    pub condition: String,  // 条件式（extern参照）
    pub branch: Box<WorkflowStrategyOp>,
}

/// 待機条件
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WaitCondition {
    /// タイマー待機
    Timer {
        duration: Duration,
    },
    /// イベント待機
    Event {
        event_type: String,
        filter: Option<HashMap<String, Value>>,
    },
    /// シグナル待機
    Signal {
        signal_name: String,
    },
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

/// Activity定義IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityIR {
    pub name: String,
    pub description: Option<String>,
    pub inputs: Vec<ActivityParam>,
    pub outputs: Vec<ActivityParam>,
    pub timeout: Option<Duration>,
    pub retry_policy: Option<RetryPolicy>,
    pub implementation: ActivityImplementation,
}

/// Workflow step definition for execution engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: String,
    pub step_type: WorkflowStepType,
    pub body: serde_json::Value,
}

/// Workflow step types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowStepType {
    /// HTTP call step
    HttpCall,
    /// Database query step
    DbQuery,
    /// Database rewrite step
    DbRewrite,
    /// Return step
    Return,
    /// Activity execution step
    Activity,
    /// Sub-workflow execution step
    SubWorkflow,
}

/// Activityパラメータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
}

/// Activity実装種別
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActivityImplementation {
    /// Rust関数
    Function {
        function_name: String,
    },
    /// HTTPエンドポイント
    Http {
        url: String,
        method: String,
        headers: HashMap<String, String>,
    },
    /// 外部プロセス
    Process {
        command: String,
        args: Vec<String>,
        env: HashMap<String, String>,
    },
    /// GraphQLクエリ
    GraphQL {
        query: String,
        endpoint: String,
    },
}

/// ワークフロー実行状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowExecution {
    pub id: WorkflowExecutionId,
    pub workflow_id: String,
    pub status: ExecutionStatus,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: Option<HashMap<String, serde_json::Value>>,
    pub current_graph: GraphRef,
    pub execution_history: Vec<ExecutionEvent>,
    pub retry_count: u32,
    pub timeout_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// 実行状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
    Compensating,
}

/// 実行イベント（イベントソーシング用）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEvent {
    pub id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: ExecutionEventType,
    pub payload: HashMap<String, serde_json::Value>,
}

/// 実行イベント種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionEventType {
    Started,
    ActivityScheduled,
    ActivityStarted,
    ActivityCompleted,
    ActivityFailed,
    DecisionMade,
    TimerScheduled,
    TimerFired,
    SignalReceived,
    WorkflowCompleted,
    WorkflowFailed,
    WorkflowCancelled,
    CompensationStarted,
    CompensationCompleted,
}

/// Sagaパターン定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SagaPattern {
    pub name: String,
    pub description: Option<String>,
    pub main_activities: Vec<String>,  // Activity名リスト
    pub compensation_activities: Vec<String>,  // 補償Activity名リスト
    pub timeout: Option<Duration>,
}

/// ワークフロー実行結果
#[derive(Debug, Clone)]
pub struct WorkflowResult {
    pub execution_id: WorkflowExecutionId,
    pub status: ExecutionStatus,
    pub outputs: Option<HashMap<String, Value>>,
    pub error: Option<String>,
    pub execution_time: Duration,
}

/// Activity実行結果
#[derive(Debug, Clone)]
pub struct ActivityResult {
    pub activity_id: ActivityExecutionId,
    pub status: ActivityStatus,
    pub outputs: Option<HashMap<String, Value>>,
    pub error: Option<String>,
    pub execution_time: Duration,
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

// GraphRef is imported from kotoba-core::types::GraphRef_

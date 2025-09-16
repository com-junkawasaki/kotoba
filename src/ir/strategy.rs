//! Strategy-IR（極小戦略表現 + Temporal拡張）

use serde::{Deserialize, Serialize};
use crate::types::*;
use std::time::Duration;
use std::collections::HashMap;

/// 戦略演算子
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum StrategyOp {
    /// 1回だけ適用
    Once {
        rule: String,  // ルール名またはハッシュ
    },

    /// 適用可能になるまで繰り返し
    Exhaust {
        rule: String,
        #[serde(default)]
        order: Order,
        #[serde(skip_serializing_if = "Option::is_none")]
        measure: Option<String>,
    },

    /// 条件付き繰り返し
    While {
        rule: String,
        pred: String,  // 述語名
        #[serde(default)]
        order: Order,
    },

    /// 順次実行
    Seq {
        strategies: Vec<Box<StrategyOp>>,
    },

    /// 選択実行（最初に成功したもの）
    Choice {
        strategies: Vec<Box<StrategyOp>>,
    },

    /// 優先順位付き選択
    Priority {
        strategies: Vec<PrioritizedStrategy>,
    },

    // ===== Temporal Workflow パターン拡張 =====

    /// 並列実行
    Parallel {
        branches: Vec<Box<StrategyOp>>,
        #[serde(default)]
        completion_condition: CompletionCondition,
    },

    /// 条件分岐（Decision）
    Decision {
        conditions: Vec<DecisionBranch>,
        default_branch: Option<Box<StrategyOp>>,
    },

    /// タイマー/イベント待ち
    Wait {
        condition: WaitCondition,
        timeout: Option<Duration>,
    },

    /// Sagaパターン（補償トランザクション）
    Saga {
        main_flow: Box<StrategyOp>,
        compensation: Box<StrategyOp>,
    },

    /// Activity実行
    Activity {
        activity_ref: String,
        input_mapping: HashMap<String, String>,
        retry_policy: Option<RetryPolicy>,
    },

    /// 子ワークフロー実行
    SubWorkflow {
        workflow_ref: String,
        input_mapping: HashMap<String, String>,
    },
}

/// 優先順位付き戦略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrioritizedStrategy {
    pub strategy: Box<StrategyOp>,
    pub priority: i32,
}

/// 適用順序
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub enum Order {
    #[default]
    #[serde(rename = "topdown")]
    TopDown,

    #[serde(rename = "bottomup")]
    BottomUp,

    #[serde(rename = "fair")]
    Fair,
}

/// 戦略IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyIR {
    pub strategy: StrategyOp,
}

/// 戦略実行結果
#[derive(Debug, Clone)]
pub struct StrategyResult {
    pub applied_count: usize,
    pub final_graph: crate::graph::GraphRef,
    pub patches: Vec<crate::ir::patch::Patch>,
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
    pub branch: Box<StrategyOp>,
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

/// 戦略実行結果（拡張）
#[derive(Debug, Clone)]
pub struct ExtendedStrategyResult {
    pub base_result: StrategyResult,
    pub workflow_events: Vec<WorkflowEvent>,
    pub parallel_results: Option<Vec<StrategyResult>>,
}

/// ワークフローイベント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub event_type: WorkflowEventType,
    pub payload: HashMap<String, Value>,
}

/// ワークフローイベント種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEventType {
    ActivityScheduled,
    ActivityStarted,
    ActivityCompleted,
    ActivityFailed,
    DecisionMade,
    TimerScheduled,
    TimerFired,
    SignalReceived,
    BranchCompleted,
    BranchFailed,
}

/// 外部述語/測度トレイト（拡張）
pub trait ExtendedExterns: Externs {
    /// ワークフロー条件評価
    fn evaluate_condition(&self, condition: &str, context: &HashMap<String, Value>) -> bool;

    /// Activity実行
    fn execute_activity(&self, activity_ref: &str, inputs: HashMap<String, Value>) -> Result<HashMap<String, Value>, String>;

    /// イベント購読
    fn wait_for_event(&self, event_type: &str, filter: Option<&HashMap<String, Value>>, timeout: Option<Duration>) -> Result<HashMap<String, Value>, String>;

    /// シグナル送信
    fn send_signal(&self, signal_name: &str, payload: HashMap<String, Value>) -> Result<(), String>;
}

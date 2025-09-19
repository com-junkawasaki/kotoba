//! Strategy-IR（極小戦略表現）

use serde::{Deserialize, Serialize};
use crate::types::*;

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
    pub final_graph: GraphRef_,
    pub patches: Vec<crate::ir::patch::Patch>,
}

/// 外部述語/測度トレイト
pub trait Externs {
    /// 次数が指定値以上かチェック
    fn deg_ge(&self, v: VertexId, k: u32) -> bool;

    /// エッジ数が非増加かチェック（停止測度）
    fn edge_count_nonincreasing(&self, g0: &GraphRef_, g1: &GraphRef_) -> bool;

    /// カスタム述語
    fn custom_pred(&self, name: &str, args: &[Value]) -> bool;
}

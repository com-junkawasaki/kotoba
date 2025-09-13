//! Query-IR（GQL論理プラン代数）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::*;

/// 論理演算子
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum LogicalOp {
    /// ノードスキャン
    NodeScan {
        label: Label,
        as_: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        props: Option<Properties>,
    },

    /// インデックススキャン
    IndexScan {
        label: Label,
        as_: String,
        index: String,
        value: Value,
    },

    /// フィルタ
    Filter {
        pred: Predicate,
        input: Box<LogicalOp>,
    },

    /// エッジ展開
    Expand {
        edge: EdgePattern,
        to_as: String,
        from: Box<LogicalOp>,
    },

    /// 結合
    Join {
        left: Box<LogicalOp>,
        right: Box<LogicalOp>,
        on: Vec<String>,  // 結合キー
    },

    /// 射影
    Project {
        cols: Vec<String>,
        input: Box<LogicalOp>,
    },

    /// グループ化
    Group {
        keys: Vec<String>,
        aggregations: Vec<Aggregation>,
        input: Box<LogicalOp>,
    },

    /// ソート
    Sort {
        keys: Vec<SortKey>,
        input: Box<LogicalOp>,
    },

    /// リミット
    Limit {
        count: usize,
        input: Box<LogicalOp>,
    },

    /// 重複除去
    Distinct {
        input: Box<LogicalOp>,
    },
}

/// エッジパターン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgePattern {
    pub label: Label,
    pub dir: Direction,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Properties>,
}

/// 方向
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Direction {
    #[serde(rename = "out")]
    Out,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "both")]
    Both,
}

/// 述語
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Predicate {
    Eq { eq: [Expr; 2] },
    Ne { ne: [Expr; 2] },
    Lt { lt: [Expr; 2] },
    Le { le: [Expr; 2] },
    Gt { gt: [Expr; 2] },
    Ge { ge: [Expr; 2] },
    And { and: Vec<Predicate> },
    Or { or: Vec<Predicate> },
    Not { not: Box<Predicate> },
}

/// 式
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Expr {
    Var(String),
    Const(Value),
    Fn { fn_: String, args: Vec<Expr> },
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Var(v) => write!(f, "{}", v),
            Expr::Const(val) => write!(f, "{:?}", val),
            Expr::Fn { fn_, args } => {
                write!(f, "{}({})", fn_, args.len())
            }
        }
    }
}

/// 集計関数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregation {
    pub fn_: String,
    pub args: Vec<String>,
    pub as_: String,
}

/// ソートキー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortKey {
    pub expr: Expr,
    pub asc: bool,
}

/// 論理プラン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanIR {
    pub plan: LogicalOp,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

/// 実行結果行
#[derive(Debug, Clone)]
pub struct Row {
    pub values: HashMap<String, Value>,
}

/// 結果ストリーム
pub type RowStream = Vec<Row>;

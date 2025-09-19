//! Rule-IR（DPO型付き属性グラフ書換え）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::*;

/// ガード条件（名前付き述語）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Guard {
    pub ref_: String,  // 述語名（例: "deg_ge"）
    pub args: HashMap<String, Value>,
}

/// グラフパターン要素
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphElement {
    pub id: String,  // 変数名
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<Label>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub props: Option<Properties>,
}

/// エッジ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeDef {
    pub id: String,
    pub src: String,
    pub dst: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_: Option<Label>,
}

/// グラフパターン
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPattern {
    pub nodes: Vec<GraphElement>,
    pub edges: Vec<EdgeDef>,
}

/// 負の条件（NAC: Negative Application Condition）- ルール用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleNac {
    pub nodes: Vec<GraphElement>,
    pub edges: Vec<EdgeDef>,
}

/// DPOルール定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleIR {
    pub name: String,
    pub types: HashMap<String, Vec<Label>>,  // 型定義
    pub lhs: GraphPattern,                   // Left-hand side (L)
    pub context: GraphPattern,               // Context (K)
    pub rhs: GraphPattern,                   // Right-hand side (R)
    pub nacs: Vec<Nac>,                      // Negative conditions
    pub guards: Vec<Guard>,                  // ガード条件
}

/// ルールマッチ結果
#[derive(Debug, Clone)]
pub struct Match {
    pub mapping: HashMap<String, VertexId>,  // 変数→頂点IDマッピング
    pub score: f64,                         // マッチスコア
}

/// 複数マッチ結果
pub type Matches = Vec<Match>;

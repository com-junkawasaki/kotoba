//! 論理プランナー（GQL → 論理プラン）

use kotoba_core::{ir::*, types::*};
use kotoba_errors::KotobaError;
use kotoba_core::types::Result;

/// 論理プランナー
#[derive(Debug)]
pub struct LogicalPlanner;

impl Default for LogicalPlanner {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicalPlanner {
    pub fn new() -> Self {
        Self
    }

    /// GQL文字列を論理プランに変換
    pub fn parse_gql(&self, gql: &str) -> Result<PlanIR> {
        // 簡易パーサー（実際の実装ではPEGパーサー等を使用）
        // ここではサンプルクエリのみ対応

        if gql.trim().to_lowercase().starts_with("match") {
            self.parse_match_query(gql)
        } else {
            Err(KotobaError::Parse(format!("Unsupported GQL query: {}", gql)))
        }
    }

    /// MATCHクエリのパース
    fn parse_match_query(&self, _gql: &str) -> Result<PlanIR> {
        // 非常に簡易的なパーサー
        // 実際の実装では構文解析器を使用

        let plan = LogicalOp::NodeScan {
            label: "Person".to_string(),
            as_: "n".to_string(),
            props: None,
        };

        Ok(PlanIR {
            plan,
            limit: Some(100),
        })
    }

    /// 論理プランを最適化
    pub fn optimize(&self, plan: &PlanIR, _catalog: &Catalog) -> PlanIR {
        // 簡易的な最適化
        // 実際の実装ではコストベース最適化を実装

        plan.clone()
    }
}

/// コスト推定器
#[derive(Debug)]
pub struct CostEstimator;

impl Default for CostEstimator {
    fn default() -> Self {
        Self::new()
    }
}

impl CostEstimator {
    pub fn new() -> Self {
        Self
    }

    /// 論理演算子のコストを推定
    pub fn estimate_cost(&self, op: &LogicalOp, catalog: &Catalog) -> f64 {
        match op {
            LogicalOp::NodeScan { label, .. } => {
                // ラベル別の頂点数に基づくコスト
                catalog.get_label(label)
                    .map(|_| 100.0)  // 仮のコスト
                    .unwrap_or(1000.0)
            }
            LogicalOp::Expand { .. } => 50.0,
            LogicalOp::Filter { .. } => 10.0,
            LogicalOp::Join { .. } => 200.0,
            LogicalOp::Project { .. } => 5.0,
            LogicalOp::Sort { .. } => 100.0,
            LogicalOp::Limit { .. } => 1.0,
            LogicalOp::Distinct { .. } => 50.0,
            LogicalOp::IndexScan { .. } => 10.0,
            LogicalOp::Group { .. } => 150.0,
        }
    }
}

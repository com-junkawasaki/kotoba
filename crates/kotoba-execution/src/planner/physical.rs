//! 物理プランナー（論理プラン → 物理プラン）

use kotoba_core::ir::*;
use kotoba_core::types::*;
use crate::planner::{PhysicalPlan, PhysicalOp};

/// 物理演算子
#[derive(Debug, Clone)]
pub enum PhysicalOp {
    /// ノードスキャン
    NodeScan {
        label: Label,
        as_: String,
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
        input: Box<PhysicalOp>,
    },

    /// エッジ展開
    Expand {
        edge: EdgePattern,
        to_as: String,
        input: Box<PhysicalOp>,
    },

    /// ネステッドループ結合
    NestedLoopJoin {
        left: Box<PhysicalOp>,
        right: Box<PhysicalOp>,
        on: Vec<String>,
    },

    /// ハッシュ結合
    HashJoin {
        left: Box<PhysicalOp>,
        right: Box<PhysicalOp>,
        on: Vec<String>,
    },

    /// 射影
    Project {
        cols: Vec<String>,
        input: Box<PhysicalOp>,
    },

    /// グループ化
    Group {
        keys: Vec<String>,
        aggregations: Vec<Aggregation>,
        input: Box<PhysicalOp>,
    },

    /// ソート
    Sort {
        keys: Vec<SortKey>,
        input: Box<PhysicalOp>,
    },

    /// リミット
    Limit {
        count: usize,
        input: Box<PhysicalOp>,
    },

    /// 重複除去
    Distinct {
        input: Box<PhysicalOp>,
    },
}

/// 物理プラン
#[derive(Debug, Clone)]
pub struct PhysicalPlan {
    pub op: PhysicalOp,
    pub estimated_cost: f64,
}

/// 物理プランナー
#[derive(Debug)]
pub struct PhysicalPlanner;

impl PhysicalPlanner {
    pub fn new() -> Self {
        Self
    }

    /// 論理プランを物理プランに変換
    pub fn plan_to_physical(&self, logical: &PlanIR, catalog: &Catalog) -> Result<PhysicalPlan> {
        let op = self.convert_logical_op(&logical.plan, catalog)?;
        let cost = self.estimate_cost(&op, catalog);

        Ok(PhysicalPlan {
            op,
            estimated_cost: cost,
        })
    }

    /// 論理演算子を物理演算子に変換
    fn convert_logical_op(&self, logical: &LogicalOp, catalog: &Catalog) -> Result<PhysicalOp> {
        match logical {
            LogicalOp::NodeScan { label, as_, props } => {
                // インデックスが存在する場合はIndexScanを使用
                if self.has_index(catalog, label, props) {
                    Ok(PhysicalOp::IndexScan {
                        label: label.clone(),
                        as_: as_.clone(),
                        index: format!("{}_index", label),
                        value: Value::String("dummy".to_string()), // 仮
                    })
                } else {
                    Ok(PhysicalOp::NodeScan {
                        label: label.clone(),
                        as_: as_.clone(),
                        props: props.clone(),
                    })
                }
            }

            LogicalOp::IndexScan { label, as_, index, value } => {
                Ok(PhysicalOp::IndexScan {
                    label: label.clone(),
                    as_: as_.clone(),
                    index: index.clone(),
                    value: value.clone(),
                })
            }

            LogicalOp::Filter { pred, input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Filter {
                    pred: pred.clone(),
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Expand { edge, to_as, from } => {
                let input_op = self.convert_logical_op(from, catalog)?;
                Ok(PhysicalOp::Expand {
                    edge: edge.clone(),
                    to_as: to_as.clone(),
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Join { left, right, on } => {
                let left_op = self.convert_logical_op(left, catalog)?;
                let right_op = self.convert_logical_op(right, catalog)?;

                // コストに基づいて結合アルゴリズムを選択
                let left_cost = self.estimate_cost(&left_op, catalog);
                let right_cost = self.estimate_cost(&right_op, catalog);

                if left_cost < right_cost && left_cost < 1000.0 {
                    Ok(PhysicalOp::NestedLoopJoin {
                        left: Box::new(left_op),
                        right: Box::new(right_op),
                        on: on.clone(),
                    })
                } else {
                    Ok(PhysicalOp::HashJoin {
                        left: Box::new(left_op),
                        right: Box::new(right_op),
                        on: on.clone(),
                    })
                }
            }

            LogicalOp::Project { cols, input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Project {
                    cols: cols.clone(),
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Group { keys, aggregations, input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Group {
                    keys: keys.clone(),
                    aggregations: aggregations.clone(),
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Sort { keys, input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Sort {
                    keys: keys.clone(),
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Limit { count, input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Limit {
                    count: *count,
                    input: Box::new(input_op),
                })
            }

            LogicalOp::Distinct { input } => {
                let input_op = self.convert_logical_op(input, catalog)?;
                Ok(PhysicalOp::Distinct {
                    input: Box::new(input_op),
                })
            }
        }
    }

    /// インデックスが存在するかチェック
    fn has_index(&self, catalog: &Catalog, label: &Label, props: &Option<Properties>) -> bool {
        if let Some(props) = props {
            // プロパティベースのインデックスをチェック
            catalog.indexes.iter().any(|idx| {
                idx.label == *label &&
                props.contains_key(&idx.properties[0])
            })
        } else {
            false
        }
    }

    /// 物理演算子のコストを推定
    fn estimate_cost(&self, op: &PhysicalOp, catalog: &Catalog) -> f64 {
        match op {
            PhysicalOp::NodeScan { .. } => 100.0,
            PhysicalOp::IndexScan { .. } => 10.0,
            PhysicalOp::Filter { input, .. } => self.estimate_cost(input, catalog) + 10.0,
            PhysicalOp::Expand { input, .. } => self.estimate_cost(input, catalog) + 50.0,
            PhysicalOp::NestedLoopJoin { left, right, .. } => {
                let left_cost = self.estimate_cost(left, catalog);
                let right_cost = self.estimate_cost(right, catalog);
                left_cost * right_cost * 2.0
            }
            PhysicalOp::HashJoin { left, right, .. } => {
                let left_cost = self.estimate_cost(left, catalog);
                let right_cost = self.estimate_cost(right, catalog);
                left_cost + right_cost + 100.0
            }
            PhysicalOp::Project { input, .. } => self.estimate_cost(input, catalog) + 5.0,
            PhysicalOp::Group { input, .. } => self.estimate_cost(input, catalog) + 150.0,
            PhysicalOp::Sort { input, .. } => self.estimate_cost(input, catalog) + 100.0,
            PhysicalOp::Limit { input, .. } => self.estimate_cost(input, catalog) + 1.0,
            PhysicalOp::Distinct { input, .. } => self.estimate_cost(input, catalog) + 50.0,
        }
    }
}

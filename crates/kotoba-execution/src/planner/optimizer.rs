//! クエリ最適化器

use kotoba_core::{types::*, ir::*};

/// 最適化ルール
#[derive(Debug)]
pub enum OptimizationRule {
    /// 述語押下げ
    PushDownPredicates,

    /// 結合順序最適化
    JoinOrderOptimization,

    /// 不要な射影除去
    EliminateUnnecessaryProjections,

    /// 定数畳み込み
    ConstantFolding,

    /// インデックス選択
    IndexSelection,
}

/// クエリ最適化器
#[derive(Debug)]
pub struct QueryOptimizer {
    rules: Vec<OptimizationRule>,
}

impl QueryOptimizer {
    pub fn new() -> Self {
        Self {
            rules: vec![
                OptimizationRule::PushDownPredicates,
                OptimizationRule::JoinOrderOptimization,
                OptimizationRule::EliminateUnnecessaryProjections,
                OptimizationRule::ConstantFolding,
                OptimizationRule::IndexSelection,
            ],
        }
    }

    /// 論理プランを最適化
    pub fn optimize(&self, plan: &PlanIR, catalog: &Catalog) -> PlanIR {
        let mut optimized = plan.clone();

        for rule in &self.rules {
            optimized = self.apply_rule(optimized, rule, catalog);
        }

        optimized
    }

    /// 最適化ルールを適用
    fn apply_rule(&self, plan: PlanIR, rule: &OptimizationRule, catalog: &Catalog) -> PlanIR {
        match rule {
            OptimizationRule::PushDownPredicates => {
                self.push_down_predicates(plan)
            }
            OptimizationRule::JoinOrderOptimization => {
                self.optimize_join_order(plan, catalog)
            }
            OptimizationRule::EliminateUnnecessaryProjections => {
                self.eliminate_unnecessary_projections(plan)
            }
            OptimizationRule::ConstantFolding => {
                self.constant_folding(plan)
            }
            OptimizationRule::IndexSelection => {
                self.select_indexes(plan, catalog)
            }
        }
    }

    /// 述語押下げ
    fn push_down_predicates(&self, plan: PlanIR) -> PlanIR {
        let optimized_plan = self.push_down_predicates_op(&plan.plan);
        PlanIR {
            plan: optimized_plan,
            limit: plan.limit,
        }
    }

    fn push_down_predicates_op(&self, op: &LogicalOp) -> LogicalOp {
        match op {
            LogicalOp::Filter { pred, input } => {
                // フィルタを可能な限り下位に押下げ
                match input.as_ref() {
                    LogicalOp::Join { left, right, on } => {
                        // 結合の場合、述語を適切な側に分配
                        let (left_pred, right_pred, remaining_pred) =
                            self.split_predicate_for_join(pred, on);

                        let new_left = if let Some(lp) = left_pred {
                            Box::new(LogicalOp::Filter {
                                pred: lp,
                                input: left.clone(),
                            })
                        } else {
                            left.clone()
                        };

                        let new_right = if let Some(rp) = right_pred {
                            Box::new(LogicalOp::Filter {
                                pred: rp,
                                input: right.clone(),
                            })
                        } else {
                            right.clone()
                        };

                        if let Some(rem_pred) = remaining_pred {
                            LogicalOp::Filter {
                                pred: rem_pred,
                                input: Box::new(LogicalOp::Join {
                                    left: new_left,
                                    right: new_right,
                                    on: on.clone(),
                                }),
                            }
                        } else {
                            LogicalOp::Join {
                                left: new_left,
                                right: new_right,
                                on: on.clone(),
                            }
                        }
                    }
                    _ => {
                        // その他の場合、再帰的に最適化
                        LogicalOp::Filter {
                            pred: pred.clone(),
                            input: Box::new(self.push_down_predicates_op(input)),
                        }
                    }
                }
            }
            LogicalOp::Join { left, right, on } => {
                LogicalOp::Join {
                    left: Box::new(self.push_down_predicates_op(left)),
                    right: Box::new(self.push_down_predicates_op(right)),
                    on: on.clone(),
                }
            }
            LogicalOp::Project { cols, input } => {
                LogicalOp::Project {
                    cols: cols.clone(),
                    input: Box::new(self.push_down_predicates_op(input)),
                }
            }
            // その他の演算子はそのまま
            _ => op.clone(),
        }
    }

    /// 結合の述語を左右に分割
    fn split_predicate_for_join(&self, pred: &Predicate, join_keys: &[String])
        -> (Option<Predicate>, Option<Predicate>, Option<Predicate>) {
        match pred {
            Predicate::And { and } => {
                let mut left_preds = Vec::new();
                let mut right_preds = Vec::new();
                let mut remaining = Vec::new();

                for p in and {
                    let (l, r, rem) = self.split_predicate_for_join(p, join_keys);
                    if let Some(lp) = l { left_preds.push(lp); }
                    if let Some(rp) = r { right_preds.push(rp); }
                    if let Some(rem_p) = rem { remaining.push(rem_p); }
                }

                let left = if left_preds.is_empty() {
                    None
                } else if left_preds.len() == 1 {
                    Some(left_preds.into_iter().next().unwrap())
                } else {
                    Some(Predicate::And { and: left_preds })
                };

                let right = if right_preds.is_empty() {
                    None
                } else if right_preds.len() == 1 {
                    Some(right_preds.into_iter().next().unwrap())
                } else {
                    Some(Predicate::And { and: right_preds })
                };

                let rem = if remaining.is_empty() {
                    None
                } else if remaining.len() == 1 {
                    Some(remaining.into_iter().next().unwrap())
                } else {
                    Some(Predicate::And { and: remaining })
                };

                (left, right, rem)
            }
            Predicate::Eq { eq } if eq.len() == 2 => {
                // 等価述語の場合、結合キーに関連するかチェック
                let left_vars = self.extract_variables(&eq[0]);
                let right_vars = self.extract_variables(&eq[1]);

                if self.contains_join_key(&left_vars, join_keys) &&
                   self.contains_join_key(&right_vars, join_keys) {
                    // 結合条件として扱う
                    (None, None, Some(pred.clone()))
                } else if self.contains_join_key(&left_vars, join_keys) {
                    (Some(pred.clone()), None, None)
                } else if self.contains_join_key(&right_vars, join_keys) {
                    (None, Some(pred.clone()), None)
                } else {
                    (None, None, Some(pred.clone()))
                }
            }
            _ => (None, None, Some(pred.clone())),
        }
    }

    /// 式から変数を抽出
    fn extract_variables(&self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Var(v) => vec![v.clone()],
            Expr::Fn { args, .. } => args.iter()
                .flat_map(|arg| self.extract_variables(arg))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// 結合キーが含まれるかチェック
    fn contains_join_key(&self, vars: &[String], join_keys: &[String]) -> bool {
        vars.iter().any(|v| join_keys.contains(v))
    }

    /// 結合順序最適化
    fn optimize_join_order(&self, plan: PlanIR, catalog: &Catalog) -> PlanIR {
        let optimized_plan = self.optimize_join_order_op(&plan.plan, catalog);
        PlanIR {
            plan: optimized_plan,
            limit: plan.limit,
        }
    }

    fn optimize_join_order_op(&self, op: &LogicalOp, catalog: &Catalog) -> LogicalOp {
        match op {
            LogicalOp::Join { left, right, on } => {
                // DPベースの結合順序最適化（簡易版）
                let left_cost = self.estimate_cost(left, catalog);
                let right_cost = self.estimate_cost(right, catalog);

                if left_cost > right_cost {
                    // コストが小さい方を左側に
                    LogicalOp::Join {
                        left: Box::new(self.optimize_join_order_op(right, catalog)),
                        right: Box::new(self.optimize_join_order_op(left, catalog)),
                        on: on.clone(),
                    }
                } else {
                    LogicalOp::Join {
                        left: Box::new(self.optimize_join_order_op(left, catalog)),
                        right: Box::new(self.optimize_join_order_op(right, catalog)),
                        on: on.clone(),
                    }
                }
            }
            _ => op.clone(),
        }
    }

    /// 演算子のコストを推定
    fn estimate_cost(&self, op: &LogicalOp, catalog: &Catalog) -> f64 {
        match op {
            LogicalOp::NodeScan { label, .. } => {
                catalog.get_label(label)
                    .map(|_| 100.0)
                    .unwrap_or(1000.0)
            }
            LogicalOp::Join { left, right, .. } => {
                let left_cost = self.estimate_cost(left, catalog);
                let right_cost = self.estimate_cost(right, catalog);
                left_cost * right_cost
            }
            _ => 10.0,
        }
    }

    /// 不要な射影除去
    fn eliminate_unnecessary_projections(&self, plan: PlanIR) -> PlanIR {
        let optimized_plan = self.eliminate_unnecessary_projections_op(&plan.plan);
        PlanIR {
            plan: optimized_plan,
            limit: plan.limit,
        }
    }

    fn eliminate_unnecessary_projections_op(&self, op: &LogicalOp) -> LogicalOp {
        match op {
            LogicalOp::Project { cols, input } => {
                match input.as_ref() {
                    LogicalOp::Project { cols: inner_cols, input: inner_input } => {
                        // 連続する射影をマージ
                        let merged_cols = cols.iter()
                            .filter(|col| inner_cols.contains(col))
                            .cloned()
                            .collect();

                        LogicalOp::Project {
                            cols: merged_cols,
                            input: inner_input.clone(),
                        }
                    }
                    _ => LogicalOp::Project {
                        cols: cols.clone(),
                        input: Box::new(self.eliminate_unnecessary_projections_op(input)),
                    }
                }
            }
            _ => op.clone(),
        }
    }

    /// 定数畳み込み
    fn constant_folding(&self, plan: PlanIR) -> PlanIR {
        // 簡易的な定数畳み込みの実装
        plan
    }

    /// インデックス選択
    fn select_indexes(&self, plan: PlanIR, catalog: &Catalog) -> PlanIR {
        let optimized_plan = self.select_indexes_op(&plan.plan, catalog);
        PlanIR {
            plan: optimized_plan,
            limit: plan.limit,
        }
    }

    fn select_indexes_op(&self, op: &LogicalOp, catalog: &Catalog) -> LogicalOp {
        match op {
            LogicalOp::Filter { pred, input } => {
                match input.as_ref() {
                    LogicalOp::NodeScan { label, as_, props: _ } => {
                        // インデックスが存在する場合、IndexScanに変換
                        if let Some(index) = self.find_best_index(catalog, label, pred) {
                            LogicalOp::Filter {
                                pred: pred.clone(),
                                input: Box::new(LogicalOp::IndexScan {
                                    label: label.clone(),
                                    as_: as_.clone(),
                                    index: index.name,
                                    value: self.extract_index_value(pred, &index.properties[0]),
                                }),
                            }
                        } else {
                            LogicalOp::Filter {
                                pred: pred.clone(),
                                input: Box::new(self.select_indexes_op(input, catalog)),
                            }
                        }
                    }
                    _ => LogicalOp::Filter {
                        pred: pred.clone(),
                        input: Box::new(self.select_indexes_op(input, catalog)),
                    }
                }
            }
            _ => op.clone(),
        }
    }

    /// 最適なインデックスを選択
    fn find_best_index(&self, catalog: &Catalog, label: &Label, pred: &Predicate) -> Option<IndexDef> {
        catalog.indexes.iter()
            .filter(|idx| &idx.label == label)
            .find(|idx| self.can_use_index(pred, &idx.properties[0]))
            .cloned()
    }

    /// インデックスを使用できるかチェック
    fn can_use_index(&self, pred: &Predicate, prop: &PropertyKey) -> bool {
        match pred {
            Predicate::Eq { eq } if eq.len() == 2 => {
                let left_vars = self.extract_variables(&eq[0]);
                let right_vars = self.extract_variables(&eq[1]);

                // プロパティに対する等価条件
                left_vars.contains(prop) || right_vars.contains(prop)
            }
            _ => false,
        }
    }

    /// インデックス値を取得
    fn extract_index_value(&self, pred: &Predicate, prop: &PropertyKey) -> Value {
        match pred {
            Predicate::Eq { eq } if eq.len() == 2 => {
                if let Expr::Var(var) = &eq[0] {
                    if var == prop {
                        if let Expr::Const(val) = &eq[1] {
                            return val.clone();
                        }
                    }
                }
                if let Expr::Var(var) = &eq[1] {
                    if var == prop {
                        if let Expr::Const(val) = &eq[0] {
                            return val.clone();
                        }
                    }
                }
            }
            _ => {}
        }
        Value::Null
    }
}

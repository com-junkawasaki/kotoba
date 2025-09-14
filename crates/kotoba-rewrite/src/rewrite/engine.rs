//! 書換えエンジン

use kotoba_core::{types::*, ir::*};
use kotoba_graph::prelude::*;
use crate::rewrite::*;

/// 書換えエンジン
#[derive(Debug)]
pub struct RewriteEngine {
    matcher: RuleMatcher,
    applier: RuleApplier,
}

impl RewriteEngine {
    pub fn new() -> Self {
        Self {
            matcher: RuleMatcher::new(),
            applier: RuleApplier::new(),
        }
    }

    /// ルールをマッチングして適用
    pub fn match_rule(&self, graph: &GraphRef, rule: &RuleIR, catalog: &Catalog) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
        self.matcher.find_matches(graph, rule, catalog)
    }

    /// ルールを適用してパッチを生成
    pub fn rewrite(&self, graph: &GraphRef, rule: &RuleIR, strategy: &StrategyIR) -> Result<Patch, Box<dyn std::error::Error>> {
        match &strategy.strategy {
            StrategyOp::Once { rule: rule_name } => {
                self.apply_once(graph, rule, rule_name)
            }
            StrategyOp::Exhaust { rule: rule_name, order, measure } => {
                self.apply_exhaust(graph, rule, rule_name, order, measure.as_deref())
            }
            StrategyOp::While { rule: rule_name, pred, order } => {
                self.apply_while(graph, rule, rule_name, pred, order)
            }
            StrategyOp::Seq { strategies } => {
                self.apply_sequence(graph, rule, strategies)
            }
            StrategyOp::Choice { strategies } => {
                self.apply_choice(graph, rule, strategies)
            }
            StrategyOp::Priority { strategies } => {
                self.apply_priority(graph, rule, strategies)
            }
        }
    }

    /// 1回だけ適用
    fn apply_once(&self, graph: &GraphRef, rule: &RuleIR, _rule_name: &str) -> Result<Patch, Box<dyn std::error::Error>> {
        let matches = self.matcher.find_matches(graph, rule, &Catalog::empty())?;

        if let Some(match_) = matches.into_iter().next() {
            self.applier.apply_rule(graph, rule, &match_)
        } else {
            Ok(Patch::empty())
        }
    }

    /// 適用可能になるまで繰り返し
    fn apply_exhaust(&self, graph: &GraphRef, rule: &RuleIR, _rule_name: &str,
                     order: &Order, _measure: Option<&str>) -> Result<Patch, Box<dyn std::error::Error>> {
        let mut total_patch = Patch::empty();
        let mut iteration = 0;
        let max_iterations = 1000; // 無限ループ防止

        loop {
            let matches = self.matcher.find_matches(graph, rule, &Catalog::empty())?;

            if matches.is_empty() || iteration >= max_iterations {
                break;
            }

            // 最初のマッチを選択（orderに基づいて）
            let match_ = self.select_match(&matches, order)?;

            let patch = self.applier.apply_rule(graph, rule, &match_)?;
            total_patch = total_patch.merge(patch);

            iteration += 1;
        }

        Ok(total_patch)
    }

    /// 条件付き繰り返し
    fn apply_while(&self, graph: &GraphRef, rule: &RuleIR, _rule_name: &str,
                   pred: &str, order: &Order) -> Result<Patch> {
        let mut total_patch = Patch::empty();

        loop {
            let matches = self.matcher.find_matches(graph, rule, &Catalog::empty())?;

            if matches.is_empty() {
                break;
            }

            // 述語を評価（簡易実装）
            if !self.evaluate_predicate(pred, graph) {
                break;
            }

            let match_ = self.select_match(&matches, order)?;
            let patch = self.applier.apply_rule(graph, rule, &match_)?;
            total_patch = total_patch.merge(patch);
        }

        Ok(total_patch)
    }

    /// 順次実行
    fn apply_sequence(&self, graph: &GraphRef, rule: &RuleIR,
                      strategies: &[Box<StrategyOp>]) -> Result<Patch> {
        let mut total_patch = Patch::empty();

        for strategy in strategies {
            let strategy_ir = StrategyIR {
                strategy: *strategy.clone(),
            };
            let patch = self.rewrite(graph, rule, &strategy_ir)?;
            total_patch = total_patch.merge(patch);
        }

        Ok(total_patch)
    }

    /// 選択実行
    fn apply_choice(&self, graph: &GraphRef, rule: &RuleIR,
                    strategies: &[Box<StrategyOp>]) -> Result<Patch> {
        for strategy in strategies {
            let strategy_ir = StrategyIR {
                strategy: *strategy.clone(),
            };
            let patch = self.rewrite(graph, rule, &strategy_ir)?;

            if !patch.is_empty() {
                return Ok(patch);
            }
        }

        Ok(Patch::empty())
    }

    /// 優先順位付き選択
    fn apply_priority(&self, graph: &GraphRef, rule: &RuleIR,
                      strategies: &[PrioritizedStrategy]) -> Result<Patch> {
        // 優先順位でソート
        let mut sorted_strategies = strategies.to_vec();
        sorted_strategies.sort_by_key(|s| s.priority);

        for prioritized in sorted_strategies {
            let strategy_ir = StrategyIR {
                strategy: *prioritized.strategy,
            };
            let patch = self.rewrite(graph, rule, &strategy_ir)?;

            if !patch.is_empty() {
                return Ok(patch);
            }
        }

        Ok(Patch::empty())
    }

    /// マッチを選択
    fn select_match(&self, matches: &[Match], order: &Order) -> Result<Match> {
        if matches.is_empty() {
            return Err(KotobaError::Rewrite("No matches found".to_string()));
        }

        match order {
            Order::TopDown => Ok(matches[0].clone()), // 最初のマッチ
            Order::BottomUp => Ok(matches[matches.len() - 1].clone()), // 最後のマッチ
            Order::Fair => Ok(matches[0].clone()), // 簡易的に最初のマッチ
        }
    }

    /// 述語を評価
    fn evaluate_predicate(&self, pred: &str, graph: &GraphRef) -> bool {
        // 簡易的な述語評価（実際の実装ではRust関数を呼び出し）
        match pred {
            "edge_count_nonincreasing" => {
                // エッジ数が非増加（停止測度）
                graph.read().edge_count() <= 1000 // 仮の閾値
            }
            "deg_ge" => {
                // 次数チェック（簡易版）
                true
            }
            _ => true, // デフォルトでtrue
        }
    }
}

/// 外部関数インターフェース
pub trait RewriteExterns {
    fn deg_ge(&self, v: VertexId, k: u32) -> bool;
    fn edge_count_nonincreasing(&self, g0: &GraphRef, g1: &GraphRef) -> bool;
    fn custom_measure(&self, name: &str, args: &[Value]) -> f64;
}

//! 書換えエンジン

use kotoba_core::ir::*;
use kotoba_graph::prelude::*;
use kotoba_core::types::*;
use crate::rewrite::{RuleApplier, RuleMatcher, DPOMatch};
use kotoba_core::ir::catalog::Catalog;
use kotoba_cid::CidManager;
use std::collections::HashMap;
use kotoba_errors::KotobaError;

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
    pub fn match_rule(&self, graph: &GraphRef, rule: &RuleIR, catalog: &Catalog) -> Result<Vec<Match>> {
        self.matcher.find_matches(graph, rule, catalog)
    }

    /// ルールを適用してパッチを生成
    pub fn rewrite(&self, graph: &GraphRef, rule: &RuleIR, strategy: &StrategyIR) -> Result<Patch> {
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
    fn apply_once(&self, graph: &GraphRef, rule: &RuleIR, _rule_name: &str) -> Result<Patch> {
        let matches = self.matcher.find_matches(graph, rule, &Catalog::empty())?;

        if let Some(match_) = matches.into_iter().next() {
            self.applier.apply_rule(graph, rule, &match_)
        } else {
            Ok(Patch::empty())
        }
    }

    /// 適用可能になるまで繰り返し
    fn apply_exhaust(&self, graph: &GraphRef, rule: &RuleIR, _rule_name: &str,
                     order: &Order, _measure: Option<&str>) -> Result<Patch> {
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

    /// RuleDPOでマッチングと適用を行う
    pub fn match_and_apply_rule_dpo(&self, graph: &GraphRef, rule_dpo: &RuleDPO, cid_manager: &mut CidManager) -> Result<Option<GraphInstance>> {
        // GraphをGraphInstanceに変換
        let graph_instance = graph.to_graph_instance_with_cid(cid_manager)?;

        // 簡易的なマッチング（実際の実装ではグラフマッチングアルゴリズムが必要）
        let matches = self.find_dpo_matches(&graph_instance, rule_dpo)?;

        if matches.is_empty() {
            return Ok(None);
        }

        // 最初のマッチを選択（実際の実装ではより洗練された選択）
        let match_info = &matches[0];

        // DPO適用
        let result_graph = self.apply_dpo_rule(&graph_instance, rule_dpo, match_info, cid_manager)?;

        Ok(Some(result_graph))
    }

    /// DPOマッチング
    fn find_dpo_matches(&self, graph: &GraphInstance, rule: &RuleDPO) -> Result<Vec<DPOMatch>> {
        // 簡易版の実装 - 実際にはグラフマッチングが必要
        let mut matches = Vec::new();

        // LHSパターンとグラフの比較（簡易版）
        if self.graph_contains_pattern(graph, &rule.l) {
            let match_info = DPOMatch {
                node_mapping: HashMap::new(), // 簡易版
                edge_mapping: HashMap::new(),
            };
            matches.push(match_info);
        }

        Ok(matches)
    }

    /// グラフがパターンを含むかチェック（簡易版）
    fn graph_contains_pattern(&self, graph: &GraphInstance, pattern: &GraphInstance) -> bool {
        // 簡易版の実装 - 実際にはグラフ同型性チェックが必要
        // ノード数とエッジ数の基本チェック
        if graph.core.nodes.len() < pattern.core.nodes.len() ||
           graph.core.edges.len() < pattern.core.edges.len() {
            return false;
        }

        // ラベルベースの簡単なチェック
        for pattern_node in &pattern.core.nodes {
            let has_matching_node = graph.core.nodes.iter()
                .any(|node| node.labels == pattern_node.labels && node.r#type == pattern_node.r#type);
            if !has_matching_node {
                return false;
            }
        }

        true
    }

    /// DPOルールを適用
    fn apply_dpo_rule(&self, host_graph: &GraphInstance, rule: &RuleDPO, match_info: &DPOMatch, cid_manager: &mut CidManager) -> Result<GraphInstance> {
        let mut result_graph = host_graph.clone();

        // L - K（削除）
        self.remove_l_minus_k(&mut result_graph, rule, match_info)?;

        // R - K（追加）
        self.add_r_minus_k(&mut result_graph, rule, match_info)?;

        // CID再計算
        let new_cid = cid_manager.compute_graph_cid(&result_graph.core)?;
        result_graph.cid = new_cid;

        Ok(result_graph)
    }

    /// L - K の削除
    fn remove_l_minus_k(&self, graph: &mut GraphInstance, rule: &RuleDPO, match_info: &DPOMatch) -> Result<()> {
        // 簡易版の実装
        // 実際の実装ではマッピングに基づいて正確な削除が必要

        // Lに含まれKに含まれないノード/エッジを削除
        let nodes_to_remove: Vec<String> = rule.l.core.nodes.iter()
            .filter(|lhs_node| {
                !rule.k.core.nodes.iter()
                    .any(|k_node| k_node.cid == lhs_node.cid)
            })
            .map(|node| node.cid.as_str().to_string())
            .collect();

        let edges_to_remove: Vec<String> = rule.l.core.edges.iter()
            .filter(|lhs_edge| {
                !rule.k.core.edges.iter()
                    .any(|k_edge| k_edge.cid == lhs_edge.cid)
            })
            .map(|edge| edge.cid.as_str().to_string())
            .collect();

        // ノード削除
        graph.core.nodes.retain(|node| !nodes_to_remove.contains(&node.cid.as_str().to_string()));

        // エッジ削除（接続されているノードも削除された場合）
        graph.core.edges.retain(|edge| {
            !edges_to_remove.contains(&edge.cid.as_str().to_string()) &&
            nodes_to_remove.iter().all(|removed_node| {
                !edge.src.contains(removed_node) && !edge.tgt.contains(removed_node)
            })
        });

        Ok(())
    }

    /// R - K の追加
    fn add_r_minus_k(&self, graph: &mut GraphInstance, rule: &RuleDPO, match_info: &DPOMatch) -> Result<()> {
        // 簡易版の実装
        // 実際の実装ではマッピングに基づいて正確な追加が必要

        // Rに含まれKに含まれないノード/エッジを追加
        for rhs_node in &rule.r.core.nodes {
            if !rule.k.core.nodes.iter().any(|k_node| k_node.cid == rhs_node.cid) {
                graph.core.nodes.push(rhs_node.clone());
            }
        }

        for rhs_edge in &rule.r.core.edges {
            if !rule.k.core.edges.iter().any(|k_edge| k_edge.cid == rhs_edge.cid) {
                graph.core.edges.push(rhs_edge.clone());
            }
        }

        Ok(())
    }
}

/// DPOマッチング結果
#[derive(Debug, Clone)]
pub struct DPOMatch {
    pub node_mapping: std::collections::HashMap<String, String>,
    pub edge_mapping: std::collections::HashMap<String, String>,
}

/// 外部関数インターフェース
pub trait RewriteExterns {
    fn deg_ge(&self, v: VertexId, k: u32) -> bool;
    fn edge_count_nonincreasing(&self, g0: &GraphRef, g1: &GraphRef) -> bool;
    fn custom_measure(&self, name: &str, args: &[Value]) -> f64;
}

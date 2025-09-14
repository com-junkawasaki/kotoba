//! ルールマッチング

use kotoba_core::{ir::*, types::*};
use kotoba_graph::graph::*;
use std::collections::HashMap;
use kotoba_core::types::Result;

/// ルールマッチャー
#[derive(Debug)]
pub struct RuleMatcher;

impl RuleMatcher {
    pub fn new() -> Self {
        Self
    }

    /// グラフに対してルールをマッチング
    pub fn find_matches(&self, graph: &GraphRef, rule: &RuleIR, catalog: &Catalog) -> Result<Vec<Match>, Box<dyn std::error::Error>> {
        let graph = graph.read();

        // LHS（Left-hand side）のマッチング
        let mut matches = Vec::new();

        // 初期マッチング候補を生成
        let initial_candidates = self.generate_initial_candidates(&graph, &rule.lhs)?;

        for candidate in initial_candidates {
            if self.match_lhs(&graph, &rule.lhs, &candidate, catalog)? {
                // NAC（Negative Application Condition）をチェック
                if self.check_nacs(&graph, &rule.nacs, &candidate, catalog)? {
                    // ガード条件をチェック
                    if self.check_guards(&graph, &rule.guards, &candidate, catalog)? {
                        let match_score = self.calculate_match_score(&candidate);
                        matches.push(Match {
                            mapping: candidate,
                            score: match_score,
                        });
                    }
                }
            }
        }

        // スコアでソート
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(matches)
    }

    /// 初期マッチング候補を生成
    fn generate_initial_candidates(&self, graph: &Graph, lhs: &GraphPattern)
        -> Vec<HashMap<String, VertexId>> {

        if lhs.nodes.is_empty() {
            return vec![HashMap::new()];
        }

        let mut candidates = Vec::new();

        // 最初のノードから候補を生成
        let first_node = &lhs.nodes[0];
        let vertex_ids = if let Some(label) = &first_node.type_ {
            graph.vertices_by_label(label)
        } else {
            graph.vertices.keys().cloned().collect()
        };

        for vertex_id in vertex_ids {
            let mut mapping = HashMap::new();
            mapping.insert(first_node.id.clone(), vertex_id);
            candidates.push(mapping);
        }

        candidates
    }

    /// LHSパターンマッチング
    fn match_lhs(&self, graph: &Graph, lhs: &GraphPattern,
                 mapping: &HashMap<String, VertexId>, catalog: &Catalog) -> Result<bool> {

        // ノードマッチング
        for node in &lhs.nodes {
            if let Some(&vertex_id) = mapping.get(&node.id) {
                if let Some(vertex) = graph.get_vertex(&vertex_id) {
                    // ラベルチェック
                    if let Some(expected_label) = &node.type_ {
                        if !vertex.labels.contains(expected_label) {
                            return Ok(false);
                        }
                    }

                    // プロパティチェック
                    if let Some(expected_props) = &node.props {
                        for (key, expected_value) in expected_props {
                            if let Some(actual_value) = vertex.props.get(key) {
                                if !self.values_match(actual_value, expected_value) {
                                    return Ok(false);
                                }
                            } else {
                                return Ok(false);
                            }
                        }
                    }
                } else {
                    return Ok(false);
                }
            }
        }

        // エッジマッチング
        for edge in &lhs.edges {
            if let (Some(&src_id), Some(&dst_id)) = (mapping.get(&edge.src), mapping.get(&edge.dst)) {
                // エッジが存在するかチェック
                if !graph.adj_out.get(&src_id)
                    .map(|neighbors| neighbors.contains(&dst_id))
                    .unwrap_or(false) {
                    return Ok(false);
                }

                // エッジラベルチェック（簡易版）
                // 実際の実装ではエッジIDをマッピングに含める必要がある
            }
        }

        Ok(true)
    }

    /// NACチェック
    fn check_nacs(&self, graph: &Graph, nacs: &[Nac],
                  mapping: &HashMap<String, VertexId>, catalog: &Catalog) -> Result<bool> {

        for nac in nacs {
            // NACパターンがマッチしないことを確認
            if self.match_nac(graph, nac, mapping, catalog)? {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// NACマッチング
    fn match_nac(&self, graph: &Graph, nac: &Nac,
                 mapping: &HashMap<String, VertexId>, _catalog: &Catalog) -> Result<bool> {

        // NAC内のノードをマッチング
        for node in &nac.nodes {
            if let Some(&vertex_id) = mapping.get(&node.id) {
                if let Some(vertex) = graph.get_vertex(&vertex_id) {
                    // NAC条件に合致する場合、falseを返す
                    if let Some(expected_label) = &node.type_ {
                        if vertex.labels.contains(expected_label) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // NAC内のエッジをチェック
        for edge in &nac.edges {
            if let (Some(&src_id), Some(&dst_id)) = (mapping.get(&edge.src), mapping.get(&edge.dst)) {
                if graph.adj_out.get(&src_id)
                    .map(|neighbors| neighbors.contains(&dst_id))
                    .unwrap_or(false) {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// ガード条件チェック
    fn check_guards(&self, graph: &Graph, guards: &[Guard],
                    mapping: &HashMap<String, VertexId>, _catalog: &Catalog) -> bool {

        for guard in guards {
            if !self.evaluate_guard(graph, guard, mapping, _catalog) {
                return false;
            }
        }

        true
    }

    /// ガード条件評価
    fn evaluate_guard(&self, graph: &Graph, guard: &Guard,
                      mapping: &HashMap<String, VertexId>, _catalog: &Catalog) -> bool {

        match guard.ref_.as_str() {
            "deg_ge" => {
                // 次数 >= k のチェック
                if let Some(Value::Int(k)) = guard.args.get("k") {
                    if let Some(Value::String(var)) = guard.args.get("var") {
                        if let Some(&vertex_id) = mapping.get(var) {
                            let degree = graph.degree(&vertex_id);
                            return degree >= *k as usize;
                        }
                    }
                }
                false
            }
            _ => {
                // その他のガードはtrueとして扱う（実際の実装では関数テーブルを使用）
                true
            }
        }
    }

    /// 値のマッチング
    fn values_match(&self, actual: &Value, expected: &Value) -> bool {
        match (actual, expected) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            _ => false,
        }
    }

    /// マッチスコア計算
    fn calculate_match_score(&self, mapping: &HashMap<String, VertexId>) -> f64 {
        // 簡易的なスコア計算（マッピングサイズに基づく）
        mapping.len() as f64
    }
}

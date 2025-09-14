//! ルール適用

use kotoba_core::{ir::*, types::*};
use kotoba_graph::graph::*;
use std::collections::HashMap;
use kotoba_core::types::Result;
use uuid;

/// ルール適用器
#[derive(Debug)]
pub struct RuleApplier;

impl RuleApplier {
    pub fn new() -> Self {
        Self
    }

    /// ルールを適用してパッチを生成
    pub fn apply_rule(&self, graph: &GraphRef, rule: &RuleIR, match_: &Match) -> Result<Patch> {
        let mut patch = Patch::empty();

        // 削除操作（L - K）
        self.generate_deletions(&mut patch, graph, rule, match_)?;

        // 追加操作（R - K）
        self.generate_additions(&mut patch, rule, match_)?;

        // 更新操作（Kの変更）
        self.generate_updates(&mut patch, graph, rule, match_)?;

        Ok(patch)
    }

    /// 削除操作を生成（L - K）
    fn generate_deletions(&self, patch: &mut Patch, graph: &GraphRef,
                         rule: &RuleIR, match_: &Match) -> Result<()> {

        let _graph = graph.read();

        // L（左辺）の要素をK（文脈）と比較して削除対象を決定
        for lhs_node in &rule.lhs.nodes {
            let should_delete = !rule.context.nodes.iter()
                .any(|ctx_node| ctx_node.id == lhs_node.id);

            if should_delete {
                if let Some(&vertex_id) = match_.mapping.get(&lhs_node.id) {
                    patch.dels.vertices.push(vertex_id);
                }
            }
        }

        for lhs_edge in &rule.lhs.edges {
            let should_delete = !rule.context.edges.iter()
                .any(|ctx_edge| ctx_edge.id == lhs_edge.id);

            if should_delete {
                // エッジIDをマッピングから取得（簡易版）
                // 実際の実装ではエッジマッピングも必要
                if let (Some(&src_id), Some(&dst_id)) =
                    (match_.mapping.get(&lhs_edge.src), match_.mapping.get(&lhs_edge.dst)) {

                    // 対応するエッジIDを検索
                    for (edge_id, edge) in &_graph.edges {
                        if edge.src == src_id && edge.dst == dst_id {
                            if let Some(expected_label) = &lhs_edge.type_ {
                                if &edge.label == expected_label {
                                    patch.dels.edges.push(*edge_id);
                                    break;
                                }
                            } else {
                                patch.dels.edges.push(*edge_id);
                                break;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 追加操作を生成（R - K）
    fn generate_additions(&self, patch: &mut Patch, rule: &RuleIR, match_: &Match) -> Result<()> {

        // R（右辺）の要素をK（文脈）と比較して追加対象を決定
        for rhs_node in &rule.rhs.nodes {
            let should_add = !rule.context.nodes.iter()
                .any(|ctx_node| ctx_node.id == rhs_node.id);

            if should_add {
                let vertex_id = uuid::Uuid::new_v4();
                let labels = if let Some(label) = &rhs_node.type_ {
                    vec![label.clone()]
                } else {
                    Vec::new()
                };

                let props = rhs_node.props.clone().unwrap_or_default();

                patch.adds.vertices.push(AddVertex {
                    id: vertex_id,
                    labels,
                    props,
                });

                // マッピングに追加（後続のエッジ追加で使用）
                // 注意: 実際の実装ではmatch_.mappingをmutableにする必要がある
            }
        }

        for rhs_edge in &rule.rhs.edges {
            let should_add = !rule.context.edges.iter()
                .any(|ctx_edge| ctx_edge.id == rhs_edge.id);

            if should_add {
                if let (Some(&src_id), Some(&dst_id)) =
                    (match_.mapping.get(&rhs_edge.src), match_.mapping.get(&rhs_edge.dst)) {

                    let edge_id = uuid::Uuid::new_v4();
                    let label = rhs_edge.type_.clone().unwrap_or_else(|| "EDGE".to_string());

                    patch.adds.edges.push(AddEdge {
                        id: edge_id,
                        src: src_id,
                        dst: dst_id,
                        label,
                        props: HashMap::new(),
                    });
                }
            }
        }

        Ok(())
    }

    /// 更新操作を生成（Kの変更）
    fn generate_updates(&self, patch: &mut Patch, graph: &GraphRef,
                       rule: &RuleIR, match_: &Match) -> Result<()> {

        let _graph = graph.read();

        // K（文脈）の要素に対する変更を処理
        for ctx_node in &rule.context.nodes {
            if let Some(&vertex_id) = match_.mapping.get(&ctx_node.id) {
                // LHSとRHSでプロパティが異なる場合、更新が必要
                let lhs_props = rule.lhs.nodes.iter()
                    .find(|n| n.id == ctx_node.id)
                    .and_then(|n| n.props.as_ref());

                let rhs_props = rule.rhs.nodes.iter()
                    .find(|n| n.id == ctx_node.id)
                    .and_then(|n| n.props.as_ref());

                if let (Some(lhs_p), Some(rhs_p)) = (lhs_props, rhs_props) {
                    for (key, rhs_value) in rhs_p {
                        if let Some(lhs_value) = lhs_p.get(key) {
                            if !self.values_equal(lhs_value, rhs_value) {
                                patch.updates.props.push(UpdateProp {
                                    id: vertex_id,
                                    key: key.clone(),
                                    value: rhs_value.clone(),
                                });
                            }
                        } else {
                            // 新しいプロパティ
                            patch.updates.props.push(UpdateProp {
                                id: vertex_id,
                                key: key.clone(),
                                value: rhs_value.clone(),
                            });
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// 値の等価性チェック
    fn values_equal(&self, a: &Value, b: &Value) -> bool {
        match (a, b) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(x), Value::Bool(y)) => x == y,
            (Value::Int(x), Value::Int(y)) => x == y,
            (Value::String(x), Value::String(y)) => x == y,
            _ => false,
        }
    }
}

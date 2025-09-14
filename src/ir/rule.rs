//! Rule-IR（DPO型付き属性グラフ書換え）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::*;
use crate::schema::*;

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

/// 負の条件（NAC: Negative Application Condition）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nac {
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

/// RuleIRからRuleDPOへの変換
impl RuleIR {
    /// RuleIRをRuleDPOに変換
    pub fn to_rule_dpo(&self) -> kotoba_core::types::Result<RuleDPO> {
        // L, K, RのGraphPatternをGraphInstanceに変換
        let l = self.graph_pattern_to_instance(&self.lhs)?;
        let k = self.graph_pattern_to_instance(&self.context)?;
        let r = self.graph_pattern_to_instance(&self.rhs)?;

        // NACを変換
        let nacs = self.nacs.iter()
            .map(|nac| self.nac_to_dpo_nac(nac))
            .collect::<Result<Vec<_>>>()?;

        // 写像の生成（簡易版）
        let m_l = self.generate_morphism(&l, &k)?;
        let m_r = self.generate_morphism(&r, &k)?;

        Ok(RuleDPO {
            id: Id::new(&self.name)?,
            l,
            k,
            r,
            m_l,
            m_r,
            nacs,
            app_cond: None, // デフォルト
            effects: None,  // デフォルト
        })
    }

    /// GraphPatternをGraphInstanceに変換
    fn graph_pattern_to_instance(&self, pattern: &GraphPattern) -> kotoba_core::types::Result<GraphInstance> {
        use crate::cid::*;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // ノード変換
        for node_elem in &pattern.nodes {
            let node = Node {
                cid: Cid::new(&format!("node_{}", node_elem.id)), // 仮のCID生成
                labels: node_elem.type_.as_ref().map(|t| vec![t.clone()]).unwrap_or_default(),
                r#type: node_elem.type_.clone().unwrap_or_else(|| "Node".to_string()),
                ports: vec![], // 簡易版
                attrs: node_elem.props.clone().map(|props| {
                    props.into_iter().map(|(k, v)| (k, serde_json::to_value(v).unwrap())).collect()
                }),
                component_ref: None,
            };
            nodes.push(node);
        }

        // エッジ変換
        for edge_def in &pattern.edges {
            let edge = Edge {
                cid: Cid::new(&format!("edge_{}", edge_def.id)), // 仮のCID生成
                label: edge_def.type_.clone(),
                r#type: edge_def.type_.clone().unwrap_or_else(|| "EDGE".to_string()),
                src: edge_def.src.clone(),
                tgt: edge_def.dst.clone(),
                attrs: None,
            };
            edges.push(edge);
        }

        let graph_core = GraphCore {
            nodes,
            edges,
            boundary: None, // 簡易版
            attrs: None,
        };

        Ok(GraphInstance {
            core: graph_core,
            kind: GraphKind::Instance,
            cid: Cid::new(&format!("graph_{}", uuid::Uuid::new_v4())), // 仮のCID生成
            typing: None,
        })
    }

    /// NACを変換
    fn nac_to_dpo_nac(&self, nac: &Nac) -> kotoba_core::types::Result<Nac> {
        let graph = self.graph_pattern_to_instance(&GraphPattern {
            nodes: nac.nodes.clone(),
            edges: nac.edges.clone(),
        })?;

        // 簡易版の写像生成
        let node_map: HashMap<String, String> = nac.nodes.iter()
            .map(|node| (format!("nac_node_{}", node.id), format!("host_node_{}", node.id)))
            .collect();

        let edge_map: HashMap<String, String> = nac.edges.iter()
            .map(|edge| (format!("nac_edge_{}", edge.id), format!("host_edge_{}", edge.id)))
            .collect();

        let morphism_from_l = Morphisms {
            node_map,
            edge_map,
            port_map: HashMap::new(),
        };

        Ok(Nac {
            id: Id::new(&format!("nac_{}", uuid::Uuid::new_v4()))?,
            graph,
            morphism_from_l,
        })
    }

    /// 写像を生成（簡易版）
    fn generate_morphism(&self, from: &GraphInstance, to: &GraphInstance) -> kotoba_core::types::Result<Morphisms> {
        // 簡易版の実装 - 実際にはグラフマッチングが必要
        let mut node_map = HashMap::new();
        let mut edge_map = HashMap::new();

        // IDベースの単純マッピング
        for from_node in &from.core.nodes {
            for to_node in &to.core.nodes {
                if from_node.r#type == to_node.r#type {
                    node_map.insert(from_node.cid.as_str().to_string(), to_node.cid.as_str().to_string());
                    break;
                }
            }
        }

        for from_edge in &from.core.edges {
            for to_edge in &to.core.edges {
                if from_edge.r#type == to_edge.r#type {
                    edge_map.insert(from_edge.cid.as_str().to_string(), to_edge.cid.as_str().to_string());
                    break;
                }
            }
        }

        Ok(Morphisms {
            node_map,
            edge_map,
            port_map: HashMap::new(),
        })
    }
}

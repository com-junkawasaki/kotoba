//! Patch-IR（差分表現）

use serde::{Deserialize, Serialize};
use crate::types::*;

/// 頂点追加
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddVertex {
    pub id: VertexId,
    pub labels: Vec<Label>,
    pub props: Properties,
}

/// エッジ追加
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddEdge {
    pub id: EdgeId,
    pub src: VertexId,
    pub dst: VertexId,
    pub label: Label,
    pub props: Properties,
}

/// プロパティ更新
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateProp {
    pub id: VertexId,  // 頂点IDまたはエッジID
    pub key: PropertyKey,
    pub value: Value,
}

/// リリンク（エッジの端点変更）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relink {
    pub edge_id: EdgeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_src: Option<VertexId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_dst: Option<VertexId>,
}

/// パッチ操作
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Patch {
    pub adds: Adds,
    pub dels: Dels,
    pub updates: Updates,
}

/// 追加操作
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Adds {
    pub vertices: Vec<AddVertex>,
    pub edges: Vec<AddEdge>,
}

/// 削除操作
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Dels {
    pub vertices: Vec<VertexId>,
    pub edges: Vec<EdgeId>,
}

/// 更新操作
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Updates {
    pub props: Vec<UpdateProp>,
    pub relinks: Vec<Relink>,
}

impl Patch {
    /// 空のパッチを作成
    pub fn empty() -> Self {
        Self {
            adds: Adds::default(),
            dels: Dels::default(),
            updates: Updates::default(),
        }
    }

    /// パッチが空かどうか
    pub fn is_empty(&self) -> bool {
        self.adds.vertices.is_empty()
            && self.adds.edges.is_empty()
            && self.dels.vertices.is_empty()
            && self.dels.edges.is_empty()
            && self.updates.props.is_empty()
            && self.updates.relinks.is_empty()
    }

    /// 2つのパッチをマージ
    pub fn merge(mut self, other: Patch) -> Self {
        self.adds.vertices.extend(other.adds.vertices);
        self.adds.edges.extend(other.adds.edges);
        self.dels.vertices.extend(other.dels.vertices);
        self.dels.edges.extend(other.dels.edges);
        self.updates.props.extend(other.updates.props);
        self.updates.relinks.extend(other.updates.relinks);
        self
    }
}

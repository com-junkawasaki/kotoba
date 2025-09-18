//! グラフデータ構造（列指向）

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use kotoba_core::types::*;

/// 頂点データ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VertexData {
    pub id: VertexId,
    pub labels: Vec<Label>,
    pub props: Properties,
}

/// エッジデータ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EdgeData {
    pub id: EdgeId,
    pub src: VertexId,
    pub dst: VertexId,
    pub label: Label,
    pub props: Properties,
}

/// グラフ（列指向表現）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Graph {
    /// 頂点データ（ID→データ）
    pub vertices: HashMap<VertexId, VertexData>,

    /// エッジデータ（ID→データ）
    pub edges: HashMap<EdgeId, EdgeData>,

    /// 外向き隣接リスト（src→[dst])
    pub adj_out: HashMap<VertexId, HashSet<VertexId>>,

    /// 内向き隣接リスト（dst→[src])
    pub adj_in: HashMap<VertexId, HashSet<VertexId>>,

    /// ラベル別頂点インデックス（ラベル→[頂点ID]）
    pub vertex_labels: HashMap<Label, HashSet<VertexId>>,

    /// ラベル別エッジインデックス（ラベル→[エッジID]）
    pub edge_labels: HashMap<Label, HashSet<EdgeId>>,
}

impl Graph {
    /// 空のグラフを作成
    pub fn empty() -> Self {
        Self {
            vertices: HashMap::new(),
            edges: HashMap::new(),
            adj_out: HashMap::new(),
            adj_in: HashMap::new(),
            vertex_labels: HashMap::new(),
            edge_labels: HashMap::new(),
        }
    }

    /// 頂点を追加
    pub fn add_vertex(&mut self, vertex: VertexData) -> VertexId {
        let id = vertex.id;
        for label in &vertex.labels {
            self.vertex_labels.entry(label.clone()).or_default().insert(id);
        }
        self.vertices.insert(id, vertex);
        id
    }

    /// エッジを追加
    pub fn add_edge(&mut self, edge: EdgeData) -> EdgeId {
        let id = edge.id;
        let src = edge.src;
        let dst = edge.dst;

        // 隣接リスト更新
        self.adj_out.entry(src).or_default().insert(dst);
        self.adj_in.entry(dst).or_default().insert(src);

        // ラベルインデックス更新
        self.edge_labels.entry(edge.label.clone()).or_default().insert(id);

        self.edges.insert(id, edge);
        id
    }

    /// 頂点を取得
    pub fn get_vertex(&self, id: &VertexId) -> Option<&VertexData> {
        self.vertices.get(id)
    }

    /// エッジを取得
    pub fn get_edge(&self, id: &EdgeId) -> Option<&EdgeData> {
        self.edges.get(id)
    }

    /// 頂点を削除
    pub fn remove_vertex(&mut self, id: &VertexId) -> bool {
        if let Some(vertex) = self.vertices.remove(id) {
            // ラベルインデックスから削除
            for label in &vertex.labels {
                if let Some(vertices) = self.vertex_labels.get_mut(label) {
                    vertices.remove(id);
                    if vertices.is_empty() {
                        self.vertex_labels.remove(label);
                    }
                }
            }

            // 関連エッジを削除
            let mut edges_to_remove = Vec::new();
            for (edge_id, edge) in &self.edges {
                if edge.src == *id || edge.dst == *id {
                    edges_to_remove.push(*edge_id);
                }
            }
            for edge_id in edges_to_remove {
                self.remove_edge(&edge_id);
            }

            // 隣接リストから削除
            self.adj_out.remove(id);
            self.adj_in.remove(id);

            // 他の頂点の隣接リストから削除
            for adj in self.adj_out.values_mut() {
                adj.remove(id);
            }
            for adj in self.adj_in.values_mut() {
                adj.remove(id);
            }

            true
        } else {
            false
        }
    }

    /// エッジを削除
    pub fn remove_edge(&mut self, id: &EdgeId) -> bool {
        if let Some(edge) = self.edges.remove(id) {
            // 隣接リスト更新
            if let Some(out) = self.adj_out.get_mut(&edge.src) {
                out.remove(&edge.dst);
                if out.is_empty() {
                    self.adj_out.remove(&edge.src);
                }
            }
            if let Some(in_) = self.adj_in.get_mut(&edge.dst) {
                in_.remove(&edge.src);
                if in_.is_empty() {
                    self.adj_in.remove(&edge.dst);
                }
            }

            // ラベルインデックス更新
            if let Some(edges) = self.edge_labels.get_mut(&edge.label) {
                edges.remove(id);
                if edges.is_empty() {
                    self.edge_labels.remove(&edge.label);
                }
            }

            true
        } else {
            false
        }
    }

    /// ラベル別頂点を取得
    pub fn vertices_by_label(&self, label: &Label) -> HashSet<VertexId> {
        self.vertex_labels.get(label).cloned().unwrap_or_default()
    }

    /// ラベル別エッジを取得
    pub fn edges_by_label(&self, label: &Label) -> HashSet<EdgeId> {
        self.edge_labels.get(label).cloned().unwrap_or_default()
    }

    /// 頂点の次数を取得
    pub fn degree(&self, id: &VertexId) -> usize {
        let out_degree = self.adj_out.get(id).map(|s| s.len()).unwrap_or(0);
        let in_degree = self.adj_in.get(id).map(|s| s.len()).unwrap_or(0);
        out_degree + in_degree
    }

    /// 頂点数を取得
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// エッジ数を取得
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// スレッドセーフなグラフ参照
#[derive(Debug, Clone)]
pub struct GraphRef {
    inner: std::sync::Arc<RwLock<Graph>>,
}

impl GraphRef {
    pub fn new(graph: Graph) -> Self {
        Self {
            inner: std::sync::Arc::new(RwLock::new(graph)),
        }
    }

    pub fn read(&self) -> parking_lot::RwLockReadGuard<'_, Graph> {
        self.inner.read()
    }

    pub fn write(&self) -> parking_lot::RwLockWriteGuard<'_, Graph> {
        self.inner.write()
    }
}

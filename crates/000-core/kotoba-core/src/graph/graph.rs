//! グラフデータ構造（列指向）

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;
use sha2::{Sha256, Digest};
use crate::types::*;
use crate::schema::*;
use kotoba_errors::KotobaError;

/// 頂点データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VertexData {
    pub id: VertexId,
    pub labels: Vec<Label>,
    pub props: Properties,
}

/// エッジデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    pub id: EdgeId,
    pub src: VertexId,
    pub dst: VertexId,
    pub label: Label,
    pub props: Properties,
}

/// グラフ（列指向表現）
#[derive(Debug, Clone)]
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
            self.vertex_labels.entry(label.clone()).or_insert(HashSet::new()).insert(id);
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
        self.adj_out.entry(src).or_insert(HashSet::new()).insert(dst);
        self.adj_in.entry(dst).or_insert(HashSet::new()).insert(src);

        // ラベルインデックス更新
        self.edge_labels.entry(edge.label.clone()).or_insert(HashSet::new()).insert(id);

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

    /// GraphInstanceからGraphに変換
    pub fn from_graph_instance(graph_instance: &GraphInstance) -> Result<Self> {
        let mut graph = Graph::empty();
        let mut vertex_id_map = HashMap::new();

        // Node -> Vertex変換
        for node in &graph_instance.core.nodes {
            let vertex_id = uuid::Uuid::new_v4(); // 実際の実装ではCIDから決定論的に生成

            // 属性をValueに変換
            let props = if let Some(attrs) = &node.attrs {
                // TODO: AttrsからValueへの変換を実装
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            };

            let vertex_data = VertexData {
                id: vertex_id,
                labels: node.labels.clone(),
                props,
            };

            graph.add_vertex(vertex_data);
            vertex_id_map.insert(node.cid.as_str(), vertex_id);
        }

        // Edge -> Edge変換
        for edge in &graph_instance.core.edges {
            let edge_id = uuid::Uuid::new_v4(); // 実際の実装ではCIDから決定論的に生成

            // ソースとターゲットのVertex IDを取得
            let src_vertex_id = *vertex_id_map.get(edge.src.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Source node not found: {}", edge.src)))?;

            let dst_vertex_id = *vertex_id_map.get(edge.tgt.trim_start_matches('#'))
                .ok_or_else(|| KotobaError::Validation(format!("Target node not found: {}", edge.tgt)))?;

            // 属性をValueに変換
            let props = if let Some(attrs) = &edge.attrs {
                // TODO: AttrsからValueへの変換を実装
                attrs.iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            } else {
                HashMap::new()
            };

            let edge_data = EdgeData {
                id: edge_id,
                src: src_vertex_id,
                dst: dst_vertex_id,
                label: edge.r#type.clone(),
                props,
            };

            graph.add_edge(edge_data);
        }

        Ok(graph)
    }

    /// GraphからGraphInstanceに変換
    pub fn to_graph_instance(&self, graph_cid: Cid) -> GraphInstance {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // Vertex -> Node変換
        for (vertex_id, vertex_data) in &self.vertices {
            let node = Node {
                cid: generate_cid(&format!("node_{}", vertex_id)), // 実際の実装では決定論的に生成
                labels: vertex_data.labels.clone(),
                r#type: vertex_data.labels.first()
                    .cloned()
                    .unwrap_or_else(|| "Node".to_string()),
                ports: vec![], // 現在のGraph構造ではポート情報がない
                attrs: if vertex_data.props.is_empty() {
                    None
                } else {
                    Some(vertex_data.props.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>())
                },
                component_ref: None,
            };
            nodes.push(node);
        }

        // Edge -> Edge変換
        for (edge_id, edge_data) in &self.edges {
            let edge = Edge {
                cid: generate_cid(&format!("edge_{}", edge_id)), // 実際の実装では決定論的に生成
                label: Some(edge_data.label.clone()),
                r#type: edge_data.label.clone(),
                src: format!("#{}", edge_data.src),
                tgt: format!("#{}", edge_data.dst),
                attrs: if edge_data.props.is_empty() {
                    None
                } else {
                    Some(edge_data.props.iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect::<HashMap<_, _>>())
                },
            };
            edges.push(edge);
        }

        let graph_core = GraphCore {
            nodes,
            edges,
            boundary: None, // 現在のGraph構造では境界情報がない
            attrs: None,
        };

        GraphInstance {
            core: graph_core,
            kind: GraphKind::Graph,
            cid: graph_cid,
            typing: None,
        }
    }

    /// CIDを計算してGraphInstanceに変換
    pub fn to_graph_instance_with_cid(&self) -> Result<GraphInstance> {
        let graph_instance = self.to_graph_instance(generate_cid("temp"));
        // TODO: CIDマネージャーが実装されたら、ここで計算する
        Ok(GraphInstance {
            cid: generate_cid("graph"),
            ..graph_instance
        })
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

    /// GraphInstanceからGraphRefに変換
    pub fn from_graph_instance(graph_instance: &GraphInstance) -> Result<Self> {
        let graph = Graph::from_graph_instance(graph_instance)?;
        Ok(Self::new(graph))
    }

    /// GraphRefからGraphInstanceに変換
    pub fn to_graph_instance(&self, graph_cid: Cid) -> GraphInstance {
        let graph = self.read();
        graph.to_graph_instance(graph_cid)
    }

    /// CIDを計算してGraphInstanceに変換
    pub fn to_graph_instance_with_cid(&self) -> Result<GraphInstance> {
        let graph = self.read();
        graph.to_graph_instance_with_cid()
    }
}

/// CIDを生成するヘルパー関数
fn generate_cid(data: &str) -> Cid {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    let result = hasher.finalize();
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&result[..32]);
    Cid(bytes)
}

/// serde_json::ValueをValueに変換するヘルパー
pub fn serde_json_value_to_value(json_value: &serde_json::Value) -> Value {
    match json_value {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                // 注意: 現在のValue型はFloatをサポートしていないため、Stringに変換
                Value::String(f.to_string())
            } else {
                Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let strings: Vec<String> = arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            Value::Array(strings)
        },
        serde_json::Value::Object(_) => Value::String("Object".to_string()), // 簡易版
    }
}

pub fn value_to_json_value(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Bool(b) => serde_json::json!(b),
        Value::Int(i) => serde_json::json!(i),
        Value::Integer(i) => serde_json::json!(i),
        Value::String(s) => serde_json::json!(s),
        Value::Array(arr) => serde_json::Value::Array(
            arr.iter().map(|s| serde_json::Value::String(s.clone())).collect()
        ),
    }
}


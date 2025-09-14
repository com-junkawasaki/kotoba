//! # Generated from Jsonnet DSL
//!
//! This file was generated automatically from graph_simple.json
//! Do not edit manually.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use parking_lot::RwLock;

/// 頂点データ
#[derive(Debug, Clone)]
pub struct VertexData {
    pub id: VertexId,
    pub labels: Vec<Label>,
    pub props: Properties,
}

/// エッジデータ
#[derive(Debug, Clone)]
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
    pub vertices: HashMap<VertexId, VertexData>,
    pub edges: HashMap<EdgeId, EdgeData>,
}

impl Graph {
    /// 空のグラフを作成
    pub fn empty() -> Self {
        Self {
                        vertices: HashMap::new(),
                        edges: HashMap::new(),
                    }
    }
    /// 頂点を取得
    pub fn get_vertex(&self, id: &VertexId) -> Option<&VertexData> {
        self.vertices.get(id)
    }
}
//! Merkle DAG（コンテンツアドレッサブルストレージ）

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use crate::types::*;

/// Merkleノード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: ContentHash,
    pub data: Vec<u8>,
    pub children: Vec<ContentHash>,
    pub timestamp: u64,
}

/// Merkle DAG
#[derive(Debug)]
pub struct MerkleDAG {
    nodes: HashMap<ContentHash, MerkleNode>,
}

impl MerkleDAG {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// データを格納してMerkleハッシュを生成
    pub fn store(&mut self, data: &[u8], children: Vec<ContentHash>) -> ContentHash {
        let mut hasher = Sha256::new();
        hasher.update(data);
        for child in &children {
            hasher.update(child.0.as_bytes());
        }
        let hash = ContentHash(format!("{:x}", hasher.finalize()));

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let node = MerkleNode {
            hash: hash.clone(),
            data: data.to_vec(),
            children,
            timestamp,
        };

        self.nodes.insert(hash.clone(), node);
        hash
    }

    /// ハッシュからデータを取得
    pub fn get(&self, hash: &ContentHash) -> Option<&MerkleNode> {
        self.nodes.get(hash)
    }

    /// ハッシュが存在するかチェック
    pub fn contains(&self, hash: &ContentHash) -> bool {
        self.nodes.contains_key(hash)
    }

    /// 子のハッシュを取得
    pub fn get_children(&self, hash: &ContentHash) -> Option<&[ContentHash]> {
        self.nodes.get(hash).map(|node| node.children.as_slice())
    }

    /// ノード数を取得
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// 空かどうか
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// 正規化されたJSONをハッシュ化
    pub fn hash_json<T: serde::Serialize>(&mut self, value: &T) -> Result<ContentHash> {
        let json = serde_json::to_string(value)
            .map_err(|e| KotobaError::Storage(format!("JSON serialization error: {}", e)))?;

        // 正規化（キーをソート）
        let normalized: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| KotobaError::Storage(format!("JSON parse error: {}", e)))?;

        let normalized_json = serde_json::to_string(&normalized)
            .map_err(|e| KotobaError::Storage(format!("JSON serialization error: {}", e)))?;

        Ok(self.store(normalized_json.as_bytes(), Vec::new()))
    }

    /// グラフのMerkleハッシュを計算
    pub fn hash_graph(&mut self, graph: &crate::graph::Graph) -> Result<ContentHash> {
        let mut children = Vec::new();

        // 頂点をハッシュ化
        for vertex in graph.vertices.values() {
            let hash = self.hash_json(vertex)?;
            children.push(hash);
        }

        // エッジをハッシュ化
        for edge in graph.edges.values() {
            let hash = self.hash_json(edge)?;
            children.push(hash);
        }

        // グラフ全体のハッシュ
        let graph_data = format!("graph:v{}:e{}",
            graph.vertex_count(),
            graph.edge_count()
        );

        Ok(self.store(graph_data.as_bytes(), children))
    }
}

/// グラフのバージョン管理
#[derive(Debug)]
pub struct GraphVersion {
    dag: MerkleDAG,
    current_hash: Option<ContentHash>,
    history: Vec<(u64, ContentHash)>,  // (timestamp, hash)
}

impl GraphVersion {
    pub fn new() -> Self {
        Self {
            dag: MerkleDAG::new(),
            current_hash: None,
            history: Vec::new(),
        }
    }

    /// 新しいバージョンをコミット
    pub fn commit(&mut self, graph: &crate::graph::Graph) -> Result<ContentHash> {
        let hash = self.dag.hash_graph(graph)?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        self.history.push((timestamp, hash.clone()));
        self.current_hash = Some(hash.clone());

        Ok(hash)
    }

    /// 指定したハッシュのグラフを取得
    pub fn get(&self, hash: &ContentHash) -> Option<&MerkleNode> {
        self.dag.get(hash)
    }

    /// 現在のハッシュを取得
    pub fn current_hash(&self) -> Option<&ContentHash> {
        self.current_hash.as_ref()
    }

    /// バージョン履歴を取得
    pub fn history(&self) -> &[(u64, ContentHash)] {
        &self.history
    }
}

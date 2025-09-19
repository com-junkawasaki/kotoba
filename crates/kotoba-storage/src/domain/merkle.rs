//! Merkle DAG（コンテンツアドレッサブルストレージ）

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use anyhow::{anyhow, Error};
use kotoba_graph::graph::Graph;

/// Merkleノード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    pub hash: String,
    pub data: Vec<u8>,
    pub children: Vec<String>,
    pub timestamp: u64,
}

/// Merkle DAG
#[derive(Debug)]
pub struct MerkleDAG {
    nodes: HashMap<String, MerkleNode>,
}

/// Merkleツリー比較結果
#[derive(Debug, Clone)]
pub struct TreeComparison {
    /// 完全に同一かどうか
    pub identical: bool,
    /// 内容が異なるハッシュ
    pub differences: Vec<String>,
    /// selfにのみ存在するハッシュ
    pub self_only: Vec<String>,
    /// otherにのみ存在するハッシュ
    pub other_only: Vec<String>,
}

impl MerkleDAG {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
        }
    }

    /// データを格納してMerkleハッシュを生成
    pub fn store(&mut self, data: &[u8], children: Vec<String>) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        for child in &children {
            hasher.update(child.as_bytes());
        }
        let hash = format!("{:x}", hasher.finalize());

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
    pub fn get(&self, hash: &str) -> Option<&MerkleNode> {
        self.nodes.get(hash)
    }

    /// ハッシュが存在するかチェック
    pub fn contains(&self, hash: &str) -> bool {
        self.nodes.contains_key(hash)
    }

    /// 子のハッシュを取得
    pub fn get_children(&self, hash: &str) -> Option<&[String]> {
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

    /// データ整合性を検証
    pub fn verify_integrity(&self) -> Result<bool, Error> {
        for (hash, node) in &self.nodes {
            // ハッシュの正しさを検証
            let mut hasher = sha2::Sha256::new();
            hasher.update(&node.data);
            for child in &node.children {
                hasher.update(child.as_bytes());
            }

            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != *hash {
                return Ok(false);
            }

            // 子ノードの存在を確認
            for child_hash in &node.children {
                if !self.nodes.contains_key(child_hash.as_str()) {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Merkleルートを計算
    pub fn compute_root(&mut self) -> String {
        if self.nodes.is_empty() {
            return "empty".to_string();
        }

        // Collect leaf nodes (nodes with no children)
        let mut leaves: Vec<&MerkleNode> = self.nodes.values()
            .filter(|node| node.children.is_empty())
            .collect();

        // Sort leaves for consistent hash calculation
        leaves.sort_by_key(|node| &node.hash);

        if leaves.is_empty() {
            // This can happen if all nodes are interconnected and there are no leaves
            // As a fallback, use all nodes as the base.
            leaves = self.nodes.values().collect();
            leaves.sort_by_key(|node| &node.hash);
        }

        // Build the tree level by level until one root is left
        while leaves.len() > 1 {
            let mut new_nodes_for_level = Vec::new();
            for chunk in leaves.chunks(2) {
                let mut hasher = sha2::Sha256::new();
                hasher.update(chunk[0].hash.as_bytes());

                let children = if chunk.len() > 1 {
                    hasher.update(chunk[1].hash.as_bytes());
                    vec![chunk[0].hash.clone(), chunk[1].hash.clone()]
                } else {
                    // If there's an odd number of nodes, hash the last one with itself
                    hasher.update(chunk[0].hash.as_bytes());
                    vec![chunk[0].hash.clone(), chunk[0].hash.clone()]
                };

                let combined_hash = format!("{:x}", hasher.finalize());
                
                let new_node = MerkleNode {
                    hash: combined_hash,
                    data: Vec::new(),
                    children,
                    timestamp: 0,
                };
                new_nodes_for_level.push(new_node);
            }

            // Get keys before inserting to avoid borrow checker issues
            let new_hashes: Vec<String> = new_nodes_for_level.iter().map(|n| n.hash.clone()).collect();

            // Add the new level of nodes to the main nodes map
            for node in new_nodes_for_level {
                self.nodes.insert(node.hash.clone(), node);
            }

            // The next level of leaves are the nodes we just created
            leaves = new_hashes.iter().map(|hash| self.nodes.get(hash).unwrap()).collect();
        }

        leaves[0].hash.clone()
    }

    /// サブツリーのMerkleルートを計算
    pub fn compute_subtree_root(&self, root_hash: &str) -> Result<String, Error> {
        if !self.nodes.contains_key(root_hash) {
            return Err(anyhow!("Root hash not found".to_string()));
        }

        // DFSでサブツリーを走査してMerkleルートを計算
        self.compute_merkle_root_recursive(root_hash)
    }

    /// 再帰的にMerkleルートを計算
    fn compute_merkle_root_recursive(&self, hash: &str) -> Result<String, Error> {
        let node = self.nodes.get(hash)
            .ok_or_else(|| anyhow!("Node not found".to_string()))?;

        if node.children.is_empty() {
            // 葉ノード
            return Ok(hash.to_string());
        }

        // 子ノードのハッシュを集める
        let mut child_hashes = Vec::new();
        for child_hash in &node.children {
            let child_root = self.compute_merkle_root_recursive(child_hash)?;
            child_hashes.push(child_root.clone());
        }

        // 子ノードのハッシュを組み合わせて新しいハッシュを計算
        let mut hasher = sha2::Sha256::new();
        hasher.update(&node.data);
        for child_hash in child_hashes {
            hasher.update(child_hash.as_bytes());
        }

        Ok(format!("{:x}", hasher.finalize()))
    }

    /// 2つのMerkleツリーを比較
    pub fn compare_trees(&self, other: &MerkleDAG) -> TreeComparison {
        let mut differences = Vec::new();
        let mut self_only = Vec::new();
        let mut other_only = Vec::new();

        // 共通のハッシュを比較
        for (hash, self_node) in &self.nodes {
            if let Some(other_node) = other.nodes.get(hash) {
                if self_node.data != other_node.data ||
                   self_node.children != other_node.children {
                    differences.push(hash.clone());
                }
            } else {
                self_only.push(hash.clone());
            }
        }

        // otherにしかないハッシュを収集
        for hash in other.nodes.keys() {
            if !self.nodes.contains_key(hash) {
                other_only.push(hash.clone());
            }
        }

        TreeComparison {
            identical: differences.is_empty() && self_only.is_empty() && other_only.is_empty(),
            differences,
            self_only,
            other_only,
        }
    }

    /// 欠損データを特定
    pub fn find_missing_data(&self, required_hashes: &[String]) -> Vec<String> {
        required_hashes.iter()
            .filter(|hash| !self.nodes.contains_key(hash.as_str()))
            .cloned()
            .collect()
    }

    /// 破損したデータを検出
    pub fn detect_corruption(&self) -> Result<Vec<String>, Error> {
        let mut corrupted = Vec::new();

        for (hash, node) in &self.nodes {
            let mut hasher = sha2::Sha256::new();
            hasher.update(&node.data);
            for child in &node.children {
                hasher.update(child.as_bytes());
            }

            let computed_hash = format!("{:x}", hasher.finalize());
            if computed_hash != *hash {
                corrupted.push(hash.clone());
            }
        }

        Ok(corrupted)
    }

    /// ノードを挿入
    pub fn insert_node(&mut self, hash: String, node: MerkleNode) {
        self.nodes.insert(hash, node);
    }

    /// ノードを取得
    pub fn get_node(&self, hash: &str) -> Option<&MerkleNode> {
        self.nodes.get(hash)
    }

    /// 全ノードを取得（読み取り専用）
    pub fn nodes(&self) -> &HashMap<String, MerkleNode> {
        &self.nodes
    }

    /// 全ノードを置き換え
    pub fn set_nodes(&mut self, nodes: HashMap<String, MerkleNode>) {
        self.nodes = nodes;
    }

    /// 正規化されたJSONをハッシュ化
    pub fn hash_json<T: serde::Serialize>(&mut self, value: &T) -> Result<String, Error> {
        let json = serde_json::to_string(value)
            .map_err(|e| anyhow!(format!("JSON serialization error: {}", e)))?;

        // 正規化（キーをソート）
        let normalized: serde_json::Value = serde_json::from_str(&json)
            .map_err(|e| anyhow!(format!("JSON parse error: {}", e)))?;

        let normalized_json = serde_json::to_string(&normalized)
            .map_err(|e| anyhow!(format!("JSON serialization error: {}", e)))?;

        Ok(self.store(normalized_json.as_bytes(), Vec::new()))
    }

    /// グラフのMerkleハッシュを計算
    pub fn hash_graph(&mut self, graph: &Graph) -> Result<String, Error> {
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
    current_hash: Option<String>,
    history: Vec<(u64, String)>,  // (timestamp, hash)
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
    pub fn commit(&mut self, graph: &Graph) -> Result<String, Error> {
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
    pub fn get(&self, hash: &str) -> Option<&MerkleNode> {
        self.dag.get(hash)
    }

    /// 現在のハッシュを取得
    pub fn current_hash(&self) -> Option<&String> {
        self.current_hash.as_ref()
    }

    /// バージョン履歴を取得
    pub fn history(&self) -> &[(u64, String)] {
        &self.history
    }
}

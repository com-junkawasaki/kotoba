//! Merkleツリーの実装

use super::*;
use crate::{KotobaResult, KotobaError};

impl MerkleTreeBuilder {
    /// 新しいMerkleツリー構築器を作成
    pub fn new() -> Self {
        Self {
            nodes: Vec::new(),
        }
    }

    /// リーフノードを追加
    pub fn add_leaf(&mut self, data: Vec<u8>) -> String {
        let id = format!("leaf_{}", self.nodes.len());
        let hash = self.compute_hash(&data);

        let node = MerkleNode {
            id: id.clone(),
            hash,
            children: vec![],
            data: Some(data),
        };

        self.nodes.push(node);
        id
    }

    /// 中間ノードを作成
    pub fn create_intermediate(&mut self, left_id: &str, right_id: &str) -> KotobaResult<String> {
        let left_node = self.find_node(left_id)?;
        let right_node = self.find_node(right_id)?;

        let mut combined_data = left_node.hash.clone();
        combined_data.extend_from_slice(&right_node.hash);

        let id = format!("intermediate_{}", self.nodes.len());
        let hash = self.compute_hash(&combined_data);

        let node = MerkleNode {
            id: id.clone(),
            hash,
            children: vec![left_id.to_string(), right_id.to_string()],
            data: None,
        };

        self.nodes.push(node);
        Ok(id)
    }

    /// ルートノードを取得
    pub fn get_root(&self) -> Option<&MerkleNode> {
        // 最も新しいノードをルートとして扱う（簡易版）
        self.nodes.last()
    }

    /// ノードを検索
    pub fn find_node(&self, id: &str) -> KotobaResult<&MerkleNode> {
        self.nodes.iter()
            .find(|node| node.id == id)
            .ok_or_else(|| KotobaError::NotFound(format!("Node {} not found", id)))
    }

    /// ノードをIDで検索（可変参照）
    pub fn find_node_mut(&mut self, id: &str) -> KotobaResult<&mut MerkleNode> {
        self.nodes.iter_mut()
            .find(|node| node.id == id)
            .ok_or_else(|| KotobaError::NotFound(format!("Node {} not found", id)))
    }

    /// Merkleプルーフを生成
    pub fn generate_proof(&self, leaf_id: &str) -> KotobaResult<Vec<MerkleNode>> {
        let mut proof = Vec::new();
        let mut current_id = leaf_id.to_string();

        while let Some(parent_id) = self.find_parent(&current_id) {
            let parent = self.find_node(&parent_id)?;
            proof.push(parent.clone());

            // 兄弟ノードを特定
            if let Some(sibling) = self.find_sibling(&current_id, &parent_id) {
                proof.push(self.find_node(&sibling)?.clone());
            }

            current_id = parent_id;
        }

        Ok(proof)
    }

    /// Merkleプルーフを検証
    pub fn verify_proof(&self, leaf_data: &[u8], proof: &[MerkleNode], root_hash: &[u8]) -> bool {
        let mut current_hash = self.compute_hash(leaf_data);

        for node in proof {
            let mut combined = current_hash.clone();
            combined.extend_from_slice(&node.hash);
            current_hash = self.compute_hash(&combined);
        }

        current_hash == root_hash
    }

    /// 親ノードを検索
    fn find_parent(&self, child_id: &str) -> Option<String> {
        for node in &self.nodes {
            if node.children.contains(&child_id.to_string()) {
                return Some(node.id.clone());
            }
        }
        None
    }

    /// 兄弟ノードを検索
    fn find_sibling(&self, node_id: &str, parent_id: &str) -> Option<String> {
        let parent = self.find_node(parent_id).ok()?;
        let position = parent.children.iter().position(|id| id == node_id)?;

        if position == 0 && parent.children.len() > 1 {
            Some(parent.children[1].clone())
        } else if position == 1 {
            Some(parent.children[0].clone())
        } else {
            None
        }
    }

    /// ハッシュを計算（内部用）
    fn compute_hash(&self, data: &[u8]) -> Vec<u8> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hasher.finalize().to_vec()
    }

    /// ツリーの深さを取得
    pub fn depth(&self) -> usize {
        if self.nodes.is_empty() {
            return 0;
        }

        let mut max_depth = 0;
        for node in &self.nodes {
            let depth = self.calculate_depth(&node.id);
            if depth > max_depth {
                max_depth = depth;
            }
        }
        max_depth
    }

    /// ノードの深さを計算
    fn calculate_depth(&self, node_id: &str) -> usize {
        let mut depth = 0;
        let mut current_id = node_id.to_string();

        while let Some(parent_id) = self.find_parent(&current_id) {
            depth += 1;
            current_id = parent_id;
        }

        depth
    }

    /// ツリーのノード数をカウント
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// リーフノードのみをカウント
    pub fn leaf_count(&self) -> usize {
        self.nodes.iter()
            .filter(|node| node.children.is_empty())
            .count()
    }
}

impl MerkleNode {
    /// リーフノードを作成
    pub fn new_leaf(id: String, data: Vec<u8>) -> Self {
        let hash = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&data);
            hasher.finalize().to_vec()
        };

        Self {
            id,
            hash,
            children: vec![],
            data: Some(data),
        }
    }

    /// 中間ノードを作成
    pub fn new_intermediate(id: String, left: &MerkleNode, right: &MerkleNode) -> Self {
        let mut combined = left.hash.clone();
        combined.extend_from_slice(&right.hash);

        let hash = {
            use sha2::{Sha256, Digest};
            let mut hasher = Sha256::new();
            hasher.update(&combined);
            hasher.finalize().to_vec()
        };

        Self {
            id,
            hash,
            children: vec![left.id.clone(), right.id.clone()],
            data: None,
        }
    }

    /// リーフノードかどうかをチェック
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// 中間ノードかどうかをチェック
    pub fn is_intermediate(&self) -> bool {
        !self.children.is_empty() && self.data.is_none()
    }

    /// ハッシュを16進数文字列として取得
    pub fn hash_hex(&self) -> String {
        hex::encode(&self.hash)
    }
}

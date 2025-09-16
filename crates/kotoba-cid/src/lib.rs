//! CID (Content ID) システムの実装
//! Merkle DAGにおけるコンテンツアドレッシング

use kotoba_core::types::*;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;

/// ハッシュアルゴリズム
#[derive(Debug, Clone, PartialEq)]
pub enum HashAlgorithm {
    /// SHA-256
    Sha2256,
    /// BLAKE3
    Blake3,
}

/// JSON正規化モード
#[derive(Debug, Clone, PartialEq)]
pub enum CanonicalJsonMode {
    /// JCS (RFC 8785)
    JCS,
}

/// CID計算器
#[derive(Debug)]
pub struct CidCalculator {
    hash_algo: HashAlgorithm,
    canonical_json: CanonicalJsonMode,
}

/// CIDマネージャー
#[derive(Debug)]
pub struct CidManager {
    calculator: CidCalculator,
    cache: HashMap<String, Cid>,
}

/// Merkleツリー構築器
#[derive(Debug)]
pub struct MerkleTreeBuilder {
    nodes: Vec<MerkleNode>,
}

/// Merkleノード
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleNode {
    /// ノードID
    pub id: String,
    /// ハッシュ値
    pub hash: Vec<u8>,
    /// 子ノード
    pub children: Vec<String>,
    /// データ
    pub data: Option<Vec<u8>>,
}

// 実装は別ファイルに分離
mod calculator;
mod manager;
mod merkle;
mod canonical_json;

// 再エクスポート
pub use calculator::*;
pub use manager::*;
pub use merkle::*;
pub use canonical_json::*;

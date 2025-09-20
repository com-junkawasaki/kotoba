//! CID (Content ID) システムの実装
//! Merkle DAGにおけるコンテンツアドレッシング

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

/// Content ID (CID) - Merkle DAGにおけるコンテンツアドレッシング
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Cid(pub [u8; 32]);

impl Cid {
    /// SHA256ハッシュからCIDを作成
    pub fn from_sha256(hash: [u8; 32]) -> Self {
        Self(hash)
    }

    /// データをSHA256でハッシュしてCIDを計算
    pub fn compute_sha256<T: Serialize>(data: &T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_string(data)?;
        let hash = Sha256::digest(json.as_bytes());
        Ok(Self(hash.into()))
    }

    /// CIDを16進数文字列に変換
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// 16進数文字列からCIDを作成
    pub fn from_hex(hex_str: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(hex_str)?;
        if bytes.len() != 32 {
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut array = [0u8; 32];
        array.copy_from_slice(&bytes);
        Ok(Self(array))
    }

    /// CIDを文字列として取得
    pub fn as_str(&self) -> String {
        self.to_hex()
    }
}

impl AsRef<[u8]> for Cid {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl From<[u8; 32]> for Cid {
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<Cid> for [u8; 32] {
    fn from(cid: Cid) -> Self {
        cid.0
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_cid_calculator_creation() {
        let calculator = CidCalculator::new(HashAlgorithm::Sha2256, CanonicalJsonMode::JCS);
        assert_eq!(calculator.hash_algo, HashAlgorithm::Sha2256);
        assert_eq!(calculator.canonical_json, CanonicalJsonMode::JCS);
    }

    #[test]
    fn test_cid_calculator_default() {
        let calculator = CidCalculator::default();
        assert_eq!(calculator.hash_algo, HashAlgorithm::Sha2256);
        assert_eq!(calculator.canonical_json, CanonicalJsonMode::JCS);
    }

    #[test]
    fn test_cid_computation() {
        let calculator = CidCalculator::default();
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        let cid = calculator.compute_cid(&data).unwrap();
        assert_eq!(cid.0.len(), 32);

        // Same data should produce same CID
        let cid2 = calculator.compute_cid(&data).unwrap();
        assert_eq!(cid, cid2);
    }

    #[test]
    fn test_cid_verification() {
        let calculator = CidCalculator::default();
        let data = TestData {
            name: "verify".to_string(),
            value: 100,
        };

        let cid = calculator.compute_cid(&data).unwrap();
        let is_valid = calculator.verify_cid(&data, &cid).unwrap();
        assert!(is_valid);

        let different_data = TestData {
            name: "different".to_string(),
            value: 200,
        };
        let is_invalid = calculator.verify_cid(&different_data, &cid).unwrap();
        assert!(!is_invalid);
    }

    #[test]
    fn test_combined_cid() {
        let calculator = CidCalculator::default();
        let data1 = b"hello";
        let data2 = b"world";
        let data_list = vec![data1, data2];

        let cid = calculator.compute_combined_cid(&data_list).unwrap();
        assert_eq!(cid.0.len(), 32);

        // Different order should produce different CID
        let data_list_rev = vec![data2, data1];
        let cid_rev = calculator.compute_combined_cid(&data_list_rev).unwrap();
        assert_ne!(cid, cid_rev);
    }

    #[test]
    fn test_cid_manager_creation() {
        let manager = CidManager::new();
        assert_eq!(manager.cache_size(), 0);
    }

    #[test]
    fn test_cid_manager_with_calculator() {
        let calculator = CidCalculator::new(HashAlgorithm::Blake3, CanonicalJsonMode::JCS);
        let manager = CidManager::with_calculator(calculator);
        assert_eq!(manager.calculator().hash_algo, HashAlgorithm::Blake3);
    }

    #[test]
    fn test_cid_manager_caching() {
        let mut manager = CidManager::new();
        let data = TestData {
            name: "cached".to_string(),
            value: 1,
        };

        let cid = manager.calculator.compute_cid(&data).unwrap();
        let key = format!("test_{}", cid.to_hex());
        manager.cache.insert(key.clone(), cid.clone());

        let cached_cid = manager.get_cached_cid(&key);
        assert_eq!(cached_cid, Some(&cid));
    }

    #[test]
    fn test_cid_distance() {
        let manager = CidManager::new();
        let cid1 = Cid([0; 32]);
        let cid2 = Cid([1; 32]);

        let distance = manager.cid_distance(&cid1, &cid2);
        assert!(distance.is_some());
        assert!(distance.unwrap() > 0);
    }

    #[test]
    fn test_merkle_tree_builder() {
        let mut builder = MerkleTreeBuilder::new();
        assert_eq!(builder.node_count(), 0);

        let leaf1 = builder.add_leaf(b"data1".to_vec());
        let leaf2 = builder.add_leaf(b"data2".to_vec());

        assert_eq!(builder.node_count(), 2);
        assert_eq!(builder.leaf_count(), 2);

        let intermediate = builder.create_intermediate(&leaf1, &leaf2).unwrap();
        assert_eq!(builder.node_count(), 3);

        let root = builder.get_root().unwrap();
        assert_eq!(root.id, intermediate);
    }

    #[test]
    fn test_merkle_node_creation() {
        let node = MerkleNode::new_leaf("test_leaf".to_string(), b"test data".to_vec());
        assert!(node.is_leaf());
        assert!(!node.is_intermediate());
        assert_eq!(node.id, "test_leaf");
        assert!(node.data.is_some());
        assert_eq!(node.children.len(), 0);
    }

    #[test]
    fn test_merkle_intermediate_node() {
        let leaf1 = MerkleNode::new_leaf("leaf1".to_string(), b"data1".to_vec());
        let leaf2 = MerkleNode::new_leaf("leaf2".to_string(), b"data2".to_vec());
        let intermediate = MerkleNode::new_intermediate("intermediate".to_string(), &leaf1, &leaf2);

        assert!(!intermediate.is_leaf());
        assert!(intermediate.is_intermediate());
        assert_eq!(intermediate.children.len(), 2);
        assert!(intermediate.data.is_none());
    }

    #[test]
    fn test_merkle_proof() {
        let mut builder = MerkleTreeBuilder::new();

        let leaf1 = builder.add_leaf(b"data1".to_vec());
        let leaf2 = builder.add_leaf(b"data2".to_vec());
        let _intermediate = builder.create_intermediate(&leaf1, &leaf2).unwrap();

        let proof = builder.generate_proof(&leaf1).unwrap();
        assert!(!proof.is_empty());

        let root = builder.get_root().unwrap();
        let is_valid = builder.verify_proof(b"data1", &proof, &root.hash);
        assert!(is_valid);
    }

    #[test]
    fn test_merkle_tree_depth() {
        let mut builder = MerkleTreeBuilder::new();

        // Empty tree
        assert_eq!(builder.depth(), 0);

        let leaf = builder.add_leaf(b"data".to_vec());
        assert_eq!(builder.depth(), 1);

        let leaf2 = builder.add_leaf(b"data2".to_vec());
        let _intermediate = builder.create_intermediate(&leaf, &leaf2).unwrap();
        assert_eq!(builder.depth(), 2);
    }

    #[test]
    fn test_hash_algorithms() {
        let sha_calculator = CidCalculator::new(HashAlgorithm::Sha2256, CanonicalJsonMode::JCS);
        let blake_calculator = CidCalculator::new(HashAlgorithm::Blake3, CanonicalJsonMode::JCS);

        let data = TestData {
            name: "hash_test".to_string(),
            value: 123,
        };

        let sha_cid = sha_calculator.compute_cid(&data).unwrap();
        let blake_cid = blake_calculator.compute_cid(&data).unwrap();

        // Different algorithms should produce different CIDs
        assert_ne!(sha_cid, blake_cid);
        assert_eq!(sha_cid.0.len(), 32);
        assert_eq!(blake_cid.0.len(), 32);
    }

    #[test]
    fn test_cid_hex_conversion() {
        let bytes = [42; 32];
        let cid = Cid(bytes);

        let hex_str = cid.to_hex();
        let reconstructed_cid = Cid::from_hex(&hex_str).unwrap();

        assert_eq!(cid, reconstructed_cid);
        assert_eq!(hex_str.len(), 64); // 32 bytes * 2 hex chars per byte
    }

    #[test]
    fn test_cid_as_str() {
        let cid = Cid([255; 32]);
        let hex_str = cid.as_str();
        assert_eq!(hex_str, cid.to_hex());
    }

    #[test]
    fn test_json_canonicalizer() {
        let canonicalizer = JsonCanonicalizer::new(CanonicalJsonMode::JCS);

        let json = r#"{"c":3,"a":1,"b":2}"#;
        let canonical = canonicalizer.canonicalize(json).unwrap();
        let expected = r#"{"a":1,"b":2,"c":3}"#;
        assert_eq!(canonical, expected);
    }

    #[test]
    fn test_json_canonical_size() {
        let canonicalizer = JsonCanonicalizer::new(CanonicalJsonMode::JCS);

        let json = r#"  {  "a"  :  1  ,  "b"  :  2  }  "#;
        let size = canonicalizer.canonical_size(json).unwrap();
        let canonical = canonicalizer.canonicalize(json).unwrap();
        assert_eq!(size, canonical.len());
    }

    // Helper struct for testing
    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestData {
        name: String,
        value: i32,
    }
}

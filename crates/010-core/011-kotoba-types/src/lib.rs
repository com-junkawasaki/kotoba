//! # Kotoba Types
//!
//! Core type definitions for the Kotoba ecosystem, including CID systems,
//! Merkle DAG structures, and graph-related types.

use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::collections::HashMap;
use kotoba_types::Cid;

// Re-export from kotoba-types crate
pub use kotoba_types::*;

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

// Use Cid from kotoba_types instead of defining our own
// Cid implementation moved to kotoba_types

// Cid implementations moved to kotoba_types

// Cid implementations moved to kotoba_types

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

        let cid = calculator.compute_combined_cid(&data_list.iter().map(|&x| x as &[u8]).collect::<Vec<_>>()).unwrap();
        assert_eq!(cid.0.len(), 32);

        // Different order should produce different CID
        let data_list_rev = vec![data2, data1];
        let cid_rev = calculator.compute_combined_cid(&data_list_rev.iter().map(|&x| x as &[u8]).collect::<Vec<_>>()).unwrap();
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
        let key = format!("test_{}", cid.as_str());
        manager.cache.insert(key.clone(), cid.clone());

        let cached_cid = manager.get_cached_cid(&key);
        assert_eq!(cached_cid, Some(&cid));
    }

    #[test]
    fn test_cid_distance() {
        let manager = CidManager::new();
        let cid1 = kotoba_types::Cid::new(&[0; 32]);
        let cid2 = kotoba_types::Cid::new(&[1; 32]);

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
        let cid = kotoba_types::Cid::new(&bytes);

        let hex_str = cid.as_str();
        let reconstructed_cid = kotoba_types::Cid::new(hex_str.as_bytes());

        assert_eq!(cid, reconstructed_cid);
        assert_eq!(hex_str.len(), 64); // 32 bytes * 2 hex chars per byte
    }

    #[test]
    fn test_cid_as_str() {
        let cid = kotoba_types::Cid::new(&[255; 32]);
        let hex_str = cid.as_str();
        assert_eq!(hex_str, cid.as_str());
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

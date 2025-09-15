//! kotoba-storage - Kotoba Storage Components

pub mod storage;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::storage::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_core::types::*;
    use std::collections::HashMap;

    #[test]
    fn test_transaction_creation() {
        // Test Transaction creation
        let tx_id = TxId("test_tx".to_string());
        let tx = Transaction::new(tx_id.clone());

        assert_eq!(tx.id, tx_id);
        assert_eq!(tx.state, TxState::Active);
        assert!(!tx.writes.is_empty()); // Should have some initial state
        assert!(tx.start_time > 0);
    }

    #[test]
    fn test_transaction_commit() {
        // Test transaction commit
        let tx_id = TxId("test_tx".to_string());
        let tx = Transaction::new(tx_id.clone());
        let committed_tx = tx.commit();

        assert_eq!(committed_tx.state, TxState::Committed);
        assert_eq!(committed_tx.id, tx_id);
    }

    #[test]
    fn test_transaction_abort() {
        // Test transaction abort
        let tx_id = TxId("test_tx".to_string());
        let tx = Transaction::new(tx_id.clone());
        let aborted_tx = tx.abort();

        assert_eq!(aborted_tx.state, TxState::Aborted);
        assert_eq!(aborted_tx.id, tx_id);
    }

    #[test]
    fn test_tx_state_serialization() {
        // Test TxState serialization
        let active = TxState::Active;
        let json = serde_json::to_string(&active).unwrap();
        assert_eq!(json, "\"Active\"");

        let committed = TxState::Committed;
        let json = serde_json::to_string(&committed).unwrap();
        assert_eq!(json, "\"Committed\"");

        let aborted = TxState::Aborted;
        let json = serde_json::to_string(&aborted).unwrap();
        assert_eq!(json, "\"Aborted\"");
    }

    #[test]
    fn test_mvcc_manager_creation() {
        // Test MVCCManager creation
        let manager = MVCCManager::new();
        // Just check that it can be created without panicking
        assert!(true);
    }

    #[test]
    fn test_merkle_tree_creation() {
        // Test MerkleTree creation
        let tree = MerkleTree::new();
        assert_eq!(tree.root_hash().len(), 64); // SHA-256 hash length
    }

    #[test]
    fn test_lsm_tree_creation() {
        // Test LSMTree creation
        let tree = LSMTree::new();
        // Just check that it can be created
        assert!(true);
    }

    #[test]
    fn test_content_hash_consistency() {
        // Test that ContentHash is consistent
        let data = vec![1, 2, 3, 4, 5];
        let hash1 = ContentHash::sha256(&data);
        let hash2 = ContentHash::sha256(&data);
        assert_eq!(hash1, hash2);

        let different_data = vec![5, 4, 3, 2, 1];
        let hash3 = ContentHash::sha256(&different_data);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_storage_key_generation() {
        // Test StorageKey generation
        let vertex_id = VertexId::new_v4();
        let key = StorageKey::vertex(vertex_id);
        assert!(key.0.starts_with("vertex:"));

        let edge_id = EdgeId::new_v4();
        let key = StorageKey::edge(edge_id);
        assert!(key.0.starts_with("edge:"));

        let tx_id = TxId("test".to_string());
        let key = StorageKey::transaction(tx_id);
        assert!(key.0.starts_with("tx:"));
    }

    #[test]
    fn test_storage_value_serialization() {
        // Test StorageValue serialization
        let vertex_data = VertexData {
            id: VertexId::new_v4(),
            labels: vec!["Test".to_string()],
            props: {
                let mut props = HashMap::new();
                props.insert("name".to_string(), Value::String("test".to_string()));
                props
            },
        };

        let storage_value = StorageValue::Vertex(vertex_data.clone());
        let json = serde_json::to_string(&storage_value).unwrap();
        let deserialized: StorageValue = serde_json::from_str(&json).unwrap();

        match deserialized {
            StorageValue::Vertex(deserialized_vertex) => {
                assert_eq!(deserialized_vertex.labels, vertex_data.labels);
            }
            _ => panic!("Expected Vertex variant"),
        }
    }
}

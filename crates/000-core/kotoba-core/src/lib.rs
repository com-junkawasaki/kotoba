//! kotoba-core - Kotoba Core Components

pub mod types;
pub mod schema;
pub mod schema_validator;
// pub mod pgview; // Temporarily disabled due to Value type conflicts
pub mod ir;
pub mod topology;
pub mod graph;
pub mod auth;  // 認証・認可エンジン
pub mod crypto; // 暗号化エンジン
pub mod prelude {
    // Re-export commonly used items
    pub use crate::types::*;
    pub use crate::schema::*;
    // Re-export specific items from schema_validator to avoid utils conflict
    pub use crate::schema_validator::{SchemaValidator, ValidationReport};
    // pub use crate::pgview::*; // Temporarily disabled
    pub use crate::ir::*;
    pub use crate::auth::*;  // 認証・認可エンジン
    // Re-export specific items from crypto to avoid utils conflict
    pub use crate::crypto::EncryptionInfo;
    // Re-export KotobaError to avoid version conflicts
    pub use kotoba_errors::KotobaError;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;
    use std::collections::HashMap;

    #[test]
    fn test_value_serialization() {
        // Test Value enum serialization
        let null_val = Value::Null;
        let json = serde_json::to_string(&null_val).unwrap();
        assert_eq!(json, "null");

        let bool_val = Value::Bool(true);
        let json = serde_json::to_string(&bool_val).unwrap();
        assert_eq!(json, "true");

        let int_val = Value::Int(42);
        let json = serde_json::to_string(&int_val).unwrap();
        assert_eq!(json, "42");

        let str_val = Value::String("hello".to_string());
        let json = serde_json::to_string(&str_val).unwrap();
        assert_eq!(json, "\"hello\"");

        let array_val = Value::Array(vec!["a".to_string(), "b".to_string()]);
        let json = serde_json::to_string(&array_val).unwrap();
        assert_eq!(json, "[\"a\",\"b\"]");

        // Test Integer variant (compatibility)
        let integer_val = Value::Integer(123);
        let json = serde_json::to_string(&integer_val).unwrap();
        assert_eq!(json, "123");
    }

    #[test]
    fn test_value_deserialization() {
        // Test Value enum deserialization
        let null_val: Value = serde_json::from_str("null").unwrap();
        assert_eq!(null_val, Value::Null);

        let bool_val: Value = serde_json::from_str("true").unwrap();
        assert_eq!(bool_val, Value::Bool(true));

        let int_val: Value = serde_json::from_str("42").unwrap();
        assert_eq!(int_val, Value::Int(42));

        let str_val: Value = serde_json::from_str("\"hello\"").unwrap();
        assert_eq!(str_val, Value::String("hello".to_string()));

        let array_val: Value = serde_json::from_str("[\"a\",\"b\"]").unwrap();
        assert_eq!(array_val, Value::Array(vec!["a".to_string(), "b".to_string()]));
    }

    #[test]
    fn test_value_equality_and_hash() {
        // Test equality
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_eq!(Value::Int(42), Value::Int(42));
        assert_eq!(Value::String("test".to_string()), Value::String("test".to_string()));
        assert_eq!(Value::Integer(42), Value::Integer(42));

        // Test inequality
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_ne!(Value::Int(42), Value::Int(43));
        assert_ne!(Value::String("test".to_string()), Value::String("other".to_string()));

        // Test that Int and Integer are considered different (for now)
        assert_ne!(Value::Int(42), Value::Integer(42));

        // Test Hash implementation works
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Value::Null);
        set.insert(Value::Bool(true));
        set.insert(Value::String("test".to_string()));
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn test_content_hash() {
        // Test ContentHash generation
        let data = [42u8; 32];
        let hash = ContentHash::sha256(data);
        assert!(!hash.0.is_empty());
        assert!(hash.0.len() == 64); // SHA256 produces 32 bytes = 64 hex chars

        // Test that same data produces same hash
        let hash2 = ContentHash::sha256(data);
        assert_eq!(hash, hash2);

        // Test that different data produces different hash
        let data2 = [43u8; 32];
        let hash3 = ContentHash::sha256(data2);
        assert_ne!(hash, hash3);
    }

    #[test]
    fn test_uuid_types() {
        // Test that VertexId and EdgeId are UUIDs
        let vertex_id = VertexId::new_v4();
        let edge_id = EdgeId::new_v4();

        assert_ne!(vertex_id, edge_id); // Should be different UUIDs
        assert_eq!(vertex_id.get_version_num(), 4); // Should be v4 UUIDs
        assert_eq!(edge_id.get_version_num(), 4);

        // Test UUID string representation
        let id_str = vertex_id.to_string();
        assert_eq!(id_str.len(), 36); // UUID string length
        assert!(id_str.contains('-')); // Should contain hyphens
    }

    #[test]
    fn test_graph_ref() {
        // Test GraphRef_ creation and operations
        let ref1 = GraphRef_("test_hash".to_string());
        let ref2 = GraphRef_("test_hash".to_string());
        let ref3 = GraphRef_("different_hash".to_string());

        assert_eq!(ref1, ref2);
        assert_ne!(ref1, ref3);

        // Test serialization
        let json = serde_json::to_string(&ref1).unwrap();
        assert_eq!(json, "\"test_hash\"");

        // Test deserialization
        let deserialized: GraphRef_ = serde_json::from_str("\"test_hash\"").unwrap();
        assert_eq!(deserialized, ref1);
    }

    #[test]
    fn test_tx_id() {
        // Test TxId creation and operations
        let tx1 = TxId("tx_123".to_string());
        let tx2 = TxId("tx_123".to_string());
        let tx3 = TxId("tx_456".to_string());

        assert_eq!(tx1, tx2);
        assert_ne!(tx1, tx3);

        // Test ordering
        assert!(tx1 < tx3);
        assert!(tx3 > tx1);

        // Test serialization
        let json = serde_json::to_string(&tx1).unwrap();
        assert_eq!(json, "\"tx_123\"");

        // Test deserialization
        let deserialized: TxId = serde_json::from_str("\"tx_123\"").unwrap();
        assert_eq!(deserialized, tx1);
    }

    #[test]
    fn test_properties_operations() {
        // Test Properties (HashMap) operations
        let mut props = Properties::new();

        // Insert various value types
        props.insert("null_prop".to_string(), Value::Null);
        props.insert("bool_prop".to_string(), Value::Bool(true));
        props.insert("int_prop".to_string(), Value::Int(42));
        props.insert("string_prop".to_string(), Value::String("hello".to_string()));
        props.insert("array_prop".to_string(), Value::Array(vec!["a".to_string(), "b".to_string()]));

        assert_eq!(props.len(), 5);

        // Test retrieval
        assert_eq!(props.get("bool_prop"), Some(&Value::Bool(true)));
        assert_eq!(props.get("missing_prop"), None);

        // Test removal
        let removed = props.remove("int_prop");
        assert_eq!(removed, Some(Value::Int(42)));
        assert_eq!(props.len(), 4);

        // Test iteration
        let keys: Vec<_> = props.keys().collect();
        assert!(keys.contains(&&"null_prop".to_string()));
        assert!(keys.contains(&&"string_prop".to_string()));
    }

    #[test]
    fn test_properties_serialization() {
        let mut props = Properties::new();
        props.insert("name".to_string(), Value::String("test".to_string()));
        props.insert("count".to_string(), Value::Int(10));
        props.insert("active".to_string(), Value::Bool(true));

        // Test serialization
        let json = serde_json::to_string(&props).unwrap();
        let _expected = r#"{"active":true,"count":10,"name":"test"}"#;
        // Note: JSON object keys may be reordered, so we just check it parses back correctly

        // Test deserialization
        let deserialized: Properties = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 3);
        assert_eq!(deserialized.get("name"), Some(&Value::String("test".to_string())));
        assert_eq!(deserialized.get("count"), Some(&Value::Int(10)));
        assert_eq!(deserialized.get("active"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_label_and_property_key_types() {
        // Test Label type (just a String alias)
        let label: Label = "vertex_label".to_string();
        assert_eq!(label, "vertex_label");

        // Test PropertyKey type (just a String alias)
        let key: PropertyKey = "property_key".to_string();
        assert_eq!(key, "property_key");
    }

    #[test]
    fn test_value_display_and_debug() {
        // Test Debug trait
        assert_eq!(format!("{:?}", Value::Null), "Null");
        assert_eq!(format!("{:?}", Value::Bool(true)), "Bool(true)");
        assert_eq!(format!("{:?}", Value::Int(42)), "Int(42)");
        assert_eq!(format!("{:?}", Value::Integer(123)), "Integer(123)");
        assert_eq!(format!("{:?}", Value::String("hello".to_string())), "String(\"hello\")");
        assert_eq!(format!("{:?}", Value::Array(vec!["a".to_string()])), "Array([\"a\"])");
    }

    #[test]
    fn test_large_values() {
        // Test with large integer values
        let large_int = Value::Int(i64::MAX);
        let json = serde_json::to_string(&large_int).unwrap();
        let deserialized: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, large_int);

        // Test with large strings
        let large_string = "a".repeat(10000);
        let str_val = Value::String(large_string.clone());
        let json = serde_json::to_string(&str_val).unwrap();
        let deserialized: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, str_val);
    }

    #[test]
    fn test_edge_cases() {
        // Test empty string
        let empty_str = Value::String("".to_string());
        let json = serde_json::to_string(&empty_str).unwrap();
        assert_eq!(json, "\"\"");

        // Test empty array
        let empty_array = Value::Array(vec![]);
        let json = serde_json::to_string(&empty_array).unwrap();
        assert_eq!(json, "[]");

        // Test zero values
        assert_eq!(Value::Int(0), Value::Int(0));
        assert_eq!(Value::Integer(0), Value::Integer(0));
        assert_eq!(Value::Bool(false), Value::Bool(false));
    }
}

//! kotoba-core - Kotoba Core Components

pub mod types;
pub mod ir;
pub mod prelude {
    // Re-export commonly used items
    pub use crate::types::*;
    pub use crate::ir::*;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

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
    }

    #[test]
    fn test_content_hash() {
        // Test ContentHash generation
        let data = [42u8; 32];
        let hash = ContentHash::sha256(data);
        assert!(!hash.0.is_empty());

        // Test that same data produces same hash
        let hash2 = ContentHash::sha256(data);
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_uuid_types() {
        // Test that VertexId and EdgeId are UUIDs
        let vertex_id = VertexId::new_v4();
        let edge_id = EdgeId::new_v4();

        assert_ne!(vertex_id, edge_id); // Should be different UUIDs
        assert_eq!(vertex_id.get_version_num(), 4); // Should be v4 UUIDs
        assert_eq!(edge_id.get_version_num(), 4);
    }
}

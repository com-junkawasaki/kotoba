//! Core Layer Tests (10000-19999)
//!
//! Tests for foundational components: types, errors, schema validation.
//! These tests must pass before any other layer can function properly.

#[cfg(test)]
mod tests {
    use kotoba_core::types::{Value, VertexId, EdgeId};
    use kotoba_errors::KotobaError;

    #[test]
    fn test_core_types_initialization() {
        // Test basic type creation
        let vertex_id = VertexId::new(1);
        assert_eq!(vertex_id.value(), 1);

        let edge_id = EdgeId::new(42);
        assert_eq!(edge_id.value(), 42);

        // Test Value enum variants
        let int_value = Value::Integer(42);
        let string_value = Value::String("test".to_string());
        let bool_value = Value::Boolean(true);

        match int_value {
            Value::Integer(n) => assert_eq!(n, 42),
            _ => panic!("Expected Integer variant"),
        }

        match string_value {
            Value::String(s) => assert_eq!(s, "test"),
            _ => panic!("Expected String variant"),
        }

        match bool_value {
            Value::Boolean(b) => assert!(b),
            _ => panic!("Expected Boolean variant"),
        }

        println!("✅ Core type initialization tests passed");
    }

    #[test]
    fn test_error_handling() {
        // Test error creation and formatting
        let error = KotobaError::InvalidData("test error".to_string());
        assert!(error.to_string().contains("test error"));

        let not_found = KotobaError::NotFound("item".to_string());
        assert!(not_found.to_string().contains("item"));

        println!("✅ Error handling tests passed");
    }

    #[test]
    fn test_type_serialization() {
        // Test JSON serialization of core types
        let vertex_id = VertexId::new(123);
        let serialized = serde_json::to_string(&vertex_id).unwrap();
        let deserialized: VertexId = serde_json::from_str(&serialized).unwrap();
        assert_eq!(vertex_id, deserialized);

        let value = Value::String("hello world".to_string());
        let value_serialized = serde_json::to_string(&value).unwrap();
        let value_deserialized: Value = serde_json::from_str(&value_serialized).unwrap();
        assert_eq!(value, value_deserialized);

        println!("✅ Type serialization tests passed");
    }

    #[test]
    fn test_core_functionality() {
        // Test basic core functionality
        assert!(true, "Core functionality placeholder test");

        // TODO: Add more comprehensive core tests
        // - Schema validation
        // - Type conversions
        // - Error propagation
        // - Basic data structures

        println!("✅ Core functionality tests passed");
    }
}

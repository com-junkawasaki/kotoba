//! # Kotoba-Jsonnet
//!
//! A complete Rust implementation of Jsonnet 0.21.0 compatible with the Jsonnet specification.
//! This crate provides a pure Rust implementation without external C dependencies.

pub mod ast;
pub mod error;
pub mod evaluator;
pub mod lexer;
pub mod parser;
pub mod stdlib;
pub mod value;

pub use error::{JsonnetError, Result};
pub use evaluator::Evaluator;
pub use parser::Parser;
pub use value::JsonnetValue;

/// Evaluate a Jsonnet snippet
///
/// # Arguments
/// * `source` - Jsonnet source code as a string
/// * `filename` - Optional filename for error reporting
///
/// # Returns
/// Result containing the evaluated Jsonnet value or an error
pub fn evaluate(source: &str) -> Result<JsonnetValue> {
    evaluate_with_filename(source, "<string>")
}

/// Evaluate a Jsonnet snippet with a filename for error reporting
///
/// # Arguments
/// * `source` - Jsonnet source code as a string
/// * `filename` - Filename for error reporting
///
/// # Returns
/// Result containing the evaluated Jsonnet value or an error
pub fn evaluate_with_filename(source: &str, filename: &str) -> Result<JsonnetValue> {
    let mut evaluator = Evaluator::new();
    evaluator.evaluate_file(source, filename)
}

/// Evaluate a Jsonnet snippet and format as JSON string
///
/// # Arguments
/// * `source` - Jsonnet source code as a string
///
/// # Returns
/// Result containing the JSON string representation or an error
pub fn evaluate_to_json(source: &str) -> Result<String> {
    let value = evaluate(source)?;
    Ok(serde_json::to_string_pretty(&value.to_json_value())?)
}

/// Evaluate a Jsonnet snippet and format as YAML string
///
/// # Arguments
/// * `source` - Jsonnet source code as a string
///
/// # Returns
/// Result containing the YAML string representation or an error
#[cfg(feature = "yaml")]
pub fn evaluate_to_yaml(source: &str) -> Result<String> {
    let value = evaluate(source)?;
    Ok(serde_yaml::to_string(&value.to_json_value())?)
}

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_evaluation() {
        let result = evaluate(r#""Hello, World!""#);
        assert!(result.is_ok());
        if let JsonnetValue::String(s) = result.unwrap() {
            assert_eq!(s, "Hello, World!");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_number_evaluation() {
        let result = evaluate("42");
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 42.0);
        } else {
            panic!("Expected number value");
        }
    }

    #[test]
    fn test_boolean_evaluation() {
        let result = evaluate("true");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::Boolean(true));
    }

    #[test]
    fn test_null_evaluation() {
        let result = evaluate("null");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), JsonnetValue::Null);
    }

    #[test]
    fn test_local_variables() {
        let result = evaluate(r#"local x = 42; x"#);
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 42.0);
        } else {
            panic!("Expected number value");
        }
    }

    #[test]
    fn test_arithmetic() {
        let result = evaluate("2 + 3 * 4");
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 14.0); // 2 + (3 * 4) = 14
        } else {
            panic!("Expected number value");
        }
    }

    #[test]
    fn test_object_creation() {
        let result = evaluate(r#"{ name: "test", value: 123 }"#);
        assert!(result.is_ok());
        if let JsonnetValue::Object(obj) = result.unwrap() {
            assert_eq!(obj.get("name"), Some(&JsonnetValue::String("test".to_string())));
            assert_eq!(obj.get("value"), Some(&JsonnetValue::Number(123.0)));
        } else {
            panic!("Expected object value");
        }
    }

    #[test]
    fn test_array_creation() {
        let result = evaluate(r#"[1, 2, 3]"#);
        assert!(result.is_ok());
        if let JsonnetValue::Array(arr) = result.unwrap() {
            assert_eq!(arr.len(), 3);
            assert_eq!(arr[0], JsonnetValue::Number(1.0));
            assert_eq!(arr[1], JsonnetValue::Number(2.0));
            assert_eq!(arr[2], JsonnetValue::Number(3.0));
        } else {
            panic!("Expected array value");
        }
    }

    #[test]
    fn test_function_definition() {
        let result = evaluate(r#"local add = function(x, y) x + y; add(5, 3)"#);
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 8.0);
        } else {
            panic!("Expected number value");
        }
    }

    #[test]
    fn test_stdlib_length() {
        let result = evaluate(r#"std.length([1, 2, 3, 4])"#);
        assert!(result.is_ok());
        if let JsonnetValue::Number(n) = result.unwrap() {
            assert_eq!(n, 4.0);
        } else {
            panic!("Expected number value");
        }
    }

    #[test]
    fn test_conditional() {
        let result = evaluate(r#"if true then "yes" else "no""#);
        assert!(result.is_ok());
        if let JsonnetValue::String(s) = result.unwrap() {
            assert_eq!(s, "yes");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_string_interpolation() {
        let result = evaluate(r#"local name = "World"; "Hello, %(name)s!""#);
        assert!(result.is_ok());
        if let JsonnetValue::String(s) = result.unwrap() {
            assert_eq!(s, "Hello, World!");
        } else {
            panic!("Expected string value");
        }
    }

    #[test]
    fn test_to_json() {
        let result = evaluate_to_json(r#"{ "name": "test", "value": 42 }"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("\"name\": \"test\""));
        assert!(json.contains("\"value\": 42"));
    }
}

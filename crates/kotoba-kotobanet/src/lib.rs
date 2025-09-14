//! # Kotoba Kotobanet
//!
//! Kotoba-specific Jsonnet extensions extending the base Jsonnet implementation.
//! This crate provides Kotoba-specific functionality:
//!
//! - HTTP Parser: .kotoba.json configuration parsing
//! - Frontend Framework: React component definitions
//! - Deploy Configuration: Deployment settings
//! - Config Management: General configuration handling

pub mod error;
pub mod http_parser;
pub mod frontend;
pub mod deploy;
pub mod config;

pub use error::*;
pub use http_parser::*;
pub use frontend::*;
pub use deploy::*;
pub use config::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Evaluate Kotoba Jsonnet code with extensions
pub fn evaluate_kotoba(code: &str) -> Result<kotoba_jsonnet::JsonnetValue> {
    // TODO: Add Kotoba-specific extensions
    Ok(kotoba_jsonnet::evaluate(code)?)
}

/// Evaluate Kotoba Jsonnet to JSON with extensions
pub fn evaluate_kotoba_to_json(code: &str) -> Result<String> {
    // TODO: Add Kotoba-specific extensions
    Ok(kotoba_jsonnet::evaluate_to_json(code)?)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kotoba_evaluation() {
        // Test basic Jsonnet functionality still works
        let result = evaluate_kotoba(r#"{ name: "Kotoba", version: 1 }"#);
        assert!(result.is_ok());
    }

    #[test]
    fn test_kotoba_to_json() {
        let result = evaluate_kotoba_to_json(r#"{ message: "Hello Kotobanet" }"#);
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("Hello Kotobanet"));
    }

}

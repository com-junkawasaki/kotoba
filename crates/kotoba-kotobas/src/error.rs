//! Error types for Kotoba Kotobanet

use thiserror::Error;

/// Errors that can occur in Kotoba Kotobanet operations
#[derive(Debug, Error)]
pub enum KotobaNetError {
    #[error("Jsonnet evaluation error: {0}")]
    Jsonnet(#[from] kotoba_jsonnet::JsonnetError),

    #[error("HTTP parsing error: {0}")]
    HttpParse(String),

    #[error("Frontend parsing error: {0}")]
    FrontendParse(String),

    #[error("Deploy configuration error: {0}")]
    DeployConfig(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Execution error: {0}")]
    Execution(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),


    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, KotobaNetError>;

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_jsonnet::JsonnetError;

    #[test]
    fn test_kotoba_net_error_display() {
        // Test Jsonnet error variant
        let jsonnet_err = KotobaNetError::Jsonnet(JsonnetError::parse_error(1, 1, "test error"));
        assert!(jsonnet_err.to_string().contains("Jsonnet evaluation error"));

        // Test HTTP parse error
        let http_err = KotobaNetError::HttpParse("HTTP parsing failed".to_string());
        assert!(http_err.to_string().contains("HTTP parsing error"));

        // Test Frontend parse error
        let frontend_err = KotobaNetError::FrontendParse("Frontend parsing failed".to_string());
        assert!(frontend_err.to_string().contains("Frontend parsing error"));

        // Test Deploy config error
        let deploy_err = KotobaNetError::DeployConfig("Deploy config failed".to_string());
        assert!(deploy_err.to_string().contains("Deploy configuration error"));

        // Test Config error
        let config_err = KotobaNetError::Config("Config failed".to_string());
        assert!(config_err.to_string().contains("Configuration error"));

        // Test IO error
        let io_err = KotobaNetError::Io(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        assert!(io_err.to_string().contains("IO error"));

        // Test JSON error - using a valid JSON error
        let json_str = r#"{"invalid": json}"#;
        let json_err = KotobaNetError::Json(serde_json::from_str::<serde_json::Value>(json_str).unwrap_err());
        assert!(json_err.to_string().contains("JSON error"));

        // Test Regex error
        let regex_err = KotobaNetError::Regex(regex::Error::Syntax("invalid regex".to_string()));
        assert!(regex_err.to_string().contains("Regex error"));

        // Test InvalidArgument error
        let invalid_arg_err = KotobaNetError::InvalidArgument("invalid argument".to_string());
        assert!(invalid_arg_err.to_string().contains("Invalid argument"));

        // Test NotFound error
        let not_found_err = KotobaNetError::NotFound("resource not found".to_string());
        assert!(not_found_err.to_string().contains("Not found"));
    }

    #[test]
    fn test_error_from_trait_implementations() {
        // Test From<JsonnetError>
        let jsonnet_error = JsonnetError::parse_error(1, 1, "test");
        let kotoba_error: KotobaNetError = jsonnet_error.into();
        assert!(matches!(kotoba_error, KotobaNetError::Jsonnet(_)));

        // Test From<std::io::Error>
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "test");
        let kotoba_error: KotobaNetError = io_error.into();
        assert!(matches!(kotoba_error, KotobaNetError::Io(_)));

        // Test From<serde_json::Error>
        let json_str = r#"{"invalid": json}"#;
        let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();
        let kotoba_error: KotobaNetError = json_error.into();
        assert!(matches!(kotoba_error, KotobaNetError::Json(_)));

        // Test From<regex::Error>
        let regex_error = regex::Error::Syntax("test".to_string());
        let kotoba_error: KotobaNetError = regex_error.into();
        assert!(matches!(kotoba_error, KotobaNetError::Regex(_)));
    }

    #[test]
    fn test_error_debug_formatting() {
        let error = KotobaNetError::HttpParse("test error".to_string());
        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("HttpParse"));
        assert!(debug_str.contains("test error"));
    }

    #[test]
    fn test_error_variant_coverage() {
        // Ensure all variants can be created and displayed
        let json_str = r#"{"invalid": json}"#;
        let json_error = serde_json::from_str::<serde_json::Value>(json_str).unwrap_err();

        let variants = vec![
            KotobaNetError::Jsonnet(JsonnetError::parse_error(1, 1, "test")),
            KotobaNetError::HttpParse("test".to_string()),
            KotobaNetError::FrontendParse("test".to_string()),
            KotobaNetError::DeployConfig("test".to_string()),
            KotobaNetError::Config("test".to_string()),
            KotobaNetError::Io(std::io::Error::new(std::io::ErrorKind::Other, "test")),
            KotobaNetError::Json(json_error),
            KotobaNetError::Regex(regex::Error::Syntax("test".to_string())),
            KotobaNetError::InvalidArgument("test".to_string()),
            KotobaNetError::NotFound("test".to_string()),
        ];

        for variant in variants {
            // Each variant should have a non-empty display string
            let display_str = variant.to_string();
            assert!(!display_str.is_empty(), "Variant {:?} has empty display string", variant);

            // Each variant should have a debug representation
            let debug_str = format!("{:?}", variant);
            assert!(!debug_str.is_empty(), "Variant {:?} has empty debug string", variant);
        }
    }
}

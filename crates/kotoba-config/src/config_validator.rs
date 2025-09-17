//! # Configuration Validator
//!
//! Schema-based validation for configuration values with support for
//! JSON Schema, custom validation rules, and type checking.

use crate::*;
use std::collections::HashMap;

/// Configuration validator trait
#[async_trait::async_trait]
pub trait ConfigValidator: Send + Sync {
    /// Validate a configuration value
    async fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), ConfigError>;

    /// Add a validation rule
    async fn add_rule(&self, rule: ValidationRule) -> Result<(), ConfigError>;

    /// Remove a validation rule
    async fn remove_rule(&self, rule_name: &str) -> Result<(), ConfigError>;

    /// Get validation rules for a key
    async fn get_rules_for_key(&self, key: &str) -> Result<Vec<ValidationRule>, ConfigError>;
}

/// Default JSON Schema-based configuration validator
pub struct SchemaConfigValidator {
    rules: Arc<parking_lot::RwLock<HashMap<String, ValidationRule>>>,
    key_patterns: Arc<parking_lot::RwLock<HashMap<String, regex::Regex>>>,
}

impl SchemaConfigValidator {
    pub fn new() -> Self {
        Self {
            rules: Arc::new(parking_lot::RwLock::new(HashMap::new())),
            key_patterns: Arc::new(parking_lot::RwLock::new(HashMap::new())),
        }
    }

    pub fn with_default_rules() -> Self {
        let validator = Self::new();

        // Add some common validation rules
        let rules = vec![
            ValidationRule {
                name: "database.url".to_string(),
                description: "Database connection URL validation".to_string(),
                schema: serde_json::json!({
                    "type": "string",
                    "pattern": "^(postgresql|mysql|sqlite)://.*"
                }),
                enabled: true,
            },
            ValidationRule {
                name: "server.port".to_string(),
                description: "Server port validation".to_string(),
                schema: serde_json::json!({
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 65535
                }),
                enabled: true,
            },
            ValidationRule {
                name: "cache.size".to_string(),
                description: "Cache size validation".to_string(),
                schema: serde_json::json!({
                    "type": "integer",
                    "minimum": 0
                }),
                enabled: true,
            },
            ValidationRule {
                name: "log.level".to_string(),
                description: "Log level validation".to_string(),
                schema: serde_json::json!({
                    "type": "string",
                    "enum": ["error", "warn", "info", "debug", "trace"]
                }),
                enabled: true,
            },
        ];

        for rule in rules {
            let _ = validator.add_rule_sync(rule);
        }

        validator
    }

    fn add_rule_sync(&self, rule: ValidationRule) -> Result<(), ConfigError> {
        let mut rules = self.rules.write();
        let key = rule.name.clone();

        // Compile regex for key patterns if needed
        if key.contains('*') {
            let pattern = regex::Regex::new(&key.replace('.', "\\.").replace('*', ".*"))
                .map_err(|e| ConfigError::Validation(format!("Invalid key pattern: {}", e)))?;
            self.key_patterns.write().insert(key.clone(), pattern);
        }

        rules.insert(key, rule);
        Ok(())
    }

    fn validate_against_schema(value: &serde_json::Value, schema: &serde_json::Value) -> Result<(), ConfigError> {
        // Simple schema validation implementation
        // In a full implementation, you'd use a proper JSON Schema validator

        match schema.get("type").and_then(|t| t.as_str()) {
            Some("string") => {
                if !value.is_string() {
                    return Err(ConfigError::Validation("Expected string type".to_string()));
                }

                // Check pattern
                if let Some(pattern) = schema.get("pattern").and_then(|p| p.as_str()) {
                    let regex = regex::Regex::new(pattern)
                        .map_err(|e| ConfigError::Validation(format!("Invalid pattern: {}", e)))?;

                    let str_value = value.as_str().unwrap();
                    if !regex.is_match(str_value) {
                        return Err(ConfigError::Validation(format!("Value '{}' does not match pattern '{}'", str_value, pattern)));
                    }
                }

                // Check enum
                if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
                    let str_value = value.as_str().unwrap();
                    let valid_values: Vec<String> = enum_values.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect();

                    if !valid_values.contains(&str_value.to_string()) {
                        return Err(ConfigError::Validation(format!("Value '{}' not in allowed values: {:?}", str_value, valid_values)));
                    }
                }
            }
            Some("integer") => {
                if !value.is_i64() && !value.is_u64() {
                    return Err(ConfigError::Validation("Expected integer type".to_string()));
                }

                let int_value = value.as_i64().unwrap_or(value.as_u64().unwrap_or(0) as i64);

                // Check minimum
                if let Some(min) = schema.get("minimum").and_then(|m| m.as_i64()) {
                    if int_value < min {
                        return Err(ConfigError::Validation(format!("Value {} is less than minimum {}", int_value, min)));
                    }
                }

                // Check maximum
                if let Some(max) = schema.get("maximum").and_then(|m| m.as_i64()) {
                    if int_value > max {
                        return Err(ConfigError::Validation(format!("Value {} is greater than maximum {}", int_value, max)));
                    }
                }
            }
            Some("boolean") => {
                if !value.is_boolean() {
                    return Err(ConfigError::Validation("Expected boolean type".to_string()));
                }
            }
            Some("number") => {
                if !value.is_number() {
                    return Err(ConfigError::Validation("Expected number type".to_string()));
                }
            }
            _ => {
                // For unknown types, accept any value
            }
        }

        Ok(())
    }

    fn find_matching_rules(&self, key: &str) -> Vec<ValidationRule> {
        let rules = self.rules.read();
        let patterns = self.key_patterns.read();

        let mut matching_rules = Vec::new();

        // Check exact matches first
        if let Some(rule) = rules.get(key) {
            if rule.enabled {
                matching_rules.push(rule.clone());
            }
        }

        // Check pattern matches
        for (pattern_key, regex) in patterns.iter() {
            if regex.is_match(key) {
                if let Some(rule) = rules.get(pattern_key) {
                    if rule.enabled {
                        matching_rules.push(rule.clone());
                    }
                }
            }
        }

        matching_rules
    }
}

#[async_trait::async_trait]
impl ConfigValidator for SchemaConfigValidator {
    async fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
        let matching_rules = self.find_matching_rules(key);

        for rule in matching_rules {
            Self::validate_against_schema(value, &rule.schema)?;
        }

        Ok(())
    }

    async fn add_rule(&self, rule: ValidationRule) -> Result<(), ConfigError> {
        self.add_rule_sync(rule)
    }

    async fn remove_rule(&self, rule_name: &str) -> Result<(), ConfigError> {
        let mut rules = self.rules.write();
        rules.remove(rule_name);
        Ok(())
    }

    async fn get_rules_for_key(&self, key: &str) -> Result<Vec<ValidationRule>, ConfigError> {
        Ok(self.find_matching_rules(key))
    }
}

/// Type-safe configuration validator
pub struct TypeConfigValidator<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TypeConfigValidator<T> {
    pub fn new() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }
}

#[async_trait::async_trait]
impl<T: serde::de::DeserializeOwned + Send + Sync> ConfigValidator for TypeConfigValidator<T> {
    async fn validate(&self, _key: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
        // Try to deserialize the value to the target type
        serde_json::from_value::<T>(value.clone())
            .map_err(|e| ConfigError::Validation(format!("Type validation failed: {}", e)))?;
        Ok(())
    }

    async fn add_rule(&self, _rule: ValidationRule) -> Result<(), ConfigError> {
        // Type validators don't use rules
        Ok(())
    }

    async fn remove_rule(&self, _rule_name: &str) -> Result<(), ConfigError> {
        // Type validators don't use rules
        Ok(())
    }

    async fn get_rules_for_key(&self, _key: &str) -> Result<Vec<ValidationRule>, ConfigError> {
        Ok(Vec::new())
    }
}

/// Composite validator that combines multiple validators
pub struct CompositeConfigValidator {
    validators: Vec<Box<dyn ConfigValidator>>,
}

impl CompositeConfigValidator {
    pub fn new() -> Self {
        Self {
            validators: Vec::new(),
        }
    }

    pub fn add_validator(&mut self, validator: Box<dyn ConfigValidator>) {
        self.validators.push(validator);
    }
}

#[async_trait::async_trait]
impl ConfigValidator for CompositeConfigValidator {
    async fn validate(&self, key: &str, value: &serde_json::Value) -> Result<(), ConfigError> {
        for validator in &self.validators {
            validator.validate(key, value).await?;
        }
        Ok(())
    }

    async fn add_rule(&self, rule: ValidationRule) -> Result<(), ConfigError> {
        // Add rule to all validators that support it
        for validator in &self.validators {
            let _ = validator.add_rule(rule.clone()).await; // Ignore errors
        }
        Ok(())
    }

    async fn remove_rule(&self, rule_name: &str) -> Result<(), ConfigError> {
        for validator in &self.validators {
            let _ = validator.remove_rule(rule_name).await; // Ignore errors
        }
        Ok(())
    }

    async fn get_rules_for_key(&self, key: &str) -> Result<Vec<ValidationRule>, ConfigError> {
        let mut all_rules = Vec::new();

        for validator in &self.validators {
            if let Ok(rules) = validator.get_rules_for_key(key).await {
                all_rules.extend(rules);
            }
        }

        Ok(all_rules)
    }
}

/// Validation result
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_error(&mut self, error: String) {
        self.is_valid = false;
        self.errors.push(error);
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn merge(&mut self, other: ValidationResult) {
        self.is_valid &= other.is_valid;
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

/// Configuration validation utilities
pub struct ConfigValidationUtils;

impl ConfigValidationUtils {
    /// Validate database URL
    pub fn validate_database_url(url: &str) -> Result<(), ConfigError> {
        if url.is_empty() {
            return Err(ConfigError::Validation("Database URL cannot be empty".to_string()));
        }

        // Basic URL validation
        if !url.starts_with("postgresql://") &&
           !url.starts_with("mysql://") &&
           !url.starts_with("sqlite://") {
            return Err(ConfigError::Validation("Unsupported database URL scheme".to_string()));
        }

        Ok(())
    }

    /// Validate port number
    pub fn validate_port(port: i64) -> Result<(), ConfigError> {
        if port < 1 || port > 65535 {
            return Err(ConfigError::Validation(format!("Port {} is out of valid range (1-65535)", port)));
        }
        Ok(())
    }

    /// Validate email address
    pub fn validate_email(email: &str) -> Result<(), ConfigError> {
        let email_regex = regex::Regex::new(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$")
            .map_err(|e| ConfigError::Validation(format!("Invalid email regex: {}", e)))?;

        if !email_regex.is_match(email) {
            return Err(ConfigError::Validation(format!("Invalid email format: {}", email)));
        }

        Ok(())
    }

    /// Validate file path
    pub fn validate_file_path(path: &str) -> Result<(), ConfigError> {
        if path.is_empty() {
            return Err(ConfigError::Validation("File path cannot be empty".to_string()));
        }

        // Check for invalid characters
        let invalid_chars = ['<', '>', ':', '"', '|', '?', '*'];
        for &ch in &invalid_chars {
            if path.contains(ch) {
                return Err(ConfigError::Validation(format!("File path contains invalid character: {}", ch)));
            }
        }

        Ok(())
    }

    /// Validate positive number
    pub fn validate_positive_number(value: f64, field_name: &str) -> Result<(), ConfigError> {
        if value <= 0.0 {
            return Err(ConfigError::Validation(format!("{} must be positive, got {}", field_name, value)));
        }
        Ok(())
    }

    /// Validate size in bytes
    pub fn validate_size_bytes(size: &str) -> Result<u64, ConfigError> {
        if size.is_empty() {
            return Err(ConfigError::Validation("Size cannot be empty".to_string()));
        }

        // Parse size with units (e.g., "1GB", "512MB", "1024KB")
        let size_regex = regex::Regex::new(r"^(\d+)([KMGT]B?)?$")
            .map_err(|e| ConfigError::Validation(format!("Invalid size regex: {}", e)))?;

        let captures = size_regex.captures(size.to_uppercase().as_str())
            .ok_or_else(|| ConfigError::Validation(format!("Invalid size format: {}", size)))?;

        let number: u64 = captures[1].parse()
            .map_err(|_| ConfigError::Validation(format!("Invalid number in size: {}", size)))?;

        let multiplier = match captures.get(2).map(|m| m.as_str()) {
            Some("KB") | Some("K") => 1024,
            Some("MB") | Some("M") => 1024 * 1024,
            Some("GB") | Some("G") => 1024 * 1024 * 1024,
            Some("TB") | Some("T") => 1024 * 1024 * 1024 * 1024,
            _ => 1, // Bytes
        };

        Ok(number * multiplier)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_schema_validator() {
        let validator = SchemaConfigValidator::new();

        // Add a simple validation rule
        let rule = ValidationRule {
            name: "test.port".to_string(),
            description: "Port validation".to_string(),
            schema: serde_json::json!({
                "type": "integer",
                "minimum": 1,
                "maximum": 65535
            }),
            enabled: true,
        };

        validator.add_rule(rule).await.unwrap();

        // Test valid value
        let valid_value = serde_json::json!(8080);
        assert!(validator.validate("test.port", &valid_value).await.is_ok());

        // Test invalid value (too high)
        let invalid_value = serde_json::json!(70000);
        assert!(validator.validate("test.port", &invalid_value).await.is_err());

        // Test invalid type
        let wrong_type = serde_json::json!("8080");
        assert!(validator.validate("test.port", &wrong_type).await.is_err());
    }

    #[tokio::test]
    async fn test_type_validator() {
        let validator = TypeConfigValidator::<u32>::new();

        // Test valid u32
        let valid_value = serde_json::json!(42);
        assert!(validator.validate("test", &valid_value).await.is_ok());

        // Test invalid u32 (negative)
        let invalid_value = serde_json::json!(-1);
        assert!(validator.validate("test", &invalid_value).await.is_err());

        // Test invalid type
        let wrong_type = serde_json::json!("not_a_number");
        assert!(validator.validate("test", &wrong_type).await.is_err());
    }

    #[test]
    fn test_validation_utils() {
        // Test database URL validation
        assert!(ConfigValidationUtils::validate_database_url("postgresql://localhost:5432/db").is_ok());
        assert!(ConfigValidationUtils::validate_database_url("mysql://user:pass@host/db").is_ok());
        assert!(ConfigValidationUtils::validate_database_url("invalid://url").is_err());

        // Test port validation
        assert!(ConfigValidationUtils::validate_port(8080).is_ok());
        assert!(ConfigValidationUtils::validate_port(70000).is_err());
        assert!(ConfigValidationUtils::validate_port(0).is_err());

        // Test size validation
        assert_eq!(ConfigValidationUtils::validate_size_bytes("1KB").unwrap(), 1024);
        assert_eq!(ConfigValidationUtils::validate_size_bytes("512MB").unwrap(), 512 * 1024 * 1024);
        assert!(ConfigValidationUtils::validate_size_bytes("invalid").is_err());
    }

    #[tokio::test]
    async fn test_composite_validator() {
        let mut composite = CompositeConfigValidator::new();

        // Add schema validator
        let schema_validator = SchemaConfigValidator::new();
        let rule = ValidationRule {
            name: "test.value".to_string(),
            description: "Test validation".to_string(),
            schema: serde_json::json!({"type": "integer", "minimum": 0}),
            enabled: true,
        };
        schema_validator.add_rule(rule).await.unwrap();
        composite.add_validator(Box::new(schema_validator));

        // Add type validator
        composite.add_validator(Box::new(TypeConfigValidator::<i32>::new()));

        // Test valid value
        let valid_value = serde_json::json!(42);
        assert!(composite.validate("test.value", &valid_value).await.is_ok());

        // Test invalid value
        let invalid_value = serde_json::json!(-1);
        assert!(composite.validate("test.value", &invalid_value).await.is_err());
    }
}

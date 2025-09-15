//! Security configuration types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::capabilities::CapabilityConfig;

/// Main security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_config: JwtConfig,
    pub oauth2_config: Option<OAuth2Config>,
    pub mfa_config: MfaConfig,
    pub password_config: PasswordConfig,
    pub session_config: SessionConfig,
    pub capability_config: CapabilityConfig,
    pub rate_limit_config: RateLimitConfig,
    pub audit_config: AuditConfig,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            jwt_config: JwtConfig::default(),
            oauth2_config: None,
            mfa_config: MfaConfig::default(),
            password_config: PasswordConfig::default(),
            session_config: SessionConfig::default(),
            capability_config: CapabilityConfig::default(),
            rate_limit_config: RateLimitConfig::default(),
            audit_config: AuditConfig::default(),
        }
    }
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub algorithm: JwtAlgorithm,
    pub secret: String,
    pub issuer: String,
    pub audience: Vec<String>,
    pub access_token_expiration: u64,  // seconds
    pub refresh_token_expiration: u64, // seconds
    pub leeway_seconds: u64,
    pub validate_exp: bool,
    pub validate_nbf: bool,
    pub validate_aud: bool,
    pub validate_iss: bool,
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            algorithm: JwtAlgorithm::HS256,
            secret: "your-secret-key-change-in-production".to_string(),
            issuer: "kotoba".to_string(),
            audience: vec!["kotoba-users".to_string()],
            access_token_expiration: 900,    // 15 minutes
            refresh_token_expiration: 86400, // 24 hours
            leeway_seconds: 60,
            validate_exp: true,
            validate_nbf: false,
            validate_aud: true,
            validate_iss: true,
        }
    }
}

/// JWT algorithm types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum JwtAlgorithm {
    HS256,
    HS384,
    HS512,
    RS256,
    RS384,
    RS512,
    ES256,
    ES384,
    ES512,
}

impl JwtAlgorithm {
    pub fn as_str(&self) -> &'static str {
        match self {
            JwtAlgorithm::HS256 => "HS256",
            JwtAlgorithm::HS384 => "HS384",
            JwtAlgorithm::HS512 => "HS512",
            JwtAlgorithm::RS256 => "RS256",
            JwtAlgorithm::RS384 => "RS384",
            JwtAlgorithm::RS512 => "RS512",
            JwtAlgorithm::ES256 => "ES256",
            JwtAlgorithm::ES384 => "ES384",
            JwtAlgorithm::ES512 => "ES512",
        }
    }
}

/// OAuth2 configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OAuth2Config {
    #[serde(default)]
    pub providers: HashMap<String, OAuth2ProviderConfig>,
    #[serde(default)]
    pub redirect_uri: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default = "default_state_timeout")]
    pub state_timeout_seconds: u64,
}

fn default_state_timeout() -> u64 {
    600 // 10 minutes
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuth2ProviderConfig {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub userinfo_url: Option<String>,
    pub scope_separator: String,
    pub additional_params: HashMap<String, String>,
}

/// MFA configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    pub issuer: String,
    pub digits: u8,
    pub skew: u8,
    pub step: u64,
    pub backup_codes_count: usize,
    pub qr_code_size: u32,
}

impl Default for MfaConfig {
    fn default() -> Self {
        Self {
            issuer: "Kotoba".to_string(),
            digits: 6,
            skew: 1,
            step: 30,
            backup_codes_count: 10,
            qr_code_size: 200,
        }
    }
}

/// Password configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordConfig {
    pub algorithm: PasswordAlgorithm,
    pub min_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_digits: bool,
    pub require_special_chars: bool,
    pub argon2_config: Option<Argon2Config>,
    pub pbkdf2_config: Option<Pbkdf2Config>,
}

impl Default for PasswordConfig {
    fn default() -> Self {
        Self {
            algorithm: PasswordAlgorithm::Argon2,
            min_length: 8,
            require_uppercase: true,
            require_lowercase: true,
            require_digits: true,
            require_special_chars: false,
            argon2_config: Some(Argon2Config::default()),
            pbkdf2_config: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PasswordAlgorithm {
    Argon2,
    Pbkdf2,
    Bcrypt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Argon2Config {
    pub variant: Argon2Variant,
    pub version: u32,
    pub m_cost: u32,
    pub t_cost: u32,
    pub p_cost: u32,
    pub output_len: usize,
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            variant: Argon2Variant::Argon2id,
            version: argon2::Version::V0x13 as u32,
            m_cost: 65536, // 64 MB
            t_cost: 3,
            p_cost: 4,
            output_len: 32,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Argon2Variant {
    Argon2d,
    Argon2i,
    Argon2id,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pbkdf2Config {
    pub iterations: u32,
    pub output_len: usize,
}

/// Session configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    pub store_type: SessionStoreType,
    pub cookie_name: String,
    pub cookie_secure: bool,
    pub cookie_http_only: bool,
    pub cookie_same_site: SameSitePolicy,
    pub max_age_seconds: Option<u64>,
    pub idle_timeout_seconds: Option<u64>,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            store_type: SessionStoreType::Memory,
            cookie_name: "kotoba_session".to_string(),
            cookie_secure: true,
            cookie_http_only: true,
            cookie_same_site: SameSitePolicy::Lax,
            max_age_seconds: Some(86400), // 24 hours
            idle_timeout_seconds: Some(3600), // 1 hour
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SessionStoreType {
    Memory,
    Redis,
    Database,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SameSitePolicy {
    Strict,
    Lax,
    None,
}

/// Rate limit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub max_requests: u32,
    pub window_seconds: u64,
    pub burst_size: u32,
    pub exempt_ips: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_requests: 100,
            window_seconds: 60,
            burst_size: 10,
            exempt_ips: Vec::new(),
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    pub enabled: bool,
    pub log_level: AuditLogLevel,
    pub log_sensitive_data: bool,
    pub retention_days: u64,
    pub max_entries_per_day: usize,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_level: AuditLogLevel::Info,
            log_sensitive_data: false,
            retention_days: 90,
            max_entries_per_day: 10000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

/// Authentication method enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthMethod {
    Local,
    OAuth2(String), // provider name
    Ldap,
    Saml,
    Custom(String),
}

/// Validation functions
impl SecurityConfig {
    pub fn validate(&self) -> Result<(), String> {
        // Validate JWT config
        if self.jwt_config.secret.is_empty() {
            return Err("JWT secret cannot be empty".to_string());
        }

        if self.jwt_config.secret.len() < 32 {
            return Err("JWT secret should be at least 32 characters long".to_string());
        }

        // Validate OAuth2 config if present
        if let Some(oauth2) = &self.oauth2_config {
            if oauth2.providers.is_empty() {
                return Err("OAuth2 providers cannot be empty".to_string());
            }

            for (name, provider) in &oauth2.providers {
                if provider.client_id.is_empty() {
                    return Err(format!("OAuth2 provider '{}' client_id cannot be empty", name));
                }
                if provider.client_secret.is_empty() {
                    return Err(format!("OAuth2 provider '{}' client_secret cannot be empty", name));
                }
            }
        }

        // Validate password config
        if self.password_config.min_length < 8 {
            return Err("Minimum password length should be at least 8".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_validation() {
        let config = SecurityConfig::default();
        assert!(config.validate().is_err()); // Should fail due to weak secret
    }

    #[test]
    fn test_config_validation_with_strong_secret() {
        let mut config = SecurityConfig::default();
        config.jwt_config.secret = "a".repeat(32);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_jwt_algorithm_conversion() {
        assert_eq!(JwtAlgorithm::HS256.as_str(), "HS256");
        assert_eq!(JwtAlgorithm::RS256.as_str(), "RS256");
        assert_eq!(JwtAlgorithm::ES256.as_str(), "ES256");
    }
}

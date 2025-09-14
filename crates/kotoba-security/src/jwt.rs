//! JWT token management

use crate::error::{SecurityError, Result};
use crate::config::{JwtConfig, JwtAlgorithm};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// JWT claims structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,              // Subject (user ID)
    pub exp: usize,               // Expiration time
    pub iat: usize,               // Issued at
    pub nbf: Option<usize>,       // Not before
    pub iss: String,              // Issuer
    pub aud: Vec<String>,         // Audience
    pub roles: Vec<String>,       // User roles
    pub permissions: Vec<String>, // User permissions
    pub jti: String,              // JWT ID (unique identifier)
    #[serde(flatten)]
    pub custom: std::collections::HashMap<String, serde_json::Value>,
}

impl JwtClaims {
    /// Create new JWT claims
    pub fn new(
        subject: String,
        issuer: String,
        audience: Vec<String>,
        roles: Vec<String>,
        permissions: Vec<String>,
        expiration_seconds: u64,
    ) -> Self {
        let now = Utc::now();
        let exp = (now + Duration::seconds(expiration_seconds as i64)).timestamp() as usize;
        let iat = now.timestamp() as usize;
        let jti = uuid::Uuid::new_v4().to_string();

        Self {
            sub: subject,
            exp,
            iat,
            nbf: None,
            iss: issuer,
            aud: audience,
            roles,
            permissions,
            jti,
            custom: std::collections::HashMap::new(),
        }
    }

    /// Check if token is expired
    pub fn is_expired(&self) -> bool {
        let now = Utc::now().timestamp() as usize;
        self.exp < now
    }

    /// Get remaining time until expiration in seconds
    pub fn time_until_expiry(&self) -> i64 {
        let now = Utc::now().timestamp();
        (self.exp as i64) - now
    }

    /// Add custom claim
    pub fn add_custom_claim(&mut self, key: String, value: serde_json::Value) {
        self.custom.insert(key, value);
    }

    /// Get custom claim
    pub fn get_custom_claim(&self, key: &str) -> Option<&serde_json::Value> {
        self.custom.get(key)
    }

    /// Check if user has specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }

    /// Check if user has specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if user has any of the specified roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the specified permissions
    pub fn has_all_permissions(&self, permissions: &[&str]) -> bool {
        permissions.iter().all(|perm| self.has_permission(perm))
    }
}

/// Token pair (access + refresh)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
    pub scope: Option<String>,
}

impl TokenPair {
    /// Create new token pair
    pub fn new(access_token: String, refresh_token: String, expires_in: u64) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            scope: None,
        }
    }

    /// Create token pair with scope
    pub fn with_scope(access_token: String, refresh_token: String, expires_in: u64, scope: String) -> Self {
        Self {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in,
            scope: Some(scope),
        }
    }
}

/// JWT service for token management
pub struct JwtService {
    config: JwtConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtService {
    /// Create new JWT service
    pub fn new(config: JwtConfig) -> Result<Self> {
        let algorithm = Self::map_algorithm(&config.algorithm)?;
        let encoding_key = Self::create_encoding_key(&config.algorithm, &config.secret)?;
        let decoding_key = Self::create_decoding_key(&config.algorithm, &config.secret)?;
        let mut validation = Validation::new(algorithm);

        // Configure validation
        validation.leeway = config.leeway_seconds;
        validation.validate_exp = config.validate_exp;
        validation.validate_nbf = config.validate_nbf;
        validation.validate_aud = config.validate_aud;
        validation.validate_iss = config.validate_iss;

        if config.validate_aud {
            validation.aud = Some(HashSet::from_iter(config.audience.iter().cloned()));
        }

        if config.validate_iss {
            validation.iss = Some(HashSet::from_iter(vec![config.issuer.clone()]));
        }

        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            validation,
        })
    }

    /// Generate access token
    pub fn generate_access_token(&self, user_id: &str, roles: Vec<String>) -> Result<String> {
        let permissions = Vec::new(); // Will be populated based on roles
        let claims = JwtClaims::new(
            user_id.to_string(),
            self.config.issuer.clone(),
            self.config.audience.clone(),
            roles,
            permissions,
            self.config.access_token_expiration,
        );

        self.encode_token(&claims)
    }

    /// Generate refresh token
    pub fn generate_refresh_token(&self, user_id: &str) -> Result<String> {
        let mut claims = JwtClaims::new(
            user_id.to_string(),
            self.config.issuer.clone(),
            self.config.audience.clone(),
            Vec::new(),
            Vec::new(),
            self.config.refresh_token_expiration,
        );

        // Add token type identifier
        claims.add_custom_claim("token_type".to_string(), serde_json::Value::String("refresh".to_string()));

        self.encode_token(&claims)
    }

    /// Generate token pair (access + refresh)
    pub fn generate_token_pair(&self, user_id: &str, roles: Vec<String>) -> Result<TokenPair> {
        let access_token = self.generate_access_token(user_id, roles)?;
        let refresh_token = self.generate_refresh_token(user_id)?;

        Ok(TokenPair::new(
            access_token,
            refresh_token,
            self.config.access_token_expiration,
        ))
    }

    /// Validate and decode token
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => SecurityError::TokenExpired,
                    _ => SecurityError::TokenInvalid,
                }
            })?;

        Ok(token_data.claims)
    }

    /// Refresh access token using refresh token
    pub fn refresh_access_token(&self, refresh_token: &str) -> Result<TokenPair> {
        let claims = self.validate_token(refresh_token)?;

        // Check if it's a refresh token
        if let Some(token_type) = claims.get_custom_claim("token_type") {
            if let serde_json::Value::String(token_type_str) = token_type {
                if token_type_str != "refresh" {
                    return Err(SecurityError::TokenInvalid);
                }
            } else {
                return Err(SecurityError::TokenInvalid);
            }
        } else {
            return Err(SecurityError::TokenInvalid);
        }

        // Generate new token pair
        let roles = claims.roles.clone();
        self.generate_token_pair(&claims.sub, roles)
    }

    /// Extract token from Authorization header
    pub fn extract_token_from_header(header: &str) -> Result<&str> {
        if !header.starts_with("Bearer ") {
            return Err(SecurityError::TokenInvalid);
        }

        let token = header.trim_start_matches("Bearer ").trim();
        if token.is_empty() {
            return Err(SecurityError::TokenInvalid);
        }

        Ok(token)
    }

    /// Encode token with claims
    fn encode_token(&self, claims: &JwtClaims) -> Result<String> {
        let header = Header::new(self.validation.algorithms[0]);
        encode(&header, claims, &self.encoding_key)
            .map_err(|e| SecurityError::Jwt(e))
    }

    /// Map JwtAlgorithm to jsonwebtoken Algorithm
    fn map_algorithm(algorithm: &JwtAlgorithm) -> Result<Algorithm> {
        match algorithm {
            JwtAlgorithm::HS256 => Ok(Algorithm::HS256),
            JwtAlgorithm::HS384 => Ok(Algorithm::HS384),
            JwtAlgorithm::HS512 => Ok(Algorithm::HS512),
            JwtAlgorithm::RS256 => Ok(Algorithm::RS256),
            JwtAlgorithm::RS384 => Ok(Algorithm::RS384),
            JwtAlgorithm::RS512 => Ok(Algorithm::RS512),
            JwtAlgorithm::ES256 => Ok(Algorithm::ES256),
            JwtAlgorithm::ES384 => Ok(Algorithm::ES384),
            // ES512 is not supported in jsonwebtoken crate
            JwtAlgorithm::ES512 => Err(SecurityError::Configuration("ES512 algorithm not supported".to_string())),
        }
    }

    /// Create encoding key based on algorithm
    fn create_encoding_key(algorithm: &JwtAlgorithm, secret: &str) -> Result<EncodingKey> {
        match algorithm {
            JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512 => {
                Ok(EncodingKey::from_secret(secret.as_bytes()))
            }
            JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512 => {
                Ok(EncodingKey::from_rsa_pem(secret.as_bytes())
                    .map_err(|e| SecurityError::Configuration(format!("RSA key error: {}", e)))?)
            }
            JwtAlgorithm::ES256 | JwtAlgorithm::ES384 | JwtAlgorithm::ES512 => {
                Ok(EncodingKey::from_ec_pem(secret.as_bytes())
                    .map_err(|e| SecurityError::Configuration(format!("EC key error: {}", e)))?)
            }
        }
    }

    /// Create decoding key based on algorithm
    fn create_decoding_key(algorithm: &JwtAlgorithm, secret: &str) -> Result<DecodingKey> {
        match algorithm {
            JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512 => {
                Ok(DecodingKey::from_secret(secret.as_bytes()))
            }
            JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512 => {
                Ok(DecodingKey::from_rsa_pem(secret.as_bytes())
                    .map_err(|e| SecurityError::Configuration(format!("RSA key error: {}", e)))?)
            }
            JwtAlgorithm::ES256 | JwtAlgorithm::ES384 | JwtAlgorithm::ES512 => {
                Ok(DecodingKey::from_ec_pem(secret.as_bytes())
                    .map_err(|e| SecurityError::Configuration(format!("EC key error: {}", e)))?)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn create_test_config() -> JwtConfig {
        JwtConfig {
            algorithm: JwtAlgorithm::HS256,
            secret: "test-secret-key-that-is-long-enough-for-security-purposes".to_string(),
            issuer: "test-issuer".to_string(),
            audience: vec!["test-audience".to_string()],
            access_token_expiration: 300,   // 5 minutes
            refresh_token_expiration: 3600, // 1 hour
            leeway_seconds: 60,
            validate_exp: true,
            validate_nbf: false,
            validate_aud: true,
            validate_iss: true,
        }
    }

    #[test]
    fn test_jwt_service_creation() {
        let config = create_test_config();
        let service = JwtService::new(config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_token_generation_and_validation() {
        let config = create_test_config();
        let service = JwtService::new(config).unwrap();

        let user_id = "user123";
        let roles = vec!["admin".to_string(), "user".to_string()];

        // Generate token
        let token = service.generate_access_token(user_id, roles.clone()).unwrap();

        // Validate token
        let claims = service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.roles, roles);
        assert!(!claims.is_expired());
        assert!(claims.time_until_expiry() > 0);
    }

    #[test]
    fn test_token_pair_generation() {
        let config = create_test_config();
        let service = JwtService::new(config).unwrap();

        let user_id = "user123";
        let roles = vec!["admin".to_string()];

        let token_pair = service.generate_token_pair(user_id, roles).unwrap();

        assert!(!token_pair.access_token.is_empty());
        assert!(!token_pair.refresh_token.is_empty());
        assert_eq!(token_pair.token_type, "Bearer");
        assert_eq!(token_pair.expires_in, 300);
    }

    #[test]
    fn test_token_refresh() {
        let config = create_test_config();
        let service = JwtService::new(config).unwrap();

        let user_id = "user123";
        let roles = vec!["admin".to_string()];

        // Generate initial token pair
        let token_pair = service.generate_token_pair(user_id, roles.clone()).unwrap();

        // Refresh token
        let new_pair = service.refresh_access_token(&token_pair.refresh_token).unwrap();

        assert!(!new_pair.access_token.is_empty());
        assert!(!new_pair.refresh_token.is_empty());

        // Validate new access token
        let claims = service.validate_token(&new_pair.access_token).unwrap();
        assert_eq!(claims.sub, user_id);
        assert_eq!(claims.roles, roles);
    }

    #[test]
    fn test_expired_token() {
        let mut config = create_test_config();
        config.access_token_expiration = 1; // 1 second
        let service = JwtService::new(config).unwrap();

        let user_id = "user123";
        let roles = vec!["user".to_string()];

        let token = service.generate_access_token(user_id, roles).unwrap();

        // Wait for token to expire
        thread::sleep(Duration::from_secs(2));

        // Token should be expired
        let result = service.validate_token(&token);
        assert!(matches!(result, Err(SecurityError::TokenExpired)));
    }

    #[test]
    fn test_invalid_token() {
        let config = create_test_config();
        let service = JwtService::new(config).unwrap();

        let result = service.validate_token("invalid.token.here");
        assert!(matches!(result, Err(SecurityError::TokenInvalid)));
    }

    #[test]
    fn test_header_token_extraction() {
        assert_eq!(
            JwtService::extract_token_from_header("Bearer abc123").unwrap(),
            "abc123"
        );

        assert!(JwtService::extract_token_from_header("Bearer ").is_err());
        assert!(JwtService::extract_token_from_header("Basic abc123").is_err());
        assert!(JwtService::extract_token_from_header("abc123").is_err());
    }

    #[test]
    fn test_claims_methods() {
        let claims = JwtClaims::new(
            "user123".to_string(),
            "test-issuer".to_string(),
            vec!["test-audience".to_string()],
            vec!["admin".to_string(), "user".to_string()],
            vec!["read".to_string(), "write".to_string()],
            300,
        );

        assert!(claims.has_role("admin"));
        assert!(!claims.has_role("superuser"));
        assert!(claims.has_any_role(&["admin", "superuser"]));
        assert!(claims.has_permission("read"));
        assert!(claims.has_all_permissions(&["read", "write"]));
        assert!(!claims.has_all_permissions(&["read", "delete"]));

        // Test custom claims
        let mut claims = claims;
        claims.add_custom_claim("department".to_string(), serde_json::Value::String("engineering".to_string()));
        assert_eq!(claims.get_custom_claim("department").unwrap().as_str().unwrap(), "engineering");
    }
}

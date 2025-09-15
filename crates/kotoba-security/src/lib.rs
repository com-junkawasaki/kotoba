//! # Kotoba Security
//!
//! Comprehensive security components for Kotoba graph database system.
//!
//! This crate provides:
//! - JWT token generation and validation
//! - OAuth2/OpenID Connect integration
//! - Multi-factor authentication (TOTP)
//! - Secure password hashing
//! - Session management

pub mod jwt;
pub mod oauth2;
pub mod mfa;
pub mod password;
pub mod session;
pub mod error;
pub mod config;
pub mod capabilities;

pub use jwt::{JwtService, JwtClaims, TokenPair};
pub use oauth2::{OAuth2Service, OAuth2Provider, OAuth2Tokens};
pub use crate::config::OAuth2Config;
pub use mfa::{MfaService, MfaSecret, MfaCode};
pub use password::{PasswordService, PasswordHash};
pub use session::{SessionManager, SessionData};
pub use error::{SecurityError, Result};
pub use config::{SecurityConfig, AuthMethod};
pub use capabilities::{Capability, CapabilitySet, CapabilityService, ResourceType, Action};

use serde::{Deserialize, Serialize};

/// User identity representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub username: Option<String>,
    pub roles: Vec<String>,
    pub mfa_enabled: bool,
    pub email_verified: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Authentication result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResult {
    pub user: User,
    pub token_pair: Option<TokenPair>,
    pub mfa_required: bool,
    pub redirect_url: Option<String>,
}

/// Authorization check result
#[derive(Debug, Clone)]
pub struct AuthzResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

/// Principal for authorization decisions
#[derive(Debug, Clone)]
pub struct Principal {
    pub user_id: String,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    pub capabilities: CapabilitySet,
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
}

/// Resource for authorization checks
#[derive(Debug, Clone)]
pub struct Resource {
    pub resource_type: ResourceType,
    pub resource_id: Option<String>,
    pub action: Action,
    pub attributes: std::collections::HashMap<String, serde_json::Value>,
}

impl Resource {
    /// Helper method to get resource type as string (for backward compatibility)
    pub fn resource_type_as_str(&self) -> &str {
        match &self.resource_type {
            ResourceType::Graph => "graph",
            ResourceType::FileSystem => "filesystem",
            ResourceType::Network => "network",
            ResourceType::Environment => "environment",
            ResourceType::System => "system",
            ResourceType::Plugin => "plugin",
            ResourceType::Query => "query",
            ResourceType::Admin => "admin",
            ResourceType::User => "user",
            ResourceType::Custom(name) => name,
        }
    }

    /// Helper method to get action as string (for backward compatibility)
    pub fn action_as_str(&self) -> &str {
        match &self.action {
            Action::Read => "read",
            Action::Write => "write",
            Action::Execute => "execute",
            Action::Delete => "delete",
            Action::Create => "create",
            Action::Update => "update",
            Action::Admin => "admin",
            Action::Custom(name) => name,
        }
    }
}

/// Main security service combining all components
pub struct SecurityService {
    jwt: JwtService,
    oauth2: Option<OAuth2Service>,
    mfa: MfaService,
    password: PasswordService,
    session: SessionManager,
    capabilities: CapabilityService,
}

impl SecurityService {
    /// Create a new security service with the given configuration
    pub async fn new(config: SecurityConfig) -> Result<Self> {
        let jwt = JwtService::new(config.jwt_config)?;
        let oauth2 = if let Some(oauth2_config) = config.oauth2_config {
            Some(OAuth2Service::new(oauth2_config).await?)
        } else {
            None
        };
        let mfa = MfaService::new();
        let password = PasswordService::new();
        let session = SessionManager::new(config.session_config);
        let capabilities = CapabilityService::with_config(config.capability_config);

        Ok(Self {
            jwt,
            oauth2,
            mfa,
            password,
            session,
            capabilities,
        })
    }

    /// Authenticate a user with username/email and password
    pub async fn authenticate_local(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<AuthResult> {
        // Implementation will be added
        todo!("Implement local authentication")
    }

    /// Start OAuth2 authentication flow
    pub async fn start_oauth2_flow(&self, provider: OAuth2Provider) -> Result<String> {
        self.oauth2
            .as_ref()
            .ok_or_else(|| SecurityError::Configuration("OAuth2 not configured".to_string()))?
            .get_authorization_url(provider)
            .await
    }

    /// Complete OAuth2 authentication flow
    pub async fn complete_oauth2_flow(
        &self,
        provider: OAuth2Provider,
        code: &str,
        state: &str,
    ) -> Result<AuthResult> {
        // Implementation will be added
        todo!("Implement OAuth2 flow completion")
    }

    /// Generate MFA secret and QR code for user
    pub fn setup_mfa(&self, user_id: &str) -> Result<(String, String)> {
        self.mfa.generate_secret(user_id)
    }

    /// Verify MFA code
    pub fn verify_mfa(&self, secret: &str, code: &str) -> Result<bool> {
        self.mfa.verify_code(secret, code)
    }

    /// Validate JWT token
    pub fn validate_token(&self, token: &str) -> Result<JwtClaims> {
        self.jwt.validate_token(token)
    }

    /// Generate new token pair
    pub fn generate_tokens(&self, user_id: &str, roles: Vec<String>) -> Result<TokenPair> {
        self.jwt.generate_token_pair(user_id, roles)
    }

    /// Refresh access token
    pub fn refresh_token(&self, refresh_token: &str) -> Result<TokenPair> {
        self.jwt.refresh_access_token(refresh_token)
    }

    /// Check authorization for principal on resource using capabilities
    pub fn check_authorization(
        &self,
        principal: &Principal,
        resource: &Resource,
    ) -> AuthzResult {
        // First check capabilities (primary authorization mechanism)
        let scope = resource.resource_id.as_deref();
        let allowed = self.capabilities.check_capability(
            &principal.capabilities,
            &resource.resource_type,
            &resource.action,
            scope,
        );

        if allowed {
            return AuthzResult {
                allowed: true,
                reason: None,
            };
        }

        // Fallback to legacy role-based permissions for backward compatibility
        // This can be removed in future versions when all permissions are migrated to capabilities
        self.check_legacy_authorization(principal, resource)
    }

    /// Legacy role-based authorization (for backward compatibility)
    fn check_legacy_authorization(
        &self,
        principal: &Principal,
        resource: &Resource,
    ) -> AuthzResult {
        // Simple role-based check for backward compatibility
        // In practice, this should be migrated to capabilities

        let required_permission = format!("{}:{}", resource.resource_type_as_str(), resource.action_as_str());

        if principal.permissions.contains(&required_permission) {
            return AuthzResult {
                allowed: true,
                reason: Some("Legacy permission check passed".to_string()),
            };
        }

        // Check admin roles
        if principal.roles.contains(&"admin".to_string()) {
            return AuthzResult {
                allowed: true,
                reason: Some("Admin role override".to_string()),
            };
        }

        AuthzResult {
            allowed: false,
            reason: Some(format!("Missing capability for {}:{}", required_permission, principal.user_id)),
        }
    }

    /// Hash password
    pub fn hash_password(&self, password: &str) -> Result<PasswordHash> {
        self.password.hash_password(password)
    }

    /// Verify password against hash
    pub fn verify_password(&self, password: &str, hash: &PasswordHash) -> Result<bool> {
        self.password.verify_password(password, hash)
    }

    /// Grant capabilities to a principal
    pub fn grant_capabilities(
        &self,
        principal_caps: &CapabilitySet,
        new_caps: Vec<Capability>,
    ) -> CapabilitySet {
        self.capabilities.grant_capabilities(principal_caps, new_caps)
    }

    /// Revoke capabilities from a principal
    pub fn revoke_capabilities(
        &self,
        principal_caps: &CapabilitySet,
        caps_to_revoke: Vec<Capability>,
    ) -> CapabilitySet {
        self.capabilities.revoke_capabilities(principal_caps, caps_to_revoke)
    }

    /// Create an attenuated capability set for safer operations
    pub fn attenuate_capabilities(
        &self,
        cap_set: &CapabilitySet,
        restrictions: Vec<Capability>,
    ) -> CapabilitySet {
        self.capabilities.attenuate_capabilities(cap_set, restrictions)
    }

    /// Create a principal with specific capabilities
    pub fn create_principal_with_capabilities(
        &self,
        user_id: String,
        capabilities: CapabilitySet,
        roles: Vec<String>,
        permissions: Vec<String>,
        attributes: std::collections::HashMap<String, serde_json::Value>,
    ) -> Principal {
        Principal {
            user_id,
            roles,
            permissions,
            capabilities,
            attributes,
        }
    }

    /// Create a resource for authorization checks
    pub fn create_resource(
        &self,
        resource_type: ResourceType,
        action: Action,
        resource_id: Option<String>,
        attributes: std::collections::HashMap<String, serde_json::Value>,
    ) -> Resource {
        Resource {
            resource_type,
            resource_id,
            action,
            attributes,
        }
    }
}

/// Convenience function to create a security service
pub async fn init_security(config: SecurityConfig) -> Result<SecurityService> {
    SecurityService::new(config).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_service_creation() {
        // Test will be added once config is implemented
        // let config = SecurityConfig::default();
        // let service = SecurityService::new(config);
        // assert!(service.is_ok());
    }
}

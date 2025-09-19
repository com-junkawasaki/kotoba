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
pub mod audit;
pub mod rbac;
pub mod abac;
pub mod policy;
pub mod error;
pub mod config;
pub mod capabilities;

use policy::PolicyService;

pub use jwt::{JwtService, JwtClaims, TokenPair};
pub use oauth2::{OAuth2Service, OAuth2Provider, OAuth2Tokens};
pub use crate::config::OAuth2Config;
pub use mfa::{MfaService, MfaSecret, MfaCode};
pub use password::{PasswordService, PasswordHash};
pub use session::{SessionManager, SessionData};
pub use audit::{AuditService, AuditEvent, AuditEventType, AuditSeverity, AuditResult};
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
    audit: AuditService,
    policy: Option<PolicyService>,
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
        let audit = AuditService::new(config.audit_config);
        let policy = None; // Policy service needs to be set up separately

        Ok(Self {
            jwt,
            oauth2,
            mfa,
            password,
            session,
            capabilities,
            audit,
            policy,
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

    /// Log an audit event
    pub async fn log_audit_event(&self, event: AuditEvent) -> Result<()> {
        self.audit.log_event(event).await
    }

    /// Log authentication event
    pub async fn log_authentication(
        &self,
        user_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        result: AuditResult,
        message: &str,
    ) -> Result<()> {
        self.audit.log_authentication(user_id, ip_address, user_agent, result, message).await
    }

    /// Log authorization event
    pub async fn log_authorization(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        result: AuditResult,
        ip_address: Option<&str>,
    ) -> Result<()> {
        self.audit.log_authorization(user_id, resource, action, result, ip_address).await
    }

    /// Log data access event
    pub async fn log_data_access(
        &self,
        user_id: &str,
        resource: &str,
        action: &str,
        result: AuditResult,
        metadata: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<()> {
        self.audit.log_data_access(user_id, resource, action, result, metadata).await
    }

    /// Get audit events
    pub async fn get_audit_events(
        &self,
        start_time: Option<chrono::DateTime<chrono::Utc>>,
        end_time: Option<chrono::DateTime<chrono::Utc>>,
        event_type: Option<&AuditEventType>,
        user_id: Option<&str>,
        limit: Option<usize>,
    ) -> Result<Vec<AuditEvent>> {
        self.audit.get_events(start_time, end_time, event_type, user_id, limit).await
    }

    /// Get audit statistics
    pub async fn get_audit_statistics(&self) -> Result<crate::audit::AuditStatistics> {
        self.audit.get_statistics().await
    }

    /// Clean up old audit events
    pub async fn cleanup_audit_events(&self) -> Result<usize> {
        self.audit.cleanup_old_events().await
    }

    /// Set up policy service with RBAC and ABAC
    pub fn setup_policy_service(&mut self, config: crate::policy::PolicyEngineConfig) -> Result<()> {
        // Create RBAC service
        let rbac_service = crate::rbac::RBACService::new();

        // Create ABAC service with simple providers (can be customized later)
        let user_provider = std::sync::Arc::new(std::sync::Mutex::new(
            crate::abac::SimpleUserAttributeProvider::new()
        ));

        let resource_provider = std::sync::Arc::new(std::sync::Mutex::new(
            crate::abac::SimpleResourceAttributeProvider::new()
        ));

        let env_provider = std::sync::Arc::new(std::sync::Mutex::new(
            crate::abac::SimpleEnvironmentAttributeProvider::new()
        ));

        // For now, use the trait objects directly
        // In a real implementation, you'd want to use Arc<Mutex<>> for thread safety
        let abac_service = crate::abac::ABACService::new(
            Box::new(SimpleUserProviderWrapper(user_provider)),
            Box::new(SimpleResourceProviderWrapper(resource_provider)),
            Box::new(SimpleEnvProviderWrapper(env_provider)),
        );

        let policy_service = crate::policy::PolicyService::with_services(
            config,
            Some(rbac_service),
            Some(abac_service),
        );

        self.policy = Some(policy_service);
        Ok(())
    }

    /// Set policy service directly
    pub fn set_policy_service(&mut self, policy_service: PolicyService) {
        self.policy = Some(policy_service);
    }

    /// Check access permission using unified RBAC/ABAC policy engine
    pub async fn check_access_policy(
        &self,
        principal_id: &str,
        resource_type: &ResourceType,
        resource_id: Option<&str>,
        action: &Action,
    ) -> Result<bool> {
        let policy_service = self.policy.as_ref()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        let principal_id_string = principal_id.to_string();
        let resource_id_string = resource_id.map(|s| s.to_string());

        policy_service.check_permission(&principal_id_string, resource_type, resource_id_string.as_ref(), action).await
    }

    /// Authorize action with detailed policy decision
    pub async fn authorize_action(
        &self,
        principal_id: &str,
        resource_type: &ResourceType,
        resource_id: Option<&str>,
        action: &Action,
    ) -> Result<crate::policy::UnifiedPolicyDecision> {
        let policy_service = self.policy.as_ref()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        let principal_id_string = principal_id.to_string();
        let resource_id_string = resource_id.map(|s| s.to_string());

        policy_service.authorize(&principal_id_string, resource_type, resource_id_string.as_ref(), action).await
    }

    /// Add RBAC role
    pub fn add_role(&mut self, role: crate::rbac::Role) -> Result<()> {
        let policy_service = self.policy.as_mut()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        policy_service.add_role(role)
    }

    /// Assign role to principal
    pub fn assign_role(&mut self, assignment: crate::rbac::RoleAssignment) -> Result<()> {
        let policy_service = self.policy.as_mut()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        policy_service.assign_role(assignment)
    }

    /// Add ABAC policy
    pub fn add_policy(&mut self, policy: crate::abac::Policy) -> Result<()> {
        let policy_service = self.policy.as_mut()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        policy_service.add_policy(policy)
    }

    /// Setup common roles and policies
    pub fn setup_common_policies(&mut self) -> Result<()> {
        let policy_service = self.policy.as_mut()
            .ok_or_else(|| SecurityError::Configuration("Policy service not configured".to_string()))?;

        policy_service.setup_common_policies()
    }

    /// Get policy service for advanced operations
    pub fn policy_service(&self) -> Option<&PolicyService> {
        self.policy.as_ref()
    }

    /// Get mutable policy service
    pub fn policy_service_mut(&mut self) -> Option<&mut PolicyService> {
        self.policy.as_mut()
    }
}

// Wrapper types for thread-safe attribute providers
pub struct SimpleUserProviderWrapper(std::sync::Arc<std::sync::Mutex<crate::abac::SimpleUserAttributeProvider>>);

#[async_trait::async_trait(?Send)]
impl crate::abac::UserAttributeProvider for SimpleUserProviderWrapper {
    async fn get_attributes(&self, principal_id: &crate::abac::PrincipalId) -> Result<crate::abac::UserAttributes> {
        let provider = self.0.lock().unwrap();
        provider.get_attributes(principal_id).await
    }
}

pub struct SimpleResourceProviderWrapper(std::sync::Arc<std::sync::Mutex<crate::abac::SimpleResourceAttributeProvider>>);

#[async_trait::async_trait(?Send)]
impl crate::abac::ResourceAttributeProvider for SimpleResourceProviderWrapper {
    async fn get_attributes(&self, resource_type: &ResourceType, resource_id: Option<&crate::abac::ResourceId>) -> Result<crate::abac::ResourceAttributes> {
        let provider = self.0.lock().unwrap();
        provider.get_attributes(resource_type, resource_id).await
    }
}

pub struct SimpleEnvProviderWrapper(std::sync::Arc<std::sync::Mutex<crate::abac::SimpleEnvironmentAttributeProvider>>);

#[async_trait::async_trait(?Send)]
impl crate::abac::EnvironmentAttributeProvider for SimpleEnvProviderWrapper {
    async fn get_attributes(&self) -> Result<crate::abac::EnvironmentAttributes> {
        let provider = self.0.lock().unwrap();
        provider.get_attributes().await
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

    #[test]
    fn test_user_creation() {
        let created_at = chrono::Utc::now();
        let updated_at = chrono::Utc::now();

        let user = User {
            id: "user123".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            roles: vec!["user".to_string(), "editor".to_string()],
            mfa_enabled: true,
            email_verified: false,
            created_at,
            updated_at,
        };

        assert_eq!(user.id, "user123");
        assert_eq!(user.email, "test@example.com");
        assert_eq!(user.username, Some("testuser".to_string()));
        assert_eq!(user.roles.len(), 2);
        assert!(user.mfa_enabled);
        assert!(!user.email_verified);
    }

    #[test]
    fn test_auth_result_creation() {
        let user = User {
            id: "user123".to_string(),
            email: "test@example.com".to_string(),
            username: None,
            roles: vec![],
            mfa_enabled: false,
            email_verified: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let auth_result = AuthResult {
            user: user.clone(),
            token_pair: None,
            mfa_required: false,
            redirect_url: None,
        };

        assert_eq!(auth_result.user.id, "user123");
        assert!(auth_result.token_pair.is_none());
        assert!(!auth_result.mfa_required);
        assert!(auth_result.redirect_url.is_none());
    }

    #[test]
    fn test_authz_result_creation() {
        let allowed_result = AuthzResult {
            allowed: true,
            reason: None,
        };

        let denied_result = AuthzResult {
            allowed: false,
            reason: Some("Insufficient permissions".to_string()),
        };

        assert!(allowed_result.allowed);
        assert!(allowed_result.reason.is_none());

        assert!(!denied_result.allowed);
        assert_eq!(denied_result.reason, Some("Insufficient permissions".to_string()));
    }

    #[test]
    fn test_principal_creation() {
        use std::collections::HashMap;

        let mut attributes = HashMap::new();
        attributes.insert("department".to_string(), serde_json::json!("engineering"));

        let principal = Principal {
            user_id: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            capabilities: CapabilitySet::new(vec![]),
            attributes,
        };

        assert_eq!(principal.user_id, "user123");
        assert_eq!(principal.roles.len(), 1);
        assert_eq!(principal.permissions.len(), 1);
        assert_eq!(principal.attributes.get("department"), Some(&serde_json::json!("engineering")));
    }

    #[test]
    fn test_resource_creation() {
        use std::collections::HashMap;

        let mut attributes = HashMap::new();
        attributes.insert("owner".to_string(), serde_json::json!("user123"));

        let resource = Resource {
            resource_type: ResourceType::Graph,
            resource_id: Some("graph123".to_string()),
            action: Action::Read,
            attributes,
        };

        assert!(matches!(resource.resource_type, ResourceType::Graph));
        assert_eq!(resource.resource_id, Some("graph123".to_string()));
        assert!(matches!(resource.action, Action::Read));
        assert_eq!(resource.attributes.get("owner"), Some(&serde_json::json!("user123")));
    }

    #[test]
    fn test_resource_helper_methods() {
        let resource = Resource {
            resource_type: ResourceType::Graph,
            resource_id: None,
            action: Action::Read,
            attributes: std::collections::HashMap::new(),
        };

        assert_eq!(resource.resource_type_as_str(), "graph");
        assert_eq!(resource.action_as_str(), "read");
    }

    #[test]
    fn test_user_serialization() {
        let user = User {
            id: "user123".to_string(),
            email: "test@example.com".to_string(),
            username: Some("testuser".to_string()),
            roles: vec!["user".to_string()],
            mfa_enabled: true,
            email_verified: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&user);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("user123"));
        assert!(json_str.contains("test@example.com"));
        assert!(json_str.contains("testuser"));
        assert!(json_str.contains("user"));
        assert!(json_str.contains("true"));
    }

    #[test]
    fn test_auth_result_serialization() {
        let user = User {
            id: "user123".to_string(),
            email: "test@example.com".to_string(),
            username: None,
            roles: vec![],
            mfa_enabled: false,
            email_verified: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let auth_result = AuthResult {
            user: user.clone(),
            token_pair: None,
            mfa_required: true,
            redirect_url: None,
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&auth_result);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("user123"));
        assert!(json_str.contains("true"));
        assert!(json_str.contains("null"));
    }

    #[test]
    fn test_principal_serialization() {
        use std::collections::HashMap;

        let principal = Principal {
            user_id: "user123".to_string(),
            roles: vec!["user".to_string()],
            permissions: vec!["read".to_string()],
            capabilities: CapabilitySet::new(vec![]),
            attributes: HashMap::new(),
        };

        // Test JSON serialization
        let json_result = serde_json::to_string(&principal);
        assert!(json_result.is_ok());

        let json_str = json_result.unwrap();
        assert!(json_str.contains("user123"));
        assert!(json_str.contains("user"));
        assert!(json_str.contains("read"));
    }

    #[test]
    fn test_user_clone() {
        let user = User {
            id: "user123".to_string(),
            email: "test@example.com".to_string(),
            username: None,
            roles: vec![],
            mfa_enabled: false,
            email_verified: false,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        let cloned = user.clone();

        assert_eq!(user.id, cloned.id);
        assert_eq!(user.email, cloned.email);
        assert_eq!(user.username, cloned.username);
        assert_eq!(user.roles, cloned.roles);
        assert_eq!(user.mfa_enabled, cloned.mfa_enabled);
        assert_eq!(user.email_verified, cloned.email_verified);
    }

    #[test]
    fn test_resource_type_variants() {
        let resource = Resource {
            resource_type: ResourceType::Graph,
            resource_id: None,
            action: Action::Read,
            attributes: std::collections::HashMap::new(),
        };
        assert_eq!(resource.resource_type_as_str(), "graph");

        let resource = Resource {
            resource_type: ResourceType::FileSystem,
            resource_id: None,
            action: Action::Write,
            attributes: std::collections::HashMap::new(),
        };
        assert_eq!(resource.resource_type_as_str(), "filesystem");

        let resource = Resource {
            resource_type: ResourceType::Custom("custom_type".to_string()),
            resource_id: None,
            action: Action::Read,
            attributes: std::collections::HashMap::new(),
        };
        assert_eq!(resource.resource_type_as_str(), "custom_type");
    }

    #[test]
    fn test_action_variants() {
        let actions = vec![
            (Action::Read, "read"),
            (Action::Write, "write"),
            (Action::Execute, "execute"),
            (Action::Delete, "delete"),
            (Action::Create, "create"),
            (Action::Update, "update"),
            (Action::Admin, "admin"),
            (Action::Custom("custom_action".to_string()), "custom_action"),
        ];

        for (action, expected_str) in actions {
            let resource = Resource {
                resource_type: ResourceType::Graph,
                resource_id: None,
                action,
                attributes: std::collections::HashMap::new(),
            };
            assert_eq!(resource.action_as_str(), expected_str);
        }
    }

    #[test]
    fn test_capability_creation() {
        let capability = Capability {
            resource_type: ResourceType::Graph,
            action: Action::Read,
            scope: Some("graph123".to_string()),
        };

        assert!(matches!(capability.resource_type, ResourceType::Graph));
        assert!(matches!(capability.action, Action::Read));
        assert_eq!(capability.scope, Some("graph123".to_string()));
    }

    #[test]
    fn test_capability_set_creation() {
        let capabilities = vec![
            Capability {
                resource_type: ResourceType::Graph,
                action: Action::Read,
                scope: None,
            },
            Capability {
                resource_type: ResourceType::FileSystem,
                action: Action::Write,
                scope: Some("*.txt".to_string()),
            },
        ];

        let capability_set = CapabilitySet::new(capabilities);

        assert_eq!(capability_set.capabilities.len(), 2);
        assert!(matches!(capability_set.capabilities[0].resource_type, ResourceType::Graph));
        assert!(matches!(capability_set.capabilities[1].action, Action::Write));
    }
}

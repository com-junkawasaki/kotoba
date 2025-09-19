//! Unified Policy Engine combining RBAC and ABAC
//!
//! This module provides a comprehensive policy engine that integrates:
//! - Role-Based Access Control (RBAC) for role-based permissions
//! - Attribute-Based Access Control (ABAC) for fine-grained attribute-based policies
//! - Unified policy evaluation with configurable precedence

use crate::capabilities::{CapabilitySet, ResourceType, Action};
use crate::error::{SecurityError, Result};
use crate::rbac::{RBACService, RoleAssignment, PrincipalId};
use crate::abac::{ABACService, PolicyDecision, UserAttributeProvider, ResourceAttributeProvider, EnvironmentAttributeProvider};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Policy evaluation mode
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyMode {
    /// RBAC only
    RBACOnly,
    /// ABAC only
    ABACOnly,
    /// RBAC first, then ABAC if RBAC doesn't apply
    RBACFirst,
    /// ABAC first, then RBAC if ABAC doesn't apply
    ABACFirst,
    /// Both RBAC and ABAC, deny takes precedence
    Combined,
}

/// Unified policy decision
#[derive(Debug, Clone, PartialEq)]
pub enum UnifiedPolicyDecision {
    Allow,
    Deny,
    NotApplicable,
}

/// Unified Policy Engine Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEngineConfig {
    pub mode: PolicyMode,
    pub rbac_enabled: bool,
    pub abac_enabled: bool,
    pub default_deny: bool, // If true, deny access when no policies apply
}

impl Default for PolicyEngineConfig {
    fn default() -> Self {
        Self {
            mode: PolicyMode::Combined,
            rbac_enabled: true,
            abac_enabled: true,
            default_deny: false,
        }
    }
}

/// Unified Policy Engine combining RBAC and ABAC
pub struct UnifiedPolicyEngine {
    config: PolicyEngineConfig,
    rbac_service: Option<RBACService>,
    abac_service: Option<ABACService>,
}

impl UnifiedPolicyEngine {
    /// Create a new unified policy engine
    pub fn new(config: PolicyEngineConfig) -> Self {
        Self {
            config,
            rbac_service: None,
            abac_service: None,
        }
    }

    /// Create with RBAC service
    pub fn with_rbac(mut self, rbac_service: RBACService) -> Self {
        self.rbac_service = Some(rbac_service);
        self
    }

    /// Create with ABAC service
    pub fn with_abac(mut self, abac_service: ABACService) -> Self {
        self.abac_service = Some(abac_service);
        self
    }

    /// Set RBAC service
    pub fn set_rbac_service(&mut self, rbac_service: RBACService) {
        self.rbac_service = Some(rbac_service);
    }

    /// Set ABAC service
    pub fn set_abac_service(&mut self, abac_service: ABACService) {
        self.abac_service = Some(abac_service);
    }

    /// Evaluate access request using unified policy evaluation
    pub async fn evaluate_access(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        match self.config.mode {
            PolicyMode::RBACOnly => {
                self.evaluate_rbac_only(principal_id, resource_type, action)
            }
            PolicyMode::ABACOnly => {
                self.evaluate_abac_only(principal_id, resource_type, resource_id, action).await
            }
            PolicyMode::RBACFirst => {
                self.evaluate_rbac_first(principal_id, resource_type, resource_id, action).await
            }
            PolicyMode::ABACFirst => {
                self.evaluate_abac_first(principal_id, resource_type, resource_id, action).await
            }
            PolicyMode::Combined => {
                self.evaluate_combined(principal_id, resource_type, resource_id, action).await
            }
        }
    }

    /// RBAC-only evaluation
    fn evaluate_rbac_only(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        if !self.config.rbac_enabled {
            return Ok(UnifiedPolicyDecision::NotApplicable);
        }

        let rbac_service = self.rbac_service.as_ref()
            .ok_or_else(|| SecurityError::Configuration("RBAC service not configured".to_string()))?;

        match rbac_service.check_permission(principal_id, resource_type, action, None)? {
            true => Ok(UnifiedPolicyDecision::Allow),
            false => Ok(UnifiedPolicyDecision::Deny),
        }
    }

    /// ABAC-only evaluation
    async fn evaluate_abac_only(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        if !self.config.abac_enabled {
            return Ok(UnifiedPolicyDecision::NotApplicable);
        }

        let abac_service = self.abac_service.as_ref()
            .ok_or_else(|| SecurityError::Configuration("ABAC service not configured".to_string()))?;

        match abac_service.check_access(principal_id, resource_type, resource_id, action).await? {
            PolicyDecision::Allow => Ok(UnifiedPolicyDecision::Allow),
            PolicyDecision::Deny => Ok(UnifiedPolicyDecision::Deny),
            PolicyDecision::NotApplicable => Ok(UnifiedPolicyDecision::NotApplicable),
        }
    }

    /// RBAC first, then ABAC if RBAC doesn't apply
    async fn evaluate_rbac_first(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        // Try RBAC first
        let rbac_result = self.evaluate_rbac_only(principal_id, resource_type, action)?;

        match rbac_result {
            UnifiedPolicyDecision::Allow | UnifiedPolicyDecision::Deny => {
                // RBAC made a decision
                Ok(rbac_result)
            }
            UnifiedPolicyDecision::NotApplicable => {
                // RBAC doesn't apply, try ABAC
                self.evaluate_abac_only(principal_id, resource_type, resource_id, action).await
            }
        }
    }

    /// ABAC first, then RBAC if ABAC doesn't apply
    async fn evaluate_abac_first(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        // Try ABAC first
        let abac_result = self.evaluate_abac_only(principal_id, resource_type, resource_id, action).await?;

        match abac_result {
            UnifiedPolicyDecision::Allow | UnifiedPolicyDecision::Deny => {
                // ABAC made a decision
                Ok(abac_result)
            }
            UnifiedPolicyDecision::NotApplicable => {
                // ABAC doesn't apply, try RBAC
                self.evaluate_rbac_only(principal_id, resource_type, action)
            }
        }
    }

    /// Combined evaluation: both RBAC and ABAC
    async fn evaluate_combined(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        let rbac_result = if self.config.rbac_enabled {
            Some(self.evaluate_rbac_only(principal_id, resource_type, action)?)
        } else {
            None
        };

        let abac_result = if self.config.abac_enabled {
            Some(self.evaluate_abac_only(principal_id, resource_type, resource_id, action).await?)
        } else {
            None
        };

        // Combine results with deny taking precedence
        let mut has_allow = false;
        let mut has_deny = false;

        if let Some(UnifiedPolicyDecision::Allow) = rbac_result {
            has_allow = true;
        }
        if let Some(UnifiedPolicyDecision::Deny) = rbac_result {
            has_deny = true;
        }

        if let Some(UnifiedPolicyDecision::Allow) = abac_result {
            has_allow = true;
        }
        if let Some(UnifiedPolicyDecision::Deny) = abac_result {
            has_deny = true;
        }

        // Deny takes precedence over allow
        if has_deny {
            Ok(UnifiedPolicyDecision::Deny)
        } else if has_allow {
            Ok(UnifiedPolicyDecision::Allow)
        } else if rbac_result.is_some() || abac_result.is_some() {
            // At least one system was enabled and both returned NotApplicable
            if self.config.default_deny {
                Ok(UnifiedPolicyDecision::Deny)
            } else {
                Ok(UnifiedPolicyDecision::NotApplicable)
            }
        } else {
            // No systems enabled
            Ok(UnifiedPolicyDecision::NotApplicable)
        }
    }

    /// Get effective capabilities for a principal (RBAC only)
    pub fn get_principal_capabilities(&self, principal_id: &PrincipalId) -> Result<CapabilitySet> {
        let rbac_service = self.rbac_service.as_ref()
            .ok_or_else(|| SecurityError::Configuration("RBAC service not configured".to_string()))?;

        rbac_service.get_principal_capabilities(principal_id)
    }

    /// Add RBAC role
    pub fn add_rbac_role(&mut self, role: crate::rbac::Role) -> Result<()> {
        let rbac_service = self.rbac_service.as_mut()
            .ok_or_else(|| SecurityError::Configuration("RBAC service not configured".to_string()))?;

        rbac_service.add_role(role)
    }

    /// Assign RBAC role
    pub fn assign_rbac_role(&mut self, assignment: RoleAssignment) -> Result<()> {
        let rbac_service = self.rbac_service.as_mut()
            .ok_or_else(|| SecurityError::Configuration("RBAC service not configured".to_string()))?;

        rbac_service.assign_role(assignment)
    }

    /// Add ABAC policy
    pub fn add_abac_policy(&mut self, policy: crate::abac::Policy) -> Result<()> {
        let abac_service = self.abac_service.as_mut()
            .ok_or_else(|| SecurityError::Configuration("ABAC service not configured".to_string()))?;

        abac_service.add_policy(policy)
    }

    /// Setup common policies for both RBAC and ABAC
    pub fn setup_common_policies(&mut self) -> Result<()> {
        // Setup RBAC roles
        if let Some(rbac_service) = &mut self.rbac_service {
            rbac_service.create_common_roles()?;
        }

        // Setup ABAC policies
        if let Some(abac_service) = &mut self.abac_service {
            abac_service.setup_common_policies()?;
        }

        Ok(())
    }

    /// Get configuration
    pub fn config(&self) -> &PolicyEngineConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: PolicyEngineConfig) {
        self.config = config;
    }
}

impl Default for UnifiedPolicyEngine {
    fn default() -> Self {
        Self::new(PolicyEngineConfig::default())
    }
}

/// High-level Policy Service for application integration
pub struct PolicyService {
    engine: UnifiedPolicyEngine,
}

impl PolicyService {
    /// Create a new policy service with default configuration
    pub fn new() -> Self {
        Self {
            engine: UnifiedPolicyEngine::new(PolicyEngineConfig::default()),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: PolicyEngineConfig) -> Self {
        Self {
            engine: UnifiedPolicyEngine::new(config),
        }
    }

    /// Initialize with RBAC and ABAC services
    pub fn with_services(
        config: PolicyEngineConfig,
        rbac_service: Option<RBACService>,
        abac_service: Option<ABACService>,
    ) -> Self {
        let mut engine = UnifiedPolicyEngine::new(config);

        if let Some(rbac) = rbac_service {
            engine.set_rbac_service(rbac);
        }

        if let Some(abac) = abac_service {
            engine.set_abac_service(abac);
        }

        Self { engine }
    }

    /// Check access permission
    pub async fn check_permission(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<bool> {
        match self.engine.evaluate_access(principal_id, resource_type, resource_id, action).await? {
            UnifiedPolicyDecision::Allow => Ok(true),
            UnifiedPolicyDecision::Deny => Ok(false),
            UnifiedPolicyDecision::NotApplicable => {
                // Default behavior when no policies apply
                Ok(!self.engine.config().default_deny)
            }
        }
    }

    /// Authorize action with detailed result
    pub async fn authorize(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&crate::abac::ResourceId>,
        action: &Action,
    ) -> Result<UnifiedPolicyDecision> {
        self.engine.evaluate_access(principal_id, resource_type, resource_id, action).await
    }

    /// Add RBAC role
    pub fn add_role(&mut self, role: crate::rbac::Role) -> Result<()> {
        self.engine.add_rbac_role(role)
    }

    /// Assign role to principal
    pub fn assign_role(&mut self, assignment: RoleAssignment) -> Result<()> {
        self.engine.assign_rbac_role(assignment)
    }

    /// Add ABAC policy
    pub fn add_policy(&mut self, policy: crate::abac::Policy) -> Result<()> {
        self.engine.add_abac_policy(policy)
    }

    /// Setup common roles and policies
    pub fn setup_common_policies(&mut self) -> Result<()> {
        self.engine.setup_common_policies()
    }

    /// Get policy engine for advanced operations
    pub fn engine(&self) -> &UnifiedPolicyEngine {
        &self.engine
    }

    /// Get mutable policy engine
    pub fn engine_mut(&mut self) -> &mut UnifiedPolicyEngine {
        &mut self.engine
    }
}

impl Default for PolicyService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::abac::{SimpleUserAttributeProvider, SimpleResourceAttributeProvider, SimpleEnvironmentAttributeProvider, UserAttributes, ResourceAttributes, AttributeValue};
    use crate::rbac::Role;

    #[test]
    fn test_policy_engine_config() {
        let config = PolicyEngineConfig::default();
        assert_eq!(config.mode, PolicyMode::Combined);
        assert!(config.rbac_enabled);
        assert!(config.abac_enabled);
    }

    #[tokio::test]
    async fn test_unified_policy_engine_rbac_only() {
        let config = PolicyEngineConfig {
            mode: PolicyMode::RBACOnly,
            rbac_enabled: true,
            abac_enabled: false,
            default_deny: false,
        };

        let mut rbac_service = RBACService::new();
        let role = Role::new("reader".to_string(), "Reader".to_string());
        rbac_service.add_role(role).unwrap();

        let mut engine = UnifiedPolicyEngine::new(config).with_rbac(rbac_service);

        // Should return NotApplicable since no roles are assigned
        let result = engine.evaluate_access(
            &"user1".to_string(),
            &ResourceType::Graph,
            None,
            &Action::Read,
        ).await.unwrap();

        assert_eq!(result, UnifiedPolicyDecision::Deny);
    }

    #[tokio::test]
    async fn test_unified_policy_engine_abac_only() {
        let config = PolicyEngineConfig {
            mode: PolicyMode::ABACOnly,
            rbac_enabled: false,
            abac_enabled: true,
            default_deny: false,
        };

        let user_provider = Box::new(
            SimpleUserAttributeProvider::new()
                .add_user(
                    "user1".to_string(),
                    UserAttributes::new()
                        .with_attribute("role".to_string(), AttributeValue::String("admin".to_string())),
                )
        );

        let resource_provider = Box::new(
            SimpleResourceAttributeProvider::new()
                .add_resource(
                    "graph".to_string(),
                    ResourceAttributes::new(ResourceType::Graph, None),
                )
        );

        let env_provider = Box::new(SimpleEnvironmentAttributeProvider::new());

        let abac_service = ABACService::new(user_provider, resource_provider, env_provider);
        let mut engine = UnifiedPolicyEngine::new(config).with_abac(abac_service);

        // Should return Allow due to admin policy
        let result = engine.evaluate_access(
            &"user1".to_string(),
            &ResourceType::Graph,
            None,
            &Action::Read,
        ).await.unwrap();

        assert_eq!(result, UnifiedPolicyDecision::Allow);
    }

    #[tokio::test]
    async fn test_policy_service() {
        let config = PolicyEngineConfig {
            mode: PolicyMode::Combined,
            rbac_enabled: true,
            abac_enabled: true,
            default_deny: false,
        };

        let user_provider = Box::new(
            SimpleUserAttributeProvider::new()
                .add_user(
                    "user1".to_string(),
                    UserAttributes::new()
                        .with_attribute("role".to_string(), AttributeValue::String("admin".to_string())),
                )
        );

        let resource_provider = Box::new(
            SimpleResourceAttributeProvider::new()
                .add_resource(
                    "graph".to_string(),
                    ResourceAttributes::new(ResourceType::Graph, None),
                )
        );

        let env_provider = Box::new(SimpleEnvironmentAttributeProvider::new());

        let abac_service = ABACService::new(user_provider, resource_provider, env_provider);
        let rbac_service = RBACService::new();

        let mut policy_service = PolicyService::with_services(
            config,
            Some(rbac_service),
            Some(abac_service),
        );

        policy_service.setup_common_policies().unwrap();

        // Check permission
        let allowed = policy_service.check_permission(
            &"user1".to_string(),
            &ResourceType::Graph,
            None,
            &Action::Read,
        ).await.unwrap();

        assert!(allowed);
    }
}

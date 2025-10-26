//! Kotoba Security - Pure Kernel & Effects Shell Architecture
//!
//! ## Pure Kernel & Effects Shell Architecture
//!
//! This crate provides security services with clear separation:
//!
//! - **Pure Kernel**: Security policies, access control rules, validation logic
//! - **Effects Shell**: Authentication, authorization, token management, external services

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pure security principal (user/role identity)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Principal {
    /// Principal ID
    pub id: String,
    /// Principal type
    pub principal_type: PrincipalType,
    /// Principal attributes
    pub attributes: HashMap<String, String>,
}

impl Principal {
    /// Create a user principal
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::User,
            attributes: HashMap::new(),
        }
    }

    /// Create a service principal
    pub fn service(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            principal_type: PrincipalType::Service,
            attributes: HashMap::new(),
        }
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Principal types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrincipalType {
    User,
    Service,
    System,
}

/// Pure resource representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Resource {
    /// Resource type (e.g., "document", "api")
    pub resource_type: String,
    /// Resource identifier
    pub resource_id: String,
    /// Resource attributes
    pub attributes: HashMap<String, String>,
}

impl Resource {
    /// Create a new resource
    pub fn new(resource_type: impl Into<String>, resource_id: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            attributes: HashMap::new(),
        }
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Pure security action
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Action {
    Read,
    Write,
    Delete,
    Execute,
    Admin,
    Custom(String),
}

/// Pure access request
#[derive(Debug, Clone, PartialEq)]
pub struct AccessRequest {
    /// Principal making the request
    pub principal: Principal,
    /// Action being requested
    pub action: Action,
    /// Resource being accessed
    pub resource: Resource,
    /// Request context
    pub context: HashMap<String, String>,
}

impl AccessRequest {
    /// Create a new access request
    pub fn new(principal: Principal, action: Action, resource: Resource) -> Self {
        Self {
            principal,
            action,
            resource,
            context: HashMap::new(),
        }
    }

    /// Add context
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }
}

/// Pure security policy
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPolicy {
    /// Policy ID
    pub id: String,
    /// Policy name
    pub name: String,
    /// Policy rules
    pub rules: Vec<PolicyRule>,
}

impl SecurityPolicy {
    /// Create a new policy
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            rules: vec![],
        }
    }

    /// Add a rule
    pub fn with_rule(mut self, rule: PolicyRule) -> Self {
        self.rules.push(rule);
        self
    }
}

/// Pure policy rule
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyRule {
    /// Rule effect (allow/deny)
    pub effect: PolicyEffect,
    /// Principal constraints
    pub principals: Vec<PrincipalConstraint>,
    /// Action constraints
    pub actions: Vec<ActionConstraint>,
    /// Resource constraints
    pub resources: Vec<ResourceConstraint>,
    /// Additional conditions
    pub conditions: Vec<PolicyCondition>,
}

impl PolicyRule {
    /// Create an allow rule
    pub fn allow() -> Self {
        Self {
            effect: PolicyEffect::Allow,
            principals: vec![],
            actions: vec![],
            resources: vec![],
            conditions: vec![],
        }
    }

    /// Create a deny rule
    pub fn deny() -> Self {
        Self {
            effect: PolicyEffect::Deny,
            principals: vec![],
            actions: vec![],
            resources: vec![],
            conditions: vec![],
        }
    }

    /// Add principal constraint
    pub fn with_principal(mut self, constraint: PrincipalConstraint) -> Self {
        self.principals.push(constraint);
        self
    }

    /// Add action constraint
    pub fn with_action(mut self, constraint: ActionConstraint) -> Self {
        self.actions.push(constraint);
        self
    }

    /// Add resource constraint
    pub fn with_resource(mut self, constraint: ResourceConstraint) -> Self {
        self.resources.push(constraint);
        self
    }
}

/// Policy effect
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Principal constraint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PrincipalConstraint {
    /// Match specific principal ID
    PrincipalId(String),
    /// Match principal type
    PrincipalType(PrincipalType),
    /// Match principal attribute
    Attribute(String, String),
}

/// Action constraint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionConstraint {
    /// Match specific action
    Action(Action),
    /// Match action pattern
    ActionPattern(String),
}

/// Resource constraint
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ResourceConstraint {
    /// Match resource type
    ResourceType(String),
    /// Match resource ID
    ResourceId(String),
    /// Match resource attribute
    Attribute(String, String),
}

/// Policy condition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PolicyCondition {
    /// Time-based condition
    TimeRange(String, String),
    /// IP-based condition
    IpRange(String),
    /// Custom condition
    Custom(String, serde_json::Value),
}

/// Pure access decision
#[derive(Debug, Clone, PartialEq)]
pub enum AccessDecision {
    /// Access allowed
    Allow,
    /// Access denied
    Deny(String), // reason
}

/// Pure policy evaluator - no side effects
pub struct PurePolicyEvaluator {
    policies: Vec<SecurityPolicy>,
}

impl PurePolicyEvaluator {
    /// Create a new policy evaluator
    pub fn new(policies: Vec<SecurityPolicy>) -> Self {
        Self { policies }
    }

    /// Evaluate access request (pure function)
    pub fn evaluate(&self, request: &AccessRequest) -> AccessDecision {
        // Default to deny
        let mut final_decision = AccessDecision::Deny("No matching policy".to_string());

        for policy in &self.policies {
            for rule in &policy.rules {
                if self.rule_matches(request, rule) {
                    match rule.effect {
                        PolicyEffect::Allow => {
                            final_decision = AccessDecision::Allow;
                        }
                        PolicyEffect::Deny => {
                            return AccessDecision::Deny(format!("Policy '{}' denies access", policy.name));
                        }
                    }
                }
            }
        }

        final_decision
    }

    /// Check if rule matches request (pure function)
    fn rule_matches(&self, request: &AccessRequest, rule: &PolicyRule) -> bool {
        // Check principals
        if !rule.principals.is_empty() &&
           !rule.principals.iter().any(|c| self.constraint_matches_principal(c, &request.principal)) {
            return false;
        }

        // Check actions
        if !rule.actions.is_empty() &&
           !rule.actions.iter().any(|c| self.constraint_matches_action(c, &request.action)) {
            return false;
        }

        // Check resources
        if !rule.resources.is_empty() &&
           !rule.resources.iter().any(|c| self.constraint_matches_resource(c, &request.resource)) {
            return false;
        }

        // Check conditions
        if !rule.conditions.is_empty() &&
           !rule.conditions.iter().all(|c| self.condition_matches(c, request)) {
            return false;
        }

        true
    }

    fn constraint_matches_principal(&self, constraint: &PrincipalConstraint, principal: &Principal) -> bool {
        match constraint {
            PrincipalConstraint::PrincipalId(id) => principal.id == *id,
            PrincipalConstraint::PrincipalType(pt) => principal.principal_type == *pt,
            PrincipalConstraint::Attribute(key, value) =>
                principal.attributes.get(key).map_or(false, |v| v == value),
        }
    }

    fn constraint_matches_action(&self, constraint: &ActionConstraint, action: &Action) -> bool {
        match constraint {
            ActionConstraint::Action(a) => a == action,
            ActionConstraint::ActionPattern(pattern) => {
                // Simple wildcard matching
                pattern == "*" || format!("{:?}", action).to_lowercase().contains(&pattern.to_lowercase())
            }
        }
    }

    fn constraint_matches_resource(&self, constraint: &ResourceConstraint, resource: &Resource) -> bool {
        match constraint {
            ResourceConstraint::ResourceType(rt) => resource.resource_type == *rt,
            ResourceConstraint::ResourceId(id) => resource.resource_id == *id,
            ResourceConstraint::Attribute(key, value) =>
                resource.attributes.get(key).map_or(false, |v| v == value),
        }
    }

    fn condition_matches(&self, condition: &PolicyCondition, request: &AccessRequest) -> bool {
        match condition {
            PolicyCondition::TimeRange(_, _) => {
                // Would check current time against range
                true // simplified
            }
            PolicyCondition::IpRange(_) => {
                // Would check request IP against range
                true // simplified
            }
            PolicyCondition::Custom(_, _) => {
                // Would evaluate custom condition
                true // simplified
            }
        }
    }

    /// Validate policies (pure function)
    pub fn validate_policies(&self) -> Result<(), SecurityError> {
        for policy in &self.policies {
            if policy.id.is_empty() {
                return Err(SecurityError::InvalidPolicy("Empty policy ID".to_string()));
            }

            for rule in &policy.rules {
                if rule.principals.is_empty() && rule.actions.is_empty() && rule.resources.is_empty() {
                    return Err(SecurityError::InvalidPolicy("Rule has no constraints".to_string()));
                }
            }
        }

        Ok(())
    }
}

/// Effects Shell security service
pub struct SecurityService {
    evaluator: PurePolicyEvaluator,
}

impl SecurityService {
    /// Create a new security service
    pub fn new(policies: Vec<SecurityPolicy>) -> Result<Self, SecurityError> {
        let evaluator = PurePolicyEvaluator::new(policies);
        evaluator.validate_policies()?;

        Ok(Self { evaluator })
    }

    /// Authorize access request (effects: may involve external services)
    pub async fn authorize(&self, request: AccessRequest) -> Result<AuthorizationResult, SecurityError> {
        // Pure evaluation
        let decision = self.evaluator.evaluate(&request);

        match decision {
            AccessDecision::Allow => {
                // In real implementation, might log access, update metrics, etc.
                Ok(AuthorizationResult::Allowed)
            }
            AccessDecision::Deny(reason) => {
                // In real implementation, might log denial, trigger alerts, etc.
                Ok(AuthorizationResult::Denied(reason))
            }
        }
    }

    /// Authenticate principal (effects: external auth services, token validation)
    pub async fn authenticate(&self, credentials: &AuthCredentials) -> Result<AuthenticationResult, SecurityError> {
        // In real implementation, this would validate against external auth service
        match credentials {
            AuthCredentials::Token(token) if token == "valid_token" => {
                Ok(AuthenticationResult::Success(Principal::user("test_user")))
            }
            _ => Ok(AuthenticationResult::Failed("Invalid credentials".to_string())),
        }
    }
}

/// Authorization result
#[derive(Debug, Clone)]
pub enum AuthorizationResult {
    Allowed,
    Denied(String),
}

/// Authentication credentials
#[derive(Debug, Clone)]
pub enum AuthCredentials {
    Token(String),
    UsernamePassword(String, String),
    ApiKey(String),
}

/// Authentication result
#[derive(Debug, Clone)]
pub enum AuthenticationResult {
    Success(Principal),
    Failed(String),
}

/// Security errors
#[derive(Debug, Clone)]
pub enum SecurityError {
    InvalidPolicy(String),
    AuthenticationFailed(String),
    AuthorizationFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_policy_evaluation() {
        let policy = SecurityPolicy::new("test_policy", "Test Policy")
            .with_rule(
                PolicyRule::allow()
                    .with_principal(PrincipalConstraint::PrincipalType(PrincipalType::User))
                    .with_action(ActionConstraint::Action(Action::Read))
                    .with_resource(ResourceConstraint::ResourceType("document".to_string()))
            );

        let evaluator = PurePolicyEvaluator::new(vec![policy]);

        let request = AccessRequest::new(
            Principal::user("alice"),
            Action::Read,
            Resource::new("document", "doc1"),
        );

        let decision = evaluator.evaluate(&request);
        assert!(matches!(decision, AccessDecision::Allow));
    }

    #[test]
    fn test_pure_policy_deny() {
        let policy = SecurityPolicy::new("deny_policy", "Deny Policy")
            .with_rule(
                PolicyRule::deny()
                    .with_principal(PrincipalConstraint::PrincipalId("bad_user".to_string()))
            );

        let evaluator = PurePolicyEvaluator::new(vec![policy]);

        let request = AccessRequest::new(
            Principal::user("bad_user"),
            Action::Read,
            Resource::new("document", "doc1"),
        );

        let decision = evaluator.evaluate(&request);
        assert!(matches!(decision, AccessDecision::Deny(_)));
    }

    #[tokio::test]
    async fn test_security_service() {
        let policies = vec![SecurityPolicy::new("allow_users", "Allow Users")
            .with_rule(PolicyRule::allow().with_principal(PrincipalConstraint::PrincipalType(PrincipalType::User)))];

        let service = SecurityService::new(policies).unwrap();

        let request = AccessRequest::new(
            Principal::user("alice"),
            Action::Read,
            Resource::new("document", "doc1"),
        );

        let result = service.authorize(request).await.unwrap();
        assert!(matches!(result, AuthorizationResult::Allowed));
    }

    #[tokio::test]
    async fn test_authentication() {
        let policies = vec![];
        let service = SecurityService::new(policies).unwrap();

        let result = service.authenticate(&AuthCredentials::Token("valid_token".to_string())).await.unwrap();
        assert!(matches!(result, AuthenticationResult::Success(_)));

        let result = service.authenticate(&AuthCredentials::Token("invalid_token".to_string())).await.unwrap();
        assert!(matches!(result, AuthenticationResult::Failed(_)));
    }
}

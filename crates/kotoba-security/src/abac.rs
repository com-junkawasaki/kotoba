//! Attribute-Based Access Control (ABAC) implementation
//!
//! This module provides comprehensive ABAC functionality including:
//! - Attribute definitions for users, resources, and environment
//! - Policy definitions with conditions and rules
//! - Policy evaluation engine
//! - Attribute collection and context building

use crate::capabilities::{ResourceType, Action};
use crate::error::{SecurityError, Result};
use chrono::{Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a policy
pub type PolicyId = String;

/// Unique identifier for a user/principal
pub type PrincipalId = String;

/// Unique identifier for a resource
pub type ResourceId = String;

/// Attribute value types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttributeValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Array(Vec<AttributeValue>),
    Object(HashMap<String, AttributeValue>),
}

impl AttributeValue {
    /// Convert to string for comparison
    pub fn as_string(&self) -> Option<&str> {
        match self {
            AttributeValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Convert to integer for comparison
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            AttributeValue::Integer(i) => Some(*i),
            _ => None,
        }
    }

    /// Convert to boolean for comparison
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            AttributeValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Convert to float for comparison
    pub fn as_float(&self) -> Option<f64> {
        match self {
            AttributeValue::Float(f) => Some(*f),
            _ => None,
        }
    }
}

/// User/Principal attributes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserAttributes {
    pub attributes: HashMap<String, AttributeValue>,
}

impl UserAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_attribute(mut self, key: String, value: AttributeValue) -> Self {
        self.attributes.insert(key, value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    pub fn set(&mut self, key: String, value: AttributeValue) {
        self.attributes.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) {
        self.attributes.remove(key);
    }
}

/// Resource attributes
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceAttributes {
    pub resource_type: ResourceType,
    pub resource_id: Option<ResourceId>,
    pub attributes: HashMap<String, AttributeValue>,
}

impl ResourceAttributes {
    pub fn new(resource_type: ResourceType, resource_id: Option<ResourceId>) -> Self {
        Self {
            resource_type,
            resource_id,
            attributes: HashMap::new(),
        }
    }

    pub fn with_attribute(mut self, key: String, value: AttributeValue) -> Self {
        self.attributes.insert(key, value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    pub fn set(&mut self, key: String, value: AttributeValue) {
        self.attributes.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) {
        self.attributes.remove(key);
    }
}

/// Environment attributes (context)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EnvironmentAttributes {
    pub attributes: HashMap<String, AttributeValue>,
}

impl EnvironmentAttributes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_attribute(mut self, key: String, value: AttributeValue) -> Self {
        self.attributes.insert(key, value);
        self
    }

    pub fn get(&self, key: &str) -> Option<&AttributeValue> {
        self.attributes.get(key)
    }

    pub fn set(&mut self, key: String, value: AttributeValue) {
        self.attributes.insert(key, value);
    }

    pub fn remove(&mut self, key: &str) {
        self.attributes.remove(key);
    }

    /// Create environment attributes with common context
    pub fn with_common_context() -> Self {
        let now = chrono::Utc::now();
        Self::new()
            .with_attribute("time.hour".to_string(), AttributeValue::Integer(now.hour() as i64))
            .with_attribute("time.day".to_string(), AttributeValue::Integer(now.day() as i64))
            .with_attribute("time.month".to_string(), AttributeValue::Integer(now.month() as i64))
            .with_attribute("time.year".to_string(), AttributeValue::Integer(now.year() as i64))
            .with_attribute("time.weekday".to_string(), AttributeValue::Integer(now.weekday().num_days_from_monday() as i64))
            .with_attribute("timestamp".to_string(), AttributeValue::Integer(now.timestamp()))
    }
}

/// Combined access context for policy evaluation
#[derive(Debug, Clone)]
pub struct AccessContext {
    pub user: UserAttributes,
    pub resource: ResourceAttributes,
    pub environment: EnvironmentAttributes,
    pub action: Action,
}

impl AccessContext {
    pub fn new(
        user: UserAttributes,
        resource: ResourceAttributes,
        environment: EnvironmentAttributes,
        action: Action,
    ) -> Self {
        Self {
            user,
            resource,
            environment,
            action,
        }
    }

    /// Get attribute value by path (e.g., "user.role", "resource.owner", "env.time.hour")
    pub fn get_attribute(&self, path: &str) -> Option<&AttributeValue> {
        let parts: Vec<&str> = path.split('.').collect();
        if parts.is_empty() {
            return None;
        }

        match parts[0] {
            "user" => {
                if parts.len() == 1 {
                    return None;
                }
                self.user.get(&parts[1..].join("."))
            }
            "resource" => {
                if parts.len() == 1 {
                    return None;
                }
                self.resource.get(&parts[1..].join("."))
            }
            "env" | "environment" => {
                if parts.len() == 1 {
                    return None;
                }
                self.environment.get(&parts[1..].join("."))
            }
            _ => None,
        }
    }
}

/// Policy effect (allow or deny)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PolicyEffect {
    Allow,
    Deny,
}

/// Policy target (what the policy applies to)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyTarget {
    pub resource_types: Vec<ResourceType>,
    pub actions: Vec<Action>,
    pub conditions: Vec<PolicyCondition>,
}

impl PolicyTarget {
    pub fn new() -> Self {
        Self {
            resource_types: Vec::new(),
            actions: Vec::new(),
            conditions: Vec::new(),
        }
    }

    pub fn with_resource_type(mut self, resource_type: ResourceType) -> Self {
        self.resource_types.push(resource_type);
        self
    }

    pub fn with_action(mut self, action: Action) -> Self {
        self.actions.push(action);
        self
    }

    pub fn with_condition(mut self, condition: PolicyCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Check if this target matches the given context
    pub fn matches(&self, context: &AccessContext) -> Result<bool> {
        // Check resource type
        if !self.resource_types.is_empty() &&
           !self.resource_types.contains(&context.resource.resource_type) {
            return Ok(false);
        }

        // Check action
        if !self.actions.is_empty() &&
           !self.actions.contains(&context.action) {
            return Ok(false);
        }

        // Check conditions
        for condition in &self.conditions {
            if !condition.evaluate(context)? {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Policy condition operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterThanOrEqual,
    LessThanOrEqual,
    Contains,
    NotContains,
    In,
    NotIn,
    Regex,
    Exists,
    NotExists,
}

/// Policy condition for attribute-based rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCondition {
    pub attribute_path: String,
    pub operator: ConditionOperator,
    pub value: AttributeValue,
}

impl PolicyCondition {
    pub fn new(attribute_path: String, operator: ConditionOperator, value: AttributeValue) -> Self {
        Self {
            attribute_path,
            operator,
            value,
        }
    }

    /// Evaluate the condition against the access context
    pub fn evaluate(&self, context: &AccessContext) -> Result<bool> {
        let attribute_value = match context.get_attribute(&self.attribute_path) {
            Some(value) => value,
            None => {
                // If attribute doesn't exist, only Exists/NotExists operators can match
                return match self.operator {
                    ConditionOperator::NotExists => Ok(true),
                    ConditionOperator::Exists => Ok(false),
                    _ => Ok(false),
                };
            }
        };

        match self.operator {
            ConditionOperator::Exists => Ok(true),
            ConditionOperator::NotExists => Ok(false),
            ConditionOperator::Equals => Ok(attribute_value == &self.value),
            ConditionOperator::NotEquals => Ok(attribute_value != &self.value),
            ConditionOperator::GreaterThan => {
                match (attribute_value.as_integer(), self.value.as_integer()) {
                    (Some(a), Some(b)) => Ok(a > b),
                    _ => Ok(false),
                }
            }
            ConditionOperator::LessThan => {
                match (attribute_value.as_integer(), self.value.as_integer()) {
                    (Some(a), Some(b)) => Ok(a < b),
                    _ => Ok(false),
                }
            }
            ConditionOperator::GreaterThanOrEqual => {
                match (attribute_value.as_integer(), self.value.as_integer()) {
                    (Some(a), Some(b)) => Ok(a >= b),
                    _ => Ok(false),
                }
            }
            ConditionOperator::LessThanOrEqual => {
                match (attribute_value.as_integer(), self.value.as_integer()) {
                    (Some(a), Some(b)) => Ok(a <= b),
                    _ => Ok(false),
                }
            }
            ConditionOperator::Contains => {
                match (attribute_value.as_string(), self.value.as_string()) {
                    (Some(haystack), Some(needle)) => Ok(haystack.contains(needle)),
                    _ => Ok(false),
                }
            }
            ConditionOperator::NotContains => {
                match (attribute_value.as_string(), self.value.as_string()) {
                    (Some(haystack), Some(needle)) => Ok(!haystack.contains(needle)),
                    _ => Ok(false),
                }
            }
            ConditionOperator::In => {
                // Check if attribute value is in the array specified by self.value
                match &self.value {
                    AttributeValue::Array(values) => Ok(values.contains(attribute_value)),
                    _ => Ok(false),
                }
            }
            ConditionOperator::NotIn => {
                // Check if attribute value is NOT in the array specified by self.value
                match &self.value {
                    AttributeValue::Array(values) => Ok(!values.contains(attribute_value)),
                    _ => Ok(false),
                }
            }
            ConditionOperator::Regex => {
                match (attribute_value.as_string(), self.value.as_string()) {
                    (Some(text), Some(pattern)) => {
                        match regex::Regex::new(pattern) {
                            Ok(re) => Ok(re.is_match(text)),
                            Err(_) => Ok(false),
                        }
                    }
                    _ => Ok(false),
                }
            }
        }
    }
}

/// ABAC Policy definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: PolicyId,
    pub name: String,
    pub description: Option<String>,
    pub effect: PolicyEffect,
    pub target: PolicyTarget,
    pub priority: i32, // Higher priority policies are evaluated first
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Policy {
    pub fn new(id: PolicyId, name: String, effect: PolicyEffect) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            effect,
            target: PolicyTarget::new(),
            priority: 0,
            active: true,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_target(mut self, target: PolicyTarget) -> Self {
        self.target = target;
        self.updated_at = chrono::Utc::now();
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self.updated_at = chrono::Utc::now();
        self
    }

    pub fn deactivate(mut self) -> Self {
        self.active = false;
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Evaluate this policy against the access context
    pub fn evaluate(&self, context: &AccessContext) -> Result<Option<PolicyEffect>> {
        if !self.active {
            return Ok(None);
        }

        if self.target.matches(context)? {
            Ok(Some(self.effect.clone()))
        } else {
            Ok(None)
        }
    }
}

/// Policy evaluation result
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny,
    NotApplicable,
}

/// ABAC Policy Engine
#[derive(Debug)]
pub struct PolicyEngine {
    policies: HashMap<PolicyId, Policy>,
}

impl PolicyEngine {
    /// Create a new policy engine
    pub fn new() -> Self {
        Self {
            policies: HashMap::new(),
        }
    }

    /// Add a policy to the engine
    pub fn add_policy(&mut self, policy: Policy) -> Result<()> {
        if self.policies.contains_key(&policy.id) {
            return Err(SecurityError::Configuration(
                format!("Policy '{}' already exists", policy.id)
            ));
        }
        self.policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    /// Update an existing policy
    pub fn update_policy(&mut self, policy: Policy) -> Result<()> {
        if !self.policies.contains_key(&policy.id) {
            return Err(SecurityError::Configuration(
                format!("Policy '{}' does not exist", policy.id)
            ));
        }
        self.policies.insert(policy.id.clone(), policy);
        Ok(())
    }

    /// Remove a policy
    pub fn remove_policy(&mut self, policy_id: &PolicyId) -> Result<()> {
        if self.policies.remove(policy_id).is_none() {
            return Err(SecurityError::Configuration(
                format!("Policy '{}' does not exist", policy_id)
            ));
        }
        Ok(())
    }

    /// Get a policy by ID
    pub fn get_policy(&self, policy_id: &PolicyId) -> Option<&Policy> {
        self.policies.get(policy_id)
    }

    /// List all policies
    pub fn list_policies(&self) -> Vec<&Policy> {
        self.policies.values().collect()
    }

    /// Evaluate access request against all policies
    pub fn evaluate_access(&self, context: &AccessContext) -> Result<PolicyDecision> {
        let mut applicable_policies: Vec<&Policy> = self.policies.values()
            .filter(|p| p.active)
            .collect();

        // Sort by priority (higher priority first)
        applicable_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        let mut has_allow = false;
        let mut has_deny = false;

        for policy in applicable_policies {
            match policy.evaluate(context)? {
                Some(PolicyEffect::Allow) => has_allow = true,
                Some(PolicyEffect::Deny) => has_deny = true,
                None => continue,
            }
        }

        // Deny takes precedence over allow
        if has_deny {
            Ok(PolicyDecision::Deny)
        } else if has_allow {
            Ok(PolicyDecision::Allow)
        } else {
            Ok(PolicyDecision::NotApplicable)
        }
    }

    /// Create common policies for quick setup
    pub fn create_common_policies(&mut self) -> Result<()> {
        // Policy 1: Administrators can do everything
        let admin_policy = Policy::new(
            "admin_full_access".to_string(),
            "Administrator Full Access".to_string(),
            PolicyEffect::Allow,
        )
        .with_description("Allows administrators full access to all resources".to_string())
        .with_target(
            PolicyTarget::new()
                .with_condition(PolicyCondition::new(
                    "user.role".to_string(),
                    ConditionOperator::Equals,
                    AttributeValue::String("admin".to_string()),
                ))
        )
        .with_priority(1000);

        // Policy 2: Users can read their own data
        let user_own_data_policy = Policy::new(
            "user_own_data".to_string(),
            "User Own Data Access".to_string(),
            PolicyEffect::Allow,
        )
        .with_description("Allows users to access their own data".to_string())
        .with_target(
            PolicyTarget::new()
                .with_resource_type(ResourceType::User)
                .with_condition(PolicyCondition::new(
                    "user.id".to_string(),
                    ConditionOperator::Equals,
                    AttributeValue::String("resource.owner".to_string()), // This should be evaluated dynamically
                ))
        )
        .with_priority(500);

        // Policy 3: Content editors can modify during business hours
        let business_hours_policy = Policy::new(
            "business_hours_edit".to_string(),
            "Business Hours Editing".to_string(),
            PolicyEffect::Allow,
        )
        .with_description("Allows content editors to modify content during business hours".to_string())
        .with_target(
            PolicyTarget::new()
                .with_resource_type(ResourceType::Graph)
                .with_action(Action::Update)
                .with_condition(PolicyCondition::new(
                    "user.role".to_string(),
                    ConditionOperator::Equals,
                    AttributeValue::String("content_editor".to_string()),
                ))
                .with_condition(PolicyCondition::new(
                    "env.time.hour".to_string(),
                    ConditionOperator::GreaterThanOrEqual,
                    AttributeValue::Integer(9),
                ))
                .with_condition(PolicyCondition::new(
                    "env.time.hour".to_string(),
                    ConditionOperator::LessThan,
                    AttributeValue::Integer(17),
                ))
        )
        .with_priority(200);

        // Policy 4: Deny access to sensitive resources outside office network
        let network_restriction_policy = Policy::new(
            "network_restriction".to_string(),
            "Network Access Restriction".to_string(),
            PolicyEffect::Deny,
        )
        .with_description("Denies access to sensitive resources outside office network".to_string())
        .with_target(
            PolicyTarget::new()
                .with_resource_type(ResourceType::Admin)
                .with_condition(PolicyCondition::new(
                    "env.network.type".to_string(),
                    ConditionOperator::NotEquals,
                    AttributeValue::String("office".to_string()),
                ))
        )
        .with_priority(800);

        self.add_policy(admin_policy)?;
        self.add_policy(user_own_data_policy)?;
        self.add_policy(business_hours_policy)?;
        self.add_policy(network_restriction_policy)?;

        Ok(())
    }
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// ABAC Service combining attribute collection and policy evaluation
pub struct ABACService {
    policy_engine: PolicyEngine,
    user_attribute_provider: Box<dyn UserAttributeProvider>,
    resource_attribute_provider: Box<dyn ResourceAttributeProvider>,
    environment_attribute_provider: Box<dyn EnvironmentAttributeProvider>,
}

impl ABACService {
    /// Create a new ABAC service with providers
    pub fn new(
        user_provider: Box<dyn UserAttributeProvider>,
        resource_provider: Box<dyn ResourceAttributeProvider>,
        environment_provider: Box<dyn EnvironmentAttributeProvider>,
    ) -> Self {
        Self {
            policy_engine: PolicyEngine::new(),
            user_attribute_provider: user_provider,
            resource_attribute_provider: resource_provider,
            environment_attribute_provider: environment_provider,
        }
    }

    /// Create ABAC service with existing policy engine
    pub fn with_engine(
        policy_engine: PolicyEngine,
        user_provider: Box<dyn UserAttributeProvider>,
        resource_provider: Box<dyn ResourceAttributeProvider>,
        environment_provider: Box<dyn EnvironmentAttributeProvider>,
    ) -> Self {
        Self {
            policy_engine,
            user_attribute_provider: user_provider,
            resource_attribute_provider: resource_provider,
            environment_attribute_provider: environment_provider,
        }
    }

    /// Add a policy
    pub fn add_policy(&mut self, policy: Policy) -> Result<()> {
        self.policy_engine.add_policy(policy)
    }

    /// Check access for a principal on a resource with specific action
    pub async fn check_access(
        &self,
        principal_id: &PrincipalId,
        resource_type: &ResourceType,
        resource_id: Option<&ResourceId>,
        action: &Action,
    ) -> Result<PolicyDecision> {
        // Collect attributes
        let user_attrs = self.user_attribute_provider.get_attributes(principal_id).await?;
        let resource_attrs = self.resource_attribute_provider.get_attributes(resource_type, resource_id).await?;
        let env_attrs = self.environment_attribute_provider.get_attributes().await?;

        // Build access context
        let context = AccessContext::new(user_attrs, resource_attrs, env_attrs, action.clone());

        // Evaluate policies
        self.policy_engine.evaluate_access(&context)
    }

    /// Get all policies
    pub fn get_policies(&self) -> Vec<&Policy> {
        self.policy_engine.list_policies()
    }

    /// Create common policies and setup
    pub fn setup_common_policies(&mut self) -> Result<()> {
        self.policy_engine.create_common_policies()
    }
}

/// Trait for providing user attributes
#[async_trait::async_trait(?Send)]
pub trait UserAttributeProvider: Send + Sync {
    async fn get_attributes(&self, principal_id: &PrincipalId) -> Result<UserAttributes>;
}

/// Trait for providing resource attributes
#[async_trait::async_trait(?Send)]
pub trait ResourceAttributeProvider: Send + Sync {
    async fn get_attributes(&self, resource_type: &ResourceType, resource_id: Option<&ResourceId>) -> Result<ResourceAttributes>;
}

/// Trait for providing environment attributes
#[async_trait::async_trait(?Send)]
pub trait EnvironmentAttributeProvider: Send + Sync {
    async fn get_attributes(&self) -> Result<EnvironmentAttributes>;
}

/// Simple in-memory attribute providers for testing/demo
pub struct SimpleUserAttributeProvider {
    attributes: HashMap<PrincipalId, UserAttributes>,
}

impl SimpleUserAttributeProvider {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    pub fn add_user(mut self, principal_id: PrincipalId, attributes: UserAttributes) -> Self {
        self.attributes.insert(principal_id, attributes);
        self
    }
}

#[async_trait::async_trait(?Send)]
impl UserAttributeProvider for SimpleUserAttributeProvider {
    async fn get_attributes(&self, principal_id: &PrincipalId) -> Result<UserAttributes> {
        self.attributes.get(principal_id)
            .cloned()
            .ok_or_else(|| SecurityError::Configuration(
                format!("User '{}' not found", principal_id)
            ))
    }
}

pub struct SimpleResourceAttributeProvider {
    attributes: HashMap<String, ResourceAttributes>,
}

impl SimpleResourceAttributeProvider {
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    pub fn add_resource(mut self, key: String, attributes: ResourceAttributes) -> Self {
        self.attributes.insert(key, attributes);
        self
    }
}

#[async_trait::async_trait(?Send)]
impl ResourceAttributeProvider for SimpleResourceAttributeProvider {
    async fn get_attributes(&self, resource_type: &ResourceType, resource_id: Option<&ResourceId>) -> Result<ResourceAttributes> {
        let key = match resource_id {
            Some(id) => format!("{}:{}", resource_type.as_str(), id),
            None => resource_type.as_str().to_string(),
        };

        self.attributes.get(&key)
            .cloned()
            .ok_or_else(|| SecurityError::Configuration(
                format!("Resource '{}' not found", key)
            ))
    }
}

pub struct SimpleEnvironmentAttributeProvider;

impl SimpleEnvironmentAttributeProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait(?Send)]
impl EnvironmentAttributeProvider for SimpleEnvironmentAttributeProvider {
    async fn get_attributes(&self) -> Result<EnvironmentAttributes> {
        Ok(EnvironmentAttributes::with_common_context())
    }
}

impl ResourceType {
    fn as_str(&self) -> &str {
        match self {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_attribute_values() {
        let string_val = AttributeValue::String("test".to_string());
        assert_eq!(string_val.as_string(), Some("test"));

        let int_val = AttributeValue::Integer(42);
        assert_eq!(int_val.as_integer(), Some(42));

        let bool_val = AttributeValue::Boolean(true);
        assert_eq!(bool_val.as_boolean(), Some(true));
    }

    #[tokio::test]
    async fn test_policy_condition_evaluation() {
        let condition = PolicyCondition::new(
            "user.age".to_string(),
            ConditionOperator::GreaterThan,
            AttributeValue::Integer(18),
        );

        let user_attrs = UserAttributes::new()
            .with_attribute("age".to_string(), AttributeValue::Integer(25));

        let resource_attrs = ResourceAttributes::new(ResourceType::Graph, None);
        let env_attrs = EnvironmentAttributes::new();

        let context = AccessContext::new(
            user_attrs,
            resource_attrs,
            env_attrs,
            Action::Read,
        );

        assert!(condition.evaluate(&context).unwrap());
    }

    #[tokio::test]
    async fn test_policy_evaluation() {
        let mut policy_engine = PolicyEngine::new();

        let policy = Policy::new(
            "test_policy".to_string(),
            "Test Policy".to_string(),
            PolicyEffect::Allow,
        )
        .with_target(
            PolicyTarget::new()
                .with_resource_type(ResourceType::Graph)
                .with_action(Action::Read)
                .with_condition(PolicyCondition::new(
                    "user.role".to_string(),
                    ConditionOperator::Equals,
                    AttributeValue::String("admin".to_string()),
                ))
        );

        policy_engine.add_policy(policy).unwrap();

        let user_attrs = UserAttributes::new()
            .with_attribute("role".to_string(), AttributeValue::String("admin".to_string()));
        let resource_attrs = ResourceAttributes::new(ResourceType::Graph, None);
        let env_attrs = EnvironmentAttributes::new();

        let context = AccessContext::new(
            user_attrs,
            resource_attrs,
            env_attrs,
            Action::Read,
        );

        let result = policy_engine.evaluate_access(&context).unwrap();
        assert_eq!(result, PolicyDecision::Allow);
    }

    #[tokio::test]
    async fn test_abac_service() {
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

        let mut abac = ABACService::new(user_provider, resource_provider, env_provider);
        abac.setup_common_policies().unwrap();

        let result = abac.check_access(
            &"user1".to_string(),
            &ResourceType::Graph,
            None,
            &Action::Read,
        ).await.unwrap();

        assert_eq!(result, PolicyDecision::Allow);
    }
}

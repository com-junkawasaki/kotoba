//! # Kotoba Capabilities
//!
//! Capability-based security system inspired by Deno's permission model.
//! Provides fine-grained, explicit permissions for secure resource access.
//!
//! ## Overview
//!
//! Capabilities represent specific permissions to perform actions on resources.
//! Unlike role-based access control (RBAC), capabilities are granted explicitly
//! and can be attenuated (restricted) for safer operations.
//!
//! ## Key Concepts
//!
//! - **Capability**: A specific permission to perform an action on a resource
//! - **CapabilitySet**: A collection of capabilities granted to a principal
//! - **Attenuation**: Creating a more restricted capability set from an existing one
//! - **Principal**: An entity (user, service, process) that holds capabilities
//!
//! ## Examples
//!
//! ```rust
//! use kotoba_security::capabilities::*;
//!
//! // Create specific capabilities
//! let read_users = Capability::new(ResourceType::Graph, Action::Read, Some("users:*".to_string()));
//! let write_posts = Capability::new(ResourceType::Graph, Action::Write, Some("posts:owned".to_string()));
//!
//! // Create a capability set
//! let mut cap_set = CapabilitySet::new();
//! cap_set.add_capability(read_users);
//! cap_set.add_capability(write_posts);
//!
//! // Check permissions
//! let service = CapabilityService::new();
//! assert!(service.check_capability(&cap_set, &ResourceType::Graph, &Action::Read, Some("users:123")));
//! ```

use serde::{Deserialize, Serialize};

/// Resource types that can be protected by capabilities
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ResourceType {
    /// Graph database operations
    Graph,
    /// File system access
    FileSystem,
    /// Network access
    Network,
    /// Environment variables
    Environment,
    /// System operations
    System,
    /// Plugin/Extension operations
    Plugin,
    /// Query execution
    Query,
    /// Administrative operations
    Admin,
    /// User management
    User,
    /// Custom resource type
    Custom(String),
}

impl Default for ResourceType {
    fn default() -> Self {
        ResourceType::Custom(String::new())
    }
}

/// Actions that can be performed on resources
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute/run access
    Execute,
    /// Delete access
    Delete,
    /// Create access
    Create,
    /// Update/modify access
    Update,
    /// Administrative access
    Admin,
    /// Custom action
    Custom(String),
}

/// Represents a specific capability/permission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capability {
    /// The type of resource this capability applies to
    pub resource_type: ResourceType,
    /// The action allowed on the resource
    pub action: Action,
    /// Optional scope/pattern limiting the capability (e.g., "users:*", "files:/tmp/**")
    pub scope: Option<String>,
    /// Optional conditions or constraints
    pub conditions: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl Capability {
    /// Create a new capability
    pub fn new(resource_type: ResourceType, action: Action, scope: Option<String>) -> Self {
        Self {
            resource_type,
            action,
            scope,
            conditions: None,
        }
    }

    /// Create a capability with conditions
    pub fn with_conditions(
        resource_type: ResourceType,
        action: Action,
        scope: Option<String>,
        conditions: std::collections::HashMap<String, serde_json::Value>,
    ) -> Self {
        Self {
            resource_type,
            action,
            scope,
            conditions: Some(conditions),
        }
    }

    /// Check if this capability matches a request
    pub fn matches(&self, resource_type: &ResourceType, action: &Action, scope: Option<&str>) -> bool {
        // Check resource type and action
        if &self.resource_type != resource_type || &self.action != action {
            return false;
        }

        // If capability has no scope restriction, allow all
        if self.scope.is_none() {
            return true;
        }

        // If request has no scope but capability does, deny
        if scope.is_none() {
            return false;
        }

        let cap_scope = self.scope.as_ref().unwrap();
        let req_scope = scope.unwrap();

        // Simple pattern matching (can be extended with glob patterns)
        self.scope_matches(cap_scope, req_scope)
    }

    /// Check if capability scope matches request scope
    fn scope_matches(&self, cap_scope: &str, req_scope: &str) -> bool {
        // Exact match
        if cap_scope == req_scope {
            return true;
        }

        // Wildcard matching
        if cap_scope.ends_with(":*") {
            let prefix = &cap_scope[..cap_scope.len() - 2];
            return req_scope.starts_with(prefix) && req_scope[prefix.len()..].starts_with(':');
        }

        if cap_scope == "*" {
            return true;
        }

        // TODO: Implement more sophisticated pattern matching (globs, etc.)
        false
    }

    /// Create an attenuated (more restrictive) version of this capability
    pub fn attenuate(mut self, new_scope: Option<String>) -> Self {
        // Can only make scope more restrictive, not less
        match (&self.scope, &new_scope) {
            (Some(current), Some(new)) => {
                if !self.scope_matches(current, new) {
                    // New scope is more restrictive, keep it
                    self.scope = new_scope;
                }
            }
            (Some(_), None) => {
                // Removing scope restriction - not allowed for attenuation
                // Keep original scope
            }
            (None, Some(new_scope)) => {
                // Adding scope restriction - allowed
                self.scope = Some(new_scope.to_string());
            }
            (None, None) => {
                // No change
            }
        }
        self
    }
}

/// A set of capabilities granted to a principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    /// The capabilities in this set
    pub capabilities: Vec<Capability>,
    /// Optional metadata about this capability set
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

impl CapabilitySet {
    /// Create a new empty capability set
    pub fn new() -> Self {
        Self {
            capabilities: Vec::new(),
            metadata: None,
        }
    }

    /// Create a capability set with metadata
    pub fn with_metadata(metadata: std::collections::HashMap<String, serde_json::Value>) -> Self {
        Self {
            capabilities: Vec::new(),
            metadata: Some(metadata),
        }
    }

    /// Add a capability to this set
    pub fn add_capability(&mut self, capability: Capability) {
        // Avoid duplicates
        if !self.capabilities.contains(&capability) {
            self.capabilities.push(capability);
        }
    }

    /// Remove a capability from this set
    pub fn remove_capability(&mut self, capability: &Capability) {
        self.capabilities.retain(|c| c != capability);
    }

    /// Check if this set contains a specific capability
    pub fn has_capability(&self, capability: &Capability) -> bool {
        self.capabilities.contains(capability)
    }

    /// Check if this set allows a specific action on a resource
    pub fn allows(&self, resource_type: &ResourceType, action: &Action, scope: Option<&str>) -> bool {
        self.capabilities.iter().any(|cap| cap.matches(resource_type, action, scope))
    }

    /// Get all capabilities for a specific resource type
    pub fn capabilities_for_resource(&self, resource_type: &ResourceType) -> Vec<&Capability> {
        self.capabilities.iter()
            .filter(|cap| &cap.resource_type == resource_type)
            .collect()
    }

    /// Create an attenuated capability set (more restrictive)
    pub fn attenuate(&self, restrictions: Vec<Capability>) -> CapabilitySet {
        let mut new_set = CapabilitySet::new();

        // Only keep capabilities that are allowed by the restrictions
        for restriction in restrictions {
            for cap in &self.capabilities {
                if cap.resource_type == restriction.resource_type &&
                   cap.action == restriction.action {
                    let attenuated = cap.clone().attenuate(restriction.scope.clone());
                    new_set.add_capability(attenuated);
                }
            }
        }

        new_set
    }

    /// Combine this capability set with another (union)
    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut combined = self.clone();
        for cap in &other.capabilities {
            combined.add_capability(cap.clone());
        }
        combined
    }

    /// Create intersection of this set with another
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        let mut result = CapabilitySet::new();
        for cap in &self.capabilities {
            if other.capabilities.contains(cap) {
                result.add_capability(cap.clone());
            }
        }
        result
    }

    /// Check if this set is empty
    pub fn is_empty(&self) -> bool {
        self.capabilities.is_empty()
    }

    /// Get the number of capabilities
    pub fn len(&self) -> usize {
        self.capabilities.len()
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::new()
    }
}

/// Service for managing and validating capabilities
pub struct CapabilityService {
    /// Optional configuration
    config: CapabilityConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityConfig {
    /// Whether to enable capability logging
    pub enable_logging: bool,
    /// Whether to enable capability auditing
    pub enable_auditing: bool,
    /// Default attenuation policies
    pub default_attenuation: Option<Vec<Capability>>,
}

impl Default for CapabilityConfig {
    fn default() -> Self {
        Self {
            enable_logging: false,
            enable_auditing: false,
            default_attenuation: None,
        }
    }
}

impl CapabilityService {
    /// Create a new capability service with default config
    pub fn new() -> Self {
        Self {
            config: CapabilityConfig::default(),
        }
    }

    /// Create a capability service with custom config
    pub fn with_config(config: CapabilityConfig) -> Self {
        Self { config }
    }

    /// Check if a capability set allows a specific action
    pub fn check_capability(
        &self,
        cap_set: &CapabilitySet,
        resource_type: &ResourceType,
        action: &Action,
        scope: Option<&str>,
    ) -> bool {
        let allowed = cap_set.allows(resource_type, action, scope);

        if self.config.enable_logging {
            println!("Capability check: {:?}::{:?} on {:?} -> {}", resource_type, action, scope, allowed);
        }

        // TODO: Add auditing logic here

        allowed
    }

    /// Grant capabilities to a principal (returns updated capability set)
    pub fn grant_capabilities(
        &self,
        existing_caps: &CapabilitySet,
        new_caps: Vec<Capability>,
    ) -> CapabilitySet {
        let mut updated = existing_caps.clone();
        for cap in new_caps {
            updated.add_capability(cap);
        }
        updated
    }

    /// Revoke capabilities from a principal
    pub fn revoke_capabilities(
        &self,
        existing_caps: &CapabilitySet,
        caps_to_revoke: Vec<Capability>,
    ) -> CapabilitySet {
        let mut updated = existing_caps.clone();
        for cap in caps_to_revoke {
            updated.remove_capability(&cap);
        }
        updated
    }

    /// Create an attenuated capability set for safer operations
    pub fn attenuate_capabilities(
        &self,
        cap_set: &CapabilitySet,
        restrictions: Vec<Capability>,
    ) -> CapabilitySet {
        cap_set.attenuate(restrictions)
    }

    /// Create predefined capability sets for common use cases
    pub fn create_preset_capability_set(preset: PresetCapabilitySet) -> CapabilitySet {
        let mut cap_set = CapabilitySet::new();

        match preset {
            PresetCapabilitySet::ReadOnly => {
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Read, None));
                cap_set.add_capability(Capability::new(ResourceType::Query, Action::Execute, None));
            }
            PresetCapabilitySet::ReadWrite => {
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Read, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Write, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Create, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Update, None));
                cap_set.add_capability(Capability::new(ResourceType::Query, Action::Execute, None));
            }
            PresetCapabilitySet::Admin => {
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Read, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Write, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Create, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Update, None));
                cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Delete, None));
                cap_set.add_capability(Capability::new(ResourceType::Query, Action::Execute, None));
                cap_set.add_capability(Capability::new(ResourceType::User, Action::Admin, None));
                cap_set.add_capability(Capability::new(ResourceType::Admin, Action::Admin, None));
            }
            PresetCapabilitySet::NetworkAccess => {
                cap_set.add_capability(Capability::new(ResourceType::Network, Action::Read, None));
                cap_set.add_capability(Capability::new(ResourceType::Network, Action::Write, None));
            }
            PresetCapabilitySet::FileSystemRead => {
                cap_set.add_capability(Capability::new(ResourceType::FileSystem, Action::Read, None));
            }
        }

        cap_set
    }
}

/// Predefined capability sets for common use cases
#[derive(Debug, Clone)]
pub enum PresetCapabilitySet {
    /// Read-only access to graphs and queries
    ReadOnly,
    /// Read-write access to graphs and queries
    ReadWrite,
    /// Full administrative access
    Admin,
    /// Network access permissions
    NetworkAccess,
    /// File system read permissions
    FileSystemRead,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_creation() {
        let cap = Capability::new(ResourceType::Graph, Action::Read, Some("users:*".to_string()));
        assert_eq!(cap.resource_type, ResourceType::Graph);
        assert_eq!(cap.action, Action::Read);
        assert_eq!(cap.scope, Some("users:*".to_string()));
    }

    #[test]
    fn test_capability_matching() {
        let cap = Capability::new(ResourceType::Graph, Action::Read, Some("users:*".to_string()));

        // Exact match
        assert!(cap.matches(&ResourceType::Graph, &Action::Read, Some("users:*")));

        // Pattern match
        assert!(cap.matches(&ResourceType::Graph, &Action::Read, Some("users:123")));

        // No match - wrong resource
        assert!(!cap.matches(&ResourceType::Network, &Action::Read, Some("users:123")));

        // No match - wrong action
        assert!(!cap.matches(&ResourceType::Graph, &Action::Write, Some("users:123")));
    }

    #[test]
    fn test_capability_set_operations() {
        let mut cap_set = CapabilitySet::new();

        let read_cap = Capability::new(ResourceType::Graph, Action::Read, None);
        let write_cap = Capability::new(ResourceType::Graph, Action::Write, None);

        cap_set.add_capability(read_cap.clone());
        cap_set.add_capability(write_cap.clone());

        assert!(cap_set.has_capability(&read_cap));
        assert!(cap_set.has_capability(&write_cap));
        assert_eq!(cap_set.len(), 2);

        // Test allowance checking
        assert!(cap_set.allows(&ResourceType::Graph, &Action::Read, None));
        assert!(cap_set.allows(&ResourceType::Graph, &Action::Write, None));
        assert!(!cap_set.allows(&ResourceType::Graph, &Action::Delete, None));
    }

    #[test]
    fn test_capability_attenuation() {
        let broad_cap = Capability::new(ResourceType::Graph, Action::Read, None);
        let attenuated = broad_cap.clone().attenuate(Some("users:*".to_string()));

        // Original capability allows all
        assert!(broad_cap.matches(&ResourceType::Graph, &Action::Read, Some("posts:123")));

        // Attenuated capability only allows specific scope
        assert!(attenuated.matches(&ResourceType::Graph, &Action::Read, Some("users:123")));
        assert!(!attenuated.matches(&ResourceType::Graph, &Action::Read, Some("posts:123")));
    }

    #[test]
    fn test_capability_service() {
        let service = CapabilityService::new();
        let mut cap_set = CapabilitySet::new();
        cap_set.add_capability(Capability::new(ResourceType::Graph, Action::Read, None));

        assert!(service.check_capability(&cap_set, &ResourceType::Graph, &Action::Read, None));
        assert!(!service.check_capability(&cap_set, &ResourceType::Graph, &Action::Write, None));
    }

    #[test]
    fn test_preset_capability_sets() {
        let readonly = CapabilityService::create_preset_capability_set(PresetCapabilitySet::ReadOnly);
        assert!(readonly.allows(&ResourceType::Graph, &Action::Read, None));
        assert!(readonly.allows(&ResourceType::Query, &Action::Execute, None));
        assert!(!readonly.allows(&ResourceType::Graph, &Action::Write, None));

        let admin = CapabilityService::create_preset_capability_set(PresetCapabilitySet::Admin);
        assert!(admin.allows(&ResourceType::Graph, &Action::Delete, None));
        assert!(admin.allows(&ResourceType::Admin, &Action::Admin, None));
    }
}

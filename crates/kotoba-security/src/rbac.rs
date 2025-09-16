//! Role-Based Access Control (RBAC) implementation
//!
//! This module provides comprehensive RBAC functionality including:
//! - Role definitions and hierarchies
//! - Role assignments to users/principals
//! - Role-based permission evaluation
//! - Role inheritance and composition

use crate::capabilities::{Capability, CapabilitySet, ResourceType, Action};
use crate::error::{SecurityError, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Unique identifier for a role
pub type RoleId = String;

/// Unique identifier for a user/principal
pub type PrincipalId = String;

/// Represents a role in the RBAC system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier for the role
    pub id: RoleId,
    /// Human-readable name
    pub name: String,
    /// Description of the role
    pub description: Option<String>,
    /// Parent roles (for inheritance)
    pub parent_roles: HashSet<RoleId>,
    /// Capabilities granted by this role
    pub capabilities: CapabilitySet,
    /// Additional attributes/metadata
    pub attributes: HashMap<String, serde_json::Value>,
    /// Whether this role is active
    pub active: bool,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last modification timestamp
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl Role {
    /// Create a new role
    pub fn new(id: RoleId, name: String) -> Self {
        let now = chrono::Utc::now();
        Self {
            id,
            name,
            description: None,
            parent_roles: HashSet::new(),
            capabilities: CapabilitySet::new(),
            attributes: HashMap::new(),
            active: true,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create a role with description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Add a parent role for inheritance
    pub fn add_parent_role(mut self, parent_role_id: RoleId) -> Self {
        self.parent_roles.insert(parent_role_id);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Add a capability to this role
    pub fn add_capability(mut self, capability: Capability) -> Self {
        self.capabilities.add_capability(capability);
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Add multiple capabilities to this role
    pub fn add_capabilities(mut self, capabilities: Vec<Capability>) -> Self {
        for cap in capabilities {
            self.capabilities.add_capability(cap);
        }
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Set role as inactive
    pub fn deactivate(mut self) -> Self {
        self.active = false;
        self.updated_at = chrono::Utc::now();
        self
    }

    /// Check if role is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get all capabilities including inherited ones
    pub fn get_all_capabilities(&self, role_registry: &RoleRegistry) -> Result<CapabilitySet> {
        let mut all_caps = self.capabilities.clone();

        // Add capabilities from parent roles
        for parent_id in &self.parent_roles {
            if let Some(parent_role) = role_registry.get_role(parent_id) {
                if parent_role.is_active() {
                    let parent_caps = parent_role.get_all_capabilities(role_registry)?;
                    all_caps = all_caps.union(&parent_caps);
                }
            } else {
                return Err(SecurityError::Configuration(
                    format!("Parent role '{}' not found", parent_id)
                ));
            }
        }

        Ok(all_caps)
    }
}

/// Registry for managing roles and their relationships
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleRegistry {
    roles: HashMap<RoleId, Role>,
}

impl RoleRegistry {
    /// Create a new role registry
    pub fn new() -> Self {
        Self {
            roles: HashMap::new(),
        }
    }

    /// Add a role to the registry
    pub fn add_role(&mut self, role: Role) -> Result<()> {
        if self.roles.contains_key(&role.id) {
            return Err(SecurityError::Configuration(
                format!("Role '{}' already exists", role.id)
            ));
        }

        // Validate parent roles exist
        for parent_id in &role.parent_roles {
            if !self.roles.contains_key(parent_id) {
                return Err(SecurityError::Configuration(
                    format!("Parent role '{}' does not exist", parent_id)
                ));
            }
        }

        self.roles.insert(role.id.clone(), role);
        Ok(())
    }

    /// Update an existing role
    pub fn update_role(&mut self, role: Role) -> Result<()> {
        if !self.roles.contains_key(&role.id) {
            return Err(SecurityError::Configuration(
                format!("Role '{}' does not exist", role.id)
            ));
        }

        // Validate parent roles exist
        for parent_id in &role.parent_roles {
            if !self.roles.contains_key(parent_id) {
                return Err(SecurityError::Configuration(
                    format!("Parent role '{}' does not exist", parent_id)
                ));
            }
        }

        self.roles.insert(role.id.clone(), role);
        Ok(())
    }

    /// Remove a role from the registry
    pub fn remove_role(&mut self, role_id: &RoleId) -> Result<()> {
        if let Some(role) = self.roles.get(role_id) {
            // Check if role is referenced as parent by other roles
            for (id, r) in &self.roles {
                if id != role_id && r.parent_roles.contains(role_id) {
                    return Err(SecurityError::Configuration(
                        format!("Cannot remove role '{}' as it is referenced by role '{}'", role_id, id)
                    ));
                }
            }
        }

        self.roles.remove(role_id);
        Ok(())
    }

    /// Get a role by ID
    pub fn get_role(&self, role_id: &RoleId) -> Option<&Role> {
        self.roles.get(role_id)
    }

    /// Get all roles
    pub fn get_all_roles(&self) -> Vec<&Role> {
        self.roles.values().collect()
    }

    /// Get active roles only
    pub fn get_active_roles(&self) -> Vec<&Role> {
        self.roles.values().filter(|r| r.is_active()).collect()
    }

    /// Check if a role exists
    pub fn role_exists(&self, role_id: &RoleId) -> bool {
        self.roles.contains_key(role_id)
    }

    /// Get all child roles (roles that inherit from the given role)
    pub fn get_child_roles(&self, role_id: &RoleId) -> Vec<&Role> {
        self.roles.values()
            .filter(|r| r.parent_roles.contains(role_id))
            .collect()
    }

    /// Get role hierarchy path (from root to leaf)
    pub fn get_role_hierarchy(&self, role_id: &RoleId) -> Result<Vec<RoleId>> {
        let mut path = Vec::new();
        let mut visited = HashSet::new();
        let mut current_id = role_id.clone();

        while !visited.contains(&current_id) {
            visited.insert(current_id.clone());

            if let Some(role) = self.get_role(&current_id) {
                path.push(current_id.clone());

                // For simplicity, pick the first parent (can be extended for multiple inheritance)
                if let Some(parent_id) = role.parent_roles.iter().next() {
                    current_id = parent_id.clone();
                } else {
                    break; // No more parents
                }
            } else {
                return Err(SecurityError::Configuration(
                    format!("Role '{}' not found in hierarchy", current_id)
                ));
            }

            // Prevent infinite loops
            if path.len() > 100 {
                return Err(SecurityError::Configuration(
                    "Circular role inheritance detected".to_string()
                ));
            }
        }

        Ok(path)
    }
}

impl Default for RoleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Role assignment mapping users to roles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignment {
    /// User/Principal ID
    pub principal_id: PrincipalId,
    /// Assigned role ID
    pub role_id: RoleId,
    /// Assignment context/scope (optional)
    pub scope: Option<String>,
    /// Assignment conditions (optional)
    pub conditions: Option<HashMap<String, serde_json::Value>>,
    /// Assignment timestamp
    pub assigned_at: chrono::DateTime<chrono::Utc>,
    /// Assignment expiration (optional)
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether assignment is active
    pub active: bool,
}

/// Role assignment manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleAssignmentManager {
    assignments: HashMap<(PrincipalId, RoleId), RoleAssignment>,
}

impl RoleAssignmentManager {
    /// Create a new role assignment manager
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
        }
    }

    /// Assign a role to a principal
    pub fn assign_role(&mut self, assignment: RoleAssignment) -> Result<()> {
        let key = (assignment.principal_id.clone(), assignment.role_id.clone());

        if self.assignments.contains_key(&key) {
            return Err(SecurityError::Configuration(
                format!("Role '{}' is already assigned to principal '{}'",
                       assignment.role_id, assignment.principal_id)
            ));
        }

        self.assignments.insert(key, assignment);
        Ok(())
    }

    /// Revoke a role from a principal
    pub fn revoke_role(&mut self, principal_id: &PrincipalId, role_id: &RoleId) -> Result<()> {
        let key = (principal_id.clone(), role_id.clone());
        if self.assignments.remove(&key).is_none() {
            return Err(SecurityError::Configuration(
                format!("Role '{}' is not assigned to principal '{}'",
                       role_id, principal_id)
            ));
        }
        Ok(())
    }

    /// Get all roles assigned to a principal
    pub fn get_principal_roles(&self, principal_id: &PrincipalId) -> Vec<&RoleAssignment> {
        self.assignments.values()
            .filter(|assignment| {
                &assignment.principal_id == principal_id &&
                assignment.active &&
                assignment.expires_at.map_or(true, |exp| chrono::Utc::now() < exp)
            })
            .collect()
    }

    /// Get all principals with a specific role
    pub fn get_role_principals(&self, role_id: &RoleId) -> Vec<&RoleAssignment> {
        self.assignments.values()
            .filter(|assignment| {
                &assignment.role_id == role_id &&
                assignment.active &&
                assignment.expires_at.map_or(true, |exp| chrono::Utc::now() < exp)
            })
            .collect()
    }

    /// Check if a principal has a specific role
    pub fn has_role(&self, principal_id: &PrincipalId, role_id: &RoleId) -> bool {
        self.get_principal_roles(principal_id)
            .iter()
            .any(|assignment| &assignment.role_id == role_id)
    }

    /// Get all active assignments
    pub fn get_all_assignments(&self) -> Vec<&RoleAssignment> {
        self.assignments.values()
            .filter(|assignment| {
                assignment.active &&
                assignment.expires_at.map_or(true, |exp| chrono::Utc::now() < exp)
            })
            .collect()
    }
}

impl Default for RoleAssignmentManager {
    fn default() -> Self {
        Self::new()
    }
}

/// RBAC Service combining roles, assignments, and evaluation
#[derive(Debug)]
pub struct RBACService {
    role_registry: RoleRegistry,
    assignment_manager: RoleAssignmentManager,
}

impl RBACService {
    /// Create a new RBAC service
    pub fn new() -> Self {
        Self {
            role_registry: RoleRegistry::new(),
            assignment_manager: RoleAssignmentManager::new(),
        }
    }

    /// Create RBAC service with existing registry and assignments
    pub fn with_data(role_registry: RoleRegistry, assignment_manager: RoleAssignmentManager) -> Self {
        Self {
            role_registry,
            assignment_manager,
        }
    }

    /// Add a role to the system
    pub fn add_role(&mut self, role: Role) -> Result<()> {
        self.role_registry.add_role(role)
    }

    /// Assign a role to a principal
    pub fn assign_role(&mut self, assignment: RoleAssignment) -> Result<()> {
        // Validate role exists
        if !self.role_registry.role_exists(&assignment.role_id) {
            return Err(SecurityError::Configuration(
                format!("Role '{}' does not exist", assignment.role_id)
            ));
        }

        self.assignment_manager.assign_role(assignment)
    }

    /// Revoke a role from a principal
    pub fn revoke_role(&mut self, principal_id: &PrincipalId, role_id: &RoleId) -> Result<()> {
        self.assignment_manager.revoke_role(principal_id, role_id)
    }

        /// Check if a principal has permission for a specific action on a resource
        pub fn check_permission(
            &self,
            principal_id: &PrincipalId,
            resource_type: &ResourceType,
            action: &Action,
            scope: Option<&str>,
        ) -> Result<bool> {
            let principal_assignments = self.assignment_manager.get_principal_roles(principal_id);

            for assignment in principal_assignments {
                if let Some(role) = self.role_registry.get_role(&assignment.role_id) {
                    if !role.is_active() {
                        continue;
                    }

                    let all_capabilities = role.get_all_capabilities(&self.role_registry)?;
                    if all_capabilities.allows(resource_type, action, scope) {
                        return Ok(true);
                    }
                }
            }

            Ok(false)
        }

    /// Get all effective capabilities for a principal
    pub fn get_principal_capabilities(&self, principal_id: &PrincipalId) -> Result<CapabilitySet> {
        let principal_assignments = self.assignment_manager.get_principal_roles(principal_id);
        let mut all_capabilities = CapabilitySet::new();

        for assignment in principal_assignments {
            if let Some(role) = self.role_registry.get_role(&assignment.role_id) {
                if role.is_active() {
                    let role_capabilities = role.get_all_capabilities(&self.role_registry)?;
                    all_capabilities = all_capabilities.union(&role_capabilities);
                }
            }
        }

        Ok(all_capabilities)
    }

    /// Get all roles assigned to a principal
    pub fn get_principal_roles(&self, principal_id: &PrincipalId) -> Vec<&Role> {
        let assignments = self.assignment_manager.get_principal_roles(principal_id);
        assignments.iter()
            .filter_map(|assignment| self.role_registry.get_role(&assignment.role_id))
            .collect()
    }

    /// List all roles
    pub fn list_roles(&self) -> Vec<&Role> {
        self.role_registry.get_all_roles()
    }

    /// List all active role assignments
    pub fn list_assignments(&self) -> Vec<&RoleAssignment> {
        self.assignment_manager.get_all_assignments()
    }

    /// Create common roles for quick setup
    pub fn create_common_roles(&mut self) -> Result<()> {
        // Administrator role
        let admin_role = Role::new("admin".to_string(), "Administrator".to_string())
            .with_description("Full system access".to_string())
            .add_capability(Capability::new(ResourceType::Admin, Action::Admin, None));

        // User manager role
        let user_manager_role = Role::new("user_manager".to_string(), "User Manager".to_string())
            .with_description("Manage user accounts".to_string())
            .add_capability(Capability::new(ResourceType::User, Action::Read, None))
            .add_capability(Capability::new(ResourceType::User, Action::Create, None))
            .add_capability(Capability::new(ResourceType::User, Action::Update, None));

        // Content editor role
        let content_editor_role = Role::new("content_editor".to_string(), "Content Editor".to_string())
            .with_description("Edit content and data".to_string())
            .add_capability(Capability::new(ResourceType::Graph, Action::Read, None))
            .add_capability(Capability::new(ResourceType::Graph, Action::Create, None))
            .add_capability(Capability::new(ResourceType::Graph, Action::Update, None));

        // Content viewer role
        let content_viewer_role = Role::new("content_viewer".to_string(), "Content Viewer".to_string())
            .with_description("View content and data".to_string())
            .add_capability(Capability::new(ResourceType::Graph, Action::Read, None));

        // Add roles to registry
        self.add_role(admin_role)?;
        self.add_role(user_manager_role)?;
        self.add_role(content_editor_role)?;
        self.add_role(content_viewer_role)?;

        Ok(())
    }
}

impl Default for RBACService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_creation() {
        let role = Role::new("test_role".to_string(), "Test Role".to_string())
            .with_description("A test role".to_string());

        assert_eq!(role.id, "test_role");
        assert_eq!(role.name, "Test Role");
        assert_eq!(role.description, Some("A test role".to_string()));
        assert!(role.is_active());
    }

    #[test]
    fn test_role_registry() {
        let mut registry = RoleRegistry::new();

        let role = Role::new("test_role".to_string(), "Test Role".to_string());
        registry.add_role(role.clone()).unwrap();

        assert!(registry.role_exists(&"test_role".to_string()));
        assert_eq!(registry.get_role(&"test_role".to_string()).unwrap().name, "Test Role");
    }

    #[test]
    fn test_role_assignment() {
        let mut manager = RoleAssignmentManager::new();

        let assignment = RoleAssignment {
            principal_id: "user1".to_string(),
            role_id: "role1".to_string(),
            scope: None,
            conditions: None,
            assigned_at: chrono::Utc::now(),
            expires_at: None,
            active: true,
        };

        manager.assign_role(assignment).unwrap();

        let user_roles = manager.get_principal_roles(&"user1".to_string());
        assert_eq!(user_roles.len(), 1);
        assert_eq!(user_roles[0].role_id, "role1");
    }

    #[test]
    fn test_rbac_permission_check() {
        let mut rbac = RBACService::new();

        // Create a role with read permission
        let role = Role::new("reader".to_string(), "Reader".to_string())
            .add_capability(Capability::new(ResourceType::Graph, Action::Read, None));

        rbac.add_role(role).unwrap();

        // Assign role to user
        let assignment = RoleAssignment {
            principal_id: "user1".to_string(),
            role_id: "reader".to_string(),
            scope: None,
            conditions: None,
            assigned_at: chrono::Utc::now(),
            expires_at: None,
            active: true,
        };

        rbac.assign_role(assignment).unwrap();

        // Check permission
        assert!(rbac.check_permission(&"user1".to_string(), &ResourceType::Graph, &Action::Read, None).unwrap());
        assert!(!rbac.check_permission(&"user1".to_string(), &ResourceType::Graph, &Action::Write, None).unwrap());
    }

    #[test]
    fn test_role_hierarchy() {
        let mut registry = RoleRegistry::new();

        // Create parent role
        let parent_role = Role::new("parent".to_string(), "Parent Role".to_string())
            .add_capability(Capability::new(ResourceType::Graph, Action::Read, None));

        // Create child role that inherits from parent
        let child_role = Role::new("child".to_string(), "Child Role".to_string())
            .add_parent_role("parent".to_string())
            .add_capability(Capability::new(ResourceType::Graph, Action::Write, None));

        registry.add_role(parent_role).unwrap();
        registry.add_role(child_role).unwrap();

        let child = registry.get_role(&"child".to_string()).unwrap();
        let all_caps = child.get_all_capabilities(&registry).unwrap();

        // Child should have both read (from parent) and write (own) permissions
        assert!(all_caps.has_capability(&ResourceType::Graph, &Action::Read, None));
        assert!(all_caps.has_capability(&ResourceType::Graph, &Action::Write, None));
    }
}

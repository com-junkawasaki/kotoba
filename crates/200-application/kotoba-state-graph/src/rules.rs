//! Provides generic, reusable graph rewrite rules for common UI state transitions.
//!
//! These rules are written in Rust and registered with the `RewriteEngine`.
//! They can be called from `.kotobas` scripts via `std.ext.db.rewrite(...)`.

use kotoba_storage::KeyValueStore;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::warn;

/// A placeholder for RuleIR
pub type RuleIR = ();

/// State graph manager with KeyValueStore backend
pub struct StateGraphManager<T: KeyValueStore + 'static> {
    storage: Arc<T>,
    component_prefix: String,
    state_prefix: String,
}

impl<T: KeyValueStore + 'static> StateGraphManager<T> {
    /// Create new state graph manager
    pub fn new(storage: Arc<T>) -> Self {
        Self {
            storage,
            component_prefix: "ui:component:".to_string(),
            state_prefix: "ui:state:".to_string(),
        }
    }

    /// Create or update a UI component
    pub async fn create_component(&self, component_id: &str, component_type: &str, props: Value) -> anyhow::Result<()> {
        let key = format!("{}{}", self.component_prefix, component_id);
        let component_data = serde_json::json!({
            "id": component_id,
            "type": component_type,
            "props": props,
            "created_at": chrono::Utc::now().timestamp(),
        });

        let data = serde_json::to_vec(&component_data)?;
        self.storage.put(key.as_bytes(), &data).await?;
        Ok(())
    }

    /// Get a UI component
    pub async fn get_component(&self, component_id: &str) -> anyhow::Result<Option<Value>> {
        let key = format!("{}{}", self.component_prefix, component_id);
        match self.storage.get(key.as_bytes()).await? {
            Some(data) => {
                let component: Value = serde_json::from_slice(&data)?;
                Ok(Some(component))
            }
            None => Ok(None)
        }
    }

    /// Update component properties
    pub async fn update_component_props(&self, component_id: &str, props: Value) -> anyhow::Result<()> {
        let key = format!("{}{}", self.component_prefix, component_id);

        // Get existing component
        let mut existing = match self.get_component(component_id).await? {
            Some(component) => component,
            None => return Err(anyhow::anyhow!("Component {} not found", component_id))
        };

        // Update props
        if let Some(component_obj) = existing.as_object_mut() {
            component_obj.insert("props".to_string(), props);
            component_obj.insert("updated_at".to_string(), serde_json::json!(chrono::Utc::now().timestamp()));
        }

        let data = serde_json::to_vec(&existing)?;
        self.storage.put(key.as_bytes(), &data).await?;
        Ok(())
    }

    /// Set component state
    pub async fn set_component_state(&self, component_id: &str, state_key: &str, state_value: Value) -> anyhow::Result<()> {
        let key = format!("{}{}:{}", self.state_prefix, component_id, state_key);
        let state_data = serde_json::json!({
            "component_id": component_id,
            "key": state_key,
            "value": state_value,
            "updated_at": chrono::Utc::now().timestamp(),
        });

        let data = serde_json::to_vec(&state_data)?;
        self.storage.put(key.as_bytes(), &data).await?;
        Ok(())
    }

    /// Get component state
    pub async fn get_component_state(&self, component_id: &str, state_key: &str) -> anyhow::Result<Option<Value>> {
        let key = format!("{}{}:{}", self.state_prefix, component_id, state_key);
        match self.storage.get(key.as_bytes()).await? {
            Some(data) => {
                let state: Value = serde_json::from_slice(&data)?;
                Ok(state.get("value").cloned())
            }
            None => Ok(None)
        }
    }

    /// Apply a state transition rule
    pub async fn apply_state_rule(&self, component_id: &str, rule_name: &str, params: Value) -> anyhow::Result<()> {
        match rule_name {
            "update_prop" => self.apply_update_prop_rule(component_id, params).await,
            "toggle_boolean" => self.apply_toggle_boolean_rule(component_id, params).await,
            _ => {
                warn!("Unknown state rule: {}", rule_name);
                Err(anyhow::anyhow!("Unknown state rule: {}", rule_name))
            }
        }
    }

    /// Apply update_prop rule
    async fn apply_update_prop_rule(&self, component_id: &str, params: Value) -> anyhow::Result<()> {
        let prop_name = params.get("prop")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'prop' parameter"))?;

        let value = params.get("value")
            .ok_or_else(|| anyhow::anyhow!("Missing 'value' parameter"))?;

        self.update_component_props(component_id, serde_json::json!({ prop_name: value })).await
    }

    /// Apply toggle_boolean rule
    async fn apply_toggle_boolean_rule(&self, component_id: &str, params: Value) -> anyhow::Result<()> {
        let prop_name = params.get("prop")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'prop' parameter"))?;

        // Get current component
        let component = self.get_component(component_id).await?
            .ok_or_else(|| anyhow::anyhow!("Component {} not found", component_id))?;

        let current_value = component.get("props")
            .and_then(|p| p.get(prop_name))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let new_value = !current_value;
        self.update_component_props(component_id, serde_json::json!({ prop_name: new_value })).await
    }
}

/// Creates a collection of standard UI rewrite rules to be registered with the engine.
/// TODO: Implement proper RuleIR integration
pub fn get_standard_ui_rules() -> Vec<(&'static str, RuleIR)> {
    vec![
        ("update_prop", update_prop_rule()),
        // Future rules can be added here:
        // ("toggle_boolean", toggle_boolean_rule()),
        // ("add_child_component", add_child_component_rule()),
    ]
}

/// Create a new state graph manager with KeyValueStore
pub fn create_state_graph_manager<T: KeyValueStore + 'static>(storage: Arc<T>) -> StateGraphManager<T> {
    StateGraphManager::new(storage)
}

/// Creates the `update_prop` rewrite rule.
///
/// This rule finds a node based on a GQL query and updates one of its properties.
///
/// **Parameters expected from Jsonnet:**
/// - `query`: A GQL query string that uniquely identifies a target node.
/// - `prop`: The name of the property to update (e.g., "isVisible").
/// - `value`: The new value for the property.
fn update_prop_rule() -> RuleIR {
    // This is a mock implementation. A real implementation would construct
    // a proper RuleIR object that defines the Left-Hand-Side (LHS) for matching
    // and the Right-Hand-Side (RHS) for rewriting.

    println!("'update_prop' rule registered.");

    // The RuleIR would internally define a process like this:
    // 1. LHS: Match a single vertex `v`.
    // 2. Guard: Check if `v` matches the provided `query` parameter.
    // 3. RHS: Create a patch operation `Patch::UpdateProp` on vertex `v`
    //    using the `prop` and `value` parameters.

    () // Placeholder for RuleIR object
}

// Example of what a real implementation might look like:
/*
fn update_prop_rule() -> RuleIR {
    RuleIR::new(
        // LHS: Match any vertex `n`
        GraphPattern::new().add_vertex("n"),
        // NAC: No negative application conditions
        GraphPattern::new(),
        // RHS: The same vertex `n`
        GraphPattern::new().add_vertex("n"),
        // Mapping from LHS to RHS
        vec![("n", "n")],
        // Guard condition to filter the matched node
        Some(Guard::new(
            "params.query", // This would need a way to execute a sub-query
        )),
        // Patch generation logic
        |match_context, params| {
            let node_id = match_context.get_vertex_id("n").unwrap();
            let prop_name = params.get("prop").unwrap().as_str().unwrap();
            let new_value = params.get("value").unwrap().clone();

            vec![Patch::UpdateProp {
                vertex_id: node_id,
                prop_key: prop_name.to_string(),
                new_value,
            }]
        },
    )
}
*/

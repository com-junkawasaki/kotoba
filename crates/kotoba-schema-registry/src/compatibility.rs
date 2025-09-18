//! # kotoba-schema-registry::compatibility
//!
//! Implements strict compatibility checking for JSON Schemas.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;

/// Defines the compatibility check mode for schema evolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompatibilityMode {
    /// No compatibility check is performed.
    None,
    /// The new schema must be able to read data from the previous schema.
    Backward,
    /// The previous schema must be able to read data from the new schema.
    Forward,
    /// Both Backward and Forward compatibility must be met.
    Full,
}

/// Checks if two schemas are compatible based on the given mode.
pub fn check(old_schema: &Value, new_schema: &Value, mode: CompatibilityMode) -> Result<(), String> {
    if mode == CompatibilityMode::None {
        return Ok(());
    }

    // We only handle object schemas at the top level for now.
    if !old_schema.is_object() || !new_schema.is_object() {
        return Err("Compatibility check only supported for object schemas.".to_string());
    }
    if old_schema.get("type").and_then(Value::as_str) != Some("object")
        || new_schema.get("type").and_then(Value::as_str) != Some("object")
    {
        return Err(
            "Compatibility check only supported for schemas with top-level type 'object'."
                .to_string(),
        );
    }

    match mode {
        CompatibilityMode::None => Ok(()),
        CompatibilityMode::Backward => check_backward(old_schema, new_schema),
        CompatibilityMode::Forward => check_forward(old_schema, new_schema),
        CompatibilityMode::Full => {
            check_backward(old_schema, new_schema).map_err(|e| format!("[Backward check failed] {}", e))?;
            check_forward(old_schema, new_schema).map_err(|e| format!("[Forward check failed] {}", e))
        }
    }
}

fn get_properties(schema: &Value) -> HashSet<&str> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .map(|props| props.keys().map(|k| k.as_str()).collect())
        .unwrap_or_default()
}

fn get_required(schema: &Value) -> HashSet<&str> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| arr.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default()
}

/// Backward check: A valid document for `old_schema` must be valid for `new_schema`.
fn check_backward(old_schema: &Value, new_schema: &Value) -> Result<(), String> {
    let old_props = get_properties(old_schema);
    let new_props = get_properties(new_schema);
    let old_required = get_required(old_schema);
    let new_required = get_required(new_schema);

    // Rule: Cannot remove any property that was required in the old schema.
    let removed_props: HashSet<_> = old_props.difference(&new_props).cloned().collect();
    for prop in removed_props {
        if old_required.contains(prop) {
            return Err(format!("Required property '{}' was removed.", prop));
        }
    }

    // Rule: Cannot make an optional field required.
    let added_required: HashSet<_> = new_required.difference(&old_required).cloned().collect();
    if !added_required.is_empty() {
        return Err(format!(
            "Optional properties {:?} were made required.",
            added_required
        ));
    }

    Ok(())
}

/// Forward check: A valid document for `new_schema` must be valid for `old_schema`.
fn check_forward(old_schema: &Value, new_schema: &Value) -> Result<(), String> {
    let old_props = get_properties(old_schema);
    let new_props = get_properties(new_schema);
    let old_required = get_required(old_schema);
    let new_required = get_required(new_schema);

    // Rule: Cannot add a new required field.
    let added_props: HashSet<_> = new_props.difference(&old_props).cloned().collect();
    for prop in &added_props {
        if new_required.contains(prop) {
            return Err(format!("New required property '{}' was added.", prop));
        }
    }

    // Check if `additionalProperties` is false in old schema. If so, new properties are not allowed.
    let allows_additional = match old_schema.get("additionalProperties") {
        Some(v) => v.as_bool().unwrap_or(true), // default allows additional properties
        None => true,
    };
    if !allows_additional && !added_props.is_empty() {
        return Err(format!(
            "New properties {:?} added, but old schema does not allow additional properties.",
            added_props
        ));
    }

    // Rule: Cannot remove a field that was required in the old schema.
    let removed_required: HashSet<_> = old_required.difference(&new_required).cloned().collect();
    if !removed_required.is_empty() {
        return Err(format!(
            "Required properties {:?} were removed.",
            removed_required
        ));
    }

    Ok(())
}

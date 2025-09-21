//! # Intermediate Representation (IR) for Kotoba
//!
//! This module provides the intermediate representation system for Kotoba,
//! including catalog-IR, rule-IR, query-IR, patch-IR, and strategy-IR.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::types::*;

// Re-export all IR modules
pub mod catalog;
pub mod rule;
pub mod query;
pub mod patch;
pub mod strategy;

// Re-export for convenience
pub use catalog::*;
pub use rule::*;
pub use query::*;
pub use patch::*;
pub use strategy::*;

// Core IR types that need to be defined here

/// Property key type
pub type PropertyKey = String;

/// Value type for IR
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    /// Null value
    Null,
    /// Boolean value
    Bool(bool),
    /// Integer value
    Int(i64),
    /// Float value
    Float(f64),
    /// String value
    String(String),
    /// List value
    List(Vec<Value>),
    /// Map value
    Map(HashMap<String, Value>),
}

impl Value {
    /// Check if the value is null
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Check if the value is a boolean
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Check if the value is an integer
    pub fn is_int(&self) -> bool {
        matches!(self, Value::Int(_))
    }

    /// Check if the value is a float
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if the value is a string
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if the value is a list
    pub fn is_list(&self) -> bool {
        matches!(self, Value::List(_))
    }

    /// Check if the value is a map
    pub fn is_map(&self) -> bool {
        matches!(self, Value::Map(_))
    }
}

impl Default for Value {
    fn default() -> Self {
        Value::Null
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::List(a), Value::List(b)) => a == b,
            (Value::Map(a), Value::Map(b)) => a == b,
            _ => false,
        }
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "\"{}\"", s),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
        }
    }
}

/// Property definition for catalog-IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PropertyDef {
    /// Property name
    pub name: PropertyKey,
    /// Property type
    pub r#type: ValueType,
    /// Whether the property can be null
    pub nullable: bool,
    /// Default value
    pub default: Option<Value>,
}

impl PropertyDef {
    /// Create a new property definition
    pub fn new(name: PropertyKey, r#type: ValueType) -> Self {
        Self {
            name,
            r#type,
            nullable: false,
            default: None,
        }
    }

    /// Set nullable flag
    pub fn nullable(mut self, nullable: bool) -> Self {
        self.nullable = nullable;
        self
    }

    /// Set default value
    pub fn default(mut self, default: Value) -> Self {
        self.default = Some(default);
        self
    }
}

/// Value type for property definitions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueType {
    /// Null type
    #[serde(rename = "null")]
    Null,
    /// Boolean type
    #[serde(rename = "bool")]
    Bool,
    /// Integer type
    #[serde(rename = "int")]
    Int,
    /// Float type
    #[serde(rename = "float")]
    Float,
    /// String type
    #[serde(rename = "string")]
    String,
    /// List type
    #[serde(rename = "list")]
    List(Box<ValueType>),
    /// Map type
    #[serde(rename = "map")]
    Map,
}

impl ValueType {
    /// Check if this type is compatible with another type
    pub fn is_compatible(&self, other: &ValueType) -> bool {
        match (self, other) {
            (ValueType::Null, _) => true,
            (_, ValueType::Null) => true,
            (ValueType::Bool, ValueType::Bool) => true,
            (ValueType::Int, ValueType::Int) => true,
            (ValueType::Float, ValueType::Float) => true,
            (ValueType::String, ValueType::String) => true,
            (ValueType::List(a), ValueType::List(b)) => a.is_compatible(b),
            (ValueType::Map, ValueType::Map) => true,
            _ => false,
        }
    }
}

impl PartialEq for ValueType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (ValueType::Null, ValueType::Null) => true,
            (ValueType::Bool, ValueType::Bool) => true,
            (ValueType::Int, ValueType::Int) => true,
            (ValueType::Float, ValueType::Float) => true,
            (ValueType::String, ValueType::String) => true,
            (ValueType::List(a), ValueType::List(b)) => a == b,
            (ValueType::Map, ValueType::Map) => true,
            _ => false,
        }
    }
}

impl Default for ValueType {
    fn default() -> Self {
        ValueType::String
    }
}

/// Label definition for catalog-IR
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LabelDef {
    /// Label name
    pub name: Label,
    /// Properties
    pub properties: Vec<PropertyDef>,
    /// Super labels (inheritance)
    pub super_labels: Option<Vec<Label>>,
}

impl LabelDef {
    /// Create a new label definition
    pub fn new(name: Label) -> Self {
        Self {
            name,
            properties: Vec::new(),
            super_labels: None,
        }
    }

    /// Add a property
    pub fn with_property(mut self, prop: PropertyDef) -> Self {
        self.properties.push(prop);
        self
    }

    /// Set super labels
    pub fn with_super_labels(mut self, super_labels: Vec<Label>) -> Self {
        self.super_labels = Some(super_labels);
        self
    }
}

/// Index definition for catalog-IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexDef {
    /// Index name
    pub name: String,
    /// Label to index
    pub label: Label,
    /// Properties to index
    pub properties: Vec<PropertyKey>,
    /// Whether the index is unique
    pub unique: bool,
}

impl IndexDef {
    /// Create a new index definition
    pub fn new(name: String, label: Label) -> Self {
        Self {
            name,
            label,
            properties: Vec::new(),
            unique: false,
        }
    }

    /// Add a property to index
    pub fn with_property(mut self, prop: PropertyKey) -> Self {
        self.properties.push(prop);
        self
    }

    /// Set unique flag
    pub fn unique(mut self, unique: bool) -> Self {
        self.unique = unique;
        self
    }
}

/// Invariant definition for catalog-IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    /// Invariant name
    pub name: String,
    /// Expression
    pub expr: String,
    /// Error message
    pub message: String,
}

impl Invariant {
    /// Create a new invariant
    pub fn new(name: String, expr: String, message: String) -> Self {
        Self { name, expr, message }
    }
}

/// Catalog for IR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    /// Label definitions
    pub labels: HashMap<Label, LabelDef>,
    /// Index definitions
    pub indexes: Vec<IndexDef>,
    /// Invariants
    pub invariants: Vec<Invariant>,
}

impl Catalog {
    /// Create an empty catalog
    pub fn empty() -> Self {
        Self {
            labels: HashMap::new(),
            indexes: Vec::new(),
            invariants: Vec::new(),
        }
    }

    /// Add a label definition
    pub fn add_label(&mut self, def: LabelDef) {
        self.labels.insert(def.name.clone(), def);
    }

    /// Get a label definition
    pub fn get_label(&self, name: &Label) -> Option<&LabelDef> {
        self.labels.get(name)
    }

    /// Add an index definition
    pub fn add_index(&mut self, def: IndexDef) {
        self.indexes.push(def);
    }

    /// Add an invariant
    pub fn add_invariant(&mut self, inv: Invariant) {
        self.invariants.push(inv);
    }
}

impl Default for Catalog {
    fn default() -> Self {
        Self::empty()
    }
}

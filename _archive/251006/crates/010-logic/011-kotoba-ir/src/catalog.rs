//! Catalog-IR（スキーマ/索引/不変量）

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use kotoba_types::{Value as KotobaValue, PropertyKey, Label};

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
    pub default: Option<KotobaValue>,
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
    pub fn default(mut self, default: KotobaValue) -> Self {
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

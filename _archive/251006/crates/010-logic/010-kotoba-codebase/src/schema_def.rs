//! # Schema Definitions
//!
//! This module provides schema definitions for the Kotoba codebase.

use super::*;
use serde::{Deserialize, Serialize};
use crate::type_def::TypeDef;

/// Schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDef {
    /// Schema name
    pub name: String,
    /// Schema version
    pub version: String,
    /// Entity types
    pub entity_types: Vec<EntityTypeDef>,
    /// Attribute definitions
    pub attributes: Vec<AttributeDef>,
    /// Relations between entities
    pub relations: Vec<RelationDef>,
    /// Schema constraints
    pub constraints: Vec<SchemaConstraint>,
    /// Metadata
    pub metadata: SchemaMetadata,
}

impl SchemaDef {
    /// Create a new schema definition
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            entity_types: Vec::new(),
            attributes: Vec::new(),
            relations: Vec::new(),
            constraints: Vec::new(),
            metadata: SchemaMetadata::default(),
        }
    }

    /// Add an entity type
    pub fn with_entity_type(mut self, entity_type: EntityTypeDef) -> Self {
        self.entity_types.push(entity_type);
        self
    }

    /// Add an attribute
    pub fn with_attribute(mut self, attribute: AttributeDef) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Add a relation
    pub fn with_relation(mut self, relation: RelationDef) -> Self {
        self.relations.push(relation);
        self
    }

    /// Add a constraint
    pub fn with_constraint(mut self, constraint: SchemaConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

/// Entity type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeDef {
    /// Entity type name
    pub name: String,
    /// Attributes for this entity type
    pub attributes: Vec<AttributeRef>,
    /// Parent entity types
    pub parents: Vec<String>,
    /// Entity type metadata
    pub metadata: EntityMetadata,
}

/// Attribute reference
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeRef {
    /// Attribute name
    pub name: String,
    /// Is required
    pub required: bool,
    /// Cardinality
    pub cardinality: Cardinality,
}

/// Cardinality specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Cardinality {
    /// Single value
    Single,
    /// Multiple values
    Multiple,
    /// Optional single value
    Optional,
    /// Optional multiple values
    OptionalMultiple,
}

/// Attribute definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeDef {
    /// Attribute name
    pub name: String,
    /// Attribute type
    pub attr_type: TypeDef,
    /// Attribute constraints
    pub constraints: Vec<AttributeConstraint>,
    /// Metadata
    pub metadata: AttributeMetadata,
}

/// Attribute constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttributeConstraint {
    /// Unique constraint
    Unique,
    /// Index constraint
    Indexed,
    /// Range constraint
    Range(Value, Value),
    /// Enum constraint
    Enum(Vec<Value>),
    /// Custom constraint
    Custom(String),
}

/// Relation definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationDef {
    /// Relation name
    pub name: String,
    /// Source entity type
    pub source: String,
    /// Target entity type
    pub target: String,
    /// Relation type
    pub relation_type: RelationType,
    /// Relation metadata
    pub metadata: RelationMetadata,
}

/// Relation type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationType {
    /// One-to-one
    OneToOne,
    /// One-to-many
    OneToMany,
    /// Many-to-one
    ManyToOne,
    /// Many-to-many
    ManyToMany,
}

/// Schema constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SchemaConstraint {
    /// Referential integrity
    ReferentialIntegrity(String, String),
    /// Domain constraint
    DomainConstraint(String, Value, Value),
    /// Cardinality constraint
    CardinalityConstraint(String, Cardinality),
    /// Custom constraint
    Custom(String),
}

/// Schema metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaMetadata {
    /// Schema description
    pub description: Option<String>,
    /// Author
    pub author: Option<String>,
    /// Creation date
    pub created_at: Option<String>,
    /// Dependencies on other schemas
    pub dependencies: Vec<String>,
}

/// Entity metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EntityMetadata {
    /// Entity description
    pub description: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Icon or visual representation
    pub icon: Option<String>,
}

/// Attribute metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttributeMetadata {
    /// Attribute description
    pub description: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Category for grouping
    pub category: Option<String>,
    /// Default value
    pub default_value: Option<Value>,
}

/// Relation metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelationMetadata {
    /// Relation description
    pub description: Option<String>,
    /// Display name
    pub display_name: Option<String>,
    /// Is bidirectional
    pub bidirectional: bool,
}

//! # Type Definitions
//!
//! This module provides type definitions for the Kotoba codebase.

use super::*;
use serde::{Deserialize, Serialize};

/// Type definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDef {
    /// Type name
    pub name: String,
    /// Type kind
    pub kind: TypeKind,
    /// Type parameters
    pub parameters: Vec<TypeParameter>,
    /// Constraints
    pub constraints: Vec<TypeConstraint>,
    /// Metadata
    pub metadata: TypeMetadata,
}

impl TypeDef {
    /// Create a new type definition
    pub fn new(name: String, kind: TypeKind) -> Self {
        Self {
            name,
            kind,
            parameters: Vec::new(),
            constraints: Vec::new(),
            metadata: TypeMetadata::default(),
        }
    }

    /// Add a type parameter
    pub fn with_parameter(mut self, param: TypeParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Add a constraint
    pub fn with_constraint(mut self, constraint: TypeConstraint) -> Self {
        self.constraints.push(constraint);
        self
    }
}

/// Type kind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeKind {
    /// Primitive types
    Primitive(PrimitiveType),
    /// Product type (struct/record)
    Product(Vec<FieldDef>),
    /// Sum type (enum/union)
    Sum(Vec<VariantDef>),
    /// Function type
    Function {
        inputs: Vec<TypeDef>,
        output: Box<TypeDef>,
    },
    /// Generic type
    Generic {
        base: Box<TypeDef>,
        args: Vec<TypeDef>,
    },
    /// Reference to another type
    Reference(DefRef),
}

/// Primitive types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrimitiveType {
    /// Integer types
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    /// Floating point types
    F32, F64,
    /// Boolean
    Bool,
    /// String
    String,
    /// Bytes
    Bytes,
    /// Hash
    Hash,
    /// Entity ID
    EntityId,
    /// Attribute ID
    AttributeId,
    /// Value
    Value,
}

/// Field definition for product types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDef {
    /// Field name
    pub name: String,
    /// Field type
    pub field_type: TypeDef,
    /// Is optional
    pub optional: bool,
    /// Default value
    pub default: Option<Value>,
}

/// Variant definition for sum types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariantDef {
    /// Variant name
    pub name: String,
    /// Variant data
    pub data: Option<TypeDef>,
}

/// Type parameter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeParameter {
    /// Parameter name
    pub name: String,
    /// Parameter bounds
    pub bounds: Vec<TypeConstraint>,
}

/// Type constraint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TypeConstraint {
    /// Subtype constraint
    Subtype(TypeDef),
    /// Trait implementation
    Implements(String),
    /// Custom constraint
    Custom(String),
}

/// Type metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypeMetadata {
    /// Type description
    pub description: Option<String>,
    /// Size in bytes (if known)
    pub size: Option<usize>,
    /// Alignment requirement
    pub alignment: Option<usize>,
    /// Is copyable
    pub copyable: bool,
    /// Is serializable
    pub serializable: bool,
}

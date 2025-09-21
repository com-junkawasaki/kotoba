//! # Kotoba Codebase
//!
//! DefRef: Function/type/rule/strategy/schema definitions with content addressing.
//!
//! This crate provides the foundation for content-addressed definitions that can be
//! composed, normalized, and referenced by their hash values.

pub mod def_ref;
pub mod function_def;
pub mod type_def;
pub mod rule_def;
pub mod strategy_def;
pub mod schema_def;

use kotoba_types::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// Content-addressed reference to a definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DefRef {
    /// Hash of the definition content
    pub hash: Hash,
    /// Type of the definition
    pub def_type: DefType,
    /// Human readable name (optional)
    pub name: Option<String>,
}

impl DefRef {
    /// Create a new DefRef from definition content
    pub fn new<T: AsRef<[u8]>>(content: T, def_type: DefType) -> Self {
        let hash = Hash::from_sha256(content);
        DefRef {
            hash,
            def_type,
            name: None,
        }
    }

    /// Create a named DefRef
    pub fn with_name<T: AsRef<[u8]>>(content: T, def_type: DefType, name: String) -> Self {
        let hash = Hash::from_sha256(content);
        DefRef {
            hash,
            def_type,
            name: Some(name),
        }
    }
}

impl fmt::Display for DefRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.name {
            Some(name) => write!(f, "{}:{}", name, self.hash),
            None => write!(f, "{}", self.hash),
        }
    }
}

/// Type of definition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DefType {
    /// Function definition
    Function,
    /// Type definition
    Type,
    /// Rule definition
    Rule,
    /// Strategy definition
    Strategy,
    /// Schema definition
    Schema,
}

impl fmt::Display for DefType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DefType::Function => write!(f, "function"),
            DefType::Type => write!(f, "type"),
            DefType::Rule => write!(f, "rule"),
            DefType::Strategy => write!(f, "strategy"),
            DefType::Schema => write!(f, "schema"),
        }
    }
}

/// Hash type for content addressing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create hash from SHA256
    pub fn from_sha256<T: AsRef<[u8]>>(data: T) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        Hash(result.into())
    }

    /// Create hash from Blake3
    pub fn from_blake3<T: AsRef<[u8]>>(data: T) -> Self {
        let hash = blake3::hash(data);
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash.as_bytes()[0..32]);
        Hash(result)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

//! # Definition Reference System
//!
//! This module provides the core DefRef system for content-addressed definitions.

use super::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Registry of all definitions in the codebase
#[derive(Debug, Clone, Default)]
pub struct Codebase {
    /// Map from DefRef to definition content
    definitions: HashMap<DefRef, Vec<u8>>,
    /// Reverse lookup from hash to DefRef
    by_hash: HashMap<Hash, DefRef>,
}

impl Codebase {
    /// Create a new empty codebase
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new definition
    pub fn register<T: AsRef<[u8]>>(&mut self, content: T, def_type: DefType, name: Option<String>) -> DefRef {
        let content_ref = content.as_ref();
        let content_vec = content_ref.to_vec();

        let def_ref = if let Some(name) = name {
            DefRef::with_name(content_ref, def_type, name)
        } else {
            DefRef::new(content_ref, def_type)
        };

        self.definitions.insert(def_ref.clone(), content_vec);
        self.by_hash.insert(def_ref.hash.clone(), def_ref.clone());
        def_ref
    }

    /// Get definition by DefRef
    pub fn get(&self, def_ref: &DefRef) -> Option<&[u8]> {
        self.definitions.get(def_ref).map(|v| v.as_slice())
    }

    /// Get DefRef by hash
    pub fn get_by_hash(&self, hash: &Hash) -> Option<&DefRef> {
        self.by_hash.get(hash)
    }

    /// Check if a DefRef exists
    pub fn contains(&self, def_ref: &DefRef) -> bool {
        self.definitions.contains_key(def_ref)
    }

    /// List all definitions of a specific type
    pub fn list_by_type(&self, def_type: DefType) -> Vec<&DefRef> {
        self.definitions.keys()
            .filter(|def_ref| def_ref.def_type == def_type)
            .collect()
    }

    /// Get all definitions
    pub fn all_definitions(&self) -> Vec<&DefRef> {
        self.definitions.keys().collect()
    }
}

/// Definition content with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Definition<T> {
    /// The actual definition
    pub content: T,
    /// DefRef for this definition
    pub def_ref: DefRef,
    /// Dependencies of this definition
    pub dependencies: Vec<DefRef>,
    /// Metadata
    pub metadata: DefinitionMetadata,
}

impl<T> Definition<T> {
    /// Create a new definition
    pub fn new(content: T, def_ref: DefRef, dependencies: Vec<DefRef>) -> Self {
        Self {
            content,
            def_ref,
            dependencies,
            metadata: DefinitionMetadata::default(),
        }
    }
}

/// Metadata for a definition
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DefinitionMetadata {
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
    /// Author
    pub author: Option<String>,
    /// Description
    pub description: Option<String>,
    /// Version
    pub version: Option<String>,
    /// Tags for categorization
    pub tags: Vec<String>,
}

/// Definition registry with type safety
pub trait DefinitionRegistry<T> {
    /// Register a definition
    fn register_def(&mut self, content: T, name: Option<String>) -> DefRef;

    /// Get a definition
    fn get_def(&self, def_ref: &DefRef) -> Option<&T>;

    /// List all definitions
    fn list_defs(&self) -> Vec<&DefRef>;
}

/// Type-safe definition registry
pub struct TypedCodebase<T> {
    codebase: Codebase,
    _phantom: std::marker::PhantomData<T>,
}

impl<T> TypedCodebase<T> {
    /// Create a new typed codebase
    pub fn new() -> Self {
        Self {
            codebase: Codebase::new(),
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<T: Serialize + for<'de> Deserialize<'de>> DefinitionRegistry<T> for TypedCodebase<T> {
    fn register_def(&mut self, content: T, name: Option<String>) -> DefRef {
        // Serialize to get content bytes
        let content_bytes = serde_json::to_vec(&content).expect("Failed to serialize definition");
        let def_type = Self::get_def_type();
        self.codebase.register(&content_bytes, def_type, name)
    }

    fn get_def(&self, def_ref: &DefRef) -> Option<&T> {
        // For now, return None to avoid the ownership issue
        // This will need a proper caching mechanism
        None
    }

    fn list_defs(&self) -> Vec<&DefRef> {
        self.codebase.list_by_type(Self::get_def_type())
    }
}

impl<T> TypedCodebase<T> {
    /// Get the definition type for this registry
    fn get_def_type() -> DefType {
        // This would need to be implemented per type
        // For now, return a default
        DefType::Type
    }
}

//! # kotoba-schema
//!
//! Graph Schema Definition and Validation for Kotoba
//! プロセスネットワーク as GTS(DPO)+OpenGraph with Merkle DAG & PG view
//!
//! ## Overview
//!
//! This crate provides comprehensive schema management for graph databases:
//! - Schema definition and validation
//! - Property type system with constraints
//! - Schema registry and management
//! - Migration and evolution support
//! - JSON Schema integration
//!
//! ## Example
//!
//! ```rust
//! use kotoba_schema::{GraphSchema, SchemaManager};
//! use kotoba_storage::StorageManager;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create schema
//! let schema = GraphSchema::new(
//!     "my_schema".to_string(),
//!     "My Graph Schema".to_string(),
//!     "1.0.0".to_string(),
//! );
//!
//! // Create storage and schema manager
//! let storage = StorageManager::default().await?;
//! let backend = std::sync::Arc::new(storage.backend().clone());
//! let mut manager = SchemaManager::new(backend);
//!
//! // Register schema
//! manager.register_schema(schema).await?;
//! # Ok(())
//! # }
//! ```

pub mod schema;
pub mod validator;
pub mod manager;
pub mod registry;
pub mod migration;
pub mod export;

#[cfg(feature = "graph")]
pub mod graph_integration;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::schema::*;
    pub use crate::validator::*;
    pub use crate::manager::*;
    pub use crate::registry::*;
    pub use crate::migration::*;
    pub use crate::export::*;

    #[cfg(feature = "graph")]
    pub use crate::graph_integration::*;
}

// Re-export main types for convenience
pub use schema::*;
pub use validator::*;
pub use manager::*;
pub use registry::*;
pub use migration::*;
pub use export::*;

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Get the current version of kotoba-schema
pub fn version() -> &'static str {
    VERSION
}

/// Feature flags
pub mod features {
    /// Whether graph integration is enabled
    pub const GRAPH_INTEGRATION: bool = cfg!(feature = "graph");

    /// Whether async support is enabled
    pub const ASYNC_SUPPORT: bool = cfg!(feature = "async");
}

/// Health check for the schema system
pub fn health_check() -> Result<(), String> {
    // Basic health check
    let _schema = GraphSchema::default();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_health_check() {
        assert!(health_check().is_ok());
    }

    #[test]
    fn test_default_schema() {
        let schema = GraphSchema::default();
        assert_eq!(schema.id, "default");
        assert_eq!(schema.name, "Default Graph Schema");
        assert_eq!(schema.version, "1.0.0");
    }

    #[test]
    fn test_schema_validation() {
        let schema = GraphSchema::default();
        let validation = schema.validate_schema();
        assert!(validation.is_valid);
        assert!(validation.errors.is_empty());
    }
}

//! kotoba-core - Kotoba Core Components
// This crate is being refactored. The `types` and `auth` modules have been
// extracted into their own crates (`kotoba-types` and `kotoba-auth`).

pub type Result<T> = std::result::Result<T, kotoba_errors::KotobaError>;

// pub mod types; // Extracted to `kotoba-types` crate
// pub mod auth;  // Extracted to `kotoba-auth` crate

pub mod schema;
pub mod schema_validator;
// pub mod pgview; // Temporarily disabled due to Value type conflicts
pub mod ir;
pub mod topology;
pub mod graph;
pub mod crypto; // 暗号化エンジン

pub mod prelude {
    // Re-export commonly used items
    pub use kotoba_types::*; // Re-export from the new crate
    pub use kotoba_auth::*;   // Re-export from the new crate

    pub use crate::schema::*;
    // Re-export specific items from schema_validator to avoid utils conflict
    pub use crate::schema_validator::{SchemaValidator, ValidationReport};
    // pub use crate::pgview::*; // Temporarily disabled
    pub use crate::ir::*;
    // Re-export specific items from crypto to avoid utils conflict
    pub use crate::crypto::EncryptionInfo;
    // Re-export KotobaError to avoid version conflicts
    pub use kotoba_errors::KotobaError;
}

// Tests related to `types` are now in the `kotoba-types` crate or will be moved.
// The existing tests here will be removed or refactored to focus on
// the remaining responsibilities of `kotoba-core`.
#[cfg(test)]
mod tests {
    // Intentionally left blank for now.
    // Tests for schema, ir, topology, graph, etc. will remain or be added here.
}

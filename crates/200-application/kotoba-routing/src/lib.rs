//! `kotoba-routing`
//!
//! A declarative, graph-based HTTP routing engine for the Kotoba ecosystem.
//! It transforms `.kotoba` route files into executable workflows with KeyValueStore backend.

pub mod engine;
pub mod schema;

pub use engine::*;
pub use schema::*;

// Re-export KeyValueStore for convenience
pub use kotoba_storage::KeyValueStore;

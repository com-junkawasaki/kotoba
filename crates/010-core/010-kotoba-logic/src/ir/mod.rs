//! Intermediate Representation (IR) for Kotoba

pub mod catalog;
pub mod rule;
pub mod query;
pub mod patch;
pub mod strategy;

// Re-export everything for convenience
pub use catalog::*;
pub use rule::*;
pub use query::*;
pub use patch::*;
pub use strategy::*;

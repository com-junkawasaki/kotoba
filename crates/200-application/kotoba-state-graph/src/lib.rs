//! `kotoba-state-graph`
//!
//! A library for managing UI state as a graph within the Kotoba ecosystem with KeyValueStore backend.
//! It provides:
//!   - A standard schema for representing UI components and their state.
//!   - Generic, reusable graph rewrite rules for common UI state transitions.
//!   - A high-level `.kotobas` accessor library to abstract away GQL and rewrite logic.

pub mod rules;
pub mod schema;

pub use rules::*;
pub use schema::*;

// Re-export KeyValueStore for convenience
pub use kotoba_storage::KeyValueStore;

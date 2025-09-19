//! `kotoba-routing`
//!
//! A declarative, graph-based HTTP routing engine for the Kotoba ecosystem.
//! It transforms `.kotoba` route files into executable workflows.

pub mod engine;
pub mod schema;

pub use engine::*;
pub use schema::*;

//! # Kotoba Intermediate Representation (IR)
//!
//! Pure intermediate representation system for Kotoba, providing:
//! - catalog-IR: schema/index/invariant definitions
//! - rule-IR: DPO typed attribute graph rewriting
//! - query-IR: GQL logical plan algebra
//! - patch-IR: differential expressions
//! - strategy-IR: minimal strategy expressions
//!
//! This is a pure IR layer that depends only on basic types and provides
//! the foundation for rewrite kernels and composition systems.

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

// Core IR types
pub use crate::catalog::*;
pub use crate::rule::*;
pub use crate::query::*;
pub use crate::patch::*;
pub use crate::strategy::*;

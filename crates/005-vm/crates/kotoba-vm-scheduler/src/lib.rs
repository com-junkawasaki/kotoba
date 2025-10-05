//! Merkle DAG: vm-scheduler
//! Advanced task scheduler with hardware-aware scheduling policies and
//! content-addressable caching for redundancy elimination.
//!
//! The vm-scheduler crate provides:
//! - **MemoizationEngine**: Content-addressable caching for task results
//! - **DataflowRuntime**: Runtime for task execution with dependency management
//! - **HardwareAwareScheduler**: Advanced scheduler with multiple scheduling policies
//! - **ExecutionEngine**: High-level execution engine orchestrating scheduling and execution
//!
//! ## Key Components
//!
//! - [`MemoizationEngine`]: Content-addressable caching system
//! - [`DataflowRuntime`]: Task execution runtime with dependency resolution
//! - [`Scheduler`]: Hardware-aware task scheduler with multiple policies
//! - [`ExecutionEngine`]: High-level execution engine for DAGs
//!
//! ## Usage
//!
//! ```rust
//! use vm_scheduler::{ExecutionEngine, MemoizationEngineImpl, DataflowRuntimeImpl, HardwareAwareScheduler};
//!
//! let engine = ExecutionEngine::default();
//! // ... register DAGs and execute them
//! ```
//!
//! See the unit tests for detailed usage examples.

#![allow(dead_code)] // TODO: Remove this later on

// Memoization engine for content-addressable caching
pub mod memoization;

// Dataflow runtime for task execution
pub mod runtime;

// Hardware-aware task scheduler
pub mod scheduler;

// High-level execution engine
pub mod execution;

// Re-export all public items from submodules for convenient access
pub use crate::memoization::*;
pub use crate::runtime::*;
pub use crate::scheduler::*;
pub use crate::execution::*;

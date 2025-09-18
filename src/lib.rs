//! # Kotoba: Core Graph Processing System
//!
//! GP2-based Graph Rewriting Language with ISO GQL-compliant queries,
//! MVCC+Merkle persistence, and distributed execution.
//!
//! ## Core Architecture
//!
//! Kotoba's core consists of the following published crates:
//! - `kotoba-core`: Core types and IR definitions
//! - `kotoba-errors`: Unified error handling
//! - `kotoba-graph`: Graph data structures
//! - `kotoba-storage`: Persistence layer with MVCC and Merkle DAG
//! - `kotoba-execution`: Query execution and planning
//! - `kotoba-rewrite`: Graph rewriting engine
//! - `kotoba-cli`: Basic command-line interface
//!
//! ## Optional Features
//!
//! Additional functionality is available through optional crates:
//! - Workflow engine, HTTP server, distributed execution, etc.
//! (These are currently disabled to focus on core stability)

// Re-export from core published crates only
pub use kotoba_core as core;
pub use kotoba_graph as graph;
pub use kotoba_storage as storage;
pub use kotoba_execution as execution;
pub use kotoba_rewrite as rewrite;

// Optional crates (commented out for core focus)
// pub use kotoba_distributed as distributed;
// pub use kotoba_network as network;
// pub use kotoba_cid as cid;
// pub use kotoba_cli as cli;
// pub use kotoba_deploy; // Temporarily disabled until crate is fixed
// pub use kotoba_web as web; // まだpublishされていないため一時的にコメントアウト

// Local modules
// pub mod cid; // Moved to kotoba-cid crate
// pub mod cli; // Moved to kotoba-cli crate
// pub mod pgview; // Moved to kotoba-core
// pub mod schema; // Moved to kotoba-core
// pub mod schema_test; // Moved to kotoba-core
// pub mod distributed; // Moved to kotoba-distributed crate
// pub mod network_protocol; // Moved to kotoba-network crate
// pub mod schema_validator; // Moved to kotoba-core
// pub mod topology; // Excluded from publish
// pub mod types; // Moved to kotoba-core
// pub mod frontend; // Moved to kotoba2tsx
// pub mod http; // Moved to kotoba-server

// Convenient re-exports for common usage
pub use kotoba_core::prelude::*;
pub use kotoba_graph::prelude::*;
// pub use kotoba_storage::prelude::*; // Storage crate has issues with prelude
pub use kotoba_execution::prelude::*;
pub use kotoba_rewrite::prelude::*;
// pub use kotoba_distributed::prelude::*; // Distributed crate has no prelude yet
// pub use kotoba_network::prelude::*; // Network crate has no prelude yet
// pub use kotoba_cid::prelude::*; // CID crate has no prelude yet
// pub use kotoba_cli::prelude::*; // CLI crate has no prelude yet
// pub use kotoba_deploy::*; // Temporarily disabled until crate is fixed
// pub use kotoba_web::prelude::*; // まだpublishされていないため一時的にコメントアウト

// Examples and topology are excluded from publish
// #[cfg(feature = "examples")]
// pub mod examples;
// // pub mod topology; // Excluded from publish

// pub use topology::*;

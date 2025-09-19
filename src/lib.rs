//! # Kotoba: Core Graph Processing System
//!
//! A comprehensive graph processing platform featuring GP2-based graph rewriting,
//! complete Event Sourcing, ISO GQL-compliant queries, MVCC+Merkle persistence,
//! and distributed execution using the Port/Adapter (Hexagonal) Architecture.
//!
//! ## Architecture Overview
//!
//! Kotoba is built with the **Port/Adapter (Hexagonal) Architecture**:
//!
//! - **🎯 Application Layer**: Business logic (Event Sourcing, Graph Queries, Rewriting)
//! - **🔧 Infrastructure Layer**: Storage adapters (RocksDB, Redis, In-Memory)
//! - **🏛️ Presentation Layer**: CLI, HTTP APIs, Web interfaces
//!
//! ## Key Features
//!
//! - **Complete Event Sourcing**: Immutable events, projections, materialized views
//! - **ISO GQL-compliant Queries**: Industry-standard graph query language
//! - **Port/Adapter Pattern**: Clean separation of business logic and infrastructure
//! - **Multiple Storage Backends**: RocksDB, Redis, In-Memory implementations
//! - **Graph Rewriting**: GP2-based graph transformations
//! - **Distributed Execution**: Multi-node coordination and consensus
//!
//! ## Usage
//!
//! ```rust
//! use kotoba::*;
//!
//! // Create storage adapter
//! let storage = kotoba_storage::MemoryAdapter::new();
//!
//! // Create event stream
//! let event_stream = kotoba_event_stream::EventStream::new(storage);
//!
//! // Execute GQL query
//! let result = kotoba_query_engine::execute_gql("MATCH (n) RETURN n", &event_stream).await;
//! ```
//!
//! ## Crate Organization
//!
//! - **000-core**: Foundation types, error handling, CID
//! - **100-storage**: Storage adapters and persistence
//! - **200-application**: Business logic and domain services
//! - **300-workflow**: Workflow orchestration
//! - **400-language**: Language support (Jsonnet, KotobaScript)
//! - **500-services**: HTTP servers and APIs
//! - **600-deployment**: Deployment and scaling
//! - **900-tools**: Development tools and CLI

// Re-export main components for convenience (when available)
// Note: These re-exports are optional and will only work if the corresponding crates are available
// and properly implemented in the current build configuration.

// Core error type
pub use kotoba_errors::KotobaError;

// pub use kotoba_core as core; // Temporarily disabled - may not be available
// pub use kotoba_storage as storage; // Temporarily disabled - may not be available
// pub use kotoba_event_stream as event_stream; // Temporarily disabled - may not be available
// pub use kotoba_query_engine as query_engine; // Temporarily disabled - may not be available
// pub use kotoba_execution as execution; // Temporarily disabled - may not be available
// pub use kotoba_rewrite as rewrite; // Temporarily disabled - may not be available
// pub use kotoba_routing as routing; // Temporarily disabled - may not be available
// pub use kotoba_state_graph as state_graph; // Temporarily disabled - may not be available
// pub use kotoba_jsonnet as jsonnet; // Temporarily disabled - may not be available
// pub use kotoba_kotobas as kotobas; // Temporarily disabled - may not be available

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const NAME: &str = "Kotoba";
pub const DESCRIPTION: &str = "Core Graph Processing System (GP2 + Event Sourcing + ISO GQL) - Port/Adapter Architecture";

// Public modules
pub mod topology;
//! # Kotoba: GP2系グラフ書換え言語
//!
//! ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム

pub mod types;
pub mod ir;
pub mod graph;
pub mod execution;
pub mod storage;
pub mod planner;
pub mod rewrite;

pub use types::{Value, Properties, GraphRef_, TxId, ContentHash, KotobaError, Result};
pub use ir::*;
pub use graph::{Graph, GraphRef, VertexData, EdgeData, VertexBuilder, EdgeBuilder};
pub use execution::*;
pub use storage::*;
pub use planner::*;
pub use rewrite::*;

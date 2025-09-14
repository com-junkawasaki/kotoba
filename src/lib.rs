//! # Kotoba: GP2系グラフ書換え言語
//!
//! ISO GQL準拠クエリ、MVCC+Merkle永続、分散実行まで一貫させたグラフ処理システム
//!
//! ## Multi-Crate Architecture
//!
//! Kotobaは以下のcrateに分割されています：
//! - `kotoba-core`: 基本型とIR定義
//! - `kotoba-graph`: グラフデータ構造
//! - `kotoba-storage`: 永続化層
//! - `kotoba-execution`: クエリ実行とプランナー
//! - `kotoba-rewrite`: グラフ書き換え
//! - `kotoba-web`: WebフレームワークとHTTP

// Re-export from individual crates
pub use kotoba_core as core;
pub use kotoba_graph as graph;
pub use kotoba_storage as storage;
pub use kotoba_execution as execution;
pub use kotoba_rewrite as rewrite;
// pub use kotoba_web as web; // まだpublishされていないため一時的にコメントアウト

// Convenient re-exports for common usage
pub use kotoba_core::prelude::*;
pub use kotoba_graph::prelude::*;
pub use kotoba_storage::prelude::*;
pub use kotoba_execution::prelude::*;
pub use kotoba_rewrite::prelude::*;
// pub use kotoba_web::prelude::*; // まだpublishされていないため一時的にコメントアウト

// Examples
#[cfg(feature = "examples")]
pub mod examples;
pub mod topology;

pub use topology::*;

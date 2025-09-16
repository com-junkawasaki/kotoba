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
//! - `kotoba-server`: ServerフレームワークとHTTP
//! - `kotoba-distributed`: 分散実行エンジン
//! - `kotoba-network`: ネットワーク通信プロトコル
//! - `kotoba-cid`: CID (Content ID) システム
//! - `kotoba-cli`: コマンドラインインターフェース

// Re-export from individual crates
pub use kotoba_core as core;
pub use kotoba_graph as graph;
pub use kotoba_storage as storage;
pub use kotoba_execution as execution;
pub use kotoba_rewrite as rewrite;
pub use kotoba_distributed as distributed;
pub use kotoba_network as network;
pub use kotoba_cid as cid;
pub use kotoba_cli as cli;
// pub use kotoba_deploy; // Temporarily disabled until crate is fixed
// pub use kotoba_web as web; // まだpublishされていないため一時的にコメントアウト

// Local modules
// pub mod cid; // Moved to kotoba-cid crate
// pub mod cli; // Moved to kotoba-cli crate
pub mod pgview;
pub mod schema;
pub mod schema_test;
// pub mod distributed; // Moved to kotoba-distributed crate
// pub mod network_protocol; // Moved to kotoba-network crate
pub mod schema_validator;
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

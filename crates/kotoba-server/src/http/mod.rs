//! HTTPサーバーモジュール
//!
//! このモジュールはKotobaのIRを使ってHTTPサーバーを構築するためのコンポーネントを提供します。
//! 主な機能:
//! - .kotoba.json/.kotobaファイルのパース
//! - HTTPリクエストのグラフ変換
//! - ルーティングとミドルウェア処理
//! - レスポンス生成

pub mod ir;
pub mod parser;
pub mod handlers;
pub mod engine;
pub mod server;

// HTTP関連の型を再エクスポート
pub use ir::*;
pub use parser::*;
pub use handlers::*;
pub use engine::*;
pub use server::*;

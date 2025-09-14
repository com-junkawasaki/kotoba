//! Kotoba Web Framework - Next.js風フルスタックWebフレームワーク
//!
//! このモジュールはKotobaのIRを使ってNext.jsのようなフルスタックWebフレームワークを実装します。
//! 特徴:
//! - App Router風ファイルベースルーティング
//! - Server Components / Client Components
//! - REST API / GraphQL API
//! - データベース統合
//! - 認証・認可システム
//! - ミドルウェアシステム
//! - Jsonnetベースの設定ファイル

pub mod component_ir;
pub mod route_ir;
pub mod render_ir;
pub mod build_ir;
pub mod api_ir;
pub mod framework;

// Re-export
pub use component_ir::*;
pub use route_ir::*;
pub use render_ir::*;
pub use build_ir::*;
pub use api_ir::*;
pub use framework::{WebFramework, ComponentRenderer, ComponentRegistry, BuildEngine};

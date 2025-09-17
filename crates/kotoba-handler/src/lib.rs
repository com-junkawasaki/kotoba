//! Kotoba Unified Handler
//!
//! このクレートはKotobaエコシステム全体の統合的なhandlerを提供します。
//! 既存のkotoba-jsonnetとkotoba-kotobasの機能を統合し、
//! サーバー、CLI、WASM実行を統一的に扱います。

pub mod error;
pub mod types;
pub mod handler;
pub mod executor;
pub mod runtime;
pub mod integration;

#[cfg(feature = "server")]
pub mod server;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "websocket")]
pub mod websocket;

#[cfg(feature = "cli")]
pub mod cli;

pub use error::{HandlerError, Result};
pub use types::*;
pub use handler::UnifiedHandler;
pub use executor::HandlerExecutor;

/// Handlerの初期化と実行を簡略化するためのヘルパー関数
#[cfg(feature = "server")]
pub async fn run_server(addr: &str) -> Result<()> {
    server::run(addr).await
}

/// WASM環境でのhandler初期化
#[cfg(feature = "wasm")]
pub fn init_wasm_handler() -> Result<wasm::WasmHandler> {
    wasm::WasmHandler::new()
}

/// CLI経由でのhandler実行
#[cfg(feature = "cli")]
pub async fn execute_cli_handler(file: &str, args: Vec<String>) -> Result<String> {
    cli::execute_handler(file, args).await
}

/// 最もシンプルなhandler実行関数
pub async fn execute_simple_handler(content: &str, context: HandlerContext) -> Result<String> {
    let handler = UnifiedHandler::new();
    handler.execute(content, context).await
}

//! Kotoba CLI - Deno-inspired command line interface
//!
//! Merkle DAG: cli_interface (build_order: 10)
//! Provides: Cli, Commands, ConfigManager, ProgressBar, LogFormatter
//! Dependencies: types, distributed_engine, network_protocol, cid_system

use clap::{Parser, Subcommand};

// Re-export core types for CLI interface
pub use config::ConfigManager;
pub use logging::LogFormatter;
pub use utils::ProgressBar;

// Import modules
pub mod commands;
pub mod config;
pub mod logging;
pub mod utils;

/// Kotoba CLIのメイン構造体
/// Merkle DAG: cli_interface -> Cli component
#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - GP2-based Graph Rewriting Language")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Kotoba CLIのサブコマンド
/// Merkle DAG: cli_interface -> Commands component
#[derive(Subcommand)]
pub enum Commands {
    /// プロジェクト情報を表示
    Info {
        /// 詳細表示
        #[arg(short, long)]
        verbose: bool,
    },

    /// 指定されたKotobaファイルを評価
    Eval {
        /// 評価するファイルのパス
        path: String,

        /// Top-level argument (code) - can be specified multiple times
        #[arg(long = "tla-code", value_names = ["KEY", "CODE"])]
        tla_code: Vec<String>,

        /// Top-level argument (string) - can be specified multiple times
        #[arg(long = "tla-str", value_names = ["KEY", "STRING"])]
        tla_str: Vec<String>,
    },

    /// ドキュメント生成・管理コマンド
    #[command(subcommand)]
    Docs(DocsCommand),
}

/// ドキュメント関連サブコマンド
/// Merkle DAG: docs_cli -> docs generate, docs serve, docs search, docs init
#[derive(Subcommand)]
pub enum DocsCommand {
    /// ドキュメントを生成
    Generate {
        /// ソースディレクトリ
        #[arg(short, long, default_value = "src")]
        source: String,

        /// 出力ディレクトリ
        #[arg(short, long, default_value = "docs")]
        output: String,

        /// 設定ファイル
        #[arg(short, long)]
        config: Option<String>,

        /// ウォッチモード
        #[arg(short, long)]
        watch: bool,
    },

    /// ドキュメントサーバーを起動
    Serve {
        /// ポート番号
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// ホストアドレス
        #[arg(short, long, default_value = "127.0.0.1")]
        host: String,

        /// ドキュメントディレクトリ
        #[arg(short, long, default_value = "docs")]
        dir: String,

        /// オープン後にブラウザで開く
        #[arg(short, long)]
        open: bool,
    },

    /// ドキュメントを検索
    Search {
        /// 検索クエリ
        query: String,

        /// 検索対象ディレクトリ
        #[arg(short, long, default_value = "docs")]
        dir: String,

        /// JSON形式で出力
        #[arg(short, long)]
        json: bool,
    },

    /// ドキュメント設定を初期化
    Init {
        /// 設定ファイル名
        #[arg(short, long, default_value = "kdoc.toml")]
        config: String,

        /// 強制的に上書き
        #[arg(short, long)]
        force: bool,
    },
}

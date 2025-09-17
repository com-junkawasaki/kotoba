//! Kotoba CLI - Denoを参考にしたグラフ処理システムのコマンドラインインターフェース
//!
//! このモジュールはKotobaのメインCLIを提供し、Deno CLIを参考にした使いやすい
//! インターフェースを実装します。

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Kotoba CLIのメイン構造体
#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - Graph processing system inspired by Deno")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// 設定ファイルパス
    #[arg(short, long, global = true)]
    pub config: Option<PathBuf>,

    /// ログレベル
    #[arg(short, long, global = true, default_value = "info")]
    pub log_level: String,

    /// 作業ディレクトリ
    #[arg(short = 'C', long, global = true)]
    pub cwd: Option<PathBuf>,
}

/// Kotoba CLIのサブコマンド
#[derive(Subcommand)]
pub enum Commands {
    /// .kotobaファイルを実行
    Run {
        /// 実行するファイル
        file: PathBuf,

        /// 引数
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,

        /// ウォッチモード
        #[arg(short, long)]
        watch: bool,

        /// 許可する権限
        #[arg(short = 'A', long)]
        allow_all: bool,

        /// 環境変数
        #[arg(short = 'E', long)]
        env_vars: Vec<String>,
    },

    /// サーバーを起動
    Serve {
        /// ポート番号
        #[arg(short, long, default_value = "3000")]
        port: u16,

        /// ホストアドレス
        #[arg(short = 'H', long, default_value = "127.0.0.1")]
        host: String,

        /// 設定ファイル
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// 開発モード
        #[arg(long)]
        dev: bool,
    },

    /// ファイルをコンパイル
    Compile {
        /// コンパイルするファイル
        file: PathBuf,

        /// 出力ファイル
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// 最適化レベル
        #[arg(short, long, default_value = "0")]
        optimize: u8,
    },

    /// プロジェクトを初期化
    Init {
        /// プロジェクト名
        name: Option<String>,

        /// テンプレート
        #[arg(short, long, default_value = "basic")]
        template: String,

        /// 強制的に上書き
        #[arg(short, long)]
        force: bool,
    },

    /// キャッシュを管理
    Cache {
        #[command(subcommand)]
        subcommand: CacheCommands,
    },

    /// 情報を表示
    Info {
        /// 詳細表示
        #[arg(short, long)]
        verbose: bool,
    },

    /// テストを実行
    Test {
        /// テストファイルまたはディレクトリ
        #[arg(default_value = ".")]
        path: PathBuf,

        /// フィルター
        #[arg(short, long)]
        filter: Option<String>,

        /// 詳細出力
        #[arg(short, long)]
        verbose: bool,
    },

    /// ドキュメントを生成
    Doc {
        /// ドキュメント化するファイル
        file: Option<PathBuf>,

        /// 出力ディレクトリ
        #[arg(short, long, default_value = "docs")]
        output: PathBuf,

        /// オープン
        #[arg(long)]
        open: bool,
    },

    /// フォーマット
    Fmt {
        /// フォーマットするファイル
        files: Vec<PathBuf>,

        /// チェックのみ
        #[arg(long)]
        check: bool,

        /// 書き込み
        #[arg(short, long)]
        write: bool,
    },

    /// リント
    Lint {
        /// リントするファイル
        files: Vec<PathBuf>,

        /// 設定ファイル
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// 修正を適用
        #[arg(long)]
        fix: bool,
    },

    /// REPLを起動
    Repl,

    /// ビルドツール
    Build {
        /// 実行するタスク
        task: Option<String>,

        /// 設定ファイルのパス
        #[arg(short, long)]
        config: Option<PathBuf>,

        /// ファイル監視モード
        #[arg(short, long)]
        watch: bool,

        /// 詳細出力
        #[arg(short, long)]
        verbose: bool,

        /// 利用可能なタスク一覧を表示
        #[arg(short, long)]
        list: bool,

        /// ビルドアーティファクトをクリーン
        #[arg(long)]
        clean: bool,
    },

    /// テストを実行
    Test {
        /// テストファイルまたはディレクトリ
        files: Vec<PathBuf>,

        /// フィルター
        #[arg(short, long)]
        filter: Option<String>,

        /// 詳細出力
        #[arg(short, long)]
        verbose: bool,

        /// カバレッジ収集
        #[arg(long)]
        coverage: bool,

        /// レポート形式 (pretty, json, junit, tap)
        #[arg(long, default_value = "pretty")]
        format: String,
    },

    /// タスクを実行
    Task {
        /// タスク名
        name: String,

        /// 引数
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// アップグレード
    Upgrade {
        /// バージョン
        version: Option<String>,

        /// 強制アップグレード
        #[arg(long)]
        force: bool,
    },
}

/// キャッシュサブコマンド
#[derive(Subcommand)]
pub enum CacheCommands {
    /// キャッシュをクリア
    Clear,

    /// キャッシュの場所を表示
    Dir,

    /// キャッシュのサイズを表示
    Size,

    /// キャッシュの内容を表示
    List,
}

// 実装は別ファイルに分離
pub mod commands;
pub mod config;
pub mod logging;
pub mod utils;

// 再エクスポート
pub use commands::*;
pub use config::*;
pub use logging::*;
pub use utils::*;

//! Kotoba CLI - Core functionality only
//!
//! This module provides the basic CLI for Kotoba, focusing on core features.

use clap::{Parser, Subcommand};

/// Kotoba CLIのメイン構造体
#[derive(Parser)]
#[command(name = "kotoba")]
#[command(about = "Kotoba - Graph processing system core")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Kotoba CLIのサブコマンド (Core features only)
#[derive(Subcommand)]
pub enum Commands {
    /// プロジェクト情報を表示
    Info {
        /// 詳細表示
        #[arg(short, long)]
        verbose: bool,
    },
}

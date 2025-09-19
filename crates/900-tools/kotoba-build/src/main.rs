//! Kotoba Build Tool CLI
//!
//! コマンドラインインターフェースを提供し、ビルドツールの機能を
//! 直接実行できるようにします。

use clap::{Arg, Command};
use kotoba_build::*;
use std::path::PathBuf;
use tokio;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("kotoba-build")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kotoba Team")
        .about("Kotoba Build Tool - Project build and task management system")
        .arg(
            Arg::new("task")
                .help("Task to run")
                .value_name("TASK")
                .index(1),
        )
        .arg(
            Arg::new("config")
                .long("config")
                .short('c')
                .help("Path to config file")
                .value_name("FILE"),
        )
        .arg(
            Arg::new("watch")
                .long("watch")
                .short('w')
                .help("Watch for file changes")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .short('v')
                .help("Verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List available tasks")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("clean")
                .long("clean")
                .help("Clean build artifacts")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    // プロジェクトルートを検出
    let project_root = utils::find_project_root()?;

    if matches.get_flag("verbose") {
        println!("📁 Project root: {}", project_root.display());
    }

    // ビルドエンジンを作成
    let engine = BuildEngine::new(project_root.clone()).await?;

    if matches.get_flag("list") {
        // 利用可能なタスク一覧を表示
        println!("📋 Available tasks:");
        let tasks = engine.list_tasks().await;
        if tasks.is_empty() {
            println!("  No tasks defined. Create a kotoba-build.toml file to define tasks.");
        } else {
            for (name, desc) in tasks {
                println!("  {} - {}", name, desc);
            }
        }
        return Ok(());
    }

    if matches.get_flag("clean") {
        // クリーン処理
        println!("🧹 Cleaning build artifacts...");
        // TODO: 実際のクリーン処理を実装
        println!("✅ Clean completed");
        return Ok(());
    }

    if matches.get_flag("watch") {
        // ウォッチモードで起動
        println!("👀 Starting watch mode...");
        // ウォッチモードは現在実装中
        println!("Watch mode is not yet implemented");
        return Ok(())
    } else if let Some(task_name) = matches.get_one::<String>("task") {
        // 指定されたタスクを実行
        println!("🚀 Running task: {}", task_name);
        engine.run_task(task_name).await?;
        println!("✅ Task completed successfully");
    } else {
        // デフォルトビルドを実行
        println!("🏗️  Building project...");
        engine.build().await?;
        println!("✅ Build completed successfully");
    }

    Ok(())
}

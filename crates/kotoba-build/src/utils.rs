//! ユーティリティ関数モジュール

use super::{Result, BuildError};
use std::path::{Path, PathBuf};
use std::fs;

/// プロジェクトのルートディレクトリを検出
pub fn find_project_root() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()
        .map_err(|e| BuildError::Build(format!("Failed to get current directory: {}", e)))?;

    let mut dir = current_dir.as_path();

    loop {
        // 設定ファイルが存在するかチェック
        let config_files = [
            "kotoba-build.toml",
            "kotoba-build.json",
            "kotoba-build.yaml",
            "package.json",
            "Cargo.toml",
        ];

        for config_file in &config_files {
            if dir.join(config_file).exists() {
                return Ok(dir.to_path_buf());
            }
        }

        // 親ディレクトルに移動
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            break;
        }
    }

    // ルートが見つからない場合はカレントディレクトリを使用
    Ok(current_dir)
}

/// プロジェクトタイプを検出
pub fn detect_project_type(project_root: &Path) -> ProjectType {
    // Rustプロジェクト
    if project_root.join("Cargo.toml").exists() {
        return ProjectType::Rust;
    }

    // Node.jsプロジェクト
    if project_root.join("package.json").exists() {
        return ProjectType::NodeJs;
    }

    // Pythonプロジェクト
    if project_root.join("requirements.txt").exists() ||
       project_root.join("pyproject.toml").exists() ||
       project_root.join("setup.py").exists() {
        return ProjectType::Python;
    }

    // Goプロジェクト
    if project_root.join("go.mod").exists() {
        return ProjectType::Go;
    }

    // 汎用プロジェクト
    ProjectType::Generic
}

/// プロジェクトタイプ
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectType {
    Rust,
    NodeJs,
    Python,
    Go,
    Generic,
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Rust => write!(f, "Rust"),
            ProjectType::NodeJs => write!(f, "Node.js"),
            ProjectType::Python => write!(f, "Python"),
            ProjectType::Go => write!(f, "Go"),
            ProjectType::Generic => write!(f, "Generic"),
        }
    }
}

/// ディレクトリを作成（存在しない場合のみ）
pub fn ensure_dir_exists(dir_path: &Path) -> Result<()> {
    if !dir_path.exists() {
        fs::create_dir_all(dir_path)
            .map_err(|e| BuildError::Build(format!("Failed to create directory {}: {}", dir_path.display(), e)))?;
        println!("📁 Created directory: {}", dir_path.display());
    }
    Ok(())
}

/// ファイルをコピー
pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    if let Some(parent) = dst.parent() {
        ensure_dir_exists(parent)?;
    }

    fs::copy(src, dst)
        .map_err(|e| BuildError::Build(format!("Failed to copy {} to {}: {}", src.display(), dst.display(), e)))?;

    println!("📄 Copied: {} -> {}", src.display(), dst.display());
    Ok(())
}

/// ディレクトリをコピー（再帰的）
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.exists() {
        return Ok(());
    }

    ensure_dir_exists(dst)?;

    for entry in fs::read_dir(src)
        .map_err(|e| BuildError::Build(format!("Failed to read directory {}: {}", src.display(), e)))?
    {
        let entry = entry
            .map_err(|e| BuildError::Build(format!("Failed to read entry: {}", e)))?;
        let entry_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dst_path)?;
        } else {
            copy_file(&entry_path, &dst_path)?;
        }
    }

    Ok(())
}

/// ファイルを削除
pub fn remove_file(file_path: &Path) -> Result<()> {
    if file_path.exists() {
        fs::remove_file(file_path)
            .map_err(|e| BuildError::Build(format!("Failed to remove file {}: {}", file_path.display(), e)))?;
        println!("🗑️  Removed: {}", file_path.display());
    }
    Ok(())
}

/// ディレクトリを削除（再帰的）
pub fn remove_dir_recursive(dir_path: &Path) -> Result<()> {
    if dir_path.exists() {
        fs::remove_dir_all(dir_path)
            .map_err(|e| BuildError::Build(format!("Failed to remove directory {}: {}", dir_path.display(), e)))?;
        println!("🗑️  Removed directory: {}", dir_path.display());
    }
    Ok(())
}

/// ファイルサイズを人間が読みやすい形式にフォーマット
pub fn format_file_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let log = (bytes as f64).log(base);
    let unit_index = log.floor() as usize;

    if unit_index >= UNITS.len() {
        return format!("{} {}", bytes, UNITS[0]);
    }

    let size = bytes as f64 / base.powi(unit_index as i32);
    format!("{:.1} {}", size, UNITS[unit_index])
}

/// 実行時間をフォーマット
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    let millis = duration.subsec_millis();

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else if seconds > 0 {
        format!("{}.{:03}s", seconds, millis)
    } else {
        format!("{}ms", millis)
    }
}

/// プログレスバーの作成
pub fn create_progress_bar(total: u64, message: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// スピナーの作成
pub fn create_spinner(message: &str) -> indicatif::ProgressBar {
    use indicatif::{ProgressBar, ProgressStyle};

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// 成功メッセージを表示
pub fn print_success(message: &str) {
    println!("✅ {}", message.green());
}

/// エラーメッセージを表示
pub fn print_error(message: &str) {
    println!("❌ {}", message.red());
}

/// 警告メッセージを表示
pub fn print_warning(message: &str) {
    println!("⚠️  {}", message.yellow());
}

/// 情報メッセージを表示
pub fn print_info(message: &str) {
    println!("ℹ️  {}", message.blue());
}

/// コマンドライン引数を解析してオプションを取得
pub fn parse_cli_args() -> clap::Command {
    use clap::{Arg, Command};

    Command::new("kotoba-build")
        .version(env!("CARGO_PKG_VERSION"))
        .author("Kotoba Team")
        .about("Kotoba Build Tool - Project build and task management")
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
}

/// 環境変数を取得（デフォルト値付き）
pub fn get_env_var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// プラットフォームを検出
pub fn detect_platform() -> String {
    format!("{}-{}",
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// キャッシュディレクトリを取得
pub fn get_cache_dir() -> Result<PathBuf> {
    let cache_dir = dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("kotoba-build");

    ensure_dir_exists(&cache_dir)?;
    Ok(cache_dir)
}

/// 一時ディレクトリを取得
pub fn get_temp_dir() -> Result<PathBuf> {
    let temp_dir = std::env::temp_dir().join("kotoba-build");
    ensure_dir_exists(&temp_dir)?;
    Ok(temp_dir)
}

/// 設定ファイルのテンプレートを生成
pub fn generate_config_template() -> String {
    r#"# Kotoba Build Configuration
name = "my-project"
version = "0.1.0"
description = "My awesome project"

[tasks.dev]
command = "cargo"
args = ["run"]
description = "Start development server"

[tasks.build]
command = "cargo"
args = ["build", "--release"]
description = "Build project in release mode"

[tasks.test]
command = "cargo"
args = ["test"]
description = "Run tests"

[tasks.clean]
command = "cargo"
args = ["clean"]
description = "Clean build artifacts"

[tasks.lint]
command = "cargo"
args = ["clippy"]
description = "Run linter"

[dependencies]
tokio = "1.0"
serde = "1.0"

[build]
target = "x86_64-unknown-linux-gnu"
release = false
opt_level = "0"
debug = true

[dev]
port = 3000
host = "localhost"
hot_reload = true
open = false
"#.to_string()
}

/// シェルコマンドを安全に実行
pub async fn run_command_safely(command: &str, args: &[&str], cwd: Option<&Path>) -> Result<String> {
    use tokio::process::Command;

    let mut cmd = Command::new(command);
    cmd.args(args);

    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }

    let output = cmd.output().await
        .map_err(|e| BuildError::Build(format!("Failed to execute command: {}", e)))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(BuildError::Build(format!("Command failed: {}", stderr)))
    }
}

/// プロセスが実行中かどうかをチェック
pub fn is_process_running(pid: u32) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        use nix::sys::signal;
        use nix::unistd::Pid;

        let pid = Pid::from_raw(pid as i32);
        signal::kill(pid, None).is_ok()
    }

    #[cfg(windows)]
    {
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();

        matches!(output, Ok(o) if o.status.success())
    }

    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// 利用可能なCPUコア数を取得
pub fn get_cpu_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

/// メモリ使用量を取得（MB単位）
pub fn get_memory_usage() -> Result<f64> {
    #[cfg(unix)]
    {
        use std::fs;
        let statm = fs::read_to_string("/proc/self/statm")
            .map_err(|e| BuildError::Build(format!("Failed to read memory stats: {}", e)))?;

        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as f64;
        let rss_pages: f64 = statm.split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);

        Ok((rss_pages * page_size) / (1024.0 * 1024.0))
    }

    #[cfg(windows)]
    {
        // Windowsでは簡易的な実装
        Ok(0.0)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Ok(0.0)
    }
}

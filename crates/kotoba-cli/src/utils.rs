//! ユーティリティ関数
//!
//! Merkle DAG: cli_interface -> ProgressBar component

use std::path::{Path, PathBuf};
use std::process::Command;

/// ファイルの存在チェック
pub fn file_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

/// ディレクトリの存在チェック
pub fn dir_exists(path: &Path) -> bool {
    path.exists() && path.is_dir()
}

/// ファイルの拡張子を取得
pub fn get_file_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_string())
}

/// ファイルサイズを取得
pub fn get_file_size(path: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

/// ディレクトリ内のファイルを再帰的に検索
pub fn find_files(dir: &Path, extension: Option<&str>) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut files = Vec::new();

    fn visit_dir(dir: &Path, extension: Option<&str>, files: &mut Vec<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
        if dir.is_dir() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();

                if path.is_dir() {
                    visit_dir(&path, extension, files)?;
                } else if let Some(ext) = extension {
                    if let Some(file_ext) = get_file_extension(&path) {
                        if file_ext == ext {
                            files.push(path);
                        }
                    }
                } else {
                    files.push(path);
                }
            }
        }
        Ok(())
    }

    visit_dir(dir, extension, &mut files)?;
    Ok(files)
}

/// コマンドを実行
pub fn execute_command(command: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, Box<dyn std::error::Error>> {
    let mut cmd = Command::new(command);
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd.output()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Err(format!("Command failed: {}", stderr).into())
    }
}

/// プロセスが実行中かどうかをチェック
pub fn is_process_running(pid: u32) -> bool {
    // Unix系システムでの実装
    #[cfg(unix)]
    {
        use std::process::Command;
        Command::new("kill")
            .args(&["-0", &pid.to_string()])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    // Windowsでの実装
    #[cfg(windows)]
    {
        use std::process::Command;
        Command::new("tasklist")
            .args(&["/FI", &format!("PID eq {}", pid)])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.contains(&pid.to_string())
            })
            .unwrap_or(false)
    }

    // その他のプラットフォーム
    #[cfg(not(any(unix, windows)))]
    {
        false
    }
}

/// 利用可能なポートを見つける
pub fn find_available_port(start_port: u16) -> Option<u16> {
    use std::net::TcpListener;

    for port in start_port..65535 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return Some(port);
        }
    }
    None
}

/// バージョン比較
pub fn compare_versions(version1: &str, version2: &str) -> std::cmp::Ordering {
    let v1_parts: Vec<&str> = version1.split('.').collect();
    let v2_parts: Vec<&str> = version2.split('.').collect();

    for (v1, v2) in v1_parts.iter().zip(v2_parts.iter()) {
        let v1_num = v1.parse::<u32>().unwrap_or(0);
        let v2_num = v2.parse::<u32>().unwrap_or(0);

        match v1_num.cmp(&v2_num) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }

    v1_parts.len().cmp(&v2_parts.len())
}

/// バイト数を人間が読みやすい形式に変換
pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];

    if bytes == 0 {
        return "0 B".to_string();
    }

    let base = 1024_f64;
    let log = (bytes as f64).log(base).floor() as usize;
    let unit_index = log.min(UNITS.len() - 1);
    let value = bytes as f64 / base.powi(unit_index as i32);

    format!("{:.1} {}", value, UNITS[unit_index])
}

/// 時間を人間が読みやすい形式に変換
pub fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();

    if total_seconds < 60 {
        format!("{}s", total_seconds)
    } else if total_seconds < 3600 {
        let minutes = total_seconds / 60;
        let seconds = total_seconds % 60;
        format!("{}m {}s", minutes, seconds)
    } else {
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        format!("{}h {}m", hours, minutes)
    }
}

/// 文字列をキャメルケースに変換
pub fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, ch) in s.chars().enumerate() {
        if ch == '_' || ch == '-' {
            capitalize_next = true;
        } else if capitalize_next || i == 0 {
            result.extend(ch.to_uppercase());
            capitalize_next = false;
        } else {
            result.extend(ch.to_lowercase());
        }
    }

    result
}

/// 文字列をスネークケースに変換
pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.extend(ch.to_lowercase());
    }

    result
}

/// 環境変数を取得（デフォルト値付き）
pub fn get_env_var(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// 一時ファイルを作成
pub fn create_temp_file(prefix: &str, suffix: &str) -> Result<std::fs::File, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::Write;

    let temp_path = std::env::temp_dir().join(format!("{}{}", prefix, suffix));
    let file = File::create(&temp_path)?;
    Ok(file)
}

/// プラットフォーム固有の改行文字を取得
pub fn get_line_ending() -> &'static str {
    if cfg!(windows) {
        "\r\n"
    } else {
        "\n"
    }
}

/// 現在のプラットフォーム名を取得
pub fn get_platform_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(macos) {
        "macos"
    } else if cfg!(linux) {
        "linux"
    } else {
        "unknown"
    }
}

/// プログレスバー表示
/// Merkle DAG: cli_interface -> ProgressBar component
pub struct ProgressBar {
    total: usize,
    current: usize,
    width: usize,
    title: String,
}

impl ProgressBar {
    /// 新しいプログレスバーを作成
    pub fn new(total: usize, title: impl Into<String>) -> Self {
        Self {
            total,
            current: 0,
            width: 50,
            title: title.into(),
        }
    }

    /// プログレスバーを更新
    pub fn update(&mut self, current: usize) {
        self.current = current.min(self.total);
        self.display();
    }

    /// プログレスバーをインクリメント
    pub fn inc(&mut self) {
        self.update(self.current + 1);
    }

    /// プログレスバーを完了状態にする
    pub fn finish(&mut self) {
        self.update(self.total);
        println!(); // 改行
    }

    /// プログレスバーを表示
    fn display(&self) {
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            100
        };

        let filled = (self.current as f64 / self.total as f64 * self.width as f64) as usize;
        let filled = filled.min(self.width);

        let bar = "█".repeat(filled) + &"░".repeat(self.width - filled);

        print!("\r{} [{:<width$}] {}/{} ({}%)",
               self.title,
               bar,
               self.current,
               self.total,
               percentage,
               width = self.width
        );
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        println!(); // ドロップ時に改行
    }
}

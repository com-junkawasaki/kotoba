//! フォーマッター設定管理モジュール

use super::FormatterConfig;
use std::path::PathBuf;
use tokio::fs;

/// 設定ファイルの名前
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba.toml",
    ".kotoba.toml",
    "formatter.toml",
];

/// 設定を読み込む
pub async fn load_config() -> Result<FormatterConfig, Box<dyn std::error::Error>> {
    // カレントディレクトリから設定ファイルを検索
    for file_name in CONFIG_FILE_NAMES {
        let path = PathBuf::from(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // ホームディレクトリをチェック
    if let Some(home_dir) = dirs::home_dir() {
        let config_path = home_dir.join(".config").join("kotoba").join("formatter.toml");
        if config_path.exists() {
            return load_config_from_file(&config_path).await;
        }
    }

    // デフォルト設定を使用
    Ok(FormatterConfig::default())
}

/// ファイルから設定を読み込む
pub async fn load_config_from_file(path: &PathBuf) -> Result<FormatterConfig, Box<dyn std::error::Error>> {
    let content = fs::read_to_string(path).await?;
    let config: FormatterConfig = toml::from_str(&content)?;
    Ok(config)
}

/// 設定をファイルに保存
pub async fn save_config(config: &FormatterConfig, path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // ディレクトリが存在することを確認
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(config)?;
    fs::write(path, content).await?;
    Ok(())
}

/// プロジェクト固有の設定を取得
pub async fn load_project_config(project_root: &PathBuf) -> Result<FormatterConfig, Box<dyn std::error::Error>> {
    for file_name in CONFIG_FILE_NAMES {
        let path = project_root.join(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // プロジェクト固有の設定がない場合はグローバル設定を使用
    load_config().await
}

/// 設定ファイルを検索
pub async fn find_config_file(project_root: &PathBuf) -> Option<PathBuf> {
    for file_name in CONFIG_FILE_NAMES {
        let path = project_root.join(file_name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// デフォルト設定を生成
pub fn create_default_config() -> FormatterConfig {
    FormatterConfig::default()
}

/// 設定の例を表示
pub fn print_config_example() {
    let config = FormatterConfig::default();
    let toml_content = toml::to_string_pretty(&config).unwrap_or_else(|_| {
        r#"# Kotoba Formatter Configuration

[formatter]
indent_style = "space"
indent_width = 4
line_width = 100
line_ending = "lf"
brace_style = "same_line"
trailing_comma = true
space_around_operators = true
max_empty_lines = 2
"#.to_string()
    });

    println!("Example kotoba.toml configuration:");
    println!("{}", toml_content);
}

/// 設定の検証
pub fn validate_config(config: &FormatterConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.indent_width == 0 {
        return Err("indent_width must be greater than 0".into());
    }

    if config.line_width < 40 {
        return Err("line_width must be at least 40".into());
    }

    if config.max_empty_lines > 10 {
        return Err("max_empty_lines should not exceed 10".into());
    }

    Ok(())
}

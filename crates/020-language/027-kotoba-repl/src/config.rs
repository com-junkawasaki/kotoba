//! REPL設定管理モジュール

use super::ReplConfig;

/// 設定ファイルの名前
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba-repl.toml",
    ".kotoba-repl.toml",
    "repl.toml",
];

/// 設定を読み込む
pub async fn load_config() -> Result<ReplConfig, Box<dyn std::error::Error>> {
    // カレントディレクトリから設定ファイルを検索
    for file_name in CONFIG_FILE_NAMES {
        let path = std::path::PathBuf::from(file_name);
        if path.exists() {
            return load_config_from_file(&path).await;
        }
    }

    // ホームディレクトリをチェック
    if let Some(home_dir) = dirs::home_dir() {
        let config_path = home_dir.join(".config").join("kotoba").join("repl.toml");
        if config_path.exists() {
            return load_config_from_file(&config_path).await;
        }
    }

    // デフォルト設定を使用
    Ok(ReplConfig::default())
}

/// ファイルから設定を読み込む
pub async fn load_config_from_file(path: &std::path::PathBuf) -> Result<ReplConfig, Box<dyn std::error::Error>> {
    let content = tokio::fs::read_to_string(path).await?;
    let config: ReplConfig = toml::from_str(&content)?;
    Ok(config)
}

/// 設定をファイルに保存
pub async fn save_config(config: &ReplConfig, path: &std::path::PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    // ディレクトリが存在することを確認
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let content = toml::to_string_pretty(config)?;
    tokio::fs::write(path, content).await?;
    Ok(())
}

/// 設定の検証
pub fn validate_config(config: &ReplConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.timeout == 0 {
        return Err("timeout must be greater than 0".into());
    }

    if config.max_history == 0 {
        return Err("max_history must be greater than 0".into());
    }

    Ok(())
}

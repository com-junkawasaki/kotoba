//! 設定管理モジュール

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Package Managerの設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// キャッシュディレクトリ
    pub cache_dir: PathBuf,
    /// グローバルパッケージディレクトリ
    pub global_dir: PathBuf,
    /// レジストリURL
    pub registry_url: String,
    /// デフォルトのレジストリ
    pub default_registry: String,
    /// タイムアウト設定
    pub timeout: u64,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));

        Self {
            cache_dir: home_dir.join(".kotoba").join("cache"),
            global_dir: home_dir.join(".kotoba").join("packages"),
            registry_url: "https://registry.kotoba.dev".to_string(),
            default_registry: "https://registry.kotoba.dev".to_string(),
            timeout: 30000, // 30 seconds
        }
    }
}

impl Config {
    /// 設定ファイルを読み込む
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: Config = toml::from_str(&content)?;
            Ok(config)
        } else {
            let config = Self::default();
            config.save()?;
            Ok(config)
        }
    }

    /// 設定を保存
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();
        std::fs::create_dir_all(config_path.parent().unwrap())?;

        let content = toml::to_string(self)?;
        std::fs::write(config_path, content)?;
        Ok(())
    }

    /// 設定ファイルのパスを取得
    fn config_path() -> PathBuf {
        let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        home_dir.join(".kotoba").join("config.toml")
    }

    /// プロジェクト設定を読み込む
    pub fn load_project() -> Result<super::ProjectConfig> {
        let config_path = PathBuf::from("kotoba.toml");

        if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            let config: super::ProjectConfig = toml::from_str(&content)?;
            Ok(config)
        } else {
            Err(anyhow::anyhow!("kotoba.toml not found. Run 'kotoba init' to create a project."))
        }
    }
}

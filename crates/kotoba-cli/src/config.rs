//! 設定管理
//!
//! Merkle DAG: cli_interface -> ConfigManager component

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Kotoba CLI設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// デフォルトのログレベル
    pub log_level: String,
    /// デフォルトのポート
    pub default_port: u16,
    /// キャッシュ設定
    pub cache: CacheConfig,
    /// サーバー設定
    pub server: ServerConfig,
    /// コンパイラ設定
    pub compiler: CompilerConfig,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            log_level: "info".to_string(),
            default_port: 3000,
            cache: CacheConfig::default(),
            server: ServerConfig::default(),
            compiler: CompilerConfig::default(),
        }
    }
}

/// キャッシュ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// キャッシュ有効化
    pub enabled: bool,
    /// キャッシュディレクトリ
    pub directory: PathBuf,
    /// 最大サイズ（MB）
    pub max_size_mb: usize,
    /// TTL（時間）
    pub ttl_hours: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            directory: get_cache_dir(),
            max_size_mb: 100,
            ttl_hours: 24,
        }
    }
}

/// サーバー設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// ホストアドレス
    pub host: String,
    /// ポート
    pub port: u16,
    /// タイムアウト（秒）
    pub timeout_seconds: u64,
    /// 最大接続数
    pub max_connections: usize,
    /// CORS有効化
    pub cors_enabled: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            timeout_seconds: 30,
            max_connections: 100,
            cors_enabled: true,
        }
    }
}

/// コンパイラ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    /// 最適化レベル
    pub optimization_level: u8,
    /// デバッグ情報
    pub include_debug_info: bool,
    /// ソースマップ生成
    pub generate_source_maps: bool,
    /// ターゲットアーキテクチャ
    pub target_arch: String,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            optimization_level: 0,
            include_debug_info: true,
            generate_source_maps: true,
            target_arch: std::env::consts::ARCH.to_string(),
        }
    }
}

/// 設定マネージャー
pub struct ConfigManager {
    config: CliConfig,
    config_path: PathBuf,
}

impl ConfigManager {
    /// 新しい設定マネージャーを作成
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = get_config_dir();
        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("cli.toml");
        let config = if config_path.exists() {
            Self::load_config(&config_path)?
        } else {
            CliConfig::default()
        };

        Ok(Self {
            config,
            config_path,
        })
    }

    /// 設定をファイルから読み込み
    fn load_config(path: &Path) -> Result<CliConfig, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: CliConfig = toml::from_str(&content)?;
        Ok(config)
    }

    /// 設定をファイルに保存
    pub fn save_config(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string_pretty(&self.config)?;
        std::fs::write(&self.config_path, content)?;
        Ok(())
    }

    /// 設定を取得
    pub fn get_config(&self) -> &CliConfig {
        &self.config
    }

    /// 設定を更新
    pub fn update_config<F>(&mut self, updater: F)
    where
        F: FnOnce(&mut CliConfig),
    {
        updater(&mut self.config);
    }

    /// 設定をリセット
    pub fn reset_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.config = CliConfig::default();
        self.save_config()
    }
}

/// 設定ディレクトリの取得
fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kotoba")
}

/// キャッシュディレクトリの取得
fn get_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("kotoba")
}

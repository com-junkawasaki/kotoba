//! # Kotoba Deploy Core
//!
//! Core types and configuration structures for the Kotoba deployment system.
//! This crate provides the fundamental building blocks for deployment management.

use kotoba_core::types::{Result, Value, ContentHash};
use kotoba_errors::KotobaError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// ランタイムタイプの列挙
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RuntimeType {
    /// Denoランタイム
    Deno,
    /// Node.jsランタイム
    NodeJs,
    /// Pythonランタイム
    Python,
    /// Rustランタイム
    Rust,
    /// Goランタイム
    Go,
}

/// デプロイメントのステータス
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentStatus {
    /// 作成中
    Creating,
    /// 準備中
    Preparing,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// エラー
    Error(String),
}

/// デプロイメントの優先度
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DeploymentPriority {
    /// 低優先度
    Low,
    /// 通常優先度
    Normal,
    /// 高優先度
    High,
    /// 緊急
    Critical,
}

/// リソース使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    /// CPU使用率 (0.0 - 1.0)
    pub cpu_usage: f64,
    /// メモリ使用量 (MB)
    pub memory_usage: f64,
    /// ネットワーク使用量 (MB/s)
    pub network_usage: f64,
    /// ディスク使用量 (MB)
    pub disk_usage: f64,
    /// リクエスト数/秒
    pub request_rate: f64,
}

/// デプロイメントメタデータ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployMetadata {
    /// デプロイメント名
    pub name: String,
    /// バージョン
    pub version: String,
    /// 説明
    pub description: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 作成日時
    pub created_at: DateTime<Utc>,
    /// 最終更新日時
    pub updated_at: Option<DateTime<Utc>>,
    /// 設定ファイルのハッシュ
    pub config_hash: Option<ContentHash>,
}

/// アプリケーション設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// エントリーポイント
    pub entry_point: String,
    /// ランタイムタイプ
    pub runtime: RuntimeType,
    /// 環境変数
    pub environment: HashMap<String, String>,
    /// ビルドコマンド
    pub build_command: Option<String>,
    /// スタートコマンド
    pub start_command: Option<String>,
}

/// スケーリング設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScalingConfig {
    /// 最小インスタンス数
    pub min_instances: u32,
    /// 最大インスタンス数
    pub max_instances: u32,
    /// CPU使用率の閾値 (0.0 - 1.0)
    pub cpu_threshold: f64,
    /// メモリ使用率の閾値 (0.0 - 1.0)
    pub memory_threshold: f64,
    /// 自動スケーリング有効化
    pub auto_scaling_enabled: bool,
}

/// ネットワーク設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// ドメイン設定
    pub domains: Vec<String>,
    /// SSL設定
    pub ssl: Option<SslConfig>,
    /// CORS設定
    pub cors: Option<CorsConfig>,
    /// CDN設定
    pub cdn: Option<CdnConfig>,
}

/// SSL設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// 証明書タイプ
    pub cert_type: CertType,
    /// 証明書ファイルのパス
    pub cert_path: Option<String>,
    /// キーファイルのパス
    pub key_path: Option<String>,
}

/// 証明書タイプ
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CertType {
    /// Let's Encrypt
    LetsEncrypt,
    /// カスタム証明書
    Custom,
    /// 自己署名証明書
    SelfSigned,
}

/// CORS設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    /// 許可されたオリジン
    pub allowed_origins: Vec<String>,
    /// 許可されたメソッド
    pub allowed_methods: Vec<String>,
    /// 許可されたヘッダー
    pub allowed_headers: Vec<String>,
    /// クレデンシャル許可
    pub allow_credentials: bool,
}

/// CDN設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CdnConfig {
    /// CDNプロバイダー
    pub provider: CdnProvider,
    /// CDNエッジロケーション
    pub edge_locations: Vec<String>,
    /// キャッシュ設定
    pub cache_config: CacheConfig,
}

/// CDNプロバイダー
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CdnProvider {
    /// Cloudflare
    Cloudflare,
    /// Fastly
    Fastly,
    /// Akamai
    Akamai,
    /// AWS CloudFront
    CloudFront,
}

/// キャッシュ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// キャッシュ有効期間 (秒)
    pub ttl: u32,
    /// キャッシュ無効化パターン
    pub invalidation_patterns: Vec<String>,
}

/// デプロイ設定のメイン構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployConfig {
    /// デプロイメントのメタデータ
    pub metadata: DeployMetadata,
    /// アプリケーション設定
    pub application: ApplicationConfig,
    /// スケーリング設定
    pub scaling: ScalingConfig,
    /// ネットワーク設定
    pub network: NetworkConfig,
    /// カスタム設定
    pub custom: Value,
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            metadata: DeployMetadata {
                name: "default".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                author: None,
                created_at: Utc::now(),
                updated_at: None,
                config_hash: None,
            },
            application: ApplicationConfig {
                entry_point: "index.js".to_string(),
                runtime: RuntimeType::Deno,
                environment: HashMap::new(),
                build_command: None,
                start_command: None,
            },
            scaling: ScalingConfig {
                min_instances: 1,
                max_instances: 10,
                cpu_threshold: 0.8,
                memory_threshold: 0.8,
                auto_scaling_enabled: true,
            },
            network: NetworkConfig {
                domains: vec!["localhost".to_string()],
                ssl: None,
                cors: None,
                cdn: None,
            },
            custom: Value::Null,
        }
    }
}

/// デプロイ設定ビルダー
#[derive(Debug, Clone)]
pub struct DeployConfigBuilder {
    config: DeployConfig,
}

impl DeployConfigBuilder {
    pub fn new(name: String) -> Self {
        let mut config = DeployConfig::default();
        config.metadata.name = name;
        Self { config }
    }

    pub fn version(mut self, version: String) -> Self {
        self.config.metadata.version = version;
        self
    }

    pub fn description(mut self, description: String) -> Self {
        self.config.metadata.description = Some(description);
        self
    }

    pub fn author(mut self, author: String) -> Self {
        self.config.metadata.author = Some(author);
        self
    }

    pub fn entry_point(mut self, entry_point: String) -> Self {
        self.config.application.entry_point = entry_point;
        self
    }

    pub fn runtime(mut self, runtime: RuntimeType) -> Self {
        self.config.application.runtime = runtime;
        self
    }

    pub fn environment(mut self, key: String, value: String) -> Self {
        self.config.application.environment.insert(key, value);
        self
    }

    pub fn build_command(mut self, command: String) -> Self {
        self.config.application.build_command = Some(command);
        self
    }

    pub fn start_command(mut self, command: String) -> Self {
        self.config.application.start_command = Some(command);
        self
    }

    pub fn min_instances(mut self, min: u32) -> Self {
        self.config.scaling.min_instances = min;
        self
    }

    pub fn max_instances(mut self, max: u32) -> Self {
        self.config.scaling.max_instances = max;
        self
    }

    pub fn domains(mut self, domains: Vec<String>) -> Self {
        self.config.network.domains = domains;
        self
    }

    pub fn build(self) -> DeployConfig {
        self.config
    }
}

// Re-export commonly used types
pub use DeployConfig as Config;
pub use DeployMetadata as Metadata;
pub use ApplicationConfig as AppConfig;
pub use ScalingConfig as Scaling;
pub use NetworkConfig as Network;

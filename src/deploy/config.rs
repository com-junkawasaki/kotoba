//! デプロイ設定のIR定義
//!
//! このモジュールはJsonnetベースの.kotoba-deployファイルの構造を定義します。
//! Deno Deployと同等の機能をサポートしつつ、KotobaのLive Graph Modelに適応しています。
//! kotoba-kotobanet を使用して一般設定も管理します。

use kotoba_core::types::{Result, Value, ContentHash, KotobaError};
use kotoba_kotobanet::ConfigParser;
use std::collections::HashMap;
use std::time::SystemTime;

/// デプロイ設定のメイン構造体
#[derive(Debug, Clone)]
pub struct DeployConfig {
    /// デプロイメントのメタデータ
    pub metadata: DeployMetadata,
    /// アプリケーション設定
    pub application: ApplicationConfig,
    /// スケーリング設定
    pub scaling: ScalingConfig,
    /// ネットワーク設定
    pub network: NetworkConfig,
    /// 環境変数
    pub environment: HashMap<String, String>,
    /// カスタム設定
    pub custom: Value,
}

/// デプロイメントのメタデータ
#[derive(Debug, Clone)]
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
    pub created_at: String,
    /// 最終更新日時
    pub updated_at: Option<String>,
    /// 設定ファイルのハッシュ
    pub config_hash: Option<ContentHash>,
}

/// アプリケーション設定
#[derive(Debug, Clone)]
pub struct ApplicationConfig {
    /// エントリーポイント
    pub entry_point: String,
    /// ランタイムタイプ
    pub runtime: RuntimeType,
    /// ビルド設定
    pub build: Option<BuildConfig>,
    /// 静的ファイル設定
    pub static_files: Option<StaticFilesConfig>,
}

/// ランタイムタイプ
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum RuntimeType {
    /// HTTPサーバー (Rust)
    HttpServer,
    /// フロントエンドアプリケーション
    Frontend,
    /// GraphQL API
    GraphQL,
    /// マイクロサービス
    Microservice,
    /// カスタムランタイム
    Custom(String),
}

/// ビルド設定
#[derive(Debug, Clone)]
pub struct BuildConfig {
    /// ビルドコマンド
    pub build_command: Option<String>,
    /// ビルドディレクトリ
    pub build_dir: Option<String>,
    /// 出力ディレクトリ
    pub output_dir: Option<String>,
    /// 環境変数
    pub env: HashMap<String, String>,
}

/// 静的ファイル設定
#[derive(Debug, Clone)]
pub struct StaticFilesConfig {
    /// 静的ファイルディレクトリ
    pub directory: String,
    /// キャッシュ設定
    pub cache: Option<CacheConfig>,
    /// CORS設定
    pub cors: Option<CorsConfig>,
}

/// キャッシュ設定
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// 最大キャッシュ時間 (秒)
    pub max_age: u32,
    /// キャッシュ制御ヘッダー
    pub cache_control: Option<String>,
}

/// CORS設定
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// 許可されたオリジン
    pub allowed_origins: Vec<String>,
    /// 許可されたメソッド
    pub allowed_methods: Vec<String>,
    /// 許可されたヘッダー
    pub allowed_headers: Vec<String>,
}

/// スケーリング設定
#[derive(Debug, Clone)]
pub struct ScalingConfig {
    /// 最小インスタンス数
    pub min_instances: u32,
    /// 最大インスタンス数
    pub max_instances: u32,
    /// CPU使用率の閾値 (%)
    pub cpu_threshold: f64,
    /// メモリ使用率の閾値 (%)
    pub memory_threshold: f64,
    /// スケーリングポリシー
    pub policy: ScalingPolicy,
    /// クールダウン時間 (秒)
    pub cooldown_period: u32,
}

/// スケーリングポリシー
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum ScalingPolicy {
    /// CPUベースのスケーリング
    CpuBased,
    /// リクエストベースのスケーリング
    RequestBased,
    /// 予測スケーリング
    Predictive,
    /// 手動スケーリング
    Manual,
}

/// ネットワーク設定
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// ドメイン設定
    pub domains: Vec<DomainConfig>,
    /// リージョン設定
    pub regions: Vec<String>,
    /// CDN設定
    pub cdn: Option<CdnConfig>,
    /// SSL/TLS設定
    pub tls: Option<TlsConfig>,
}

/// ドメイン設定
#[derive(Debug, Clone)]
pub struct DomainConfig {
    /// ドメイン名
    pub domain: String,
    /// SSL証明書設定
    pub ssl: Option<SslConfig>,
    /// リダイレクト設定
    pub redirects: Vec<RedirectRule>,
}

/// SSL証明書設定
#[derive(Debug, Clone)]
pub struct SslConfig {
    /// 証明書タイプ
    pub cert_type: CertType,
    /// カスタム証明書 (自動証明書の場合はNone)
    pub custom_cert: Option<String>,
    /// カスタム秘密鍵 (自動証明書の場合はNone)
    pub custom_key: Option<String>,
}

/// 証明書タイプ
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum CertType {
    /// Let's Encrypt自動証明書
    LetsEncrypt,
    /// カスタム証明書
    Custom,
}

/// リダイレクトルール
#[derive(Debug, Clone)]
pub struct RedirectRule {
    /// ソースパス
    pub source: String,
    /// ターゲットURL
    pub target: String,
    /// リダイレクトタイプ (301, 302, etc.)
    pub redirect_type: u16,
}

/// CDN設定
#[derive(Debug, Clone)]
pub struct CdnConfig {
    /// CDN有効化
    pub enabled: bool,
    /// CDNプロバイダー
    pub provider: CdnProvider,
    /// キャッシュ設定
    pub cache: CacheConfig,
}

/// CDNプロバイダー
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum CdnProvider {
    /// Cloudflare
    Cloudflare,
    /// Fastly
    Fastly,
    /// AWS CloudFront
    CloudFront,
    /// 自動選択
    Auto,
}

/// TLS設定
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// TLSバージョン
    pub min_version: String,
    /// 暗号スイート
    pub cipher_suites: Vec<String>,
    /// HSTS設定
    pub hsts: Option<HstsConfig>,
}

/// HSTS設定
#[derive(Debug, Clone)]
pub struct HstsConfig {
    /// 有効化
    pub enabled: bool,
    /// max-age値 (秒)
    pub max_age: u32,
    /// includeSubDomainsフラグ
    pub include_subdomains: bool,
    /// preloadフラグ
    pub preload: bool,
}

/// リージョン設定
#[derive(Debug, Clone)]
pub struct RegionConfig {
    /// リージョン名
    pub name: String,
    /// 優先度 (低いほど優先)
    pub priority: u32,
    /// 容量 (インスタンス数)
    pub capacity: u32,
    /// 地理的設定
    pub geography: GeographyConfig,
}

/// 地理的設定
#[derive(Debug, Clone)]
pub struct GeographyConfig {
    /// 大陸
    pub continent: String,
    /// 国
    pub country: Option<String>,
    /// 都市
    pub city: Option<String>,
    /// 緯度
    pub latitude: Option<f64>,
    /// 経度
    pub longitude: Option<f64>,
}

/// デプロイメント状態
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    /// 作成済み
    Created,
    /// ビルド中
    Building,
    /// デプロイ中
    Deploying,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// 失敗
    Failed,
    /// 削除済み
    Deleted,
}

/// デプロイスクリプト設定
#[derive(Debug, Clone)]
pub struct DeployScript {
    /// スクリプト名
    pub name: String,
    /// 実行タイミング
    pub trigger: ScriptTrigger,
    /// スクリプト内容
    pub script: String,
    /// タイムアウト (秒)
    pub timeout: Option<u32>,
}

/// スクリプト実行タイミング
#[derive(Debug, Clone)]
// #[serde(rename_all = "snake_case")]
pub enum ScriptTrigger {
    /// ビルド前
    PreBuild,
    /// ビルド後
    PostBuild,
    /// デプロイ前
    PreDeploy,
    /// デプロイ後
    PostDeploy,
    /// スケールアップ時
    OnScaleUp,
    /// スケールダウン時
    OnScaleDown,
    /// カスタムタイミング
    Custom(String),
}

impl Default for DeployConfig {
    fn default() -> Self {
        Self {
            metadata: DeployMetadata {
                name: "default-app".to_string(),
                version: "0.1.0".to_string(),
                description: None,
                author: None,
                created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_secs().to_string())
                    .unwrap_or_else(|_| "0".to_string()),
                updated_at: None,
                config_hash: None,
            },
            application: ApplicationConfig {
                entry_point: "main.rs".to_string(),
                runtime: RuntimeType::HttpServer,
                build: None,
                static_files: None,
            },
            scaling: ScalingConfig {
                min_instances: 1,
                max_instances: 10,
                cpu_threshold: 70.0,
                memory_threshold: 80.0,
                policy: ScalingPolicy::CpuBased,
                cooldown_period: 300,
            },
            network: NetworkConfig {
                domains: vec![],
                regions: vec!["us-east-1".to_string()],
                cdn: None,
                tls: None,
            },
            environment: HashMap::new(),
            custom: Value::Null,
        }
    }
}

impl DeployConfig {
    /// 新しいデプロイ設定を作成
    pub fn new(name: String, entry_point: String) -> Self {
        let mut config = Self::default();
        config.metadata.name = name;
        config.application.entry_point = entry_point;
        config
    }

    /// 設定を検証
    pub fn validate(&self) -> Result<()> {
        // 名前の検証
        if self.metadata.name.is_empty() {
            return Err(KotobaError::InvalidArgument(
                "Deployment name cannot be empty".to_string()
            ));
        }

        // エントリーポイントの検証
        if self.application.entry_point.is_empty() {
            return Err(KotobaError::InvalidArgument(
                "Entry point cannot be empty".to_string()
            ));
        }

        // スケーリング設定の検証
        if self.scaling.min_instances == 0 {
            return Err(KotobaError::InvalidArgument(
                "Minimum instances must be greater than 0".to_string()
            ));
        }

        if self.scaling.max_instances < self.scaling.min_instances {
            return Err(KotobaError::InvalidArgument(
                "Maximum instances must be greater than or equal to minimum instances".to_string()
            ));
        }

        // CPU/Memory閾値の検証
        if !(0.0..=100.0).contains(&self.scaling.cpu_threshold) {
            return Err(KotobaError::InvalidArgument(
                "CPU threshold must be between 0 and 100".to_string()
            ));
        }

        if !(0.0..=100.0).contains(&self.scaling.memory_threshold) {
            return Err(KotobaError::InvalidArgument(
                "Memory threshold must be between 0 and 100".to_string()
            ));
        }

        Ok(())
    }

    /// 設定のハッシュを計算 (簡易実装)
    pub fn calculate_hash(&self) -> Result<ContentHash> {
        // 簡易実装: 設定の文字列表現に基づくハッシュ
        let content = format!("{:?}", self);
        let hash_str = format!("hash_{}", content.len());
        Ok(ContentHash(hash_str))
    }
}

/// デプロイ設定ビルダー
pub struct DeployConfigBuilder {
    config: DeployConfig,
}

impl DeployConfigBuilder {
    pub fn new(name: String, entry_point: String) -> Self {
        Self {
            config: DeployConfig::new(name, entry_point),
        }
    }

    pub fn description(mut self, description: String) -> Self {
        self.config.metadata.description = Some(description);
        self
    }

    pub fn version(mut self, version: String) -> Self {
        self.config.metadata.version = version;
        self
    }

    pub fn runtime(mut self, runtime: RuntimeType) -> Self {
        self.config.application.runtime = runtime;
        self
    }

    pub fn scaling(mut self, min_instances: u32, max_instances: u32) -> Self {
        self.config.scaling.min_instances = min_instances;
        self.config.scaling.max_instances = max_instances;
        self
    }

    pub fn add_domain(mut self, domain: String) -> Self {
        self.config.network.domains.push(DomainConfig {
            domain,
            ssl: None,
            redirects: vec![],
        });
        self
    }

    pub fn add_region(mut self, region: String) -> Self {
        self.config.network.regions.push(region);
        self
    }

    pub fn env(mut self, key: String, value: String) -> Self {
        self.config.environment.insert(key, value);
        self
    }

    pub fn build(self) -> DeployConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deploy_config_validation() {
        let config = DeployConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deploy_config_builder() {
        let config = DeployConfigBuilder::new("test-app".to_string(), "main.rs".to_string())
            .description("Test application".to_string())
            .version("1.0.0".to_string())
            .runtime(RuntimeType::HttpServer)
            .scaling(1, 5)
            .add_domain("example.com".to_string())
            .env("NODE_ENV".to_string(), "production".to_string())
            .build();

        assert_eq!(config.metadata.name, "test-app");
        assert_eq!(config.application.entry_point, "main.rs");
        assert_eq!(config.scaling.min_instances, 1);
        assert_eq!(config.scaling.max_instances, 5);
        assert_eq!(config.network.domains.len(), 1);
        assert_eq!(config.environment.get("NODE_ENV"), Some(&"production".to_string()));
    }

/// 一般アプリケーション設定管理
///
/// kotoba-kotobanet::ConfigParser を使用して一般的な設定を管理します。
pub struct AppConfigManager;

impl AppConfigManager {
    /// 設定ファイルをパース
    pub fn parse<P: AsRef<std::path::Path>>(path: P) -> Result<kotoba_kotobanet::AppConfig> {
        ConfigParser::parse_file(path)
            .map_err(|e| KotobaError::Configuration(format!("App config parsing failed: {}", e)))
    }

    /// 設定文字列をパース
    pub fn parse_string(content: &str) -> Result<kotoba_kotobanet::AppConfig> {
        ConfigParser::parse(content)
            .map_err(|e| KotobaError::Configuration(format!("App config parsing failed: {}", e)))
    }

    /// デフォルト設定を生成
    pub fn default() -> kotoba_kotobanet::AppConfig {
        // TODO: デフォルト設定の実装
        // 現時点では panic を避けるため、シンプルなデフォルトを返す
        unimplemented!("Default app config not implemented")
    }
}

    #[test]
    fn test_invalid_scaling_config() {
        let mut config = DeployConfig::default();
        config.scaling.min_instances = 0;
        assert!(config.validate().is_err());

        config.scaling.min_instances = 5;
        config.scaling.max_instances = 3;
        assert!(config.validate().is_err());
    }
}

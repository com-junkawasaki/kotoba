//! # Kotoba Deploy CLI Library
//!
//! Library components for the Kotoba deployment CLI.
//! Provides utilities and helpers for command-line operations.

use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use indicatif::{ProgressBar, ProgressStyle};
use dirs::home_dir;
use kotoba_deploy_core::*;
use kotoba_deploy_controller::*;
use kotoba_deploy_runtime::*;
use kotoba_deploy_scaling::*;
use kotoba_deploy_network::*;

/// CLI設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// デフォルトの設定ファイルパス
    pub config_path: Option<PathBuf>,
    /// デフォルトのログレベル
    pub log_level: String,
    /// デフォルトのタイムアウト（秒）
    pub timeout_seconds: u64,
    /// デフォルトの出力形式
    pub output_format: OutputFormat,
}

/// 出力形式
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OutputFormat {
    /// JSON形式
    Json,
    /// YAML形式
    Yaml,
    /// 人間可読形式
    Human,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            config_path: None,
            log_level: "info".to_string(),
            timeout_seconds: 300,
            output_format: OutputFormat::Human,
        }
    }
}

/// CLIマネージャー
pub struct CliManager {
    config: CliConfig,
    controller: Option<DeployController>,
    runtime: Option<RuntimeManager>,
    scaling: Option<ScalingEngine>,
    network: Option<NetworkManager>,
}

impl CliManager {
    /// 新しいCLIマネージャーを作成
    pub fn new() -> Self {
        Self {
            config: CliConfig::default(),
            controller: None,
            runtime: None,
            scaling: None,
            network: None,
        }
    }

    /// 設定ファイルを読み込む
    pub fn load_config(&mut self, config_path: Option<&Path>) -> Result<()> {
        let config_path = config_path
            .map(|p| p.to_path_buf())
            .or_else(|| self.default_config_path())
            .context("No configuration file path specified")?;

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .context(format!("Failed to read config file: {:?}", config_path))?;

            if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
                self.config = serde_json::from_str(&content)
                    .context("Failed to parse JSON config")?;
            } else {
                self.config = serde_yaml::from_str(&content)
                    .context("Failed to parse YAML config")?;
            }
        }

        Ok(())
    }

    /// 設定ファイルを保存
    pub fn save_config(&self, config_path: Option<&Path>) -> Result<()> {
        let config_path = config_path
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| self.default_config_path().unwrap());

        let content = match config_path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::to_string_pretty(&self.config)?,
            _ => serde_yaml::to_string(&self.config)?,
        };

        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(&config_path, content)?;

        Ok(())
    }

    /// デフォルトの設定ファイルパスを取得
    fn default_config_path(&self) -> Option<PathBuf> {
        home_dir().map(|h| h.join(".config").join("kotoba-deploy").join("config.yaml"))
    }

    /// デプロイメント設定を読み込む
    pub fn load_deploy_config(&self, config_path: &Path) -> Result<DeployConfig> {
        let content = fs::read_to_string(config_path)
            .context(format!("Failed to read deployment config: {:?}", config_path))?;

        if config_path.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content).context("Failed to parse JSON deployment config")
        } else {
            serde_yaml::from_str(&content).context("Failed to parse YAML deployment config")
        }
    }

    /// デプロイメント設定を保存
    pub fn save_deploy_config(&self, config: &DeployConfig, config_path: &Path) -> Result<()> {
        let content = match config_path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::to_string_pretty(config)?,
            _ => serde_yaml::to_string(config)?,
        };

        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(config_path, content)?;

        Ok(())
    }

    /// プログレスバーを作成
    pub fn create_progress_bar(&self, message: &str, total: u64) -> ProgressBar {
        let pb = ProgressBar::new(total);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>7}/{len:7} {msg}")
                .unwrap()
                .progress_chars("#>-")
        );
        pb.set_message(message.to_string());
        pb
    }

    /// スピナープログレスバーを作成
    pub fn create_spinner(&self, message: &str) -> ProgressBar {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {msg}")
                .unwrap()
        );
        pb.set_message(message.to_string());
        pb
    }

    /// 設定を取得
    pub fn config(&self) -> &CliConfig {
        &self.config
    }

    /// 設定を更新
    pub fn set_config(&mut self, config: CliConfig) {
        self.config = config;
    }

    /// デプロイメントコントローラーを設定
    pub fn set_controller(&mut self, controller: DeployController) {
        self.controller = Some(controller);
    }

    /// ランタイムマネージャーを設定
    pub fn set_runtime(&mut self, runtime: RuntimeManager) {
        self.runtime = Some(runtime);
    }

    /// スケーリングエンジンを設定
    pub fn set_scaling(&mut self, scaling: ScalingEngine) {
        self.scaling = Some(scaling);
    }

    /// ネットワークマネージャーを設定
    pub fn set_network(&mut self, network: NetworkManager) {
        self.network = Some(network);
    }

    /// コントローラーを取得
    pub fn controller(&self) -> Option<&DeployController> {
        self.controller.as_ref()
    }

    /// ランタイムを取得
    pub fn runtime(&self) -> Option<&RuntimeManager> {
        self.runtime.as_ref()
    }

    /// スケーリングを取得
    pub fn scaling(&self) -> Option<&ScalingEngine> {
        self.scaling.as_ref()
    }

    /// ネットワークを取得
    pub fn network(&self) -> Option<&NetworkManager> {
        self.network.as_ref()
    }
}

/// デプロイメント情報
#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub instance_count: u32,
    pub created_at: String,
    pub endpoints: Vec<String>,
}

/// CLI結果のフォーマット
pub trait FormatOutput {
    fn format(&self, format: &OutputFormat) -> String;
}

impl FormatOutput for DeploymentInfo {
    fn format(&self, format: &OutputFormat) -> String {
        match format {
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
            OutputFormat::Yaml => serde_yaml::to_string(self).unwrap_or_default(),
            OutputFormat::Human => format!(
                "Deployment: {}\n  Status: {}\n  Instances: {}\n  Created: {}\n  Endpoints: {}",
                self.name,
                self.status,
                self.instance_count,
                self.created_at,
                self.endpoints.join(", ")
            ),
        }
    }
}

impl FormatOutput for Vec<DeploymentInfo> {
    fn format(&self, format: &OutputFormat) -> String {
        match format {
            OutputFormat::Json => serde_json::to_string_pretty(self).unwrap_or_default(),
            OutputFormat::Yaml => serde_yaml::to_string(self).unwrap_or_default(),
            OutputFormat::Human => {
                if self.is_empty() {
                    "No deployments found".to_string()
                } else {
                    self.iter()
                        .map(|d| format!("• {} ({})", d.name, d.status))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            },
        }
    }
}

/// CLIエラー
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Deployment error: {0}")]
    Deployment(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<serde_json::Error> for CliError {
    fn from(err: serde_json::Error) -> Self {
        CliError::Serialization(err.to_string())
    }
}

impl From<serde_yaml::Error> for CliError {
    fn from(err: serde_yaml::Error) -> Self {
        CliError::Serialization(err.to_string())
    }
}

/// 設定ファイルのバリデーション
pub fn validate_config(config: &DeployConfig) -> Result<(), CliError> {
    if config.metadata.name.is_empty() {
        return Err(CliError::Validation("Deployment name cannot be empty".to_string()));
    }

    if config.application.entry_point.is_empty() {
        return Err(CliError::Validation("Entry point cannot be empty".to_string()));
    }

    if !Path::new(&config.application.entry_point).exists() {
        return Err(CliError::Validation(format!("Entry point file does not exist: {}", config.application.entry_point)));
    }

    Ok(())
}

/// デプロイメントIDのバリデーション
pub fn validate_deployment_id(id: &str) -> Result<(), CliError> {
    if id.is_empty() {
        return Err(CliError::Validation("Deployment ID cannot be empty".to_string()));
    }

    if id.len() > 64 {
        return Err(CliError::Validation("Deployment ID too long (max 64 characters)".to_string()));
    }

    if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
        return Err(CliError::Validation("Deployment ID contains invalid characters".to_string()));
    }

    Ok(())
}

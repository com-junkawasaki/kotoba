//! 設定ファイル管理モジュール

use super::{DocsConfig, Result, DocsError};
use std::path::{Path, PathBuf};
use std::fs;

/// 設定ファイルの名前候補
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba-docs.toml",
    "kotoba-docs.json",
    "kotoba-docs.yaml",
    "docs.toml",
    "docs.json",
    "docs.yaml",
    ".docs.toml",
    ".docs.json",
    ".docs.yaml",
];

/// 設定マネージャー
pub struct ConfigManager {
    project_root: PathBuf,
}

impl ConfigManager {
    /// 新しい設定マネージャーを作成
    pub fn new(project_root: PathBuf) -> Self {
        Self { project_root }
    }

    /// 設定ファイルを検出して読み込む
    pub async fn load_config(&self) -> Result<DocsConfig> {
        for filename in CONFIG_FILE_NAMES {
            let config_path = self.project_root.join(filename);
            if config_path.exists() {
                return self.load_config_from_file(&config_path).await;
            }
        }

        // 設定ファイルが見つからない場合はデフォルト設定を使用
        println!("No config file found, using default configuration");
        Ok(DocsConfig::default())
    }

    /// ファイルから設定を読み込む
    pub async fn load_config_from_file(&self, path: &Path) -> Result<DocsConfig> {
        println!("Loading config from: {}", path.display());

        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| DocsError::Config(format!("Failed to read config file: {}", e)))?;

        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                let config: DocsConfig = toml::from_str(&content)
                    .map_err(|e| DocsError::Config(format!("TOML parse error: {}", e)))?;
                Ok(config)
            }
            Some("json") => {
                let config: DocsConfig = serde_json::from_str(&content)
                    .map_err(|e| DocsError::Config(format!("JSON parse error: {}", e)))?;
                Ok(config)
            }
            Some("yaml") | Some("yml") => {
                let config: DocsConfig = serde_yaml::from_str(&content)
                    .map_err(|e| DocsError::Config(format!("YAML parse error: {}", e)))?;
                Ok(config)
            }
            _ => Err(DocsError::Config(format!("Unsupported config file format: {:?}", path))),
        }
    }

    /// 設定をファイルに保存
    pub async fn save_config(&self, config: &DocsConfig, path: Option<&Path>) -> Result<()> {
        let config_path = path.unwrap_or(&self.project_root.join("kotoba-docs.toml"));

        // ディレクトリが存在することを確認
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| DocsError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = toml::to_string_pretty(config)
            .map_err(|e| DocsError::Config(format!("TOML serialization error: {}", e)))?;

        tokio::fs::write(config_path, content).await
            .map_err(|e| DocsError::Config(format!("Failed to write config file: {}", e)))?;

        println!("Config saved to: {}", config_path.display());
        Ok(())
    }

    /// 設定の検証
    pub fn validate_config(config: &DocsConfig) -> Result<()> {
        // プロジェクト名の検証
        if config.name.trim().is_empty() {
            return Err(DocsError::Config("Project name cannot be empty".to_string()));
        }

        // バージョンの検証
        if config.version.trim().is_empty() {
            return Err(DocsError::Config("Version cannot be empty".to_string()));
        }

        // 入力ディレクトリの検証
        if !config.input_dir.exists() {
            return Err(DocsError::Config(format!("Input directory does not exist: {}", config.input_dir.display())));
        }

        // 出力形式の検証
        if config.formats.is_empty() {
            return Err(DocsError::Config("At least one output format must be specified".to_string()));
        }

        // 拡張子の検証
        if config.include_extensions.is_empty() {
            return Err(DocsError::Config("At least one file extension must be specified".to_string()));
        }

        Ok(())
    }

    /// 設定ファイルのテンプレートを生成
    pub fn generate_template() -> DocsConfig {
        DocsConfig::default()
    }

    /// 設定ファイルの検索
    pub fn find_config_files(&self) -> Vec<PathBuf> {
        CONFIG_FILE_NAMES.iter()
            .map(|name| self.project_root.join(name))
            .filter(|path| path.exists())
            .collect()
    }

    /// 利用可能な設定ファイル形式を取得
    pub fn get_supported_formats() -> Vec<&'static str> {
        vec!["toml", "json", "yaml", "yml"]
    }

    /// プロジェクトタイプに基づいて設定を推測
    pub fn infer_config_from_project(&self) -> Result<DocsConfig> {
        let mut config = DocsConfig::default();

        // package.jsonが存在する場合
        let package_json = self.project_root.join("package.json");
        if package_json.exists() {
            if let Ok(package_data) = self.load_package_json(&package_json) {
                self.merge_package_config(&mut config, &package_data);
            }
        }

        // Cargo.tomlが存在する場合
        let cargo_toml = self.project_root.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(cargo_data) = self.load_cargo_toml(&cargo_toml) {
                self.merge_cargo_config(&mut config, &cargo_data);
            }
        }

        // pyproject.tomlが存在する場合
        let pyproject_toml = self.project_root.join("pyproject.toml");
        if pyproject_toml.exists() {
            if let Ok(pyproject_data) = self.load_pyproject_toml(&pyproject_toml) {
                self.merge_pyproject_config(&mut config, &pyproject_data);
            }
        }

        Ok(config)
    }

    /// package.jsonを読み込む
    fn load_package_json(&self, path: &Path) -> Result<serde_json::Value> {
        let content = fs::read_to_string(path)
            .map_err(|e| DocsError::Config(format!("Failed to read package.json: {}", e)))?;

        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| DocsError::Config(format!("Failed to parse package.json: {}", e)))?;

        Ok(value)
    }

    /// Cargo.tomlを読み込む
    fn load_cargo_toml(&self, path: &Path) -> Result<toml::Value> {
        let content = fs::read_to_string(path)
            .map_err(|e| DocsError::Config(format!("Failed to read Cargo.toml: {}", e)))?;

        let value: toml::from_str(&content)
            .map_err(|e| DocsError::Config(format!("Failed to parse Cargo.toml: {}", e)))?;

        Ok(value)
    }

    /// pyproject.tomlを読み込む
    fn load_pyproject_toml(&self, path: &Path) -> Result<toml::Value> {
        let content = fs::read_to_string(path)
            .map_err(|e| DocsError::Config(format!("Failed to read pyproject.toml: {}", e)))?;

        let value: toml::from_str(&content)
            .map_err(|e| DocsError::Config(format!("Failed to parse pyproject.toml: {}", e)))?;

        Ok(value)
    }

    /// package.jsonの設定をマージ
    fn merge_package_config(&self, config: &mut DocsConfig, package_data: &serde_json::Value) {
        if let Some(name) = package_data.get("name").and_then(|n| n.as_str()) {
            config.name = name.to_string();
        }

        if let Some(version) = package_data.get("version").and_then(|v| v.as_str()) {
            config.version = version.to_string();
        }

        if let Some(description) = package_data.get("description").and_then(|d| d.as_str()) {
            config.description = Some(description.to_string());
        }

        if let Some(repository) = package_data.get("repository").and_then(|r| r.as_str()) {
            config.repository = Some(repository.to_string());
        }

        // 拡張子を更新
        config.include_extensions = vec![
            "js".to_string(),
            "ts".to_string(),
            "jsx".to_string(),
            "tsx".to_string(),
            "md".to_string(),
        ];
    }

    /// Cargo.tomlの設定をマージ
    fn merge_cargo_config(&self, config: &mut DocsConfig, cargo_data: &toml::Value) {
        if let Some(package) = cargo_data.get("package").and_then(|p| p.as_table()) {
            if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
                config.name = name.to_string();
            }
            if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
                config.version = version.to_string();
            }
            if let Some(description) = package.get("description").and_then(|d| d.as_str()) {
                config.description = Some(description.to_string());
            }
            if let Some(repository) = package.get("repository").and_then(|r| r.as_str()) {
                config.repository = Some(repository.to_string());
            }
        }

        // 拡張子を更新
        config.include_extensions = vec![
            "rs".to_string(),
            "md".to_string(),
        ];
    }

    /// pyproject.tomlの設定をマージ
    fn merge_pyproject_config(&self, config: &mut DocsConfig, pyproject_data: &toml::Value) {
        if let Some(project) = pyproject_data.get("project").and_then(|p| p.as_table()) {
            if let Some(name) = project.get("name").and_then(|n| n.as_str()) {
                config.name = name.to_string();
            }
            if let Some(version) = project.get("version").and_then(|v| v.as_str()) {
                config.version = version.to_string();
            }
            if let Some(description) = project.get("description").and_then(|d| d.as_str()) {
                config.description = Some(description.to_string());
            }
        }

        // 拡張子を更新
        config.include_extensions = vec![
            "py".to_string(),
            "md".to_string(),
        ];
    }
}

/// 設定の自動検出とマージ
pub async fn auto_detect_config(project_root: &Path) -> Result<DocsConfig> {
    let manager = ConfigManager::new(project_root.to_path_buf());
    let mut config = manager.load_config().await?;

    // プロジェクトから設定を推測してマージ
    let inferred_config = manager.infer_config_from_project()?;
    merge_configs(&mut config, &inferred_config);

    Ok(config)
}

/// 設定をマージ
fn merge_configs(base: &mut DocsConfig, overlay: &DocsConfig) {
    // 名前がデフォルトの場合は上書き
    if base.name == "My Project" && overlay.name != "My Project" {
        base.name = overlay.name.clone();
    }

    // バージョンがデフォルトの場合は上書き
    if base.version == "0.1.0" && overlay.version != "0.1.0" {
        base.version = overlay.version.clone();
    }

    // 説明が空の場合は上書き
    if base.description.is_none() && overlay.description.is_some() {
        base.description = overlay.description.clone();
    }

    // リポジトリが空の場合は上書き
    if base.repository.is_none() && overlay.repository.is_some() {
        base.repository = overlay.repository.clone();
    }

    // 拡張子をマージ（重複を除去）
    let mut all_extensions = base.include_extensions.clone();
    for ext in &overlay.include_extensions {
        if !all_extensions.contains(ext) {
            all_extensions.push(ext.clone());
        }
    }
    base.include_extensions = all_extensions;
}

/// 設定ファイルを作成するユーティリティ関数
pub async fn create_default_config_file(project_root: &Path) -> Result<()> {
    let manager = ConfigManager::new(project_root.to_path_buf());
    let config = DocsConfig::default();
    manager.save_config(&config, None).await?;
    println!("Created default config file: kotoba-docs.toml");
    Ok(())
}

/// 設定をダンプ（デバッグ用）
pub fn dump_config(config: &DocsConfig) -> String {
    format!(
        r#"Configuration:
  Name: {}
  Version: {}
  Description: {}
  Repository: {}
  Input Dir: {}
  Output Dir: {}
  Formats: {:?}
  Extensions: {:?}
  Server: {}:{}
  Search: {}
"#,
        config.name,
        config.version,
        config.description.as_ref().unwrap_or(&"None".to_string()),
        config.repository.as_ref().unwrap_or(&"None".to_string()),
        config.input_dir.display(),
        config.output_dir.display(),
        config.formats,
        config.include_extensions,
        config.server.host,
        config.server.port,
        if config.search.enabled { "enabled" } else { "disabled" }
    )
}

//! 設定ファイル管理モジュール

use super::{BuildConfig, Result, BuildError};
use std::path::{Path, PathBuf};

/// 設定ファイルの名前候補
const CONFIG_FILE_NAMES: &[&str] = &[
    "kotoba-build.toml",
    "kotoba-build.json",
    "kotoba-build.yaml",
    "build.toml",
    "build.json",
    "build.yaml",
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
    pub async fn load_config(&self) -> Result<BuildConfig> {
        for filename in CONFIG_FILE_NAMES {
            let config_path = self.project_root.join(filename);
            if config_path.exists() {
                return self.load_config_from_file(&config_path).await;
            }
        }

        // 設定ファイルが見つからない場合はデフォルト設定を使用
        println!("No config file found, using default configuration");
        Ok(BuildConfig::default())
    }

    /// ファイルから設定を読み込む
    pub async fn load_config_from_file(&self, path: &Path) -> Result<BuildConfig> {
        println!("Loading config from: {}", path.display());

        let content = tokio::fs::read_to_string(path).await
            .map_err(|e| BuildError::Config(format!("Failed to read config file: {}", e)))?;

        match path.extension().and_then(|s| s.to_str()) {
            Some("toml") => {
                let config: BuildConfig = toml::from_str(&content)
                    .map_err(|e| BuildError::Config(format!("TOML parse error: {}", e)))?;
                Ok(config)
            }
            Some("json") => {
                let config: BuildConfig = serde_json::from_str(&content)
                    .map_err(|e| BuildError::Config(format!("JSON parse error: {}", e)))?;
                Ok(config)
            }
            Some("yaml") | Some("yml") => {
                let config: BuildConfig = serde_yaml::from_str(&content)
                    .map_err(|e| BuildError::Config(format!("YAML parse error: {}", e)))?;
                Ok(config)
            }
            _ => Err(BuildError::Config(format!("Unsupported config file format: {:?}", path))),
        }
    }

    /// 設定をファイルに保存
    pub async fn save_config(&self, config: &BuildConfig, path: Option<&Path>) -> Result<()> {
        let config_path = path.unwrap_or(&self.project_root.join("kotoba-build.toml"));

        // ディレクトリが存在することを確認
        if let Some(parent) = config_path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| BuildError::Config(format!("Failed to create config directory: {}", e)))?;
        }

        let content = match config_path.extension().and_then(|s| s.to_str()) {
            Some("json") => serde_json::to_string_pretty(config)
                .map_err(|e| BuildError::Config(format!("JSON serialization error: {}", e)))?,
            Some("yaml") | Some("yml") => serde_yaml::to_string(config)
                .map_err(|e| BuildError::Config(format!("YAML serialization error: {}", e)))?,
            _ => toml::to_string_pretty(config)
                .map_err(|e| BuildError::Config(format!("TOML serialization error: {}", e)))?,
        };

        tokio::fs::write(config_path, content).await
            .map_err(|e| BuildError::Config(format!("Failed to write config file: {}", e)))?;

        println!("Config saved to: {}", config_path.display());
        Ok(())
    }

    /// 設定ファイルのテンプレートを生成
    pub fn generate_template() -> BuildConfig {
        let mut tasks = std::collections::HashMap::new();

        tasks.insert(
            "dev".to_string(),
            super::TaskConfig {
                command: "cargo".to_string(),
                args: vec!["run".to_string()],
                description: Some("Start development server".to_string()),
            }
        );

        tasks.insert(
            "build".to_string(),
            super::TaskConfig {
                command: "cargo".to_string(),
                args: vec!["build".to_string(), "--release".to_string()],
                description: Some("Build project in release mode".to_string()),
            }
        );

        tasks.insert(
            "test".to_string(),
            super::TaskConfig {
                command: "cargo".to_string(),
                args: vec!["test".to_string()],
                description: Some("Run tests".to_string()),
            }
        );

        tasks.insert(
            "clean".to_string(),
            super::TaskConfig {
                command: "cargo".to_string(),
                args: vec!["clean".to_string()],
                description: Some("Clean build artifacts".to_string()),
            }
        );

        let mut dependencies = std::collections::HashMap::new();
        dependencies.insert("tokio".to_string(), "1.0".to_string());
        dependencies.insert("serde".to_string(), "1.0".to_string());

        BuildConfig {
            name: "my-kotoba-project".to_string(),
            version: "0.1.0".to_string(),
            description: Some("A Kotoba project".to_string()),
            tasks,
            dependencies,
        }
    }

    /// 設定の検証
    pub fn validate_config(config: &BuildConfig) -> Result<()> {
        // プロジェクト名の検証
        if config.name.trim().is_empty() {
            return Err(BuildError::Config("Project name cannot be empty".to_string()));
        }

        // バージョンの検証
        if config.version.trim().is_empty() {
            return Err(BuildError::Config("Version cannot be empty".to_string()));
        }

        // タスク名の検証
        for task_name in config.tasks.keys() {
            if task_name.trim().is_empty() {
                return Err(BuildError::Config("Task name cannot be empty".to_string()));
            }

            if task_name.contains(" ") {
                return Err(BuildError::Config(format!("Task name '{}' cannot contain spaces", task_name)));
            }
        }

        Ok(())
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
}

/// 設定の自動検出とマージ
pub async fn auto_detect_config(project_root: &Path) -> Result<BuildConfig> {
    let manager = ConfigManager::new(project_root.to_path_buf());
    let mut config = manager.load_config().await?;

    // package.jsonが存在する場合は依存関係をマージ
    let package_json = project_root.join("package.json");
    if package_json.exists() {
        if let Ok(package_config) = load_package_json_config(&package_json).await {
            merge_package_config(&mut config, &package_config);
        }
    }

    // Cargo.tomlが存在する場合は依存関係をマージ
    let cargo_toml = project_root.join("Cargo.toml");
    if cargo_toml.exists() {
        if let Ok(cargo_config) = load_cargo_config(&cargo_toml).await {
            merge_cargo_config(&mut config, &cargo_config);
        }
    }

    Ok(config)
}

/// package.jsonから設定を読み込む
async fn load_package_json_config(path: &Path) -> Result<serde_json::Value> {
    let content = tokio::fs::read_to_string(path).await?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    Ok(value)
}

/// Cargo.tomlから設定を読み込む
async fn load_cargo_config(path: &Path) -> Result<toml::Value> {
    let content = tokio::fs::read_to_string(path).await?;
    let value: toml::Value = toml::from_str(&content)?;
    Ok(value)
}

/// package.jsonの設定をマージ
fn merge_package_config(build_config: &mut BuildConfig, package_config: &serde_json::Value) {
    // プロジェクト名
    if let Some(name) = package_config.get("name").and_then(|n| n.as_str()) {
        build_config.name = name.to_string();
    }

    // バージョン
    if let Some(version) = package_config.get("version").and_then(|v| v.as_str()) {
        build_config.version = version.to_string();
    }

    // 説明
    if let Some(description) = package_config.get("description").and_then(|d| d.as_str()) {
        build_config.description = Some(description.to_string());
    }

    // スクリプトをタスクとしてマージ
    if let Some(scripts) = package_config.get("scripts").and_then(|s| s.as_object()) {
        for (script_name, script_value) in scripts {
            if let Some(script_cmd) = script_value.as_str() {
                let args: Vec<String> = script_cmd.split_whitespace().skip(1).map(|s| s.to_string()).collect();
                let command = script_cmd.split_whitespace().next().unwrap_or("echo").to_string();

                let task = super::TaskConfig {
                    command,
                    args,
                    description: Some(format!("Run {}", script_name)),
                };

                build_config.tasks.insert(script_name.clone(), task);
            }
        }
    }

    // 依存関係をマージ
    if let Some(dependencies) = package_config.get("dependencies").and_then(|d| d.as_object()) {
        for (dep_name, dep_version) in dependencies {
            if let Some(version) = dep_version.as_str() {
                build_config.dependencies.insert(dep_name.clone(), version.to_string());
            }
        }
    }
}

/// Cargo.tomlの設定をマージ
fn merge_cargo_config(build_config: &mut BuildConfig, cargo_config: &toml::Value) {
    // プロジェクト名とバージョン
    if let Some(package) = cargo_config.get("package").and_then(|p| p.as_table()) {
        if let Some(name) = package.get("name").and_then(|n| n.as_str()) {
            build_config.name = name.to_string();
        }
        if let Some(version) = package.get("version").and_then(|v| v.as_str()) {
            build_config.version = version.to_string();
        }
        if let Some(description) = package.get("description").and_then(|d| d.as_str()) {
            build_config.description = Some(description.to_string());
        }
    }

    // 依存関係をマージ
    if let Some(dependencies) = cargo_config.get("dependencies").and_then(|d| d.as_table()) {
        for (dep_name, dep_value) in dependencies {
            if let Some(dep_table) = dep_value.as_table() {
                if let Some(version) = dep_table.get("version").and_then(|v| v.as_str()) {
                    build_config.dependencies.insert(dep_name.clone(), version.to_string());
                }
            } else if let Some(version) = dep_value.as_str() {
                build_config.dependencies.insert(dep_name.clone(), version.to_string());
            }
        }
    }
}

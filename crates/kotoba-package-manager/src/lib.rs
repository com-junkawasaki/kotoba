//! Kotoba Package Manager
//!
//! Deno/npm/cargoライクなパッケージ管理システムを提供します。
//! 依存関係の解決、パッケージのインストール/アンインストール、
//! レジストリ管理などの機能を備えています。

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use url::Url;

pub mod config;
pub mod dependency;
pub mod registry;
pub mod installer;
pub mod lockfile;
pub mod cache;
mod resolver;

/// Package Managerのメイン構造体
#[derive(Debug)]
pub struct PackageManager {
    config: config::Config,
    registry: registry::Registry,
    cache: cache::Cache,
}

/// パッケージの取得元
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PackageSource {
    Registry(String), // Kotoba or Npm registry
    Git(GitSource),
    Url(Url),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GitSource {
    pub url: Url,
    pub revision: String, // branch, tag, or commit hash
}

/// パッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String, // semver::VersionをStringとして扱う
    pub source: PackageSource,
    pub cid: Option<String>, // Content ID
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub dependencies: HashMap<String, DependencyInfo>,
    pub dev_dependencies: HashMap<String, DependencyInfo>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
}

/// 依存関係情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub version: String, // semver::VersionReqをStringとして扱う
    #[serde(flatten)]
    pub source: Option<PackageSource>,
}

/// プロジェクト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String, // semver::VersionをStringとして扱う
    pub description: Option<String>,
    pub dependencies: HashMap<String, DependencyInfo>,
    pub dev_dependencies: HashMap<String, DependencyInfo>,
    pub scripts: HashMap<String, String>,
}

impl PackageManager {
    /// 新しいPackage Managerを作成
    pub async fn new() -> Result<Self> {
        let config = config::Config::load()?;
        let registry = registry::Registry::new(&config)?;
        let cache = cache::Cache::new(&config)?;

        Ok(Self {
            config,
            registry,
            cache,
        })
    }

    /// パッケージをインストール
    pub async fn install(&self) -> Result<()> {
        // kotoba.toml を読み込む
        let config_path = PathBuf::from("kotoba.toml");
        if !config_path.exists() {
            println!("kotoba.toml not found. Nothing to install.");
            return Ok(());
        }
        let toml_content = tokio::fs::read_to_string(&config_path).await?;
        let project_config: ProjectConfig = toml::from_str(&toml_content)?;

        println!("Installing packages from kotoba.toml");

        // 依存関係を解決
        let resolved = self.resolver().resolve(&project_config.dependencies).await?;

        // パッケージをダウンロードしてインストール
        self.installer().install(resolved).await?;

        println!("Installation completed!");
        Ok(())
    }

    /// パッケージをアンインストール
    pub async fn uninstall(&self, packages: Vec<String>) -> Result<()> {
        println!("Uninstalling packages: {:?}", packages);
        // TODO: 実装
        Ok(())
    }

    /// 利用可能なパッケージを検索
    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        println!("Searching for packages: {}", query);
        self.registry.search(query).await.map_err(Into::into)
    }

    /// プロジェクトを初期化
    pub async fn init(&self, name: Option<String>) -> Result<()> {
        let project_name = name.unwrap_or_else(|| "my-kotoba-project".to_string());

        let config = ProjectConfig {
            name: project_name.clone(),
            version: "0.1.0".to_string(),
            description: Some("A Kotoba project".to_string()),
            dependencies: HashMap::new(),
            dev_dependencies: HashMap::new(),
            scripts: [
                ("test".to_string(), "kotoba test".to_string()),
                ("fmt".to_string(), "kotoba fmt".to_string()),
                ("lint".to_string(), "kotoba lint".to_string()),
            ].into_iter().collect(),
        };

        // kotoba.tomlを作成
        let config_path = PathBuf::from("kotoba.toml");
        let toml_content = toml::to_string(&config)?;
        tokio::fs::write(&config_path, toml_content).await?;

        // srcディレクトリを作成
        tokio::fs::create_dir_all("src").await?;

        // 基本的なmain.kotobaファイルを作成
        let main_content = format!(r#"// {} - Kotoba Project
// This is your main entry point

fn main() {{
    println("Hello, {}!");
}}

// Export your public API
pub fn greet(name: String) -> String {{
    format("Hello, {{}}!", name)
}}
"#, project_name, project_name);

        tokio::fs::write("src/main.kotoba", main_content).await?;

        println!("✅ Initialized Kotoba project: {}", project_name);
        println!("📁 Created kotoba.toml and src/main.kotoba");
        println!("🚀 Run 'kotoba run src/main.kotoba' to get started!");

        Ok(())
    }

    /// キャッシュをクリア
    pub async fn clear_cache(&self) -> Result<()> {
        self.cache.clear().await?;
        println!("✅ Cache cleared!");
        Ok(())
    }

    /// 依存関係を解決
    fn resolver(&self) -> resolver::Resolver {
        resolver::Resolver::new()
    }

    /// パッケージインストーラー
    fn installer(&self) -> installer::Installer {
        installer::Installer::new()
    }
}

/// 便利関数
pub async fn init_project(name: Option<String>) -> Result<()> {
    let pm = PackageManager::new().await?;
    pm.init(name).await
}

pub async fn install_packages() -> Result<()> {
    let pm = PackageManager::new().await?;
    pm.install().await
}

pub async fn search_packages(query: &str) -> Result<Vec<Package>> {
    let pm = PackageManager::new().await?;
    pm.search(query).await
}

// 各モジュールの宣言（実装は別ファイル）
pub use config::*;
pub use dependency::*;
pub use registry::*;
pub use installer::*;
pub use lockfile::*;
pub use cache::*;
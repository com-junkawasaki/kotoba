//! Kotoba Package Manager
//!
//! Deno/npm/cargoライクなパッケージ管理システムを提供します。
//! 依存関係の解決、パッケージのインストール/アンインストール、
//! レジストリ管理などの機能を備えています。

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

pub mod config;
pub mod dependency;
pub mod registry;
pub mod resolver;
pub mod installer;
pub mod lockfile;
pub mod cache;

/// Package Managerのメイン構造体
#[derive(Debug)]
pub struct PackageManager {
    config: config::Config,
    registry: registry::Registry,
    cache: cache::Cache,
}

/// パッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub dependencies: HashMap<String, semver::VersionReq>,
    pub dev_dependencies: HashMap<String, semver::VersionReq>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
}

/// プロジェクト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: semver::Version,
    pub description: Option<String>,
    pub dependencies: HashMap<String, semver::VersionReq>,
    pub dev_dependencies: HashMap<String, semver::VersionReq>,
    pub scripts: HashMap<String, String>,
}

impl PackageManager {
    /// 新しいPackage Managerを作成
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
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
    pub async fn install(&self, packages: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Installing packages: {:?}", packages);

        // 依存関係を解決
        let resolved = self.resolver().resolve(&packages).await?;

        // パッケージをダウンロードしてインストール
        self.installer().install(resolved).await?;

        println!("Installation completed!");
        Ok(())
    }

    /// パッケージをアンインストール
    pub async fn uninstall(&self, packages: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Uninstalling packages: {:?}", packages);
        // TODO: 実装
        Ok(())
    }

    /// 利用可能なパッケージを検索
    pub async fn search(&self, query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        println!("Searching for packages: {}", query);
        self.registry.search(query).await
    }

    /// プロジェクトを初期化
    pub async fn init(&self, name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
        let project_name = name.unwrap_or_else(|| "my-kotoba-project".to_string());

        let config = ProjectConfig {
            name: project_name.clone(),
            version: semver::Version::parse("0.1.0")?,
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
    pub async fn clear_cache(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.cache.clear().await?;
        println!("✅ Cache cleared!");
        Ok(())
    }

    /// 依存関係を解決
    fn resolver(&self) -> &resolver::Resolver {
        // TODO: Resolver実装
        unimplemented!()
    }

    /// パッケージインストーラー
    fn installer(&self) -> &installer::Installer {
        // TODO: Installer実装
        unimplemented!()
    }
}

/// 便利関数
pub async fn init_project(name: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let pm = PackageManager::new().await?;
    pm.init(name).await
}

pub async fn install_packages(packages: Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    let pm = PackageManager::new().await?;
    pm.install(packages).await
}

pub async fn search_packages(query: &str) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
    let pm = PackageManager::new().await?;
    pm.search(query).await
}

// 各モジュールの宣言（実装は別ファイル）
pub use config::*;
pub use dependency::*;
pub use registry::*;
pub use resolver::*;
pub use installer::*;
pub use lockfile::*;
pub use cache::*;
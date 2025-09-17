//! Kotoba Package Manager
//!
//! Deno/npm/cargoライクなパッケージ管理システムを提供します。
//! 依存関係の解決、パッケージのインストール/アンインストール、
//! レジストリ管理などの機能を備えています。

use anyhow::Result;
use kotoba_cid::CidCalculator;
use std::collections::HashMap;
use std::path::PathBuf;

pub mod config;
pub mod dependency;
pub mod registry;
pub mod installer;
pub mod lockfile;
pub mod cache;
mod resolver;

pub use dependency::{DependencyInfo, Package, PackageSource, ProjectConfig};
use lockfile::Lockfile;
use resolver::Resolver;

/// Package Managerのメイン構造体
#[derive(Debug)]
pub struct PackageManager {
    config: config::Config,
    registry: registry::Registry,
    cache: cache::Cache,
    installer: installer::Installer,
}

impl PackageManager {
    /// 新しいPackage Managerを作成
    pub async fn new() -> Result<Self> {
        let config = config::Config::load()?;
        let registry = registry::Registry::new(&config)?;
        let cache = cache::Cache::new(&config)?;
        let installer = installer::Installer::new(cache.clone());

        Ok(Self {
            config,
            registry,
            cache,
            installer,
        })
    }

    /// パッケージをインストール
    pub async fn install(&self) -> Result<()> {
        let calculator = CidCalculator::default();
        let lockfile_path = PathBuf::from("kotoba.lock");

        // 1. Load project config
        let config_path = PathBuf::from("kotoba.toml");
        if !config_path.exists() {
            println!("kotoba.toml not found. Nothing to install.");
            return Ok(());
        }
        let toml_content = tokio::fs::read_to_string(&config_path).await?;
        let project_config: ProjectConfig = toml::from_str(&toml_content)?;

        // 2. Load lockfile and verify cache integrity
        let mut packages_to_install = project_config.dependencies.clone();
        let mut locked_packages: HashMap<String, crate::lockfile::LockedPackage> = HashMap::new();

        if let Some(lockfile) = Lockfile::read_from_disk(&lockfile_path).await? {
            println!("Verifying lockfile...");
            locked_packages = lockfile.packages;

            for (name, _dep_info) in &project_config.dependencies {
                if let Some(locked) = locked_packages.get(name) {
                    let package_dir = PathBuf::from("node_modules").join(name);
                    if self.cache.get_by_cid(&locked.cid).await?.is_some() && package_dir.exists() {
                        // This package is cached and installed, no need to re-resolve
                        packages_to_install.remove(name);
                    }
                }
            }
        }
        
        if packages_to_install.is_empty() {
            println!("All dependencies are up to date.");
            // A more robust implementation would still verify node_modules content here.
            return Ok(());
        }

        // 3. Resolve and download missing/invalidated packages
        println!("Resolving and installing {} packages...", packages_to_install.len());
        let mut resolved_packages = self.resolver().resolve(&packages_to_install).await?;
        
        for package in &mut resolved_packages {
            if let Some(url) = &package.tarball_url {
                let tarball_bytes = reqwest::get(url).await?.bytes().await?.to_vec();
                let cid = calculator.compute_cid(&tarball_bytes)?;
                package.cid = Some(cid.to_string());
                self.cache.store_by_cid(&cid.to_string(), &tarball_bytes).await?;
            }
        }

        // 4. Install resolved packages
        self.installer.install(resolved_packages.clone()).await?;

        // 5. Update lockfile with all packages
        let mut final_packages_map: HashMap<String, Package> = HashMap::new();

        // Add packages from the old lockfile that are still relevant
        for (name, _dep_info) in &project_config.dependencies {
             if let Some(locked) = locked_packages.get(name) {
                if packages_to_install.get(name).is_none() { // If it wasn't re-installed
                    final_packages_map.insert(name.clone(), Package {
                         name: name.clone(),
                         version: locked.version.clone(),
                         source: locked.source.clone(),
                         cid: Some(locked.cid.clone()),
                         tarball_url: None, 
                         description: None,
                         authors: vec![],
                         dependencies: HashMap::new(), // This information is lost
                         dev_dependencies: HashMap::new(),
                         repository: None,
                         license: None,
                         keywords: vec![],
                    });
                }
            }
        }
        
        // Add newly resolved packages
        for pkg in resolved_packages {
            final_packages_map.insert(pkg.name.clone(), pkg);
        }

        let lockfile = Lockfile::from_packages(&final_packages_map.values().cloned().collect::<Vec<_>>());
        lockfile.write_to_disk(&lockfile_path).await?;
        
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
    fn installer(&self) -> &installer::Installer {
        &self.installer
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
//! パッケージインストールモジュール

use super::{Package, Registry, Cache};

#[derive(Debug)]
pub struct Installer {
    registry: Registry,
    cache: Cache,
}

impl Installer {
    pub fn new(registry: Registry, cache: Cache) -> Self {
        Self { registry, cache }
    }

    pub async fn install(&self, packages: Vec<Package>) -> Result<(), Box<dyn std::error::Error>> {
        println!("Installing {} packages...", packages.len());
        // TODO: 実装
        Ok(())
    }

    pub async fn uninstall(&self, package_names: &[String]) -> Result<(), Box<dyn std::error::Error>> {
        println!("Uninstalling packages: {:?}", package_names);
        // TODO: 実装
        Ok(())
    }
}

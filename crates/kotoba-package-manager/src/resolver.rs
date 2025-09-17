//! 依存関係解決モジュール

use super::{Package, Registry, DependencyResolver};

#[derive(Debug)]
pub struct Resolver {
    registry: Registry,
}

impl Resolver {
    pub fn new(registry: Registry) -> Self {
        Self { registry }
    }

    pub async fn resolve(&self, package_names: &[String]) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        println!("Resolving dependencies...");
        // TODO: 実装
        Ok(vec![])
    }
}

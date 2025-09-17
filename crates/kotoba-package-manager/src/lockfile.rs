//! ロックファイル管理モジュール

use crate::{Package, PackageSource};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use tokio::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub packages: HashMap<String, LockedPackage>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockedPackage {
    pub version: String,
    pub source: PackageSource,
    pub cid: String,
}

impl Lockfile {
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
        }
    }

    pub async fn read_from_disk(path: &Path) -> Result<Option<Self>> {
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path).await?;
        let lockfile = serde_json::from_str(&content)?;
        Ok(Some(lockfile))
    }

    pub async fn write_to_disk(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content).await?;
        Ok(())
    }

    pub fn from_packages(packages: &[Package]) -> Self {
        let mut locked_packages = HashMap::new();
        for pkg in packages {
            if let Some(cid) = &pkg.cid {
                locked_packages.insert(
                    pkg.name.clone(),
                    LockedPackage {
                        version: pkg.version.clone(),
                        source: pkg.source.clone(),
                        cid: cid.clone(),
                    },
                );
            }
        }
        Self {
            packages: locked_packages,
        }
    }
}

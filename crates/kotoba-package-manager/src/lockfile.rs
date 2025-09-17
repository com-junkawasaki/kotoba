//! ロックファイル管理モジュール

use super::Package;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// ロックファイルの内容
#[derive(Debug, Serialize, Deserialize)]
pub struct Lockfile {
    pub version: semver::Version,
    pub packages: HashMap<String, LockedPackage>,
    pub metadata: LockMetadata,
}

/// ロックされたパッケージ情報
#[derive(Debug, Serialize, Deserialize)]
pub struct LockedPackage {
    pub version: semver::Version,
    pub checksum: String,
    pub dependencies: HashMap<String, semver::Version>,
}

/// ロックファイルのメタデータ
#[derive(Debug, Serialize, Deserialize)]
pub struct LockMetadata {
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub generator: String,
}

#[derive(Debug)]
pub struct LockfileManager {
    lockfile_path: PathBuf,
}

impl LockfileManager {
    pub fn new(project_root: PathBuf) -> Self {
        Self {
            lockfile_path: project_root.join("kotoba.lock"),
        }
    }

    pub async fn load(&self) -> Result<Option<Lockfile>, Box<dyn std::error::Error>> {
        if !self.lockfile_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&self.lockfile_path).await?;
        let lockfile: Lockfile = toml::from_str(&content)?;
        Ok(Some(lockfile))
    }

    pub async fn save(&self, lockfile: &Lockfile) -> Result<(), Box<dyn std::error::Error>> {
        let content = toml::to_string(lockfile)?;
        tokio::fs::write(&self.lockfile_path, content).await?;
        Ok(())
    }

    pub async fn update(&self, packages: &[Package]) -> Result<(), Box<dyn std::error::Error>> {
        let mut lockfile = self.load().await?.unwrap_or_else(|| Lockfile {
            version: semver::Version::parse("1.0.0").unwrap(),
            packages: HashMap::new(),
            metadata: LockMetadata {
                created_at: chrono::Utc::now(),
                generator: "kotoba-package-manager".to_string(),
            },
        });

        // パッケージ情報を更新
        for package in packages {
            let locked_package = LockedPackage {
                version: package.version.clone(),
                checksum: self.calculate_checksum(package).await?,
                dependencies: HashMap::new(), // TODO: 依存関係を計算
            };

            lockfile.packages.insert(package.name.clone(), locked_package);
        }

        self.save(&lockfile).await?;
        Ok(())
    }

    async fn calculate_checksum(&self, package: &Package) -> Result<String, Box<dyn std::error::Error>> {
        // パッケージのチェックサムを計算
        let data = serde_json::to_string(package)?;
        let mut hasher = sha2::Sha256::new();
        hasher.update(data.as_bytes());
        Ok(hex::encode(hasher.finalize()))
    }
}

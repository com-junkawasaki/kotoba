//! パッケージインストールモジュール

use crate::Package;
use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs::{self, File};
use std::io::Cursor;
use std::path::Path;
use tar::Archive;

pub struct Installer;

impl Installer {
    pub fn new() -> Self {
        Self
    }

    pub async fn install(&self, packages: Vec<Package>) -> Result<()> {
        let node_modules = Path::new("node_modules");
        if !node_modules.exists() {
            fs::create_dir(node_modules)?;
        }

        for package in packages {
            if let Some(version_info) = self.get_npm_version_info(&package).await? {
                let tarball_bytes = reqwest::get(&version_info.dist.tarball)
                    .await?
                    .bytes()
                    .await?;

                let tar = GzDecoder::new(Cursor::new(tarball_bytes));
                let mut archive = Archive::new(tar);

                let package_dir = node_modules.join(&package.name);
                if !package_dir.exists() {
                    fs::create_dir_all(&package_dir)?;
                }
                
                // unpack to `node_modules/{package_name}`
                // The tarball from npm has a `package/` directory at the root,
                // so we need to strip that prefix.
                for entry in archive.entries()? {
                    let mut entry = entry?;
                    let path = entry.path()?;
                    let stripped_path = path.strip_prefix("package/").unwrap_or(&path);
                    let final_path = package_dir.join(stripped_path);
                    if let Some(parent) = final_path.parent() {
                        if !parent.exists() {
                            fs::create_dir_all(parent)?;
                        }
                    }
                    entry.unpack(&final_path)?;
                }
            }
        }

        Ok(())
    }

    async fn get_npm_version_info(
        &self,
        package: &Package,
    ) -> Result<Option<crate::registry::NpmVersionInfo>> {
        let npm_package = crate::registry::fetch_npm_package(&package.name).await?;
        Ok(npm_package.versions.get(&package.version).cloned())
    }
}

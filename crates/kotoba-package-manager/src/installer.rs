//! パッケージインストールモジュール

use crate::{cache::Cache, lockfile::Lockfile, Package};
use anyhow::Result;
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::path::Path;
use tar::Archive;

#[derive(Debug)]
pub struct Installer {
    cache: Cache,
}

impl Installer {
    pub fn new(cache: Cache) -> Self {
        Self { cache }
    }

    pub async fn install(&self, packages: Vec<Package>) -> Result<()> {
        let node_modules = Path::new("node_modules");
        if !node_modules.exists() {
            fs::create_dir(node_modules)?;
        }

        for package in &packages {
            if let Some(cid) = &package.cid {
                if let Some(tarball_bytes) = self.cache.get_by_cid(cid).await? {
                    let tar = GzDecoder::new(Cursor::new(tarball_bytes));
                    let mut archive = Archive::new(tar);

                    let package_dir = node_modules.join(&package.name);
                    if !package_dir.exists() {
                        fs::create_dir_all(&package_dir)?;
                    }

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
        }

        let lockfile = Lockfile::from_packages(&packages);
        let lockfile_path = Path::new("kotoba.lock");
        lockfile.write_to_disk(&lockfile_path).await?;

        Ok(())
    }
}

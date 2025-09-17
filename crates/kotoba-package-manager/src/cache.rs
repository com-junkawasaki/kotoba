//! キャッシュ管理モジュール

use super::Config;
use std::path::PathBuf;
use tokio::fs;

/// キャッシュマネージャー
#[derive(Debug)]
pub struct Cache {
    cache_dir: PathBuf,
}

impl Cache {
    /// 新しいキャッシュマネージャーを作成
    pub fn new(config: &Config) -> Result<Self, Box<dyn std::error::Error>> {
        let cache_dir = config.cache_dir.clone();
        std::fs::create_dir_all(&cache_dir)?;

        Ok(Self { cache_dir })
    }

    /// パッケージをキャッシュに保存
    pub async fn store(&self, package_name: &str, version: &semver::Version, data: &[u8])
        -> Result<PathBuf, Box<dyn std::error::Error>>
    {
        let package_dir = self.cache_dir.join(package_name).join(version.to_string());
        fs::create_dir_all(&package_dir).await?;

        let tarball_path = package_dir.join("package.tar.gz");
        fs::write(&tarball_path, data).await?;

        Ok(tarball_path)
    }

    /// キャッシュからパッケージを取得
    pub async fn get(&self, package_name: &str, version: &semver::Version)
        -> Result<Option<Vec<u8>>, Box<dyn std::error::Error>>
    {
        let tarball_path = self.cache_dir
            .join(package_name)
            .join(version.to_string())
            .join("package.tar.gz");

        if tarball_path.exists() {
            let data = fs::read(&tarball_path).await?;
            Ok(Some(data))
        } else {
            Ok(None)
        }
    }

    /// パッケージがキャッシュされているか確認
    pub fn is_cached(&self, package_name: &str, version: &semver::Version) -> bool {
        let tarball_path = self.cache_dir
            .join(package_name)
            .join(version.to_string())
            .join("package.tar.gz");

        tarball_path.exists()
    }

    /// キャッシュをクリア
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.cache_dir.exists() {
            fs::remove_dir_all(&self.cache_dir).await?;
            fs::create_dir_all(&self.cache_dir).await?;
        }
        Ok(())
    }

    /// キャッシュのサイズを取得
    pub async fn size(&self) -> Result<u64, Box<dyn std::error::Error>> {
        if !self.cache_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        let mut entries = fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                total_size += Self::dir_size(&entry.path()).await?;
            } else {
                total_size += entry.metadata().await?.len();
            }
        }

        Ok(total_size)
    }

    /// ディレクトリのサイズを計算
    async fn dir_size(path: &PathBuf) -> Result<u64, Box<dyn std::error::Error>> {
        let mut size = 0u64;
        let mut entries = fs::read_dir(path).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                size += Self::dir_size(&entry.path()).await?;
            } else {
                size += entry.metadata().await?.len();
            }
        }

        Ok(size)
    }

    /// 古いキャッシュをクリーンアップ
    pub async fn cleanup(&self, max_age_days: u32) -> Result<(), Box<dyn std::error::Error>> {
        if !self.cache_dir.exists() {
            return Ok(());
        }

        let cutoff = chrono::Utc::now() - chrono::Duration::days(max_age_days as i64);

        let mut entries = fs::read_dir(&self.cache_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let metadata = entry.metadata().await?;
            let modified = chrono::DateTime::<chrono::Utc>::from(metadata.modified()?);

            if modified < cutoff {
                if entry.file_type().await?.is_dir() {
                    fs::remove_dir_all(entry.path()).await?;
                } else {
                    fs::remove_file(entry.path()).await?;
                }
            }
        }

        Ok(())
    }

    /// キャッシュの内容をリストアップ
    pub async fn list(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if !self.cache_dir.exists() {
            return Ok(vec![]);
        }

        let mut packages = Vec::new();
        let mut entries = fs::read_dir(&self.cache_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                let package_name = entry.file_name().to_string_lossy().to_string();
                packages.push(package_name);
            }
        }

        Ok(packages)
    }
}

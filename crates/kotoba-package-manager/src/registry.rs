//! レジストリ管理モジュール

use super::{Package, Config};
use anyhow::Result;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::HashMap;

/// パッケージレジストリ
#[derive(Debug)]
pub struct Registry {
    client: Client,
    config: Config,
}

/// 検索結果
#[derive(Debug, Deserialize)]
pub struct SearchResult {
    pub packages: Vec<Package>,
    pub total: usize,
}

/// パッケージメタデータ
#[derive(Debug, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String, // semver::VersionをStringとして扱う
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub dependencies: std::collections::HashMap<String, String>, // semver::VersionReqをStringとして扱う
    pub dev_dependencies: std::collections::HashMap<String, String>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
    pub checksum: String,
    pub download_url: String,
}

impl Registry {
    /// 新しいレジストリを作成
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_millis(config.timeout))
            .build()?;

        Ok(Self {
            client,
            config: config.clone(),
        })
    }

    /// パッケージを検索
    pub async fn search(&self, query: &str) -> Result<Vec<Package>> {
        let url = format!("{}/search?q={}", self.config.registry_url, urlencoding::encode(query));

        let response = self.client.get(&url).send().await?;
        let result: SearchResult = response.json().await?;

        Ok(result.packages)
    }

    /// パッケージのメタデータを取得
    pub async fn get_package(&self, name: &str, version: Option<&str>)
        -> Result<PackageMetadata>
    {
        let version_str = version.map(|v| format!("/{}", v)).unwrap_or_default();
        let url = format!("{}/packages/{}{}", self.config.registry_url, name, version_str);

        let response = self.client.get(&url).send().await?;
        let metadata: PackageMetadata = response.json().await?;

        Ok(metadata)
    }

    /// パッケージをダウンロード
    pub async fn download_package(&self, metadata: &PackageMetadata)
        -> Result<Vec<u8>>
    {
        let response = self.client.get(&metadata.download_url).send().await?;
        let bytes = response.bytes().await?;

        // チェックサムを検証
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        let checksum = hex::encode(hasher.finalize());

        if checksum != metadata.checksum {
            return Err(anyhow::anyhow!("Checksum mismatch for package {}", metadata.name));
        }

        Ok(bytes.to_vec())
    }

    /// パッケージを公開
    pub async fn publish_package(&self, package: &Package, tarball: Vec<u8>)
        -> Result<()>
    {
        let url = format!("{}/packages", self.config.registry_url);

        let form = reqwest::multipart::Form::new()
            .text("name", package.name.clone())
            .text("version", package.version.to_string())
            .part("tarball", reqwest::multipart::Part::bytes(tarball));

        let response = self.client.post(&url).multipart(form).send().await?;

        if !response.status().is_success() {
            let error_msg = response.text().await?;
            return Err(anyhow::anyhow!("Failed to publish package: {}", error_msg));
        }

        println!("✅ Published {}@{}", package.name, package.version);
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmPackage {
    pub name: String,
    pub versions: HashMap<String, NpmVersionInfo>,
    #[serde(rename = "dist-tags")]
    pub dist_tags: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmVersionInfo {
    pub name: String,
    pub version: String,
    pub dependencies: Option<HashMap<String, String>>,
    #[serde(rename = "devDependencies")]
    pub dev_dependencies: Option<HashMap<String, String>>,
    pub dist: NpmDist,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NpmDist {
    pub shasum: String,
    pub tarball: String,
}

pub async fn fetch_npm_package(package_name: &str) -> Result<NpmPackage, reqwest::Error> {
    let url = format!("https://registry.npmjs.org/{}", package_name);
    let package: NpmPackage = reqwest::get(&url).await?.json().await?;
    Ok(package)
}

//! 依存関係管理モジュール

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use url::Url;

/// パッケージの取得元
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PackageSource {
    Registry(String), // Kotoba or Npm registry
    Git(GitSource),
    Url(Url),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GitSource {
    pub url: Url,
    pub revision: String, // branch, tag, or commit hash
}

/// パッケージ情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub source: PackageSource,
    pub cid: Option<String>,
    pub tarball_url: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub dependencies: HashMap<String, DependencyInfo>,
    pub dev_dependencies: HashMap<String, DependencyInfo>,
    pub repository: Option<String>,
    pub license: Option<String>,
    pub keywords: Vec<String>,
}

/// 依存関係情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub version: String,
    #[serde(flatten)]
    pub source: Option<PackageSource>,
}

/// プロジェクト設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub dependencies: HashMap<String, DependencyInfo>,
    pub dev_dependencies: HashMap<String, DependencyInfo>,
    pub scripts: HashMap<String, String>,
}

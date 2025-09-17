//! 依存関係解決モジュール

use crate::{
    registry::{fetch_npm_package, NpmVersionInfo},
    DependencyInfo, Package, PackageSource,
};
use anyhow::Result;
use futures::future::{BoxFuture, FutureExt};
use kotoba_cid::CidCalculator;
use semver::{Version, VersionReq};
use std::collections::{HashMap, HashSet};

pub struct Resolver {
    cid_calculator: CidCalculator,
}

impl Resolver {
    pub fn new() -> Self {
        Self {
            cid_calculator: CidCalculator::default(),
        }
    }

    pub async fn resolve(
        &self,
        dependencies: &HashMap<String, DependencyInfo>,
    ) -> Result<Vec<Package>> {
        let mut resolved_packages = HashMap::new();
        let mut visited = HashSet::new();

        for (name, dep_info) in dependencies {
            self.resolve_recursive(
                name,
                &dep_info.version,
                &mut resolved_packages,
                &mut visited,
            )
            .await?;
        }

        Ok(resolved_packages.into_values().collect())
    }

    fn resolve_recursive<'a>(
        &'a self,
        package_name: &'a str,
        version_req: &'a str,
        resolved_packages: &'a mut HashMap<String, Package>,
        visited: &'a mut HashSet<(String, String)>,
    ) -> BoxFuture<'a, Result<()>> {
        async move {
            if !visited.insert((package_name.to_string(), version_req.to_string())) {
                return Ok(());
            }

            let npm_package = fetch_npm_package(package_name).await?;
            
            // Sanitize version string to handle cases like ">=1.2.3 || <2.0.0"
            let sanitized_version_req = version_req.split("||").last().unwrap_or(version_req).trim();

            // Try to parse as a specific version first
            let specific_version = Version::parse(sanitized_version_req).ok();

            let version_req_semver = VersionReq::parse(sanitized_version_req)?;

            let mut candidates: Vec<_> = npm_package
                .versions
                .values()
                .filter_map(|v| Version::parse(&v.version).ok().map(|semver| (semver, v)))
                .filter(|(semver, _)| version_req_semver.matches(semver))
                .collect();

            // Prefer the exact version match if it exists
            let best_version_info = if let Some(sv) = specific_version {
                candidates.iter().find(|(v, _)| v == &sv).map(|(_, vi)| (*vi).clone())
            } else {
                // Otherwise, get the latest satisfying version
                candidates.sort_by(|(v1, _), (v2, _)| v2.cmp(v1)); // Sort descending
                candidates.first().map(|(_, v_info)| (*v_info).clone())
            };

            if let Some(version_info) = best_version_info {
                let package = Package {
                    name: version_info.name.clone(),
                    version: version_info.version.clone(),
                    source: PackageSource::Registry("npm".to_string()),
                    cid: None, // Will be calculated after download
                    tarball_url: Some(version_info.dist.tarball),
                    description: None,
                    authors: vec![],
                    dependencies: self
                        .convert_dependencies(version_info.dependencies.as_ref())?,
                    dev_dependencies: self
                        .convert_dependencies(version_info.dev_dependencies.as_ref())?,
                    repository: None,
                    license: None,
                    keywords: vec![],
                };

                resolved_packages.insert(package.name.clone(), package.clone());

                if let Some(deps) = version_info.dependencies {
                    for (name, version) in deps {
                        self.resolve_recursive(&name, &version, resolved_packages, visited)
                            .await?;
                    }
                }
            }

            Ok(())
        }
        .boxed()
    }

    fn convert_dependencies(
        &self,
        deps: Option<&HashMap<String, String>>,
    ) -> Result<HashMap<String, DependencyInfo>> {
        let mut new_deps = HashMap::new();
        if let Some(deps_map) = deps {
            for (name, version) in deps_map {
                new_deps.insert(
                    name.clone(),
                    DependencyInfo {
                        version: version.clone(),
                        source: None,
                    },
                );
            }
        }
        Ok(new_deps)
    }
}

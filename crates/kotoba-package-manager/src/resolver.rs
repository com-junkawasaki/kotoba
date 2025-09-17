//! 依存関係解決モジュール

use crate::{
    registry::{fetch_npm_package, NpmVersionInfo},
    DependencyInfo, Package, PackageSource,
};
use anyhow::Result;
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

    async fn resolve_recursive(
        &self,
        package_name: &str,
        version_req: &str,
        resolved_packages: &mut HashMap<String, Package>,
        visited: &mut HashSet<(String, String)>,
    ) -> Result<()> {
        if !visited.insert((package_name.to_string(), version_req.to_string())) {
            return Ok(());
        }

        let npm_package = fetch_npm_package(package_name).await?;
        let version_req_semver = VersionReq::parse(version_req)?;

        let best_version_info = npm_package
            .versions
            .values()
            .filter_map(|v| Version::parse(&v.version).ok().map(|semver| (semver, v)))
            .filter(|(semver, _)| version_req_semver.matches(semver))
            .max_by(|(v1, _), (v2, _)| v1.cmp(v2))
            .map(|(_, v_info)| v_info.clone());

        if let Some(version_info) = best_version_info {
            let cid_input = format!("{}{}", version_info.name, version_info.version);
            let cid = self.cid_calculator.compute_cid(&cid_input)?;

            let package = Package {
                name: version_info.name.clone(),
                version: version_info.version.clone(),
                source: PackageSource::Registry("npm".to_string()),
                cid: Some(cid.to_string()),
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

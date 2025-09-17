//! 依存関係管理モジュール

use super::Package;
use std::collections::{HashMap, HashSet};

/// 依存関係グラフ
#[derive(Debug)]
pub struct DependencyGraph {
    packages: HashMap<String, Package>,
    dependencies: HashMap<String, HashSet<String>>,
}

/// 依存関係解決器
#[derive(Debug)]
pub struct DependencyResolver {
    graph: DependencyGraph,
}

impl DependencyGraph {
    /// 新しい依存関係グラフを作成
    pub fn new() -> Self {
        Self {
            packages: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    /// パッケージを追加
    pub fn add_package(&mut self, package: Package) {
        let name = package.name.clone();
        self.packages.insert(name.clone(), package);
        self.dependencies.insert(name, HashSet::new());
    }

    /// 依存関係を追加
    pub fn add_dependency(&mut self, from: &str, to: &str) {
        if let Some(deps) = self.dependencies.get_mut(from) {
            deps.insert(to.to_string());
        }
    }

    /// 循環依存関係をチェック
    pub fn has_cycles(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for package in self.packages.keys() {
            if self.has_cycles_util(package, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    fn has_cycles_util(&self, package: &str, visited: &mut HashSet<String>, rec_stack: &mut HashSet<String>) -> bool {
        visited.insert(package.to_string());
        rec_stack.insert(package.to_string());

        if let Some(deps) = self.dependencies.get(package) {
            for dep in deps {
                if !visited.contains(dep) {
                    if self.has_cycles_util(dep, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(dep) {
                    return true;
                }
            }
        }

        rec_stack.remove(package);
        false
    }

    /// トポロジカルソート
    pub fn topological_sort(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if self.has_cycles() {
            return Err("Circular dependency detected".into());
        }

        let mut result = Vec::new();
        let mut visited = HashSet::new();

        for package in self.packages.keys() {
            if !visited.contains(package) {
                self.topological_sort_util(package, &mut visited, &mut result);
            }
        }

        result.reverse();
        Ok(result)
    }

    fn topological_sort_util(&self, package: &str, visited: &mut HashSet<String>, result: &mut Vec<String>) {
        visited.insert(package.to_string());

        if let Some(deps) = self.dependencies.get(package) {
            for dep in deps {
                if !visited.contains(dep) {
                    self.topological_sort_util(dep, visited, result);
                }
            }
        }

        result.push(package.to_string());
    }
}

impl DependencyResolver {
    /// 新しい依存関係解決器を作成
    pub fn new() -> Self {
        Self {
            graph: DependencyGraph::new(),
        }
    }

    /// 依存関係を解決
    pub async fn resolve(&self, package_names: &[String]) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        println!("Resolving dependencies for: {:?}", package_names);

        // TODO: 実際の依存関係解決ロジックを実装
        // ここではプレースホルダー

        Ok(vec![])
    }

    /// バージョンの競合を解決
    pub fn resolve_conflicts(&self, packages: Vec<Package>) -> Result<Vec<Package>, Box<dyn std::error::Error>> {
        // バージョン競合の解決ロジック
        // セマンティックバージョニングに基づいて最適なバージョンを選択

        let mut resolved = HashMap::new();

        for package in packages {
            let existing = resolved.get(&package.name);

            match existing {
                Some(existing_pkg) => {
                    // より新しいバージョンまたは互換性のあるバージョンを選択
                    if package.version > existing_pkg.version {
                        resolved.insert(package.name.clone(), package);
                    }
                }
                None => {
                    resolved.insert(package.name.clone(), package);
                }
            }
        }

        Ok(resolved.into_values().collect())
    }
}

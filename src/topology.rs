//! # トポロジー検証モジュール
//!
//! dag.jsonnetで定義されたプロセスネットワークのトポロジーを検証する

use crate::types::{Result, KotobaError};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// ノード定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub name: String,
    pub path: String,
    pub node_type: String,
    pub description: String,
    pub dependencies: Vec<String>,
    pub provides: Vec<String>,
    pub status: String,
    pub build_order: u32,
}

/// エッジ定義
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
}

/// トポロジーグラフ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    pub topological_order: Vec<String>,
    pub reverse_topological_order: Vec<String>,
}

/// トポロジー検証器
pub struct TopologyValidator {
    graph: TopologyGraph,
}

impl TopologyValidator {
    /// 新しい検証器を作成
    pub fn new(graph: TopologyGraph) -> Self {
        Self { graph }
    }

    /// トポロジーの完全検証を実行
    pub fn validate_all(&self) -> Result<ValidationResult> {
        let mut results = Vec::new();
        let mut has_errors = false;

        // ノード存在チェック
        let node_existence = self.validate_node_existence()?;
        results.push(node_existence.clone());
        if !node_existence.is_valid { has_errors = true; }

        // エッジ整合性チェック
        let edge_integrity = self.validate_edge_integrity()?;
        results.push(edge_integrity.clone());
        if !edge_integrity.is_valid { has_errors = true; }

        // 循環依存チェック
        let cycle_detection = self.validate_no_cycles()?;
        results.push(cycle_detection.clone());
        if !cycle_detection.is_valid { has_errors = true; }

        // トポロジカル順序チェック
        let topological_order = self.validate_topological_order()?;
        results.push(topological_order.clone());
        if !topological_order.is_valid { has_errors = true; }

        // 依存関係整合性チェック
        let dependency_integrity = self.validate_dependency_integrity()?;
        results.push(dependency_integrity.clone());
        if !dependency_integrity.is_valid { has_errors = true; }

        // ビルド順序整合性チェック
        let build_order_integrity = self.validate_build_order_integrity()?;
        results.push(build_order_integrity.clone());
        if !build_order_integrity.is_valid { has_errors = true; }

        Ok(ValidationResult {
            is_valid: !has_errors,
            checks: results,
        })
    }

    /// ノード存在チェック
    pub fn validate_node_existence(&self) -> Result<ValidationCheck> {
        let mut missing_nodes = Vec::new();
        let node_names: HashSet<_> = self.graph.nodes.keys().collect();

        // エッジで参照されているノードがすべて存在するかチェック
        for edge in &self.graph.edges {
            if !node_names.contains(&edge.from) {
                missing_nodes.push(format!("Edge references missing 'from' node: {}", edge.from));
            }
            if !node_names.contains(&edge.to) {
                missing_nodes.push(format!("Edge references missing 'to' node: {}", edge.to));
            }
        }

        // トポロジカル順序で参照されているノードがすべて存在するかチェック
        for node_name in &self.graph.topological_order {
            if !node_names.contains(node_name) {
                missing_nodes.push(format!("Topological order references missing node: {}", node_name));
            }
        }

        for node_name in &self.graph.reverse_topological_order {
            if !node_names.contains(node_name) {
                missing_nodes.push(format!("Reverse topological order references missing node: {}", node_name));
            }
        }

        Ok(ValidationCheck {
            name: "Node Existence".to_string(),
            is_valid: missing_nodes.is_empty(),
            errors: missing_nodes,
            warnings: Vec::new(),
        })
    }

    /// エッジ整合性チェック
    pub fn validate_edge_integrity(&self) -> Result<ValidationCheck> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new(); // mutable warnings vector

        // エッジの一意性チェック
        let mut seen_edges = HashSet::new();
        for edge in &self.graph.edges {
            let edge_key = format!("{}->{}", edge.from, edge.to);
            if seen_edges.contains(&edge_key) {
                warnings.push(format!("Duplicate edge found: {}", edge_key));
            } else {
                seen_edges.insert(edge_key);
            }
        }

        // 自己参照エッジチェック
        for edge in &self.graph.edges {
            if edge.from == edge.to {
                errors.push(format!("Self-referencing edge found: {}", edge.from));
            }
        }

        Ok(ValidationCheck {
            name: "Edge Integrity".to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// 循環依存チェック
    pub fn validate_no_cycles(&self) -> Result<ValidationCheck> {
        let mut errors = Vec::new();

        // DFSを使用して循環を検出
        let mut visited = HashSet::new();
        let mut recursion_stack = HashSet::new();

        // 依存関係グラフを作成 (from -> to)
        let mut adj_list: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &self.graph.edges {
            adj_list.entry(edge.from.clone()).or_default().push(edge.to.clone());
        }

        // すべてのノードに対してDFSを実行
        for node in self.graph.nodes.keys() {
            if !visited.contains(node) {
                if self.has_cycle(node, &adj_list, &mut visited, &mut recursion_stack) {
                    errors.push(format!("Cycle detected involving node: {}", node));
                }
            }
        }

        Ok(ValidationCheck {
            name: "No Cycles".to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
        })
    }

    /// DFSで循環を検出する補助関数
    fn has_cycle(
        &self,
        node: &str,
        adj_list: &HashMap<String, Vec<String>>,
        visited: &mut HashSet<String>,
        recursion_stack: &mut HashSet<String>,
    ) -> bool {
        visited.insert(node.to_string());
        recursion_stack.insert(node.to_string());

        if let Some(neighbors) = adj_list.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    if self.has_cycle(neighbor, adj_list, visited, recursion_stack) {
                        return true;
                    }
                } else if recursion_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        recursion_stack.remove(node);
        false
    }

    /// トポロジカル順序チェック
    pub fn validate_topological_order(&self) -> Result<ValidationCheck> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new(); // mutable warnings vector

        // 順序の一意性チェック
        let topo_set: HashSet<_> = self.graph.topological_order.iter().collect();
        if topo_set.len() != self.graph.topological_order.len() {
            errors.push("Topological order contains duplicate nodes".to_string());
        }

        // すべてのノードが含まれているかチェック
        if topo_set.len() != self.graph.nodes.len() {
            let missing: Vec<_> = self.graph.nodes.keys()
                .filter(|n| !topo_set.contains(n))
                .collect();
            errors.push(format!("Topological order missing nodes: {:?}", missing));
        }

        // 依存関係順序の検証
        for (i, node_name) in self.graph.topological_order.iter().enumerate() {
            if let Some(node) = self.graph.nodes.get(node_name) {
                for dep in &node.dependencies {
                    // 依存先がこのノードより前に来ているはず
                    if let Some(dep_index) = self.graph.topological_order.iter().position(|n| n == dep) {
                        if dep_index >= i {
                            errors.push(format!("Dependency order violation: {} depends on {} but {} comes after",
                                node_name, dep, dep));
                        }
                    }
                }
            }
        }

        Ok(ValidationCheck {
            name: "Topological Order".to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }

    /// 依存関係整合性チェック
    pub fn validate_dependency_integrity(&self) -> Result<ValidationCheck> {
        let mut errors = Vec::new();

        // エッジとノードの依存関係が一致するかチェック
        for (node_name, node) in &self.graph.nodes {
            let edge_deps: HashSet<_> = self.graph.edges.iter()
                .filter(|e| &e.to == node_name)
                .map(|e| &e.from)
                .collect();

            let node_deps: HashSet<_> = node.dependencies.iter().collect();

            // エッジで定義されている依存関係がノードの依存関係に含まれているか
            for edge_dep in &edge_deps {
                if !node_deps.contains(edge_dep) {
                    errors.push(format!("Node {} has edge dependency {} but not in node.dependencies", node_name, edge_dep));
                }
            }

            // ノードの依存関係がエッジで定義されているか
            for node_dep in &node_deps {
                if !edge_deps.contains(node_dep) {
                    errors.push(format!("Node {} has dependency {} but no corresponding edge", node_name, node_dep));
                }
            }
        }

        Ok(ValidationCheck {
            name: "Dependency Integrity".to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings: Vec::new(),
        })
    }

    /// ビルド順序整合性チェック
    pub fn validate_build_order_integrity(&self) -> Result<ValidationCheck> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new(); // mutable warnings vector

        // ビルド順序の連続性チェック
        let mut build_orders: Vec<_> = self.graph.nodes.values()
            .map(|n| n.build_order)
            .collect();
        build_orders.sort();
        build_orders.dedup();

        let mut expected_order = 1;
        for &order in &build_orders {
            if order != expected_order {
                warnings.push(format!("Build order gap or duplicate: expected {}, found {}", expected_order, order));
            }
            expected_order += 1;
        }

        // 依存関係のビルド順序チェック
        for (node_name, node) in &self.graph.nodes {
            for dep in &node.dependencies {
                if let Some(dep_node) = self.graph.nodes.get(dep) {
                    if dep_node.build_order >= node.build_order {
                        errors.push(format!("Build order violation: {} (order {}) depends on {} (order {})",
                            node_name, node.build_order, dep, dep_node.build_order));
                    }
                }
            }
        }

        Ok(ValidationCheck {
            name: "Build Order Integrity".to_string(),
            is_valid: errors.is_empty(),
            errors,
            warnings,
        })
    }
}

/// 検証結果
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub checks: Vec<ValidationCheck>,
}

impl ValidationResult {
    /// 結果を文字列としてフォーマット
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Topology Validation: {}\n", if self.is_valid { "PASS" } else { "FAIL" }));
        output.push_str(&"=".repeat(50));
        output.push_str("\n\n");

        for check in &self.checks {
            output.push_str(&format!("{}: {}\n", check.name, if check.is_valid { "PASS" } else { "FAIL" }));

            if !check.errors.is_empty() {
                output.push_str("  Errors:\n");
                for error in &check.errors {
                    output.push_str(&format!("    - {}\n", error));
                }
            }

            if !check.warnings.is_empty() {
                output.push_str("  Warnings:\n");
                for warning in &check.warnings {
                    output.push_str(&format!("    - {}\n", warning));
                }
            }
            output.push_str("\n");
        }

        output
    }
}

/// 個別の検証チェック結果
#[derive(Debug, Clone)]
pub struct ValidationCheck {
    pub name: String,
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_graph() -> TopologyGraph {
        let mut nodes = HashMap::new();

        nodes.insert("types".to_string(), Node {
            name: "types".to_string(),
            path: "src/types.rs".to_string(),
            node_type: "foundation".to_string(),
            description: "Common types".to_string(),
            dependencies: vec![],
            provides: vec!["Value".to_string()],
            status: "completed".to_string(),
            build_order: 1,
        });

        nodes.insert("ir".to_string(), Node {
            name: "ir".to_string(),
            path: "src/ir/mod.rs".to_string(),
            node_type: "ir".to_string(),
            description: "IR layer".to_string(),
            dependencies: vec!["types".to_string()],
            provides: vec!["IR".to_string()],
            status: "completed".to_string(),
            build_order: 2,
        });

        let edges = vec![
            Edge { from: "types".to_string(), to: "ir".to_string() },
        ];

        let topological_order = vec!["types".to_string(), "ir".to_string()];
        let reverse_topological_order = vec!["ir".to_string(), "types".to_string()];

        TopologyGraph {
            nodes,
            edges,
            topological_order,
            reverse_topological_order,
        }
    }

    #[test]
    fn test_valid_topology() {
        let graph = create_test_graph();
        let validator = TopologyValidator::new(graph);
        let result = validator.validate_all().unwrap();

        assert!(result.is_valid, "Valid topology should pass all checks");
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = create_test_graph();

        // 循環依存を追加
        graph.edges.push(Edge {
            from: "ir".to_string(),
            to: "types".to_string(),
        });

        let validator = TopologyValidator::new(graph);
        let cycle_check = validator.validate_no_cycles().unwrap();

        assert!(!cycle_check.is_valid, "Cycle should be detected");
        assert!(!cycle_check.errors.is_empty(), "Should have cycle error");
    }

    #[test]
    fn test_missing_node() {
        let mut graph = create_test_graph();

        // 存在しないノードを参照するエッジを追加
        graph.edges.push(Edge {
            from: "types".to_string(),
            to: "missing".to_string(),
        });

        let validator = TopologyValidator::new(graph);
        let existence_check = validator.validate_node_existence().unwrap();

        assert!(!existence_check.is_valid, "Missing node should be detected");
    }
}

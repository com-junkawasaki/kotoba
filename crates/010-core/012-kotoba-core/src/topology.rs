//! Topology validation and graph processing module
//!
//! This module provides functionality for validating and processing
//! process network topologies defined in dag.jsonnet files.

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use kotoba_errors::KotobaError;

/// Represents a node in the topology graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Node {
    /// Node name/identifier
    pub name: String,
    /// File system path for the node
    pub path: String,
    /// Type of the node (e.g., "crate", "service", "workflow")
    pub node_type: String,
    /// Human-readable description
    pub description: String,
    /// List of dependencies (nodes this node depends on)
    pub dependencies: Vec<String>,
    /// List of capabilities this node provides
    pub provides: Vec<String>,
    /// Current status (e.g., "active", "inactive", "deprecated")
    pub status: String,
    /// Build order for topological sorting
    pub build_order: u32,
}

/// Represents an edge in the topology graph
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Edge {
    /// Source node name
    pub from: String,
    /// Target node name
    pub to: String,
}

/// Complete topology graph representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyGraph {
    /// All nodes in the topology
    pub nodes: HashMap<String, Node>,
    /// All edges in the topology
    pub edges: Vec<Edge>,
    /// Topologically sorted order of nodes
    pub topological_order: Vec<String>,
    /// Reverse topological order (dependencies first)
    pub reverse_topological_order: Vec<String>,
}

impl TopologyGraph {
    /// Create a new empty topology graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            topological_order: Vec::new(),
            reverse_topological_order: Vec::new(),
        }
    }

    /// Add a node to the topology
    pub fn add_node(&mut self, node: Node) {
        self.nodes.insert(node.name.clone(), node);
    }

    /// Add an edge to the topology
    pub fn add_edge(&mut self, edge: Edge) {
        self.edges.push(edge);
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

/// Individual validation check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    /// Name of the check
    pub name: String,
    /// Whether the check passed
    pub is_valid: bool,
    /// List of errors (empty if valid)
    pub errors: Vec<String>,
    /// List of warnings
    pub warnings: Vec<String>,
}

/// Complete validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub is_valid: bool,
    /// Individual check results
    pub checks: Vec<ValidationCheck>,
    /// Summary statistics
    pub statistics: ValidationStatistics,
}

impl ValidationResult {
    /// Create a new validation result
    pub fn new() -> Self {
        Self {
            is_valid: true,
            checks: Vec::new(),
            statistics: ValidationStatistics::default(),
        }
    }

    /// Add a check result
    pub fn add_check(&mut self, check: ValidationCheck) {
        if !check.is_valid {
            self.is_valid = false;
        }
        self.checks.push(check);
    }

    /// Format the result as a human-readable string
    pub fn format(&self) -> String {
        let mut output = format!("Topology Validation Result: {}\n", if self.is_valid { "PASS" } else { "FAIL" });
        output.push_str("================================\n\n");

        for check in &self.checks {
            let status = if check.is_valid { "✓" } else { "✗" };
            output.push_str(&format!("{} {}\n", status, check.name));

            for error in &check.errors {
                output.push_str(&format!("  Error: {}\n", error));
            }

            for warning in &check.warnings {
                output.push_str(&format!("  Warning: {}\n", warning));
            }

            output.push_str("\n");
        }

        output.push_str(&format!("Statistics:\n"));
        output.push_str(&format!("  Total checks: {}\n", self.checks.len()));
        output.push_str(&format!("  Passed: {}\n", self.checks.iter().filter(|c| c.is_valid).count()));
        output.push_str(&format!("  Failed: {}\n", self.checks.iter().filter(|c| !c.is_valid).count()));

        output
    }
}

/// Validation statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ValidationStatistics {
    /// Number of nodes
    pub node_count: usize,
    /// Number of edges
    pub edge_count: usize,
    /// Number of cycles detected
    pub cycle_count: usize,
    /// Number of orphaned nodes
    pub orphaned_nodes: usize,
}

/// Topology validator for checking graph properties
pub struct TopologyValidator {
    graph: TopologyGraph,
}

impl TopologyValidator {
    /// Create a new validator for the given topology graph
    pub fn new(graph: TopologyGraph) -> Self {
        Self { graph }
    }

    /// Run all validation checks
    pub fn validate_all(&self) -> Result<ValidationResult> {
        let mut result = ValidationResult::new();

        // Update statistics
        result.statistics.node_count = self.graph.node_count();
        result.statistics.edge_count = self.graph.edge_count();

        // Check topological ordering
        result.add_check(self.validate_topological_order()?);

        // Check for cycles
        result.add_check(self.validate_no_cycles()?);

        // Check node consistency
        result.add_check(self.validate_node_consistency()?);

        // Check edge consistency
        result.add_check(self.validate_edge_consistency()?);

        // Check build order
        result.add_check(self.validate_build_order()?);

        Ok(result)
    }

    /// Validate topological ordering
    fn validate_topological_order(&self) -> Result<ValidationCheck> {
        let mut check = ValidationCheck {
            name: "Topological Order".to_string(),
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let topo_order = &self.graph.topological_order;
        let rev_topo_order = &self.graph.reverse_topological_order;

        // Check lengths
        if topo_order.len() != self.graph.nodes.len() {
            check.is_valid = false;
            check.errors.push(format!(
                "Topological order length mismatch: expected {}, got {}",
                self.graph.nodes.len(),
                topo_order.len()
            ));
        }

        if rev_topo_order.len() != self.graph.nodes.len() {
            check.is_valid = false;
            check.errors.push(format!(
                "Reverse topological order length mismatch: expected {}, got {}",
                self.graph.nodes.len(),
                rev_topo_order.len()
            ));
        }

        // Check that all nodes are included
        for node_name in self.graph.nodes.keys() {
            if !topo_order.contains(node_name) {
                check.is_valid = false;
                check.errors.push(format!("Node '{}' missing from topological order", node_name));
            }
            if !rev_topo_order.contains(node_name) {
                check.is_valid = false;
                check.errors.push(format!("Node '{}' missing from reverse topological order", node_name));
            }
        }

        Ok(check)
    }

    /// Validate that the graph has no cycles
    fn validate_no_cycles(&self) -> Result<ValidationCheck> {
        let mut check = ValidationCheck {
            name: "No Cycles".to_string(),
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        // Simple cycle detection using DFS
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node_name in self.graph.nodes.keys() {
            if !visited.contains(node_name) {
                if self.has_cycle(node_name, &mut visited, &mut rec_stack) {
                    check.is_valid = false;
                    check.errors.push("Cycle detected in topology graph".to_string());
                    break;
                }
            }
        }

        Ok(check)
    }

    /// DFS-based cycle detection helper
    fn has_cycle(&self, node: &str, visited: &mut std::collections::HashSet<String>,
                 rec_stack: &mut std::collections::HashSet<String>) -> bool {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());

        // Check all neighbors
        for edge in &self.graph.edges {
            if edge.from == node {
                let neighbor = &edge.to;
                if !visited.contains(neighbor) {
                    if self.has_cycle(neighbor, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(neighbor) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }

    /// Validate node consistency
    fn validate_node_consistency(&self) -> Result<ValidationCheck> {
        let mut check = ValidationCheck {
            name: "Node Consistency".to_string(),
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        for (node_name, node) in &self.graph.nodes {
            // Check that dependencies exist
            for dep in &node.dependencies {
                if !self.graph.nodes.contains_key(dep) {
                    check.is_valid = false;
                    check.errors.push(format!(
                        "Node '{}' has dependency '{}' which does not exist",
                        node_name, dep
                    ));
                }
            }

            // Check build order is reasonable
            if node.build_order == 0 {
                check.warnings.push(format!("Node '{}' has build order 0", node_name));
            }
        }

        Ok(check)
    }

    /// Validate edge consistency
    fn validate_edge_consistency(&self) -> Result<ValidationCheck> {
        let mut check = ValidationCheck {
            name: "Edge Consistency".to_string(),
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        for edge in &self.graph.edges {
            // Check that both nodes exist
            if !self.graph.nodes.contains_key(&edge.from) {
                check.is_valid = false;
                check.errors.push(format!("Edge references non-existent source node '{}'", edge.from));
            }
            if !self.graph.nodes.contains_key(&edge.to) {
                check.is_valid = false;
                check.errors.push(format!("Edge references non-existent target node '{}'", edge.to));
            }

            // Check for self-loops
            if edge.from == edge.to {
                check.warnings.push(format!("Self-loop detected on node '{}'", edge.from));
            }
        }

        Ok(check)
    }

    /// Validate build order
    fn validate_build_order(&self) -> Result<ValidationCheck> {
        let mut check = ValidationCheck {
            name: "Build Order".to_string(),
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        let mut build_orders: Vec<u32> = self.graph.nodes.values()
            .map(|n| n.build_order)
            .collect();
        build_orders.sort();
        build_orders.dedup();

        // Check for gaps in build order
        if !build_orders.is_empty() {
            let min_order = build_orders[0];
            let _max_order = *build_orders.last().unwrap();

            if min_order != 1 {
                check.warnings.push(format!("Build order starts at {} instead of 1", min_order));
            }

            // Check for gaps
            for i in 1..build_orders.len() {
                let expected = build_orders[i-1] + 1;
                if build_orders[i] != expected {
                    check.warnings.push(format!(
                        "Gap in build order: missing order {}",
                        expected
                    ));
                }
            }
        }

        Ok(check)
    }
}

/// Result type alias for topology operations
pub type Result<T> = std::result::Result<T, KotobaError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_topology() {
        let graph = TopologyGraph::new();
        assert_eq!(graph.node_count(), 0);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn test_add_node() {
        let mut graph = TopologyGraph::new();
        let node = Node {
            name: "test".to_string(),
            path: "/test".to_string(),
            node_type: "crate".to_string(),
            description: "Test node".to_string(),
            dependencies: Vec::new(),
            provides: Vec::new(),
            status: "active".to_string(),
            build_order: 1,
        };

        graph.add_node(node);
        assert_eq!(graph.node_count(), 1);
        assert!(graph.nodes.contains_key("test"));
    }

    #[test]
    fn test_add_edge() {
        let mut graph = TopologyGraph::new();
        let edge = Edge {
            from: "a".to_string(),
            to: "b".to_string(),
        };

        graph.add_edge(edge);
        assert_eq!(graph.edge_count(), 1);
    }

    #[test]
    fn test_validation_result_formatting() {
        let mut result = ValidationResult::new();
        result.add_check(ValidationCheck {
            name: "Test Check".to_string(),
            is_valid: false,
            errors: vec!["Test error".to_string()],
            warnings: vec!["Test warning".to_string()],
        });

        let formatted = result.format();
        assert!(formatted.contains("FAIL"));
        assert!(formatted.contains("Test error"));
        assert!(formatted.contains("Test warning"));
    }
}

//! トポロジー検証の統合テスト
//!
//! dag.jsonnetからトポロジーデータを読み込み、
//! Rustの検証関数でトポロジーの整合性をチェックする

use kotoba::topology::{TopologyValidator, TopologyGraph, ValidationResult};
use kotoba::Result;
use std::process::Command;
use std::fs;
use std::path::Path;

/// jsonnetスクリプトを実行してトポロジーデータを生成
fn generate_topology_data() -> Result<TopologyGraph> {
    // jsonnetコマンドを実行してJSONデータを生成
    let output = Command::new("jsonnet")
        .arg("validate_topology.jsonnet")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| kotoba::KotobaError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Failed to execute jsonnet: {}", e)
        )))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(kotoba::KotobaError::Validation(
            format!("jsonnet execution failed: {}", stderr)
        ));
    }

    // JSONデータをパース
    let json_str = String::from_utf8(output.stdout)
        .map_err(|e| kotoba::KotobaError::Validation(
            format!("Invalid UTF-8 output from jsonnet: {}", e)
        ))?;

    // JSONからTopologyGraphを構築
    let data: serde_json::Value = serde_json::from_str(&json_str)
        .map_err(|e| kotoba::KotobaError::Validation(
            format!("Failed to parse jsonnet output as JSON: {}", e)
        ))?;

    let topology_graph = &data["topology_graph"];

    // ノードデータを変換
    let mut nodes = std::collections::HashMap::new();
    if let Some(nodes_obj) = topology_graph["nodes"].as_object() {
        for (node_name, node_data) in nodes_obj {
            let node = kotoba::topology::Node {
                name: node_data["name"].as_str().unwrap_or(node_name).to_string(),
                path: node_data["path"].as_str().unwrap_or("").to_string(),
                node_type: node_data["node_type"].as_str().unwrap_or("").to_string(),
                description: node_data["description"].as_str().unwrap_or("").to_string(),
                dependencies: node_data["dependencies"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect(),
                provides: node_data["provides"]
                    .as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.to_string())
                    .collect(),
                status: node_data["status"].as_str().unwrap_or("").to_string(),
                build_order: node_data["build_order"].as_u64().unwrap_or(0) as u32,
            };
            nodes.insert(node_name.clone(), node);
        }
    }

    // エッジデータを変換
    let mut edges = vec![];
    if let Some(edges_arr) = topology_graph["edges"].as_array() {
        for edge_data in edges_arr {
            if let (Some(from), Some(to)) = (
                edge_data["from"].as_str(),
                edge_data["to"].as_str()
            ) {
                edges.push(kotoba::topology::Edge {
                    from: from.to_string(),
                    to: to.to_string(),
                });
            }
        }
    }

    // トポロジカル順序を取得
    let topological_order = topology_graph["topological_order"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    let reverse_topological_order = topology_graph["reverse_topological_order"]
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect();

    Ok(TopologyGraph {
        nodes,
        edges,
        topological_order,
        reverse_topological_order,
    })
}

/// dag.jsonnetファイルが存在するかチェック
fn check_dag_file_exists() -> bool {
    Path::new("dag.jsonnet").exists()
}

/// jsonnetがインストールされているかチェック
fn check_jsonnet_installed() -> bool {
    Command::new("jsonnet")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn test_topology_validation_from_jsonnet() {
    // 前提条件のチェック
    assert!(check_dag_file_exists(), "dag.jsonnet file must exist");
    assert!(check_jsonnet_installed(), "jsonnet must be installed");

    // トポロジーデータを生成
    let topology_graph = generate_topology_data()
        .expect("Failed to generate topology data from jsonnet");

    // 基本的な構造チェック
    assert!(!topology_graph.nodes.is_empty(), "Topology must have at least one node");
    assert!(topology_graph.edges.len() >= topology_graph.nodes.len() - 1,
        "Number of edges should be at least nodes-1 for a connected graph");

    // トポロジー検証を実行
    let validator = TopologyValidator::new(topology_graph);
    let result = validator.validate_all()
        .expect("Topology validation failed");

    // 検証結果を表示
    println!("{}", result.format());

    // トポロジーが有効であることを確認
    assert!(result.is_valid, "Topology validation must pass");

    // 各チェックが成功していることを確認
    for check in &result.checks {
        assert!(check.is_valid, "Check '{}' failed: {:?}", check.name, check.errors);
    }
}

#[test]
fn test_topology_statistics() {
    // 前提条件のチェック
    if !check_dag_file_exists() || !check_jsonnet_installed() {
        return; // スキップ
    }

    let topology_graph = generate_topology_data()
        .expect("Failed to generate topology data from jsonnet");

    // 統計情報のチェック
    let node_count = topology_graph.nodes.len();
    let edge_count = topology_graph.edges.len();

    println!("Topology Statistics:");
    println!("  Nodes: {}", node_count);
    println!("  Edges: {}", edge_count);
    println!("  Topological order length: {}", topology_graph.topological_order.len());
    println!("  Reverse topological order length: {}", topology_graph.reverse_topological_order.len());

    // 基本的な不変条件
    assert_eq!(topology_graph.topological_order.len(), node_count,
        "Topological order must include all nodes");
    assert_eq!(topology_graph.reverse_topological_order.len(), node_count,
        "Reverse topological order must include all nodes");

    // 各ノードのビルド順序を確認
    let mut build_orders: Vec<u32> = topology_graph.nodes.values()
        .map(|n| n.build_order)
        .collect();
    build_orders.sort();
    build_orders.dedup();

    // ビルド順序が1から始まり、連続していることを確認
    if !build_orders.is_empty() {
        assert_eq!(build_orders[0], 1, "Build order should start from 1");
        for i in 1..build_orders.len() {
            assert!(build_orders[i] == build_orders[i-1] + 1,
                "Build orders should be consecutive");
        }
    }
}

#[test]
fn test_dependency_graph_properties() {
    // 前提条件のチェック
    if !check_dag_file_exists() || !check_jsonnet_installed() {
        return; // スキップ
    }

    let topology_graph = generate_topology_data()
        .expect("Failed to generate topology data from jsonnet");

    // 依存関係グラフのプロパティをチェック
    let mut in_degrees = std::collections::HashMap::new();
    let mut out_degrees = std::collections::HashMap::new();

    for node_name in topology_graph.nodes.keys() {
        in_degrees.insert(node_name.clone(), 0);
        out_degrees.insert(node_name.clone(), 0);
    }

    for edge in &topology_graph.edges {
        *in_degrees.get_mut(&edge.to).unwrap() += 1;
        *out_degrees.get_mut(&edge.from).unwrap() += 1;
    }

    // 各ノードの入次数と出次数を表示
    println!("Node degrees:");
    for (node_name, node) in &topology_graph.nodes {
        let in_degree = in_degrees[node_name];
        let out_degree = out_degrees[node_name];
        println!("  {}: in={}, out={}", node_name, in_degree, out_degree);

        // ノードの依存関係とエッジの整合性を確認
        assert_eq!(node.dependencies.len(), out_degree as usize,
            "Node {} dependency count mismatch", node_name);
    }

    // グラフが連結であることを確認（弱連結）
    let mut visited = std::collections::HashSet::new();
    let mut stack = vec![topology_graph.nodes.keys().next().unwrap().clone()];

    while let Some(node) = stack.pop() {
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node.clone());

        // 入辺をたどる
        for edge in &topology_graph.edges {
            if edge.to == node && !visited.contains(&edge.from) {
                stack.push(edge.from.clone());
            }
        }

        // 出辺をたどる
        for edge in &topology_graph.edges {
            if edge.from == node && !visited.contains(&edge.to) {
                stack.push(edge.to.clone());
            }
        }
    }

    assert_eq!(visited.len(), topology_graph.nodes.len(),
        "Graph must be connected (weakly connected component should include all nodes)");
}

#[test]
fn test_jsonnet_output_format() {
    // 前提条件のチェック
    if !check_dag_file_exists() || !check_jsonnet_installed() {
        return; // スキップ
    }

    // jsonnetの出力を直接確認
    let output = Command::new("jsonnet")
        .arg("validate_topology.jsonnet")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("Failed to execute jsonnet");

    let json_str = String::from_utf8(output.stdout)
        .expect("Invalid UTF-8 output");

    // JSONとしてパースできることを確認
    let data: serde_json::Value = serde_json::from_str(&json_str)
        .expect("Output is not valid JSON");

    // 必要なフィールドが存在することを確認
    assert!(data["topology_graph"].is_object(), "topology_graph field missing");
    assert!(data["validation_metadata"].is_object(), "validation_metadata field missing");
    assert!(data["validation_rules"].is_object(), "validation_rules field missing");

    // topology_graphの構造を確認
    let topology_graph = &data["topology_graph"];
    assert!(topology_graph["nodes"].is_object(), "topology_graph.nodes missing");
    assert!(topology_graph["edges"].is_array(), "topology_graph.edges missing");
    assert!(topology_graph["topological_order"].is_array(), "topology_graph.topological_order missing");
    assert!(topology_graph["reverse_topological_order"].is_array(), "topology_graph.reverse_topological_order missing");

    println!("✓ JSON output format is valid");
}

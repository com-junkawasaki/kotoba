//! グラフアルゴリズムのデモ

use kotoba::graph::*;
use kotoba::types::*;
use std::collections::HashMap;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Kotoba Graph Algorithms Demo ===\n");

    // サンプルグラフの作成（ソーシャルネットワーク）
    let graph = create_social_network_graph();
    println!("Created social network graph with {} vertices and {} edges",
             graph.vertices.len(), graph.edges.len());

    // 1. 最短経路アルゴリズム
    demonstrate_shortest_paths(&graph)?;

    // 2. 中央性指標
    demonstrate_centrality_measures(&graph)?;

    // 3. パターンマッチング
    demonstrate_pattern_matching(&graph)?;

    println!("\n=== Demo completed successfully! ===");
    Ok(())
}

/// ソーシャルネットワークグラフの作成
fn create_social_network_graph() -> Graph {
    let mut graph = Graph::empty();

    // ユーザーを追加
    let alice = graph.add_vertex(VertexData {
        id: VertexId::new("alice").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::from([
            ("name".to_string(), Value::String("Alice".to_string())),
            ("age".to_string(), Value::Int(25)),
        ]),
    });

    let bob = graph.add_vertex(VertexData {
        id: VertexId::new("bob").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::from([
            ("name".to_string(), Value::String("Bob".to_string())),
            ("age".to_string(), Value::Int(30)),
        ]),
    });

    let charlie = graph.add_vertex(VertexData {
        id: VertexId::new("charlie").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::from([
            ("name".to_string(), Value::String("Charlie".to_string())),
            ("age".to_string(), Value::Int(28)),
        ]),
    });

    let dave = graph.add_vertex(VertexData {
        id: VertexId::new("dave").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::from([
            ("name".to_string(), Value::String("Dave".to_string())),
            ("age".to_string(), Value::Int(35)),
        ]),
    });

    let eve = graph.add_vertex(VertexData {
        id: VertexId::new("eve").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::from([
            ("name".to_string(), Value::String("Eve".to_string())),
            ("age".to_string(), Value::Int(27)),
        ]),
    });

    // 友人関係を追加
    graph.add_edge(EdgeData {
        id: EdgeId::new("alice_bob").unwrap(),
        src: alice,
        dst: bob,
        label: "FOLLOWS".to_string(),
        props: HashMap::from([
            ("since".to_string(), Value::Int(2020)),
        ]),
    });

    graph.add_edge(EdgeData {
        id: EdgeId::new("bob_charlie").unwrap(),
        src: bob,
        dst: charlie,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    graph.add_edge(EdgeData {
        id: EdgeId::new("charlie_dave").unwrap(),
        src: charlie,
        dst: dave,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    graph.add_edge(EdgeData {
        id: EdgeId::new("dave_eve").unwrap(),
        src: dave,
        dst: eve,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    graph.add_edge(EdgeData {
        id: EdgeId::new("alice_charlie").unwrap(),
        src: alice,
        dst: charlie,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    graph.add_edge(EdgeData {
        id: EdgeId::new("bob_dave").unwrap(),
        src: bob,
        dst: dave,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    graph
}

/// 最短経路アルゴリズムのデモ
fn demonstrate_shortest_paths(graph: &Graph) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Shortest Path Algorithms:");
    println!("----------------------------");

    let alice = VertexId::new("alice").unwrap();

    // Dijkstraアルゴリズム
    let dijkstra_result = GraphAlgorithms::shortest_path_dijkstra(graph, alice, |_| 1.0)?;

    println!("Dijkstra from Alice:");
    for (vertex_id, &distance) in &dijkstra_result.distances {
        if distance < f64::INFINITY && *vertex_id != alice {
            println!("  {} -> {}: distance = {:.0}", alice.as_str(), vertex_id.as_str(), distance);
        }
    }

    // Bellman-Fordアルゴリズム（負の重みなしだがデモ用）
    let bellman_result = GraphAlgorithms::shortest_path_bellman_ford(graph, alice, |_| 1.0)?;

    println!("
Bellman-Ford from Alice (same result as Dijkstra for positive weights):");
    for (vertex_id, &distance) in &bellman_result.distances {
        if distance < f64::INFINITY && *vertex_id != alice {
            println!("  {} -> {}: distance = {:.0}", alice.as_str(), vertex_id.as_str(), distance);
        }
    }

    // Floyd-Warshall（全頂点間最短経路）
    let all_pairs = GraphAlgorithms::all_pairs_shortest_paths(graph, |_| 1.0)?;

    println!("
Floyd-Warshall (all-pairs shortest paths):");
    let vertices: Vec<_> = graph.vertices.keys().collect();
    for &u in &vertices {
        for &v in &vertices {
            if let Some(&dist) = all_pairs.get(&(u.clone(), v.clone())) {
                if dist < f64::INFINITY && u != v {
                    println!("  {} -> {}: {:.0}", u.as_str(), v.as_str(), dist);
                }
            }
        }
    }

    println!();
    Ok(())
}

/// 中央性指標のデモ
fn demonstrate_centrality_measures(graph: &Graph) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Centrality Measures:");
    println!("----------------------");

    // 次数中央性
    let degree_result = GraphAlgorithms::degree_centrality(graph, false);
    println!("Degree Centrality:");
    for (vertex_id, &score) in &degree_result.scores {
        println!("  {}: {:.1}", vertex_id.as_str(), score);
    }

    // 媒介中央性
    let betweenness_result = GraphAlgorithms::betweenness_centrality(graph, false);
    println!("
Betweenness Centrality:");
    for (vertex_id, &score) in &betweenness_result.scores {
        println!("  {}: {:.3}", vertex_id.as_str(), score);
    }

    // 近接中央性
    let closeness_result = GraphAlgorithms::closeness_centrality(graph, false);
    println!("
Closeness Centrality:");
    for (vertex_id, &score) in &closeness_result.scores {
        println!("  {}: {:.3}", vertex_id.as_str(), score);
    }

    // PageRank
    let pagerank_result = GraphAlgorithms::pagerank(graph, 0.85, 20, 1e-6);
    println!("
PageRank (damping=0.85, iterations=20):");
    for (vertex_id, &score) in &pagerank_result.scores {
        println!("  {}: {:.4}", vertex_id.as_str(), score);
    }

    println!();
    Ok(())
}

/// パターンマッチングのデモ
fn demonstrate_pattern_matching(graph: &Graph) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 Pattern Matching:");
    println!("-------------------");

    // パターングラフの作成（三角形構造）
    let mut pattern = Graph::empty();

    let p1 = pattern.add_vertex(VertexData {
        id: VertexId::new("p1").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::new(),
    });

    let p2 = pattern.add_vertex(VertexData {
        id: VertexId::new("p2").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::new(),
    });

    let p3 = pattern.add_vertex(VertexData {
        id: VertexId::new("p3").unwrap(),
        labels: vec!["Person".to_string()],
        props: HashMap::new(),
    });

    pattern.add_edge(EdgeData {
        id: EdgeId::new("pe1").unwrap(),
        src: p1,
        dst: p2,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    pattern.add_edge(EdgeData {
        id: EdgeId::new("pe2").unwrap(),
        src: p2,
        dst: p3,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    pattern.add_edge(EdgeData {
        id: EdgeId::new("pe3").unwrap(),
        src: p1,
        dst: p3,
        label: "FOLLOWS".to_string(),
        props: HashMap::new(),
    });

    println!("Pattern graph: triangle with 3 vertices and 3 edges");

    // 部分グラフ同型マッチング
    let match_result = GraphAlgorithms::subgraph_isomorphism(&pattern, graph);

    println!("Subgraph isomorphism matching found {} potential matches", match_result.count);

    if match_result.count > 0 {
        println!("Sample mapping:");
        if let Some(mapping) = match_result.mappings.first() {
            for (pattern_vertex, &data_vertex) in &mapping.vertex_map {
                println!("  {} -> {}", pattern_vertex.as_str(), data_vertex.as_str());
            }
        }
    } else {
        println!("No matches found - the data graph doesn't contain the triangle pattern");
    }

    println!();
    Ok(())
}

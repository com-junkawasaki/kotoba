//! シンプルな計算テストベンチマーク

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

/// 基本的なグラフ構造のシミュレーション
#[derive(Debug, Clone)]
struct SimpleGraph {
    vertices: Vec<Vertex>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
struct Vertex {
    id: String,
    labels: Vec<String>,
    properties: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
struct Edge {
    id: String,
    source: String,
    target: String,
    label: String,
    properties: HashMap<String, Value>,
}

#[derive(Debug, Clone)]
enum Value {
    Null,
    Bool(bool),
    Int(i64),
    String(String),
}

/// テスト用グラフ生成
fn create_test_graph(size: usize) -> SimpleGraph {
    let mut vertices = Vec::new();
    let mut edges = Vec::new();

    // 頂点の作成
    for i in 0..size {
        let vertex = Vertex {
            id: format!("v{}", i),
            labels: vec![format!("Node{}", i % 10)],
            properties: HashMap::from([
                ("name".to_string(), Value::String(format!("node_{}", i))),
                ("value".to_string(), Value::Int(i as i64)),
            ]),
        };
        vertices.push(vertex);
    }

    // エッジの作成
    for i in 0..size - 1 {
        let edge = Edge {
            id: format!("e{}", i),
            source: format!("v{}", i),
            target: format!("v{}", i + 1),
            label: "CONNECTS".to_string(),
            properties: HashMap::new(),
        };
        edges.push(edge);
    }

    SimpleGraph { vertices, edges }
}

/// 頂点検索ベンチマーク
fn benchmark_vertex_lookup(graph: &SimpleGraph, iterations: usize) {
    println!("=== 頂点検索ベンチマーク ===");
    let start = Instant::now();

    for i in 0..iterations {
        let target_id = format!("v{}", i % graph.vertices.len());
        let _result = graph.vertices.iter().find(|v| v.id == target_id);
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均検索時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
}

/// エッジ検索ベンチマーク
fn benchmark_edge_lookup(graph: &SimpleGraph, iterations: usize) {
    println!("=== エッジ検索ベンチマーク ===");
    let start = Instant::now();

    for i in 0..iterations {
        let target_id = format!("e{}", i % graph.edges.len());
        let _result = graph.edges.iter().find(|e| e.id == target_id);
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均検索時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
}

/// 次数計算ベンチマーク
fn benchmark_degree_calculation(graph: &SimpleGraph, iterations: usize) {
    println!("=== 次数計算ベンチマーク ===");
    let start = Instant::now();

    for i in 0..iterations {
        let vertex_id = format!("v{}", i % graph.vertices.len());
        let degree = graph.edges.iter()
            .filter(|e| e.source == vertex_id || e.target == vertex_id)
            .count();
        let _ = degree;
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均計算時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
}

/// グラフ統計ベンチマーク
fn benchmark_graph_statistics(graph: &SimpleGraph, iterations: usize) {
    println!("=== グラフ統計ベンチマーク ===");
    let start = Instant::now();

    for _ in 0..iterations {
        let vertex_count = graph.vertices.len();
        let edge_count = graph.edges.len();
        let _stats = (vertex_count, edge_count);
    }

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均計算時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
}

/// メモリ使用量推定
fn estimate_memory_usage(graph: &SimpleGraph) {
    println!("=== メモリ使用量推定 ===");
    let vertex_size = std::mem::size_of::<Vertex>();
    let edge_size = std::mem::size_of::<Edge>();

    let total_vertices = graph.vertices.len();
    let total_edges = graph.edges.len();

    let vertices_memory = total_vertices * vertex_size;
    let edges_memory = total_edges * edge_size;
    let total_memory = vertices_memory + edges_memory;

    println!("頂点数: {}", total_vertices);
    println!("エッジ数: {}", total_edges);
    println!("1頂点のサイズ: {} bytes", vertex_size);
    println!("1エッジのサイズ: {} bytes", edge_size);
    println!("頂点合計メモリ: {:.2} KB", vertices_memory as f64 / 1024.0);
    println!("エッジ合計メモリ: {:.2} KB", edges_memory as f64 / 1024.0);
    println!("総メモリ使用量: {:.2} KB", total_memory as f64 / 1024.0);
}

/// パフォーマンステストの実行
fn run_performance_tests() {
    println!("🚀 Kotoba 計算テスト開始");
    println!("=================================");

    // テストデータサイズ
    let sizes = [100, 1000, 10000];

    for size in sizes {
        println!("\n📊 データサイズ: {} 頂点", size);
        println!("---------------------------------");

        let graph = create_test_graph(size);
        let iterations = 10000;

        benchmark_vertex_lookup(&graph, iterations);
        benchmark_edge_lookup(&graph, iterations);
        benchmark_degree_calculation(&graph, iterations);
        benchmark_graph_statistics(&graph, iterations);

        if size <= 1000 {
            estimate_memory_usage(&graph);
        }
    }

    println!("\n✅ 計算テスト完了");
}

/// アルゴリズムテスト
fn run_algorithm_tests() {
    println!("\n🔍 アルゴリズムテスト");
    println!("=================================");

    let graph = create_test_graph(100);
    let start_vertex = "v0".to_string();

    // 簡易的なBFS（幅優先探索）
    println!("=== BFS テスト ===");
    let start = Instant::now();

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start_vertex.clone());
    visited.insert(start_vertex.clone());

    let mut bfs_count = 0;
    while let Some(current) = queue.pop_front() {
        bfs_count += 1;

        // 隣接頂点を探索
        for edge in &graph.edges {
            let neighbor = if edge.source == current {
                edge.target.clone()
            } else if edge.target == current {
                edge.source.clone()
            } else {
                continue;
            };

            if visited.insert(neighbor.clone()) {
                queue.push_back(neighbor);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("BFS 探索時間: {:.6} s", elapsed.as_secs_f64());
    println!("訪問した頂点数: {}", bfs_count);

    // 簡易的なDFS（深さ優先探索）
    println!("\n=== DFS テスト ===");
    let start = Instant::now();

    let mut visited = HashSet::new();
    let mut stack = Vec::new();
    let start_vertex_clone = start_vertex.clone();
    stack.push(start_vertex_clone.clone());
    visited.insert(start_vertex_clone);

    let mut dfs_count = 0;
    while let Some(current) = stack.pop() {
        dfs_count += 1;

        // 隣接頂点を探索
        for edge in &graph.edges {
            let neighbor = if edge.source == current {
                edge.target.clone()
            } else if edge.target == current {
                edge.source.clone()
            } else {
                continue;
            };

            if visited.insert(neighbor.clone()) {
                stack.push(neighbor);
            }
        }
    }

    let elapsed = start.elapsed();
    println!("DFS 探索時間: {:.6} s", elapsed.as_secs_f64());
    println!("訪問した頂点数: {}", dfs_count);
}

fn main() {
    println!("🎯 Kotoba ベンチマークテスト");
    println!("================================");

    run_performance_tests();
    run_algorithm_tests();

    println!("\n🎉 全てのテストが完了しました！");
}

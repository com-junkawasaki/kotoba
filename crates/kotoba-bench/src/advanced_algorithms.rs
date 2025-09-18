//! 高度グラフアルゴリズムベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;

/// 高度アルゴリズム用グラフ構造
#[derive(Debug, Clone)]
struct AdvancedGraph {
    vertices: Vec<AdvancedVertex>,
    edges: Vec<AdvancedEdge>,
    adjacency_list: Vec<Vec<(usize, f64)>>, // (neighbor, weight)
}

#[derive(Debug, Clone)]
struct AdvancedVertex {
    id: usize,
    degree: usize,
    out_degree: usize,
    in_degree: usize,
}

#[derive(Debug, Clone)]
struct AdvancedEdge {
    id: usize,
    source: usize,
    target: usize,
    weight: f64,
}

/// テストグラフ生成（高度アルゴリズム用）
fn create_advanced_graph(num_vertices: usize, num_edges: usize) -> AdvancedGraph {
    let mut vertices = Vec::new();
    let mut edges = Vec::new();
    let mut adjacency_list = vec![Vec::new(); num_vertices];

    // 頂点の初期化
    for i in 0..num_vertices {
        vertices.push(AdvancedVertex {
            id: i,
            degree: 0,
            out_degree: 0,
            in_degree: 0,
        });
    }

    // エッジの追加
    for i in 0..num_edges {
        let src = (i * 7) % num_vertices;
        let dst = (i * 13 + 1) % num_vertices;

        if src != dst {
            let weight = 1.0 + (i % 10) as f64 * 0.1; // 重み付きエッジ

            let edge = AdvancedEdge {
                id: i,
                source: src,
                target: dst,
                weight,
            };

            edges.push(edge);
            adjacency_list[src].push((dst, weight));

            vertices[src].out_degree += 1;
            vertices[dst].in_degree += 1;
            vertices[src].degree += 1;
            vertices[dst].degree += 1;
        }
    }

    AdvancedGraph {
        vertices,
        edges,
        adjacency_list,
    }
}

/// PageRankアルゴリズム実装
fn pagerank(graph: &AdvancedGraph, damping: f64, iterations: usize, tolerance: f64) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut ranks = vec![1.0 / n as f64; n];
    let mut new_ranks = vec![0.0; n];

    for iter in 0..iterations {
        let mut max_diff = 0.0;

        // PageRank計算
        for i in 0..n {
            let mut incoming_rank = 0.0;

            // 入ってくるエッジからの寄与を計算
            for j in 0..n {
                if let Some(edge_idx) = graph.adjacency_list[j].iter().position(|&(target, _)| target == i) {
                    let (_, weight) = graph.adjacency_list[j][edge_idx];
                    incoming_rank += ranks[j] * weight / graph.vertices[j].out_degree as f64;
                }
            }

            new_ranks[i] = (1.0 - damping) / n as f64 + damping * incoming_rank;

            max_diff = max_diff.max((new_ranks[i] - ranks[i]).abs());
        }

        // ランクを更新
        ranks.copy_from_slice(&new_ranks);

        // 収束判定
        if max_diff < tolerance {
            println!("PageRank converged at iteration {}", iter + 1);
            break;
        }
    }

    ranks
}

/// PageRankベンチマーク
fn bench_pagerank(c: &mut Criterion) {
    let sizes = [500, 1000, 2000];

    for size in sizes {
        let graph = create_advanced_graph(size, size * 3);

        c.bench_function(&format!("pagerank_{}_vertices", size), |b| {
            b.iter(|| {
                let ranks = pagerank(&graph, 0.85, 20, 1e-6);
                black_box(ranks.iter().sum::<f64>());
            });
        });
    }
}

/// 媒介中心性（Betweenness Centrality）アルゴリズム
fn betweenness_centrality(graph: &AdvancedGraph) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut centrality = vec![0.0; n];

    for s in 0..n {
        // BFSで各頂点からの距離と経路数を計算
        let mut distances = vec![-1i64; n];
        let mut num_paths = vec![0i64; n];
        let mut predecessors = vec![Vec::new(); n];

        distances[s] = 0;
        num_paths[s] = 1;

        let mut queue = VecDeque::new();
        queue.push_back(s);

        while let Some(current) = queue.pop_front() {
            for &(neighbor, _) in &graph.adjacency_list[current] {
                if distances[neighbor] == -1 {
                    // 初めて訪れる頂点
                    distances[neighbor] = distances[current] + 1;
                    queue.push_back(neighbor);
                }

                if distances[neighbor] == distances[current] + 1 {
                    // 最短経路の数
                    num_paths[neighbor] += num_paths[current];
                    predecessors[neighbor].push(current);
                }
            }
        }

        // 依存関係を計算
        let mut dependencies = vec![0.0; n];

        // 逆順で依存関係を計算
        for i in (0..n).rev() {
            for &predecessor in &predecessors[i] {
                let ratio = (num_paths[predecessor] as f64 / num_paths[i] as f64) * (1.0 + dependencies[i]);
                dependencies[predecessor] += ratio;
            }
        }

        // 媒介中心性を更新
        for i in 0..n {
            centrality[i] += dependencies[i];
        }
    }

    centrality
}

/// 媒介中心性ベンチマーク
fn bench_betweenness_centrality(c: &mut Criterion) {
    let sizes = [100, 200, 300];

    for size in sizes {
        let graph = create_advanced_graph(size, size * 2);

        c.bench_function(&format!("betweenness_centrality_{}_vertices", size), |b| {
            b.iter(|| {
                let centrality = betweenness_centrality(&graph);
                black_box(centrality.iter().sum::<f64>());
            });
        });
    }
}

/// 次数中心性（Degree Centrality）
fn degree_centrality(graph: &AdvancedGraph, normalized: bool) -> Vec<f64> {
    let n = graph.vertices.len();
    let max_possible_degree = (n - 1) as f64;

    graph.vertices.iter()
        .map(|v| {
            let centrality = v.degree as f64;
            if normalized {
                centrality / max_possible_degree
            } else {
                centrality
            }
        })
        .collect()
}

/// 次数中心性ベンチマーク
fn bench_degree_centrality(c: &mut Criterion) {
    let graph = create_advanced_graph(1000, 5000);

    c.bench_function("degree_centrality", |b| {
        b.iter(|| {
            let centrality = degree_centrality(&graph, true);
            black_box(centrality.iter().sum::<f64>());
        });
    });
}

/// 近接中心性（Closeness Centrality）
fn closeness_centrality(graph: &AdvancedGraph, normalized: bool) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut centrality = vec![0.0; n];

    for i in 0..n {
        // BFSで各頂点からの距離を計算
        let mut distances = vec![-1i64; n];
        let mut queue = VecDeque::new();

        distances[i] = 0;
        queue.push_back(i);

        while let Some(current) = queue.pop_front() {
            for &(neighbor, _) in &graph.adjacency_list[current] {
                if distances[neighbor] == -1 {
                    distances[neighbor] = distances[current] + 1;
                    queue.push_back(neighbor);
                }
            }
        }

        // 距離の合計を計算
        let total_distance: i64 = distances.iter().filter(|&&d| d != -1).sum();

        if total_distance > 0 {
            centrality[i] = 1.0 / total_distance as f64;
            if normalized {
                centrality[i] *= (n - 1) as f64;
            }
        }
    }

    centrality
}

/// 近接中心性ベンチマーク
fn bench_closeness_centrality(c: &mut Criterion) {
    let sizes = [200, 300, 500];

    for size in sizes {
        let graph = create_advanced_graph(size, size * 2);

        c.bench_function(&format!("closeness_centrality_{}_vertices", size), |b| {
            b.iter(|| {
                let centrality = closeness_centrality(&graph, true);
                black_box(centrality.iter().sum::<f64>());
            });
        });
    }
}

/// 固有ベクトル中心性（Eigenvector Centrality）の近似解法
fn eigenvector_centrality(graph: &AdvancedGraph, iterations: usize, tolerance: f64) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut centrality = vec![1.0; n];
    let mut new_centrality = vec![0.0; n];

    for iter in 0..iterations {
        let mut max_diff = 0.0;

        // 固有ベクトルを更新
        for i in 0..n {
            new_centrality[i] = 0.0;
            for &(neighbor, weight) in &graph.adjacency_list[i] {
                new_centrality[i] += centrality[neighbor] * weight;
            }

            max_diff = max_diff.max((new_centrality[i] - centrality[i]).abs());
        }

        // 正規化
        let norm: f64 = new_centrality.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for i in 0..n {
                new_centrality[i] /= norm;
            }
        }

        centrality.copy_from_slice(&new_centrality);

        // 収束判定
        if max_diff < tolerance {
            println!("Eigenvector Centrality converged at iteration {}", iter + 1);
            break;
        }
    }

    centrality
}

/// 固有ベクトル中心性ベンチマーク
fn bench_eigenvector_centrality(c: &mut Criterion) {
    let graph = create_advanced_graph(500, 2000);

    c.bench_function("eigenvector_centrality", |b| {
        b.iter(|| {
            let centrality = eigenvector_centrality(&graph, 50, 1e-6);
            black_box(centrality.iter().sum::<f64>());
        });
    });
}

/// Dijkstraアルゴリズム（単一始点最短経路）
fn dijkstra(graph: &AdvancedGraph, source: usize) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut distances = vec![f64::INFINITY; n];
    let mut visited = vec![false; n];
    let mut priority_queue = Vec::new();

    distances[source] = 0.0;
    priority_queue.push((0.0, source));

    while let Some((dist, current)) = priority_queue.pop() {
        if visited[current] {
            continue;
        }
        visited[current] = true;

        for &(neighbor, weight) in &graph.adjacency_list[current] {
            if !visited[neighbor] {
                let new_dist = dist + weight;
                if new_dist < distances[neighbor] {
                    distances[neighbor] = new_dist;
                    priority_queue.push((new_dist, neighbor));
                    // 簡易ソート（実際の実装ではヒープを使用）
                    priority_queue.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                }
            }
        }
    }

    distances
}

/// Dijkstraアルゴリズムベンチマーク
fn bench_dijkstra(c: &mut Criterion) {
    let graph = create_advanced_graph(1000, 5000);

    c.bench_function("dijkstra", |b| {
        b.iter(|| {
            let distances = dijkstra(&graph, 0);
            black_box(distances.iter().filter(|&&d| d != f64::INFINITY).sum::<f64>());
        });
    });
}

/// Floyd-Warshallアルゴリズム（全点間最短経路）
fn floyd_warshall(graph: &AdvancedGraph) -> Vec<Vec<f64>> {
    let n = graph.vertices.len();
    let mut dist = vec![vec![f64::INFINITY; n]; n];

    // 初期化
    for i in 0..n {
        dist[i][i] = 0.0;
    }

    for edge in &graph.edges {
        dist[edge.source][edge.target] = edge.weight;
    }

    // Floyd-Warshallアルゴリズム
    for k in 0..n {
        for i in 0..n {
            for j in 0..n {
                if dist[i][k] != f64::INFINITY && dist[k][j] != f64::INFINITY {
                    let new_dist = dist[i][k] + dist[k][j];
                    if new_dist < dist[i][j] {
                        dist[i][j] = new_dist;
                    }
                }
            }
        }
    }

    dist
}

/// Floyd-Warshallアルゴリズムベンチマーク
fn bench_floyd_warshall(c: &mut Criterion) {
    let sizes = [20, 30, 40];

    for size in sizes {
        let graph = create_advanced_graph(size, size * 2);

        c.bench_function(&format!("floyd_warshall_{}_vertices", size), |b| {
            b.iter(|| {
                let distances = floyd_warshall(&graph);
                black_box(distances.iter().flatten().filter(|&&d| d != f64::INFINITY).sum::<f64>());
            });
        });
    }
}

/// 連結成分分析
fn connected_components(graph: &AdvancedGraph) -> Vec<Vec<usize>> {
    let n = graph.vertices.len();
    let mut visited = vec![false; n];
    let mut components = Vec::new();

    for i in 0..n {
        if !visited[i] {
            let mut component = Vec::new();
            let mut stack = vec![i];

            while let Some(current) = stack.pop() {
                if !visited[current] {
                    visited[current] = true;
                    component.push(current);

                    // 隣接頂点を探索
                    for &(neighbor, _) in &graph.adjacency_list[current] {
                        if !visited[neighbor] {
                            stack.push(neighbor);
                        }
                    }
                }
            }

            components.push(component);
        }
    }

    components
}

/// 連結成分分析ベンチマーク
fn bench_connected_components(c: &mut Criterion) {
    let graph = create_advanced_graph(1000, 3000);

    c.bench_function("connected_components", |b| {
        b.iter(|| {
            let components = connected_components(&graph);
            black_box(components.len());
        });
    });
}

/// アルゴリズム比較ベンチマーク
fn bench_algorithm_comparison(c: &mut Criterion) {
    let graph = create_advanced_graph(500, 2000);

    let mut group = c.benchmark_group("algorithm_comparison");

    group.bench_function("degree_centrality", |b| {
        b.iter(|| {
            let centrality = degree_centrality(&graph, true);
            black_box(centrality.iter().sum::<f64>());
        });
    });

    group.bench_function("closeness_centrality", |b| {
        b.iter(|| {
            let centrality = closeness_centrality(&graph, true);
            black_box(centrality.iter().sum::<f64>());
        });
    });

    group.bench_function("eigenvector_centrality", |b| {
        b.iter(|| {
            let centrality = eigenvector_centrality(&graph, 30, 1e-4);
            black_box(centrality.iter().sum::<f64>());
        });
    });

    group.bench_function("pagerank", |b| {
        b.iter(|| {
            let ranks = pagerank(&graph, 0.85, 10, 1e-4);
            black_box(ranks.iter().sum::<f64>());
        });
    });

    group.finish();
}

criterion_group! {
    benches,
    bench_pagerank,
    bench_betweenness_centrality,
    bench_degree_centrality,
    bench_closeness_centrality,
    bench_eigenvector_centrality,
    bench_dijkstra,
    bench_floyd_warshall,
    bench_connected_components,
    bench_algorithm_comparison
}

criterion_main!(benches);

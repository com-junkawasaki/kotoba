//! 並列処理グラフアルゴリズムベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::Instant;

/// 並列処理用グラフ構造
#[derive(Debug, Clone)]
struct ParallelGraph {
    vertices: Vec<ParallelVertex>,
    edges: Vec<ParallelEdge>,
}

#[derive(Debug, Clone)]
struct ParallelVertex {
    id: usize,
    degree: usize,
    out_neighbors: Vec<usize>,
    in_neighbors: Vec<usize>,
}

#[derive(Debug, Clone)]
struct ParallelEdge {
    id: usize,
    source: usize,
    target: usize,
    weight: f64,
}

/// 大規模テストグラフ生成（並列処理用）
fn create_parallel_graph(num_vertices: usize, num_edges: usize) -> ParallelGraph {
    let mut vertices = Vec::new();
    let mut edges = Vec::new();

    // 頂点の初期化
    for i in 0..num_vertices {
        vertices.push(ParallelVertex {
            id: i,
            degree: 0,
            out_neighbors: Vec::new(),
            in_neighbors: Vec::new(),
        });
    }

    // エッジの追加
    let mut rng = rand::thread_rng();
    for i in 0..num_edges {
        let src = (i * 7) % num_vertices;
        let dst = (i * 13 + 1) % num_vertices;

        if src != dst {
            let edge = ParallelEdge {
                id: i,
                source: src,
                target: dst,
                weight: 1.0,
            };

            edges.push(edge);
            vertices[src].out_neighbors.push(dst);
            vertices[dst].in_neighbors.push(src);
            vertices[src].degree += 1;
            vertices[dst].degree += 1;
        }
    }

    ParallelGraph { vertices, edges }
}

/// 並列頂点検索ベンチマーク
fn bench_parallel_vertex_search(c: &mut Criterion) {
    let graph = create_parallel_graph(10000, 50000);

    c.bench_function("parallel_vertex_search", |b| {
        b.iter(|| {
            let result: Vec<_> = graph.vertices.par_iter()
                .filter(|v| v.degree > 5)
                .map(|v| v.id)
                .collect();
            black_box(result);
        });
    });
}

/// 並列エッジ処理ベンチマーク
fn bench_parallel_edge_processing(c: &mut Criterion) {
    let graph = create_parallel_graph(10000, 50000);

    c.bench_function("parallel_edge_processing", |b| {
        b.iter(|| {
            let result: Vec<_> = graph.edges.par_iter()
                .filter(|e| e.weight > 0.5)
                .map(|e| (e.source, e.target))
                .collect();
            black_box(result);
        });
    });
}

/// 並列次数計算ベンチマーク
fn bench_parallel_degree_calculation(c: &mut Criterion) {
    let graph = create_parallel_graph(10000, 50000);

    c.bench_function("parallel_degree_calculation", |b| {
        b.iter(|| {
            let max_degree = graph.vertices.par_iter()
                .map(|v| v.degree)
                .max()
                .unwrap_or(0);
            black_box(max_degree);
        });
    });
}

/// 並列グラフ統計ベンチマーク
fn bench_parallel_statistics(c: &mut Criterion) {
    let graph = create_parallel_graph(10000, 50000);

    c.bench_function("parallel_statistics", |b| {
        b.iter(|| {
            let stats = graph.vertices.par_iter()
                .map(|v| (v.degree, v.out_neighbors.len(), v.in_neighbors.len()))
                .reduce(|| (0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
            black_box(stats);
        });
    });
}

/// 並列BFS（幅優先探索）
fn parallel_bfs(graph: &ParallelGraph, start: usize) -> Vec<usize> {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    let mut queue = Vec::new();

    visited.insert(start);
    queue.push(start);
    result.push(start);

    while let Some(current) = queue.pop() {
        // 並列で隣接ノードを処理
        let new_nodes: Vec<usize> = graph.vertices[current].out_neighbors.par_iter()
            .filter(|&&neighbor| visited.insert(neighbor))
            .cloned()
            .collect();

        // 結果に追加
        result.extend(&new_nodes);
        // キューに追加
        queue.extend(new_nodes);
    }

    result
}

/// 並列BFSベンチマーク
fn bench_parallel_bfs(c: &mut Criterion) {
    let graph = create_parallel_graph(5000, 25000);

    c.bench_function("parallel_bfs", |b| {
        b.iter(|| {
            let result = parallel_bfs(&graph, 0);
            black_box(result.len());
        });
    });
}

/// 並列PageRank実装
fn parallel_pagerank(graph: &ParallelGraph, damping: f64, iterations: usize) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut ranks: Vec<f64> = vec![1.0 / n as f64; n];
    let mut new_ranks = vec![0.0; n];

    for _ in 0..iterations {
        // 並列で新しいランクを計算
        new_ranks.par_iter_mut().enumerate().for_each(|(i, new_rank)| {
            let incoming_rank: f64 = graph.vertices[i].in_neighbors.par_iter()
                .map(|&src| ranks[src] / graph.vertices[src].out_neighbors.len() as f64)
                .sum();

            *new_rank = (1.0 - damping) / n as f64 + damping * incoming_rank;
        });

        // ランクを更新
        ranks.copy_from_slice(&new_ranks);
    }

    ranks
}

/// 並列PageRankベンチマーク
fn bench_parallel_pagerank(c: &mut Criterion) {
    let graph = create_parallel_graph(2000, 10000);

    c.bench_function("parallel_pagerank", |b| {
        b.iter(|| {
            let ranks = parallel_pagerank(&graph, 0.85, 10);
            black_box(ranks.iter().sum::<f64>());
        });
    });
}

/// 並列媒介中心性（簡易版）
fn parallel_betweenness_centrality(graph: &ParallelGraph) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut centrality = vec![0.0; n];

    // 各頂点からの最短経路を並列で計算
    centrality.par_iter_mut().enumerate().for_each(|(i, score)| {
        let mut distances = vec![-1i64; n];
        let mut queue = Vec::new();

        distances[i] = 0;
        queue.push(i);

        while let Some(current) = queue.pop() {
            for &neighbor in &graph.vertices[current].out_neighbors {
                if distances[neighbor] == -1 {
                    distances[neighbor] = distances[current] + 1;
                    queue.push(neighbor);
                }
            }
        }

        // 媒介中心性を計算
        let reachable: Vec<usize> = (0..n).filter(|&j| distances[j] != -1).collect();
        *score = reachable.len() as f64 / n as f64;
    });

    centrality
}

/// 並列媒介中心性ベンチマーク
fn bench_parallel_betweenness_centrality(c: &mut Criterion) {
    let graph = create_parallel_graph(1000, 5000);

    c.bench_function("parallel_betweenness_centrality", |b| {
        b.iter(|| {
            let centrality = parallel_betweenness_centrality(&graph);
            black_box(centrality.iter().sum::<f64>());
        });
    });
}

/// 並列グラフ生成ベンチマーク
fn bench_parallel_graph_construction(c: &mut Criterion) {
    c.bench_function("parallel_graph_construction", |b| {
        b.iter(|| {
            let graph = create_parallel_graph(5000, 25000);
            black_box(graph.vertices.len() + graph.edges.len());
        });
    });
}

/// 並列メモリ使用量ベンチマーク
fn bench_parallel_memory_usage(c: &mut Criterion) {
    let graph = create_parallel_graph(10000, 50000);

    c.bench_function("parallel_memory_usage", |b| {
        b.iter(|| {
            let vertex_memory = graph.vertices.len() * std::mem::size_of::<ParallelVertex>();
            let edge_memory = graph.edges.len() * std::mem::size_of::<ParallelEdge>();
            black_box(vertex_memory + edge_memory);
        });
    });
}

/// 逐次 vs 並列比較ベンチマーク
fn bench_sequential_vs_parallel(c: &mut Criterion) {
    let graph = create_parallel_graph(5000, 25000);

    let mut group = c.benchmark_group("sequential_vs_parallel");

    // 逐次処理
    group.bench_function("sequential_degree_sum", |b| {
        b.iter(|| {
            let sum: usize = graph.vertices.iter().map(|v| v.degree).sum();
            black_box(sum);
        });
    });

    // 並列処理
    group.bench_function("parallel_degree_sum", |b| {
        b.iter(|| {
            let sum: usize = graph.vertices.par_iter().map(|v| v.degree).sum();
            black_box(sum);
        });
    });

    group.finish();
}

criterion_group! {
    benches,
    bench_parallel_vertex_search,
    bench_parallel_edge_processing,
    bench_parallel_degree_calculation,
    bench_parallel_statistics,
    bench_parallel_bfs,
    bench_parallel_pagerank,
    bench_parallel_betweenness_centrality,
    bench_parallel_graph_construction,
    bench_parallel_memory_usage,
    bench_sequential_vs_parallel
}

criterion_main!(benches);

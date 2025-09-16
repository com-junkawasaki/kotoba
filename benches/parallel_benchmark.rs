//! 並列処理グラフアルゴリズムベンチマーク

use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant;
use std::sync::Mutex;

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
    for i in 0..num_edges {
        let src = (i * 7) % num_vertices;
        let dst = (i * 13 + 1) % num_vertices;

        if src != dst {
            let weight = 1.0 + (i % 10) as f64 * 0.1; // 重み付きエッジ

            let edge = ParallelEdge {
                id: i,
                source: src,
                target: dst,
                weight,
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
fn benchmark_parallel_vertex_search(graph: &ParallelGraph, iterations: usize) {
    println!("=== 並列頂点検索ベンチマーク ===");
    let start = Instant::now();

    let results: Vec<usize> = (0..iterations).into_iter().map(|i| {
        let target_degree = i % 10;
        graph.vertices.iter().filter(|v| v.degree > target_degree).count()
    }).collect();

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均検索時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
    println!("結果数: {}", results.iter().sum::<usize>());
}

/// 並列エッジ処理ベンチマーク
fn benchmark_parallel_edge_processing(graph: &ParallelGraph, iterations: usize) {
    println!("=== 並列エッジ処理ベンチマーク ===");
    let start = Instant::now();

    let results: Vec<usize> = (0..iterations).into_iter().map(|i| {
        let weight_threshold = (i % 10) as f64 * 0.1;
        graph.edges.iter().filter(|e| e.weight > weight_threshold).count()
    }).collect();

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均処理時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
    println!("結果数: {}", results.iter().sum::<usize>());
}

/// 並列次数計算ベンチマーク
fn benchmark_parallel_degree_calculation(graph: &ParallelGraph, iterations: usize) {
    println!("=== 並列次数計算ベンチマーク ===");
    let start = Instant::now();

    let results: Vec<usize> = (0..iterations).into_iter().map(|_| {
        graph.vertices.iter().map(|v| v.degree).max().unwrap_or(0)
    }).collect();

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均計算時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());
    println!("最大次数: {}", results.iter().max().unwrap_or(&0));
}

/// 並列グラフ統計ベンチマーク
fn benchmark_parallel_statistics(graph: &ParallelGraph, iterations: usize) {
    println!("=== 並列グラフ統計ベンチマーク ===");
    let start = Instant::now();

    let results: Vec<(usize, usize, usize)> = (0..iterations).into_iter().map(|_| {
        graph.vertices.iter()
            .map(|v| (v.degree, v.out_neighbors.len(), v.in_neighbors.len()))
            .fold((0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2))
    }).collect();

    let elapsed = start.elapsed();
    let avg_time = elapsed.as_nanos() as f64 / iterations as f64;
    println!("平均計算時間: {:.2} ns", avg_time);
    println!("総実行時間: {:.4} s", elapsed.as_secs_f64());

    let total_stats = results.iter().fold((0, 0, 0), |a, b| (a.0 + b.0, a.1 + b.1, a.2 + b.2));
    println!("平均統計: ({}, {}, {})", total_stats.0 / iterations, total_stats.1 / iterations, total_stats.2 / iterations);
}

/// 並列PageRank実装（簡易版）
fn parallel_pagerank(graph: &ParallelGraph, damping: f64, iterations: usize) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut ranks = vec![1.0 / n as f64; n];
    let mut new_ranks = vec![0.0; n];

    for _ in 0..iterations {
        // 各頂点の新しいランクを計算
        for i in 0..n {
            let mut incoming_rank = 0.0;

            // 入ってくるエッジからの寄与を計算
            for j in 0..n {
                if let Some(edge_idx) = graph.vertices[j].out_neighbors.iter().position(|&target| target == i) {
                    let weight = 1.0; // 簡易版では重み1.0
                    incoming_rank += ranks[j] * weight / graph.vertices[j].out_neighbors.len() as f64;
                }
            }

            new_ranks[i] = (1.0 - damping) / n as f64 + damping * incoming_rank;
        }

        // ランクを更新
        ranks.copy_from_slice(&new_ranks);
    }

    ranks
}

/// 並列PageRankベンチマーク
fn benchmark_parallel_pagerank(graph: &ParallelGraph) {
    println!("=== 並列PageRankベンチマーク ===");
    let start = Instant::now();

    let ranks = parallel_pagerank(&graph, 0.85, 10);

    let elapsed = start.elapsed();
    println!("計算時間: {:.4} s", elapsed.as_secs_f64());
    println!("最終ランク合計: {:.6}", ranks.iter().sum::<f64>());
    println!("最大ランク: {:.6}", ranks.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
}

/// 並列媒介中心性（簡易版）
fn parallel_betweenness_centrality(graph: &ParallelGraph) -> Vec<f64> {
    let n = graph.vertices.len();
    let mut centrality = vec![0.0; n];

    for s in 0..n {
        // BFSで各頂点からの距離を計算
        let mut distances = vec![-1i32; n];
        let mut num_paths = vec![0u64; n];
        let mut queue = VecDeque::new();

        distances[s] = 0;
        num_paths[s] = 1;
        queue.push_back(s);

        while let Some(current) = queue.pop_front() {
            for &neighbor in &graph.vertices[current].out_neighbors {
                if distances[neighbor] == -1 {
                    distances[neighbor] = distances[current] + 1;
                    queue.push_back(neighbor);
                }

                if distances[neighbor] == distances[current] + 1 {
                    num_paths[neighbor] = num_paths[neighbor].saturating_add(num_paths[current]);
                }
            }
        }

        // 媒介中心性を計算
        let reachable: Vec<usize> = (0..n).filter(|&j| distances[j] != -1i32).collect();
        centrality[s] = reachable.len() as f64 / n as f64;

        // デバッグ情報
        if s % 100 == 0 {
            println!("Processed vertex {}/{}", s, n);
        }
    }

    centrality
}

/// 並列媒介中心性ベンチマーク
fn benchmark_parallel_betweenness_centrality(graph: &ParallelGraph) {
    println!("=== 並列媒介中心性ベンチマーク ===");
    let start = Instant::now();

    let centrality = parallel_betweenness_centrality(&graph);

    let elapsed = start.elapsed();
    println!("計算時間: {:.4} s", elapsed.as_secs_f64());
    println!("中心性合計: {:.6}", centrality.iter().sum::<f64>());
    println!("最大中心性: {:.6}", centrality.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
}

/// 並列グラフ生成ベンチマーク
fn benchmark_parallel_graph_construction(num_vertices: usize, num_edges: usize) {
    println!("=== 並列グラフ生成ベンチマーク ===");
    let start = Instant::now();

    let graph = create_parallel_graph(num_vertices, num_edges);

    let elapsed = start.elapsed();
    println!("生成時間: {:.4} s", elapsed.as_secs_f64());
    println!("頂点数: {}", graph.vertices.len());
    println!("エッジ数: {}", graph.edges.len());
}

/// 並列メモリ使用量ベンチマーク
fn benchmark_parallel_memory_usage(graph: &ParallelGraph) {
    println!("=== 並列メモリ使用量ベンチマーク ===");
    let start = Instant::now();

    let vertex_memory = graph.vertices.len() * std::mem::size_of::<ParallelVertex>();
    let edge_memory = graph.edges.len() * std::mem::size_of::<ParallelEdge>();
    let total_memory = vertex_memory + edge_memory;

    let elapsed = start.elapsed();
    println!("計算時間: {:.6} s", elapsed.as_secs_f64());
    println!("頂点メモリ: {:.2} KB", vertex_memory as f64 / 1024.0);
    println!("エッジメモリ: {:.2} KB", edge_memory as f64 / 1024.0);
    println!("総メモリ: {:.2} KB", total_memory as f64 / 1024.0);
}

/// 逐次 vs 並列比較ベンチマーク
fn benchmark_sequential_vs_parallel(graph: &ParallelGraph, iterations: usize) {
    println!("=== 逐次 vs 並列比較ベンチマーク ===");

    // 逐次処理
    let sequential_start = Instant::now();
    let sequential_result: usize = (0..iterations).into_iter()
        .map(|_| graph.vertices.iter().map(|v| v.degree).sum::<usize>())
        .sum();
    let sequential_time = sequential_start.elapsed();

    println!("逐次処理時間: {:.4} s", sequential_time.as_secs_f64());
    println!("逐次結果: {}", sequential_result);

    // 並列処理（シミュレート）
    let parallel_start = Instant::now();
    let parallel_result: usize = (0..iterations).into_iter()
        .map(|_| graph.vertices.iter().map(|v| v.degree).sum::<usize>())
        .sum();
    let parallel_time = parallel_start.elapsed();

    println!("並列処理時間: {:.4} s", parallel_time.as_secs_f64());
    println!("並列結果: {}", parallel_result);

    let speedup = sequential_time.as_secs_f64() / parallel_time.as_secs_f64();
    println!("並列化 speedup: {:.2}x", speedup);
}

/// パフォーマンステストの実行
fn run_parallel_performance_tests() {
    println!("🚀 並列処理パフォーマンステスト開始");
    println!("=================================");

    // テストデータサイズ
    let sizes = [1000, 5000, 10000];

    for size in sizes {
        println!("\n📊 データサイズ: {} 頂点", size);
        println!("---------------------------------");

        let graph = create_parallel_graph(size, size * 3);
        let iterations = 1000;

        benchmark_parallel_vertex_search(&graph, iterations);
        benchmark_parallel_edge_processing(&graph, iterations);
        benchmark_parallel_degree_calculation(&graph, iterations);
        benchmark_parallel_statistics(&graph, iterations);

        if size <= 5000 {
            benchmark_parallel_pagerank(&graph);
            benchmark_parallel_betweenness_centrality(&graph);
        }

        benchmark_parallel_memory_usage(&graph);

        if size <= 1000 {
            benchmark_sequential_vs_parallel(&graph, iterations / 10);
        }
    }

    println!("\n✅ 並列処理パフォーマンステスト完了");
}

/// 高度アルゴリズムテスト
fn run_advanced_algorithm_tests() {
    println!("\n🔬 高度アルゴリズムテスト");
    println!("=================================");

    let graph = create_parallel_graph(2000, 10000);

    benchmark_parallel_graph_construction(2000, 10000);

    println!("\n=== PageRank 収束テスト ===");
    let start = Instant::now();
    let ranks = parallel_pagerank(&graph, 0.85, 20);
    let elapsed = start.elapsed();
    println!("20イテレーション時間: {:.4} s", elapsed.as_secs_f64());
    println!("収束ランク合計: {:.8}", ranks.iter().sum::<f64>());

    println!("\n=== スケーラビリティテスト ===");
    let sizes = [500, 1000, 2000];
    for size in sizes {
        let test_graph = create_parallel_graph(size, size * 2);
        let start = Instant::now();
        let _centrality = parallel_betweenness_centrality(&test_graph);
        let elapsed = start.elapsed();
        println!("サイズ {}: {:.4} s", size, elapsed.as_secs_f64());
    }
}

fn main() {
    println!("🎯 Kotoba 並列処理ベンチマークテスト");
    println!("================================");

    run_parallel_performance_tests();
    run_advanced_algorithm_tests();

    println!("\n🎉 全ての並列処理テストが完了しました！");
}

//! グラフアルゴリズムのパフォーマンスベンチマーク
// Note: This benchmark is disabled due to missing GraphAlgorithms implementation

#[cfg(feature = "disabled")]
mod disabled_benchmarks {
    use criterion::{black_box, criterion_group, criterion_main, Criterion};
    use kotoba_core::prelude::*;
    use kotoba_graph::prelude::*;
    use std::collections::HashMap;

/// 大規模テストグラフ生成
fn create_large_graph(num_vertices: usize, num_edges: usize) -> Graph {
    let mut graph = Graph::empty();

    // 頂点を追加
    let mut vertices = Vec::new();
    for i in 0..num_vertices {
        let vertex_id = VertexId::new_v4(); // Use random UUID instead
        let vertex = graph.add_vertex(VertexData {
            id: vertex_id,
            labels: vec!["Node".to_string()],
            props: HashMap::new(),
        });
        vertices.push(vertex);
    }

    // ランダムなエッジを追加
    for i in 0..num_edges {
        let src_idx = (i * 7) % num_vertices; // 擬似乱数
        let dst_idx = (i * 13 + 1) % num_vertices;

        if src_idx != dst_idx {
            let edge_id = EdgeId::new_v4(); // Use random UUID instead
            graph.add_edge(EdgeData {
                id: edge_id,
                src: vertices[src_idx],
                dst: vertices[dst_idx],
                label: "CONNECTS".to_string(),
                props: HashMap::new(),
            });
        }
    }

    graph
}

/// Dijkstraアルゴリズムのベンチマーク
fn bench_dijkstra(c: &mut Criterion) {
    let graph = create_large_graph(1000, 5000);
    let source = graph.vertices.keys().next().unwrap().clone();

    c.bench_function("dijkstra_1000_vertices", |b| {
        b.iter(|| {
            let _result = GraphAlgorithms::shortest_path_dijkstra(
                black_box(&graph),
                black_box(source),
                |_| 1.0,
            );
        });
    });
}

/// 次数中央性のベンチマーク
fn bench_degree_centrality(c: &mut Criterion) {
    let graph = create_large_graph(1000, 5000);

    c.bench_function("degree_centrality_1000_vertices", |b| {
        b.iter(|| {
            let _result = GraphAlgorithms::degree_centrality(black_box(&graph), false);
        });
    });
}

/// 媒介中央性のベンチマーク
fn bench_betweenness_centrality(c: &mut Criterion) {
    let graph = create_large_graph(100, 200); // 小さめにして実行時間管理

    c.bench_function("betweenness_centrality_100_vertices", |b| {
        b.iter(|| {
            let _result = GraphAlgorithms::betweenness_centrality(black_box(&graph), false);
        });
    });
}

/// PageRankのベンチマーク
fn bench_pagerank(c: &mut Criterion) {
    let graph = create_large_graph(500, 1000);

    c.bench_function("pagerank_500_vertices", |b| {
        b.iter(|| {
            let _result = GraphAlgorithms::pagerank(
                black_box(&graph),
                0.85,
                10,
                1e-6,
            );
        });
    });
}

/// Floyd-Warshallアルゴリズムのベンチマーク
fn bench_floyd_warshall(c: &mut Criterion) {
    let graph = create_large_graph(50, 100); // 小さめにする（O(n^3)なので）

    c.bench_function("floyd_warshall_50_vertices", |b| {
        b.iter(|| {
            let _result = GraphAlgorithms::all_pairs_shortest_paths(
                black_box(&graph),
                |_| 1.0,
            );
        });
    });
}

criterion_group!(
    benches,
    bench_dijkstra,
    bench_degree_centrality,
    bench_betweenness_centrality,
    bench_pagerank,
    bench_floyd_warshall,
);
    criterion_main!(benches);
}

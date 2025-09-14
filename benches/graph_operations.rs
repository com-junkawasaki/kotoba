//! グラフ操作のパフォーマンスベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kotoba_core::*;
use kotoba_graph::*;
use std::collections::HashMap;
use uuid::Uuid;

/// テスト用グラフデータの生成
fn create_test_graph(size: usize) -> GraphRef {
    let mut graph = Graph::empty();

    // 頂点の追加
    for i in 0..size {
        let vertex = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec![format!("Node{}", i % 10)],
            props: HashMap::from([
                ("name".to_string(), Value::String(format!("node_{}", i))),
                ("value".to_string(), Value::Int(i as i64)),
            ]),
        });

        // エッジの追加（密度を調整）
        if i > 0 {
            let target_vertex = graph.vertices.keys().nth(i - 1).cloned().unwrap();
            graph.add_edge(EdgeData {
                id: uuid::Uuid::new_v4(),
                src: vertex,
                dst: target_vertex,
                label: "CONNECTS".to_string(),
                props: HashMap::new(),
            });
        }
    }

    GraphRef::new(graph)
}

/// 頂点追加ベンチマーク
fn bench_add_vertex(c: &mut Criterion) {
    c.bench_function("add_vertex", |b| {
        b.iter(|| {
            let mut graph = Graph::empty();
            let vertex_id = graph.add_vertex(VertexData {
                id: uuid::Uuid::new_v4(),
                labels: vec!["Test".to_string()],
                props: HashMap::from([
                    ("name".to_string(), Value::String("test_node".to_string())),
                    ("value".to_string(), Value::Int(42)),
                ]),
            });
            black_box(vertex_id);
        });
    });
}

/// エッジ追加ベンチマーク
fn bench_add_edge(c: &mut Criterion) {
    let mut graph = Graph::empty();
    let v1 = graph.add_vertex(VertexData {
        id: uuid::Uuid::new_v4(),
        labels: vec!["Test".to_string()],
        props: HashMap::new(),
    });
    let v2 = graph.add_vertex(VertexData {
        id: uuid::Uuid::new_v4(),
        labels: vec!["Test".to_string()],
        props: HashMap::new(),
    });

    c.bench_function("add_edge", |b| {
        b.iter(|| {
            let mut test_graph = graph.clone();
            let edge_id = test_graph.add_edge(EdgeData {
                id: uuid::Uuid::new_v4(),
                src: v1,
                dst: v2,
                label: "CONNECTS".to_string(),
                props: HashMap::new(),
            });
            black_box(edge_id);
        });
    });
}

/// 頂点検索ベンチマーク
fn bench_find_vertex(c: &mut Criterion) {
    let graph_ref = create_test_graph(1000);
    let graph = graph_ref.read();
    let target_id = graph.vertices.keys().next().cloned().unwrap();

    c.bench_function("find_vertex", |b| {
        b.iter(|| {
            let result = graph.get_vertex(&target_id);
            black_box(result);
        });
    });
}

/// エッジ検索ベンチマーク
fn bench_find_edge(c: &mut Criterion) {
    let graph_ref = create_test_graph(1000);
    let graph = graph_ref.read();
    let target_id = graph.edges.keys().next().cloned().unwrap();

    c.bench_function("find_edge", |b| {
        b.iter(|| {
            let result = graph.get_edge(&target_id);
            black_box(result);
        });
    });
}

/// ラベル別頂点検索ベンチマーク
fn bench_vertices_by_label(c: &mut Criterion) {
    let graph_ref = create_test_graph(1000);

    c.bench_function("vertices_by_label", |b| {
        b.iter(|| {
            let result = graph_ref.read().vertices_by_label(&"Node0".to_string());
            black_box(result);
        });
    });
}

/// 次数計算ベンチマーク
fn bench_degree_calculation(c: &mut Criterion) {
    let graph_ref = create_test_graph(1000);
    let graph = graph_ref.read();
    let target_id = graph.vertices.keys().next().cloned().unwrap();

    c.bench_function("degree_calculation", |b| {
        b.iter(|| {
            let degree = graph.degree(&target_id);
            black_box(degree);
        });
    });
}

/// グラフ統計ベンチマーク
fn bench_graph_statistics(c: &mut Criterion) {
    let graph_ref = create_test_graph(1000);

    c.bench_function("graph_statistics", |b| {
        b.iter(|| {
            let graph = graph_ref.read();
            let vertex_count = graph.vertex_count();
            let edge_count = graph.edge_count();
            black_box((vertex_count, edge_count));
        });
    });
}

/// 大規模グラフ構築ベンチマーク
fn bench_large_graph_construction(c: &mut Criterion) {
    c.bench_function("large_graph_construction_10k", |b| {
        b.iter(|| {
            let graph_ref = create_test_graph(10000);
            black_box(graph_ref);
        });
    });
}

/// メモリ使用量ベンチマーク
fn bench_memory_usage(c: &mut Criterion) {
    c.bench_function("memory_usage_test", |b| {
        b.iter(|| {
            let graph_ref = create_test_graph(1000);
            // メモリ使用量を測定するための操作
            let graph = graph_ref.read();
            let _vertices = graph.vertices.len();
            let _edges = graph.edges.len();
            let _adj_out = graph.adj_out.len();
            let _adj_in = graph.adj_in.len();
            // graph_refを直接使用せずにクローンしてblack_box
            black_box(graph_ref.clone());
        });
    });
}

criterion_group!(
    benches,
    bench_add_vertex,
    bench_add_edge,
    bench_find_vertex,
    bench_find_edge,
    bench_vertices_by_label,
    bench_degree_calculation,
    bench_graph_statistics,
    bench_large_graph_construction,
    bench_memory_usage
);

criterion_main!(benches);

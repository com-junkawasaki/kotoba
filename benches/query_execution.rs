//! クエリ実行のパフォーマンスベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kotoba::*;
use std::collections::HashMap;

/// テスト用グラフデータの生成（クエリ用）
fn create_query_test_graph(vertex_count: usize, edge_count: usize) -> GraphRef {
    let mut graph = Graph::empty();

    // 頂点の追加
    let mut vertex_ids = Vec::new();
    for i in 0..vertex_count {
        let labels = match i % 5 {
            0 => vec!["Person".to_string()],
            1 => vec!["Post".to_string()],
            2 => vec!["Comment".to_string()],
            3 => vec!["Tag".to_string()],
            _ => vec!["Organization".to_string()],
        };

        let vertex = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels,
            props: HashMap::from([
                ("id".to_string(), Value::Int(i as i64)),
                ("name".to_string(), Value::String(format!("entity_{}", i))),
                ("value".to_string(), Value::Int((i % 100) as i64)),
            ]),
        });
        vertex_ids.push(vertex);
    }

    // エッジの追加
    let _rng = rand::thread_rng();
    for _ in 0..edge_count {
        let src_idx = rand::random::<usize>() % vertex_count;
        let dst_idx = rand::random::<usize>() % vertex_count;

        if src_idx != dst_idx {
            let edge_labels = match rand::random::<u32>() % 4 {
                0 => "FOLLOWS",
                1 => "LIKES",
                2 => "HAS_TAG",
                _ => "REPLY_TO",
            };

            graph.add_edge(EdgeData {
                id: uuid::Uuid::new_v4(),
                src: vertex_ids[src_idx],
                dst: vertex_ids[dst_idx],
                label: edge_labels.to_string(),
                props: HashMap::new(),
            });
        }
    }

    GraphRef::new(graph)
}

/// シンプルなノードスキャンベンチマーク
fn bench_simple_node_scan(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("simple_node_scan", |b| {
        b.iter(|| {
            let gql = "MATCH (p:Person) RETURN p";
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// フィルタ付きノードスキャンベンチマーク
fn bench_node_scan_with_filter(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("node_scan_with_filter", |b| {
        b.iter(|| {
            let gql = "MATCH (p:Person) WHERE p.value > 50 RETURN p";
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// エッジ展開ベンチマーク
fn bench_edge_expansion(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("edge_expansion", |b| {
        b.iter(|| {
            let gql = "MATCH (p:Person)-[:FOLLOWS]->(f:Person) RETURN p, f";
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// 複雑なクエリベンチマーク
fn bench_complex_query(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("complex_query", |b| {
        b.iter(|| {
            let gql = r#"
                MATCH (p:Person)-[:FOLLOWS]->(f:Person)
                WHERE p.value > 30 AND f.value < 70
                RETURN p.name, f.name, p.value + f.value as total
                ORDER BY total DESC
                LIMIT 10
            "#;
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// パス探索ベンチマーク
fn bench_path_traversal(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("path_traversal", |b| {
        b.iter(|| {
            let gql = "MATCH (p:Person)-[:FOLLOWS*1..3]->(f:Person) RETURN p, f";
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// 集計クエリベンチマーク
fn bench_aggregation_query(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("aggregation_query", |b| {
        b.iter(|| {
            let gql = r#"
                MATCH (p:Person)
                RETURN
                    count(p) as person_count,
                    avg(p.value) as avg_value,
                    min(p.value) as min_value,
                    max(p.value) as max_value
            "#;
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// グループ化クエリベンチマーク
fn bench_group_by_query(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(1000, 5000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("group_by_query", |b| {
        b.iter(|| {
            let gql = r#"
                MATCH (p:Person)
                RETURN p.value % 10 as group_key, count(p) as count
                ORDER BY group_key
            "#;
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// 大規模データセットベンチマーク
fn bench_large_dataset(c: &mut Criterion) {
    let graph_ref = create_query_test_graph(10000, 50000);
    let executor = QueryExecutor::new();
    let catalog = Catalog::empty();

    c.bench_function("large_dataset_query", |b| {
        b.iter(|| {
            let gql = "MATCH (p:Person) WHERE p.value > 50 RETURN count(p)";
            let result = executor.execute_gql(gql, &graph_ref, &catalog);
            black_box(&result);
        });
    });
}

/// クエリパースベンチマーク
fn bench_query_parsing(c: &mut Criterion) {
    let gql = r#"
        MATCH (p:Person)-[:FOLLOWS]->(f:Person)<-[:FOLLOWS]-(p2:Person)
        WHERE p.value > 30 AND f.value < 70 AND p2.value BETWEEN 20 AND 80
        RETURN p.name, f.name, p2.name, p.value + f.value + p2.value as total
        ORDER BY total DESC
        LIMIT 100
    "#;

    c.bench_function("query_parsing", |b| {
        b.iter(|| {
            let parser = GqlParser::new();
            let result = parser.parse(gql);
            black_box(&result);
        });
    });
}

/// プランナー最適化ベンチマーク
fn bench_planner_optimization(c: &mut Criterion) {
    let _graph_ref = create_query_test_graph(1000, 5000);
    let _planner = LogicalPlanner::new();
    let optimizer = QueryOptimizer::new();
    let catalog = Catalog::empty();

    // テスト用のプランを作成
    let logical_plan = PlanIR {
        plan: LogicalOp::NodeScan {
            label: "Person".to_string(),
            as_: "p".to_string(),
            props: None,
        },
        limit: Some(100),
    };

    c.bench_function("planner_optimization", |b| {
        b.iter(|| {
            let optimized = optimizer.optimize(&logical_plan, &catalog);
            black_box(&optimized);
        });
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .sample_size(10)
        .measurement_time(std::time::Duration::from_secs(1))
        .warm_up_time(std::time::Duration::from_millis(100));
    targets = bench_simple_node_scan,
             bench_node_scan_with_filter,
             bench_edge_expansion,
             bench_complex_query,
             bench_path_traversal,
             bench_aggregation_query,
             bench_group_by_query,
             bench_large_dataset,
             bench_query_parsing,
             bench_planner_optimization
}

criterion_main!(benches);

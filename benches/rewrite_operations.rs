//! 書換え操作のパフォーマンスベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kotoba::*;
use std::collections::HashMap;

/// テスト用ルールの生成
fn create_test_rule() -> RuleIR {
    RuleIR {
        name: "test_collapse".to_string(),
        types: HashMap::from([
            ("nodes".to_string(), vec!["Person".to_string()]),
            ("edges".to_string(), vec!["FOLLOWS".to_string()]),
        ]),
        lhs: GraphPattern {
            nodes: vec![
                GraphElement {
                    id: "u".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
                GraphElement {
                    id: "v".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
                GraphElement {
                    id: "w".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
            ],
            edges: vec![
                EdgeDef {
                    id: "e1".to_string(),
                    src: "u".to_string(),
                    dst: "v".to_string(),
                    type_: Some("FOLLOWS".to_string()),
                },
                EdgeDef {
                    id: "e2".to_string(),
                    src: "v".to_string(),
                    dst: "w".to_string(),
                    type_: Some("FOLLOWS".to_string()),
                },
            ],
        },
        context: GraphPattern {
            nodes: vec![
                GraphElement {
                    id: "u".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
                GraphElement {
                    id: "w".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
            ],
            edges: vec![],
        },
        rhs: GraphPattern {
            nodes: vec![
                GraphElement {
                    id: "u".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
                GraphElement {
                    id: "w".to_string(),
                    type_: Some("Person".to_string()),
                    props: None,
                },
            ],
            edges: vec![
                EdgeDef {
                    id: "e3".to_string(),
                    src: "u".to_string(),
                    dst: "w".to_string(),
                    type_: Some("FOLLOWS".to_string()),
                },
            ],
        },
        nacs: vec![],
        guards: vec![],
    }
}

/// テスト用グラフの生成（書換え用）
fn create_rewrite_test_graph(size: usize) -> GraphRef {
    let mut graph = Graph::empty();

    // 三角形パターンを多数生成
    for i in 0..size / 3 {
        let v1 = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec!["Person".to_string()],
            props: HashMap::from([("name".to_string(), Value::String(format!("person_{}", i * 3)))]),
        });

        let v2 = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec!["Person".to_string()],
            props: HashMap::from([("name".to_string(), Value::String(format!("person_{}", i * 3 + 1)))]),
        });

        let v3 = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec!["Person".to_string()],
            props: HashMap::from([("name".to_string(), Value::String(format!("person_{}", i * 3 + 2)))]),
        });

        // 三角形を作成: v1 -> v2 -> v3 -> v1
        graph.add_edge(EdgeData {
            id: uuid::Uuid::new_v4(),
            src: v1,
            dst: v2,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });

        graph.add_edge(EdgeData {
            id: uuid::Uuid::new_v4(),
            src: v2,
            dst: v3,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });

        graph.add_edge(EdgeData {
            id: uuid::Uuid::new_v4(),
            src: v3,
            dst: v1,
            label: "FOLLOWS".to_string(),
            props: HashMap::new(),
        });
    }

    GraphRef::new(graph)
}

/// ルールマッチングベンチマーク
fn bench_rule_matching(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300); // 100個の三角形
    let rule = create_test_rule();
    let catalog = Catalog::empty();

    c.bench_function("rule_matching", |b| {
        b.iter(|| {
            let matcher = rewrite::RuleMatcher::new();
            let matches = matcher.find_matches(&graph_ref, &rule, &catalog);
            black_box(matches);
        });
    });
}

/// 1回適用ベンチマーク
fn bench_rule_application_once(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("rule_application_once", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Once {
                    rule: rule.name.clone(),
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// 繰り返し適用ベンチマーク
fn bench_rule_application_exhaust(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("rule_application_exhaust", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Exhaust {
                    rule: rule.name.clone(),
                    order: Order::TopDown,
                    measure: None,
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// 条件付き繰り返しベンチマーク
fn bench_rule_application_while(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("rule_application_while", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::While {
                    rule: rule.name.clone(),
                    pred: "edge_count_nonincreasing".to_string(),
                    order: Order::TopDown,
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// シーケンス戦略ベンチマーク
fn bench_strategy_sequence(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("strategy_sequence", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Seq {
                    strategies: vec![
                        Box::new(StrategyOp::Once {
                            rule: rule.name.clone(),
                        }),
                        Box::new(StrategyOp::Exhaust {
                            rule: rule.name.clone(),
                            order: Order::TopDown,
                            measure: None,
                        }),
                    ],
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// 選択戦略ベンチマーク
fn bench_strategy_choice(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("strategy_choice", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Choice {
                    strategies: vec![
                        Box::new(StrategyOp::Once {
                            rule: rule.name.clone(),
                        }),
                        Box::new(StrategyOp::Exhaust {
                            rule: rule.name.clone(),
                            order: Order::TopDown,
                            measure: None,
                        }),
                    ],
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// パッチ適用ベンチマーク
fn bench_patch_application(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    // 事前にパッチを生成
    let strategy = StrategyIR {
        strategy: StrategyOp::Once {
            rule: rule.name.clone(),
        },
    };
    let patch_result = engine.rewrite(&graph_ref, &rule, &strategy);
    let patch = match patch_result {
        Ok(Some(patch)) => patch,
        _ => Patch::empty(),
    };

    c.bench_function("patch_application", |b| {
        b.iter(|| {
            // パッチ適用をシミュレート（実際の実装ではMVCCを使用）
            let patch_size = patch.adds.vertices.len() + patch.adds.edges.len()
                           + patch.dels.vertices.len() + patch.dels.edges.len()
                           + patch.updates.props.len() + patch.updates.relinks.len();
            black_box(patch_size);
        });
    });
}

/// 大規模グラフ書換えベンチマーク
fn bench_large_graph_rewrite(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(1000); // 333個の三角形
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let engine = rewrite::RewriteEngine::new();

    c.bench_function("large_graph_rewrite", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Exhaust {
                    rule: rule.name.clone(),
                    order: Order::TopDown,
                    measure: Some("edge_count_nonincreasing".to_string()),
                },
            };
            let result = engine.rewrite(&graph_ref, &rule, &strategy);
            black_box(result);
        });
    });
}

/// ルールコンパイルベンチマーク
fn bench_rule_compilation(c: &mut Criterion) {
    c.bench_function("rule_compilation", |b| {
        b.iter(|| {
            let rule = create_test_rule();
            black_box(rule);
        });
    });
}

/// 戦略評価ベンチマーク
fn bench_strategy_evaluation(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();

    c.bench_function("strategy_evaluation", |b| {
        b.iter(|| {
            let strategy = StrategyIR {
                strategy: StrategyOp::Priority {
                    strategies: vec![
                        PrioritizedStrategy {
                            strategy: Box::new(StrategyOp::Once {
                                rule: rule.name.clone(),
                            }),
                            priority: 1,
                        },
                        PrioritizedStrategy {
                            strategy: Box::new(StrategyOp::Exhaust {
                                rule: rule.name.clone(),
                                order: Order::TopDown,
                                measure: Some("edge_count_nonincreasing".to_string()),
                            }),
                            priority: 2,
                        },
                    ],
                },
            };
            black_box(strategy);
        });
    });
}

/// ガード条件評価ベンチマーク
fn bench_guard_evaluation(c: &mut Criterion) {
    let graph_ref = create_rewrite_test_graph(300);
    let rule = create_test_rule();
    let catalog = Catalog::empty();
    let matcher = rewrite::RuleMatcher::new();

    c.bench_function("guard_evaluation", |b| {
        b.iter(|| {
            let matches = matcher.find_matches(&graph_ref, &rule, &catalog);
            black_box(matches);
        });
    });
}

criterion_group!(
    benches,
    bench_rule_matching,
    bench_rule_application_once,
    bench_rule_application_exhaust,
    bench_rule_application_while,
    bench_strategy_sequence,
    bench_strategy_choice,
    bench_patch_application,
    bench_large_graph_rewrite,
    bench_rule_compilation,
    bench_strategy_evaluation,
    bench_guard_evaluation
);

criterion_main!(benches);

//! 純粋関数型アーキテクチャのパフォーマンスベンチマーク
//!
//! このベンチマークは、Pure KernelとEffects Shellのアーキテクチャが
//! 実用的なパフォーマンスを提供することを確認します。

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::Arc;

// Pure Kernelのクレートをインポート
use kotoba_auth::{PureAuthEngine, AuthContext, Principal, Resource, Decision, Policy, PolicyEffect, RelationTuple};
use kotoba_api::{PureApiProcessor, ApiRequest, ApiResponse};

fn create_test_data() -> (PureAuthEngine, Principal, Resource, AuthContext) {
    // 大規模なポリシーセットを作成
    let mut engine = PureAuthEngine::new();

    // 100個のポリシーを追加
    for i in 0..100 {
        let policy = Policy {
            id: format!("policy_{}", i),
            description: format!("Test policy {}", i),
            effect: if i % 2 == 0 { PolicyEffect::Allow } else { PolicyEffect::Deny },
            actions: vec![format!("action_{}", i % 5)],
            resources: vec![format!("resource:{}:*", i % 10)],
            condition: "".to_string(),
        };
        engine = engine.with_policy(policy);
    }

    // 50個の関係性を追加
    for i in 0..50 {
        let relation = RelationTuple {
            subject_id: format!("user:{}", i),
            relation: if i % 3 == 0 { "owner" } else { "member" }.to_string(),
            object_id: format!("resource:{}:{}", i % 10, i),
        };
        engine = engine.with_relation(relation);
    }

    // テスト用のプリンシパルとリソース
    let principal = Principal {
        id: "user:test".to_string(),
        attributes: HashMap::from([
            ("role".to_string(), "admin".to_string()),
            ("department".to_string(), "engineering".to_string()),
        ]),
    };

    let resource = Resource {
        id: "resource:5:test_doc".to_string(),
        attributes: HashMap::from([
            ("resource_type".to_string(), "document".to_string()),
            ("sensitivity".to_string(), "high".to_string()),
        ]),
    };

    let context = AuthContext {
        principal: &principal,
        action: "read",
        resource: &resource,
        environment: HashMap::from([
            ("time_of_day".to_string(), "business_hours".to_string()),
            ("location".to_string(), "office".to_string()),
        ]),
    };

    (engine, principal, resource, context)
}

fn bench_auth_engine_creation(c: &mut Criterion) {
    c.bench_function("pure_auth_engine_creation", |b| {
        b.iter(|| {
            let engine = PureAuthEngine::new();
            black_box(engine);
        });
    });
}

fn bench_policy_addition(c: &mut Criterion) {
    let base_engine = PureAuthEngine::new();

    c.bench_function("policy_addition", |b| {
        b.iter(|| {
            let policy = Policy {
                id: "test_policy".to_string(),
                description: "Test policy".to_string(),
                effect: PolicyEffect::Allow,
                actions: vec!["read".to_string()],
                resources: vec!["document:*".to_string()],
                condition: "".to_string(),
            };

            let new_engine = base_engine.with_policy(policy);
            black_box(new_engine);
        });
    });
}

fn bench_relation_addition(c: &mut Criterion) {
    let base_engine = PureAuthEngine::new();

    c.bench_function("relation_addition", |b| {
        b.iter(|| {
            let relation = RelationTuple {
                subject_id: "user:alice".to_string(),
                relation: "owner".to_string(),
                object_id: "document:doc1".to_string(),
            };

            let new_engine = base_engine.with_relation(relation);
            black_box(new_engine);
        });
    });
}

fn bench_authorization_evaluation(c: &mut Criterion) {
    let (engine, principal, resource, context) = create_test_data();

    c.bench_function("authorization_evaluation", |b| {
        b.iter(|| {
            let decision = engine.evaluate(context.clone());
            black_box(decision);
        });
    });
}

fn bench_bulk_policy_operations(c: &mut Criterion) {
    c.bench_function("bulk_policy_operations_100", |b| {
        b.iter(|| {
            let mut engine = PureAuthEngine::new();

            // 100個のポリシーを順次追加
            for i in 0..100 {
                let policy = Policy {
                    id: format!("policy_{}", i),
                    description: format!("Test policy {}", i),
                    effect: PolicyEffect::Allow,
                    actions: vec![format!("action_{}", i % 5)],
                    resources: vec![format!("resource:{}:*", i % 10)],
                    condition: "".to_string(),
                };
                engine = engine.with_policy(policy);
            }

            black_box(engine);
        });
    });
}

fn bench_api_request_processing(c: &mut Criterion) {
    let processor = PureApiProcessor::new();

    let request_body = br#"{
        "request_id": "bench-test-123",
        "targets": [
            {
                "DefRef": {
                    "name": "test_def",
                    "version": "1.0.0"
                }
            }
        ],
        "context": {
            "available_defs": {},
            "graph_state": null,
            "tx_log": null,
            "environment": {
                "user": "test_user"
            },
            "limits": {
                "max_time_seconds": 30,
                "max_memory_mb": 512
            }
        },
        "options": {
            "mode": "Normal",
            "parallel": true,
            "validate": true,
            "track_provenance": true,
            "collect_witnesses": true,
            "timeout_seconds": 30,
            "max_memory_mb": 512
        }
    }"#;

    let headers = HashMap::from([
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Authorization".to_string(), "Bearer test_token".to_string()),
    ]);

    c.bench_function("api_request_processing", |b| {
        b.iter(|| {
            let result = processor.http_request_to_api_request("POST", "/api/execute", request_body, &headers);
            black_box(result);
        });
    });
}

fn bench_memory_usage_comparison(c: &mut Criterion) {
    c.bench_function("memory_usage_clone_vs_copy_on_write", |b| {
        b.iter(|| {
            // 初期エンジン作成
            let base_engine = PureAuthEngine::new();

            // ポリシーを追加してCopy-on-Write
            let policy = Policy {
                id: "memory_test_policy".to_string(),
                description: "Memory test policy".to_string(),
                effect: PolicyEffect::Allow,
                actions: vec!["read".to_string()],
                resources: vec!["test:*".to_string()],
                condition: "".to_string(),
            };

            let cow_engine = base_engine.with_policy(policy);

            // メモリ使用量を測定するためにデータを保持
            black_box((base_engine, cow_engine));
        });
    });
}

fn bench_determinism_verification(c: &mut Criterion) {
    let (engine, principal, resource, context) = create_test_data();

    c.bench_function("determinism_verification_1000", |b| {
        b.iter(|| {
            let mut results = Vec::new();

            // 1000回同じ評価を実行して決定論性を確認
            for _ in 0..1000 {
                let decision = engine.evaluate(context.clone());
                results.push(decision);
            }

            // 全ての結果が同じであることを確認
            let first = results[0];
            let all_same = results.iter().all(|&d| d == first);

            assert!(all_same, "Authorization evaluation must be deterministic");
            black_box(results);
        });
    });
}

criterion_group!(
    benches,
    bench_auth_engine_creation,
    bench_policy_addition,
    bench_relation_addition,
    bench_authorization_evaluation,
    bench_bulk_policy_operations,
    bench_api_request_processing,
    bench_memory_usage_comparison,
    bench_determinism_verification,
);
criterion_main!(benches);

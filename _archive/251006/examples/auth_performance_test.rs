//! 純粋関数型認可エンジンのパフォーマンステスト
//!
//! このプログラムは、PureAuthEngineのパフォーマンスを測定し、
//! Copy-on-Writeのオーバーヘッドを評価します。

use kotoba_auth::{PureAuthEngine, AuthContext, Principal, Resource, Decision, Policy, PolicyEffect, RelationTuple};
use std::collections::HashMap;
use std::time::{Duration, Instant};

fn main() {
    println!("🔐 Pure Functional Authorization Engine Performance Test");
    println!("======================================================");

    // 1. エンジン作成パフォーマンス
    println!("\n1️⃣ Engine Creation Performance");
    println!("================================");
    test_engine_creation();

    // 2. ポリシー追加パフォーマンス
    println!("\n2️⃣ Policy Addition Performance");
    println!("==============================");
    test_policy_addition();

    // 3. 認可評価パフォーマンス
    println!("\n3️⃣ Authorization Evaluation Performance");
    println!("======================================");
    test_authorization_evaluation();

    // 4. Copy-on-Write vs Mutable比較
    println!("\n4️⃣ Copy-on-Write vs Mutable Comparison");
    println!("====================================");
    test_copy_on_write_vs_mutable();

    // 5. 決定論性検証
    println!("\n5️⃣ Determinism Verification");
    println!("===========================");
    test_determinism();

    println!("\n✅ Performance tests completed!");
}

fn test_engine_creation() {
    let iterations = 1000;

    let start = Instant::now();
    for _ in 0..iterations {
        let _engine = PureAuthEngine::new();
    }
    let duration = start.elapsed();

    println!("Engine creation ({} iterations): {:.2} μs per operation",
             iterations, duration.as_micros() as f64 / iterations as f64);
}

fn test_policy_addition() {
    let iterations = 1000;
    let mut engine = PureAuthEngine::new();

    let start = Instant::now();
    for i in 0..iterations {
        let policy = Policy {
            id: format!("policy_{}", i),
            description: format!("Test policy {}", i),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };
        engine = engine.with_policy(policy);
    }
    let duration = start.elapsed();

    println!("Policy addition ({} iterations): {:.2} μs per operation",
             iterations, duration.as_micros() as f64 / iterations as f64);
    println!("Final engine has {} policies", engine.policies.len());
}

fn test_authorization_evaluation() {
    // 大規模なポリシーセットを作成
    let mut engine = PureAuthEngine::new();
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

    // 関係性を追加
    for i in 0..50 {
        let relation = RelationTuple {
            subject_id: format!("user:{}", i),
            relation: "owner".to_string(),
            object_id: format!("resource:{}:{}", i % 10, i),
        };
        engine = engine.with_relation(relation);
    }

    // テスト用のコンテキスト
    let principal = Principal {
        id: "user:test".to_string(),
        attributes: HashMap::new(),
    };

    let resource = Resource {
        id: "resource:5:test_doc".to_string(),
        attributes: HashMap::from([("resource_type".to_string(), "document".to_string())]),
    };

    let context = AuthContext {
        principal: &principal,
        action: "read",
        resource: &resource,
        environment: HashMap::new(),
    };

    // パフォーマンス測定
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _decision = engine.evaluate(context.clone());
    }
    let duration = start.elapsed();

    println!("Authorization evaluation ({} iterations): {:.2} μs per operation",
             iterations, duration.as_micros() as f64 / iterations as f64);
    println!("Engine has {} policies and {} relations",
             engine.policies.len(), engine.relations.len());
}

fn test_copy_on_write_vs_mutable() {
    println!("Comparing Copy-on-Write vs traditional mutable approach...");

    // Copy-on-Writeアプローチ
    let cow_start = Instant::now();
    let mut cow_engine = PureAuthEngine::new();
    for i in 0..100 {
        let policy = Policy {
            id: format!("cow_policy_{}", i),
            description: format!("COW policy {}", i),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["document:*".to_string()],
            condition: "".to_string(),
        };
        cow_engine = cow_engine.with_policy(policy);
    }
    let cow_duration = cow_start.elapsed();

    // Mutableアプローチのシミュレーション（実際には実装していないので、理論的な比較）
    println!("Copy-on-Write approach: {:.2} μs for 100 policy additions", cow_duration.as_micros());
    println!("Note: Mutable approach would modify in-place, but lose immutability benefits");
    println!("COW provides thread safety and referential transparency at the cost of allocation");
}

fn test_determinism() {
    let mut engine = PureAuthEngine::new();

    // 同じポリシーを追加
    let policy = Policy {
        id: "determinism_test".to_string(),
        description: "Determinism test policy".to_string(),
        effect: PolicyEffect::Allow,
        actions: vec!["read".to_string()],
        resources: vec!["test:*".to_string()],
        condition: "".to_string(),
    };
    engine = engine.with_policy(policy);

    let principal = Principal {
        id: "user:test".to_string(),
        attributes: HashMap::new(),
    };

    let resource = Resource {
        id: "test:resource".to_string(),
        attributes: HashMap::new(),
    };

    let context = AuthContext {
        principal: &principal,
        action: "read",
        resource: &resource,
        environment: HashMap::new(),
    };

    // 複数回評価して結果が常に同じであることを確認
    let iterations = 1000;
    let mut results = Vec::new();

    let start = Instant::now();
    for _ in 0..iterations {
        let decision = engine.evaluate(context.clone());
        results.push(decision);
    }
    let duration = start.elapsed();

    // すべての結果が同じであることを確認
    let first = results[0];
    let all_same = results.iter().all(|&d| d == first);

    println!("Determinism test ({} evaluations): {}", iterations, if all_same { "PASSED" } else { "FAILED" });
    println!("All evaluations returned: {:?}", first);
    println!("Average time per evaluation: {:.2} μs", duration.as_micros() as f64 / iterations as f64);

    if !all_same {
        println!("❌ Determinism violation detected!");
        return;
    }

    println!("✅ Pure functions are deterministic and predictable");
}

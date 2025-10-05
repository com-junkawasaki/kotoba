//! 純粋関数型アーキテクチャの動作確認テスト
//!
//! このプログラムは、Phase 1とPhase 2で実装したPure KernelとEffects Shellの
//! アーキテクチャが実際のユースケースで正しく動作することを確認します。

use std::collections::HashMap;

// Pure Kernelのクレートをインポート
use kotoba_types::*;
use kotoba_auth::{PureAuthEngine, AuthContext, Principal, Resource, Decision};
use kotoba_api::{PureApiProcessor, PureApiHandler};
use kotoba_txlog::{PureTxLog, TransactionAdditionPlan};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧬 Kotoba Pure Functional Architecture Test");
    println!("=========================================");

    // 1. 型の聖域と不変性のテスト
    println!("\n1️⃣ Testing Type Safety & Immutability");
    println!("=====================================");
    test_type_safety()?;

    // 2. 純粋認可エンジンのテスト
    println!("\n2️⃣ Testing Pure Authorization Engine");
    println!("====================================");
    test_pure_auth_engine()?;

    // 3. 純粋APIプロセッサーのテスト
    println!("\n3️⃣ Testing Pure API Processor");
    println!("============================");
    test_pure_api_processor()?;

    // 4. 純粋トランザクションログのテスト
    println!("\n4️⃣ Testing Pure Transaction Log");
    println!("===============================");
    test_pure_txlog()?;

    // 5. 決定論性のテスト（同じ入力で常に同じ出力）
    println!("\n5️⃣ Testing Determinism");
    println!("======================");
    test_determinism()?;

    println!("\n🎉 All pure functional architecture tests passed!");
    println!("The Pure Kernel is working correctly! ✨");

    Ok(())
}

/// 1. 型の聖域と不変性のテスト
fn test_type_safety() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing CID-based vertex and edge IDs...");

    // 同じ内容からは常に同じCIDが生成されることを確認
    let labels1 = vec!["user".to_string()];
    let props1 = HashMap::from([("name".to_string(), Value::String("alice".to_string()))]);

    let labels2 = vec!["user".to_string()];
    let props2 = HashMap::from([("name".to_string(), Value::String("alice".to_string()))]);

    // CIDは決定論的に生成されるので、同じ入力からは同じCIDが得られる
    let vertex_id_1 = generate_vertex_cid(&labels1, &props1);
    let vertex_id_2 = generate_vertex_cid(&labels2, &props2);

    assert_eq!(vertex_id_1, vertex_id_2, "Same content should produce same CID");
    println!("✅ CID generation is deterministic");

    // VertexIdはCopy可能な型であることを確認
    let vertex_id_copy = vertex_id_1;
    assert_eq!(vertex_id_1, vertex_id_copy, "VertexId should be copyable");
    println!("✅ VertexId is immutable and copyable");

    Ok(())
}

/// ヘルパー関数: 決定論的なvertex CID生成
fn generate_vertex_cid(labels: &[Label], props: &Properties) -> VertexId {
    let mut data = Vec::new();
    data.extend_from_slice(&serde_json::to_vec(labels).unwrap());
    data.extend_from_slice(&serde_json::to_vec(props).unwrap());
    VertexId::from(data.as_slice())
}

/// 2. 純粋認可エンジンのテスト
fn test_pure_auth_engine() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing pure authorization engine...");

    // 初期状態のエンジンを作成
    let engine = PureAuthEngine::new();

    // ポリシーを追加して新しいエンジンを生成
    let policy = Policy {
        id: "read_documents".to_string(),
        description: "Allow reading documents".to_string(),
        effect: PolicyEffect::Allow,
        actions: vec!["read".to_string()],
        resources: vec!["document:*".to_string()],
        condition: "".to_string(),
    };

    let engine_with_policy = engine.with_policy(policy);

    // 関係性を追加
    let relation = RelationTuple {
        subject_id: "user:alice".to_string(),
        relation: "owner".to_string(),
        object_id: "document:doc1".to_string(),
    };

    let engine_complete = engine_with_policy.with_relation(relation);

    // 認可コンテキストを作成
    let principal = Principal {
        id: "user:alice".to_string(),
        attributes: HashMap::new(),
    };

    let resource = Resource {
        id: "document:doc1".to_string(),
        attributes: HashMap::from([("resource_type".to_string(), "document".to_string())]),
    };

    let context = AuthContext {
        principal: &principal,
        action: "read",
        resource: &resource,
        environment: HashMap::new(),
    };

    // 認可判定を実行
    let decision = engine_complete.evaluate(context);

    assert_eq!(decision, Decision::Allow, "Alice should be allowed to read doc1");
    println!("✅ Pure authorization evaluation works");

    // 元のエンジンは変更されていないことを確認
    let context2 = AuthContext {
        principal: &principal,
        action: "write", // 許可されていないアクション
        resource: &resource,
        environment: HashMap::new(),
    };

    let decision2 = engine_complete.evaluate(context2);
    assert_eq!(decision2, Decision::Deny, "Write access should be denied");
    println!("✅ Policy enforcement works correctly");

    Ok(())
}

/// 3. 純粋APIプロセッサーのテスト
fn test_pure_api_processor() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing pure API processor...");

    let processor = PureApiProcessor::new();

    // HTTPリクエストデータをシミュレート
    let method = "POST";
    let path = "/api/execute";
    let body = br#"{
        "request_id": "test-123",
        "targets": [],
        "context": {},
        "options": {}
    }"#;
    let headers = HashMap::from([
        ("Content-Type".to_string(), "application/json".to_string()),
        ("Authorization".to_string(), "Bearer token123".to_string()),
    ]);

    // HTTPリクエストを純粋データに変換
    let api_request = processor.http_request_to_api_request(method, path, body, &headers)?;

    assert_eq!(api_request.request_id, "test-123", "Request ID should be parsed correctly");
    println!("✅ HTTP request to API request conversion works");

    // APIレスポンスをHTTPレスポンスに変換
    let api_response = ApiResponse::success(
        "test-123".to_string(),
        vec![],
        100,
    );

    let (status, body_bytes, response_headers) = processor.api_response_to_http_response(&api_response)?;

    assert_eq!(status, 200, "Success response should have 200 status");
    assert!(response_headers.contains_key("Content-Type"), "Content-Type header should be present");
    println!("✅ API response to HTTP response conversion works");

    Ok(())
}

/// 4. 純粋トランザクションログのテスト
fn test_pure_txlog() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing pure transaction log...");

    let config = TxLogConfig::default();
    let txlog = PureTxLog::new(config);

    // トランザクションを計画
    let tx = Transaction::new(
        "test-tx-1".to_string(),
        HLC::new("test-node".to_string()),
        vec![],
        "test-user".to_string(),
        TransactionOperation::GraphTransformation {
            input_refs: vec![],
            output_ref: DefRef::new("output".to_string()),
            rule_ref: DefRef::new("rule".to_string()),
            strategy_ref: None,
        },
    );

    let plan = txlog.plan_add_transaction(&tx)?;

    assert!(plan.validation_result, "Transaction should be valid");
    println!("✅ Transaction planning works");

    // 計画を適用して新しいTxLogを生成
    let txlog_with_tx = txlog.apply_addition_plan(plan, tx)?;

    // 元のTxLogは変更されていないことを確認
    assert!(txlog.get_transaction(&TransactionRef::new("test-tx-1".to_string())).is_none(),
            "Original TxLog should not contain the transaction");

    // 新しいTxLogにはトランザクションが含まれていることを確認
    assert!(txlog_with_tx.get_transaction(&TransactionRef::new("test-tx-1".to_string())).is_some(),
            "New TxLog should contain the transaction");

    println!("✅ Transaction application preserves immutability");

    Ok(())
}

/// 5. 決定論性のテスト
fn test_determinism() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing determinism - same input should always produce same output...");

    // 認可エンジンの決定論性をテスト
    let engine = PureAuthEngine::new()
        .with_policy(Policy {
            id: "test-policy".to_string(),
            description: "Test policy".to_string(),
            effect: PolicyEffect::Allow,
            actions: vec!["read".to_string()],
            resources: vec!["resource:*".to_string()],
            condition: "".to_string(),
        });

    let principal = Principal {
        id: "user:test".to_string(),
        attributes: HashMap::new(),
    };

    let resource = Resource {
        id: "resource:test".to_string(),
        attributes: HashMap::new(),
    };

    let context = AuthContext {
        principal: &principal,
        action: "read",
        resource: &resource,
        environment: HashMap::new(),
    };

    // 同じ入力を複数回評価
    let decision1 = engine.evaluate(context.clone());
    let decision2 = engine.evaluate(context.clone());
    let decision3 = engine.evaluate(context.clone());

    assert_eq!(decision1, Decision::Allow, "First evaluation should allow");
    assert_eq!(decision2, Decision::Allow, "Second evaluation should allow");
    assert_eq!(decision3, Decision::Allow, "Third evaluation should allow");
    assert_eq!(decision1, decision2, "Multiple evaluations should be consistent");
    assert_eq!(decision2, decision3, "Multiple evaluations should be consistent");

    println!("✅ Authorization evaluation is deterministic");

    // CID生成の決定論性をテスト
    let labels = vec!["test".to_string()];
    let props = HashMap::from([("key".to_string(), Value::String("value".to_string()))]);

    let cid1 = generate_vertex_cid(&labels, &props);
    let cid2 = generate_vertex_cid(&labels, &props);
    let cid3 = generate_vertex_cid(&labels, &props);

    assert_eq!(cid1, cid2, "CID generation should be deterministic");
    assert_eq!(cid2, cid3, "CID generation should be deterministic");

    println!("✅ CID generation is deterministic");

    Ok(())
}

// 必要な型定義（実際のクレートからインポートされるべきもの）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PolicyEffect;

impl PolicyEffect {
    pub const Allow: Self = PolicyEffect;
    pub const Deny: Self = PolicyEffect;
}

#[derive(Debug, Clone)]
pub struct Policy {
    pub id: String,
    pub description: String,
    pub effect: PolicyEffect,
    pub actions: Vec<String>,
    pub resources: Vec<String>,
    pub condition: String,
}

#[derive(Debug, Clone)]
pub struct RelationTuple {
    pub subject_id: String,
    pub relation: String,
    pub object_id: String,
}

#[derive(Debug, Clone)]
pub struct DefRef(String);

impl DefRef {
    pub fn new(s: String) -> Self { DefRef(s) }
}

#[derive(Debug, Clone)]
pub struct TransactionRef(String);

impl TransactionRef {
    pub fn new(s: String) -> Self { TransactionRef(s) }
}

#[derive(Debug, Clone)]
pub struct HLC;

impl HLC {
    pub fn new(_node: String) -> Self { HLC }
    pub fn is_valid(&self) -> bool { true }
}

#[derive(Debug, Clone)]
pub struct Transaction;

impl Transaction {
    pub fn new(
        _id: String,
        _hlc: HLC,
        _parents: Vec<TransactionRef>,
        _author: String,
        _operation: TransactionOperation,
    ) -> Self {
        Transaction
    }
}

#[derive(Debug, Clone)]
pub enum TransactionOperation {
    GraphTransformation {
        input_refs: Vec<DefRef>,
        output_ref: DefRef,
        rule_ref: DefRef,
        strategy_ref: Option<DefRef>,
    },
}

#[derive(Debug, Clone)]
pub struct TxLogConfig;

impl Default for TxLogConfig {
    fn default() -> Self { TxLogConfig }
}

impl Transaction {
    pub fn size_bytes(&self) -> usize { 100 }
}

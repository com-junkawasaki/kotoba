//! ストレージ操作のパフォーマンスベンチマーク

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use kotoba_core::*;
use kotoba_graph::*;
use kotoba_storage::*;
use std::collections::HashMap;
use uuid::Uuid;

/// テスト用データの生成
fn generate_test_data(size: usize) -> Vec<(String, Vec<u8>)> {
    (0..size)
        .map(|i| {
            let key = format!("test_key_{}", i);
            let value = format!("test_value_{}_with_some_additional_data_to_make_it_larger", i)
                .as_bytes()
                .to_vec();
            (key, value)
        })
        .collect()
}

/// テスト用グラフの生成
fn create_storage_test_graph(size: usize) -> Graph {
    let mut graph = Graph::empty();

    for i in 0..size {
        let vertex = graph.add_vertex(VertexData {
            id: uuid::Uuid::new_v4(),
            labels: vec![format!("Node{}", i % 10)],
            props: HashMap::from([
                ("id".to_string(), Value::Int(i as i64)),
                ("data".to_string(), Value::String(format!("data_{}", i))),
                ("timestamp".to_string(), Value::Int((i * 1000) as i64)),
            ]),
        });

        // エッジを追加してグラフを複雑化
        if i > 0 {
            let prev_vertex = graph.vertices.keys().nth(i - 1).cloned().unwrap();
            graph.add_edge(EdgeData {
                id: uuid::Uuid::new_v4(),
                src: vertex,
                dst: prev_vertex,
                label: "LINKS".to_string(),
                props: HashMap::new(),
            });
        }
    }

    graph
}

/// MVCCトランザクション作成ベンチマーク
fn bench_mvcc_transaction_creation(c: &mut Criterion) {
    let mvcc = storage::MVCCManager::new();

    c.bench_function("mvcc_transaction_creation", |b| {
        b.iter(|| {
            let tx_id = mvcc.begin_tx();
            black_box(tx_id);
        });
    });
}

/// MVCCコミットベンチマーク
fn bench_mvcc_commit(c: &mut Criterion) {
    let mvcc = storage::MVCCManager::new();

    c.bench_function("mvcc_commit", |b| {
        b.iter(|| {
            let tx_id = mvcc.begin_tx();
            let result = mvcc.commit_tx(&tx_id);
            black_box(result);
        });
    });
}

/// Merkleハッシュ計算ベンチマーク
fn bench_merkle_hashing(c: &mut Criterion) {
    let mut merkle = storage::MerkleDAG::new();
    let graph = create_storage_test_graph(100);

    c.bench_function("merkle_hashing", |b| {
        b.iter(|| {
            let hash_result = merkle.hash_graph(&graph);
            let _hash = match hash_result {
                Ok(hash) => hash,
                Err(_) => return,
            };
            black_box(_hash);
            black_box(hash);
        });
    });
}

/// Merkleノード格納ベンチマーク
fn bench_merkle_storage(c: &mut Criterion) {
    let mut merkle = storage::MerkleDAG::new();
    let test_data = b"test data for merkle storage benchmark";

    c.bench_function("merkle_storage", |b| {
        b.iter(|| {
            let hash = merkle.store(test_data, vec![]);
            black_box(hash);
        });
    });
}

/// LSMツリーデータ挿入ベンチマーク
fn bench_lsm_insert(c: &mut Criterion) {
    let test_data = generate_test_data(1000);

    c.bench_function("lsm_insert", |b| {
        b.iter(|| {
            let mut lsm = storage::LSMTree::new(std::path::PathBuf::from("/tmp/kotoba_lsm_bench"), 100);
            for (key, value) in &test_data {
                let _result = lsm.put(key.clone(), value.clone());
            black_box(_result);
            }
            black_box(lsm);
        });
    });
}

/// LSMツリーデータ検索ベンチマーク
fn bench_lsm_get(c: &mut Criterion) {
    let test_data = generate_test_data(1000);
    let mut lsm = storage::LSMTree::new(std::path::PathBuf::from("/tmp/kotoba_lsm_bench"), 100);

    // データを挿入
    for (key, value) in &test_data {
        let _ = lsm.put(key.clone(), value.clone());
    }

    c.bench_function("lsm_get", |b| {
        b.iter(|| {
            for (key, _) in &test_data {
                let result = lsm.get(key);
                black_box(result);
            }
        });
    });
}

/// LSMツリーフラッシュベンチマーク
fn bench_lsm_flush(c: &mut Criterion) {
    c.bench_function("lsm_flush", |b| {
        b.iter(|| {
            let mut lsm = storage::LSMTree::new(std::path::PathBuf::from("/tmp/kotoba_lsm_bench"), 10); // 小さい閾値で頻繁にフラッシュ

            // 多くのデータを挿入してフラッシュをトリガー
            for i in 0..50 {
                let key = format!("flush_key_{}", i);
                let value = format!("flush_value_{}_with_padding_data_to_increase_size", i).as_bytes().to_vec();
                let _result = lsm.put(key, value);
                black_box(_result);
            }

            black_box(lsm);
        });
    });
}

/// LSMツリー圧縮ベンチマーク
fn bench_lsm_compaction(c: &mut Criterion) {
    c.bench_function("lsm_compaction", |b| {
        b.iter(|| {
            let mut lsm = storage::LSMTree::new(std::path::PathBuf::from("/tmp/kotoba_lsm_bench"), 5); // 非常に小さい閾値

            // 多くのSSTableを作成
            for i in 0..100 {
                let key = format!("compact_key_{}", i);
                let value = format!("compact_value_{}", i).as_bytes().to_vec();
                let _result = lsm.put(key, value);
                black_box(_result);
            }

            black_box(lsm);
        });
    });
}

/// グラフ永続化ベンチマーク
fn bench_graph_persistence(c: &mut Criterion) {
    let graph = create_storage_test_graph(1000);
    let mut merkle = storage::MerkleDAG::new();

    c.bench_function("graph_persistence", |b| {
        b.iter(|| {
            let hash_result = merkle.hash_graph(&graph);
            match hash_result {
                Ok(hash) => {
                    let node = merkle.get(&hash);
                    black_box((hash, node));
                }
                Err(_) => black_box(()),
            }
        });
    });
}

/// バージョン管理ベンチマーク
fn bench_version_management(c: &mut Criterion) {
    let mut version_manager = storage::GraphVersion::new();
    let mut graph = create_storage_test_graph(100);

    c.bench_function("version_management", |b| {
        b.iter(|| {
            // グラフを変更
            let _new_vertex = graph.add_vertex(VertexData {
                id: uuid::Uuid::new_v4(),
                labels: vec!["NewNode".to_string()],
                props: HashMap::new(),
            });

            // バージョンをコミット
            let hash = version_manager.commit(&graph);
            black_box(hash);
        });
    });
}

/// 同時実行制御ベンチマーク
fn bench_concurrency_control(c: &mut Criterion) {
    use std::sync::Arc;
    use parking_lot::RwLock;

    let graph = Arc::new(RwLock::new(create_storage_test_graph(1000)));
    let mvcc = Arc::new(storage::MVCCManager::new());

    c.bench_function("concurrency_control", |b| {
        b.iter(|| {
            let graph_clone = Arc::clone(&graph);
            let mvcc_clone = Arc::clone(&mvcc);

            // 複数のトランザクションをシミュレート
            let tx_id = mvcc_clone.begin_tx();

            // グラフを読み取り
            {
                let graph_read = graph_clone.read();
                let vertex_count = graph_read.vertex_count();
                black_box(vertex_count);
            }

            // トランザクションをコミット
            let _ = mvcc_clone.commit_tx(&tx_id);

            black_box(tx_id);
        });
    });
}

/// 大規模データセット永続化ベンチマーク
fn bench_large_dataset_persistence(c: &mut Criterion) {
    let graph = create_storage_test_graph(10000);
    let mut merkle = storage::MerkleDAG::new();

    c.bench_function("large_dataset_persistence", |b| {
        b.iter(|| {
            let hash_result = merkle.hash_graph(&graph);
            let _hash = match hash_result {
                Ok(hash) => hash,
                Err(_) => return,
            };
            black_box(_hash);
            black_box(hash);
        });
    });
}

/// スナップショット作成ベンチマーク
fn bench_snapshot_creation(c: &mut Criterion) {
    let mvcc = storage::MVCCManager::new();
    let graph_ref = GraphRef::new(create_storage_test_graph(1000));

    c.bench_function("snapshot_creation", |b| {
        b.iter(|| {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();

            mvcc.put_snapshot(timestamp, graph_ref.clone());
            black_box(timestamp);
        });
    });
}

/// スナップショット取得ベンチマーク
fn bench_snapshot_retrieval(c: &mut Criterion) {
    let mvcc = storage::MVCCManager::new();
    let graph_ref = GraphRef::new(create_storage_test_graph(1000));

    // スナップショットを作成
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    mvcc.put_snapshot(timestamp, graph_ref);

    c.bench_function("snapshot_retrieval", |b| {
        b.iter(|| {
            let snapshot = mvcc.get_snapshot(timestamp);
            black_box(snapshot);
        });
    });
}

criterion_group!(
    benches,
    bench_mvcc_transaction_creation,
    bench_mvcc_commit,
    bench_merkle_hashing,
    bench_merkle_storage,
    bench_lsm_insert,
    bench_lsm_get,
    bench_lsm_flush,
    bench_lsm_compaction,
    bench_graph_persistence,
    bench_version_management,
    bench_concurrency_control,
    bench_large_dataset_persistence,
    bench_snapshot_creation,
    bench_snapshot_retrieval
);

criterion_main!(benches);

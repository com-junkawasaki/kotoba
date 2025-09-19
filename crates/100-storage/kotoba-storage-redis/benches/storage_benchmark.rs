//! Benchmarks for kotoba-storage-redis
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::sync::Arc;
use tokio::runtime::Runtime;
use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;

fn create_benchmark_store() -> RedisStore {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            key_prefix: "bench:storage".to_string(),
            enable_compression: true,
            connection_pool_size: 20,
            enable_metrics: false, // Disable metrics for benchmarks
            ..Default::default()
        };

        RedisStore::new(config).await.expect("Failed to create benchmark store")
    })
}

fn cleanup_benchmark_store(store: &RedisStore) {
    let rt = Runtime::new().unwrap();
    rt.block_on(async {
        // Clean up benchmark keys
        let pattern = format!("{}*", store.config.key_prefix);
        if let Ok(mut conn) = store.get_connection().await {
            let keys: Vec<String> = conn.keys(&pattern).await.unwrap_or_default();
            if !keys.is_empty() {
                let _: () = conn.del(&keys).await.unwrap_or(());
            }
            store.return_connection(conn).await;
        }
    });
}

fn bench_put_small_value(c: &mut Criterion) {
    let store = create_benchmark_store();
    cleanup_benchmark_store(&store);

    let rt = Runtime::new().unwrap();

    c.bench_function("redis_put_small", |b| {
        b.iter(|| {
            rt.block_on(async {
                let key = format!("bench_key_{}", black_box(0)).into_bytes();
                let value = black_box(b"small_value");
                store.put(&key, value).await.expect("Put should succeed");
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_get_small_value(c: &mut Criterion) {
    let store = create_benchmark_store();
    let rt = Runtime::new().unwrap();

    // Pre-populate data
    rt.block_on(async {
        let key = b"bench_get_key";
        let value = b"small_value_for_get";
        store.put(key, value).await.expect("Pre-put should succeed");
    });

    c.bench_function("redis_get_small", |b| {
        b.iter(|| {
            rt.block_on(async {
                let key = black_box(b"bench_get_key");
                let _result = store.get(key).await.expect("Get should succeed");
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_put_large_value(c: &mut Criterion) {
    let store = create_benchmark_store();
    cleanup_benchmark_store(&store);

    let rt = Runtime::new().unwrap();
    let large_value = vec![42u8; 64 * 1024]; // 64KB value

    c.bench_function("redis_put_large", |b| {
        b.iter(|| {
            rt.block_on(async {
                let key = format!("bench_large_key_{}", black_box(0)).into_bytes();
                let value = black_box(&large_value);
                store.put(&key, value).await.expect("Put large should succeed");
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_get_large_value(c: &mut Criterion) {
    let store = create_benchmark_store();
    let rt = Runtime::new().unwrap();

    // Pre-populate large data
    let large_value = vec![42u8; 64 * 1024]; // 64KB value
    rt.block_on(async {
        let key = b"bench_get_large_key";
        store.put(key, &large_value).await.expect("Pre-put large should succeed");
    });

    c.bench_function("redis_get_large", |b| {
        b.iter(|| {
            rt.block_on(async {
                let key = black_box(b"bench_get_large_key");
                let _result = store.get(key).await.expect("Get large should succeed");
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_put_batch(c: &mut Criterion) {
    let store = Arc::new(create_benchmark_store());
    cleanup_benchmark_store(&store);

    let rt = Runtime::new().unwrap();

    c.bench_function("redis_put_batch_100", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = Arc::clone(&store);
                let mut handles = vec![];

                // Batch of 100 puts
                for i in 0..100 {
                    let store_clone = Arc::clone(&store);
                    let handle = tokio::spawn(async move {
                        let key = format!("batch_key_{}", i).into_bytes();
                        let value = format!("batch_value_{}", i).into_bytes();
                        store_clone.put(&key, &value).await.expect("Batch put should succeed");
                    });
                    handles.push(handle);
                }

                // Wait for all to complete
                for handle in handles {
                    handle.await.expect("Batch handle should succeed");
                }
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_scan_prefix(c: &mut Criterion) {
    let store = create_benchmark_store();
    let rt = Runtime::new().unwrap();

    // Pre-populate data for scan
    rt.block_on(async {
        for i in 0..1000 {
            let key = format!("scan_test_key_{:04}", i).into_bytes();
            let value = format!("scan_test_value_{}", i).into_bytes();
            store.put(&key, &value).await.expect("Pre-populate should succeed");
        }
    });

    c.bench_function("redis_scan_prefix", |b| {
        b.iter(|| {
            rt.block_on(async {
                let _results = store.scan(black_box(b"scan_test_")).await.expect("Scan should succeed");
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_mixed_operations(c: &mut Criterion) {
    let store = Arc::new(create_benchmark_store());
    cleanup_benchmark_store(&store);

    let rt = Runtime::new().unwrap();

    c.bench_function("redis_mixed_ops_50_30_20", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = Arc::clone(&store);
                let mut handles = vec![];

                // 50% puts, 30% gets, 20% deletes
                for i in 0..100 {
                    let store_clone = Arc::clone(&store);
                    let operation = i % 10;

                    let handle = tokio::spawn(async move {
                        let key = format!("mixed_key_{}", i).into_bytes();
                        let value = format!("mixed_value_{}", i).into_bytes();

                        match operation {
                            0..=4 => { // 50% puts
                                store_clone.put(&key, &value).await.expect("Mixed put should succeed");
                            }
                            5..=7 => { // 30% gets
                                let _result = store_clone.get(&key).await.expect("Mixed get should succeed");
                            }
                            8..=9 => { // 20% deletes
                                let _ = store_clone.delete(&key).await.expect("Mixed delete should succeed");
                            }
                            _ => unreachable!(),
                        }
                    });
                    handles.push(handle);
                }

                // Wait for all to complete
                for handle in handles {
                    handle.await.expect("Mixed handle should succeed");
                }
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_connection_pooling(c: &mut Criterion) {
    let store = Arc::new(create_benchmark_store());
    cleanup_benchmark_store(&store);

    let rt = Runtime::new().unwrap();

    c.bench_function("redis_connection_pool_stress", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = Arc::clone(&store);
                let mut handles = vec![];

                // Stress test connection pooling with many concurrent operations
                for i in 0..200 {
                    let store_clone = Arc::clone(&store);
                    let handle = tokio::spawn(async move {
                        let key = format!("pool_key_{}", i).into_bytes();
                        let value = format!("pool_value_{}", i).into_bytes();

                        // Perform multiple operations per connection
                        store_clone.put(&key, &value).await.expect("Pool put should succeed");
                        let retrieved = store_clone.get(&key).await.expect("Pool get should succeed");
                        assert_eq!(retrieved, Some(value));
                        store_clone.delete(&key).await.expect("Pool delete should succeed");
                    });
                    handles.push(handle);
                }

                // Wait for all to complete
                for handle in handles {
                    handle.await.expect("Pool handle should succeed");
                }
            });
        });
    });

    cleanup_benchmark_store(&store);
}

fn bench_compression_thresholds(c: &mut Criterion) {
    let mut group = c.benchmark_group("redis_compression");

    // Test different compression thresholds
    let thresholds = [512, 1024, 2048, 4096];

    for &threshold in &thresholds {
        let store = create_benchmark_store();
        let rt = Runtime::new().unwrap();

        // Create a value that's larger than the threshold
        let value_size = threshold * 2;
        let large_value = vec![42u8; value_size];

        group.bench_with_input(
            format!("threshold_{}b", threshold),
            &threshold,
            |b, _threshold| {
                b.iter(|| {
                    rt.block_on(async {
                        let key = format!("compress_key_{}", black_box(0)).into_bytes();
                        let value = black_box(&large_value);
                        store.put(&key, value).await.expect("Compress put should succeed");

                        let retrieved = store.get(&key).await.expect("Compress get should succeed");
                        assert_eq!(retrieved, Some(large_value.clone()));
                    });
                });
            }
        );

        cleanup_benchmark_store(&store);
    }

    group.finish();
}

fn bench_memory_vs_redis(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_vs_redis");

    // Compare with in-memory store
    let memory_store = {
        use kotoba_memory::MemoryKeyValueStore;
        MemoryKeyValueStore::new()
    };

    let redis_store = create_benchmark_store();
    let rt = Runtime::new().unwrap();

    group.bench_function("memory_put", |b| {
        b.iter(|| {
            let key = format!("mem_key_{}", black_box(0)).into_bytes();
            let value = black_box(b"test_value");
            memory_store.put(&key, value).unwrap();
        });
    });

    group.bench_function("redis_put", |b| {
        b.iter(|| {
            rt.block_on(async {
                let key = format!("redis_key_{}", black_box(0)).into_bytes();
                let value = black_box(b"test_value");
                redis_store.put(&key, value).await.expect("Redis put should succeed");
            });
        });
    });

    group.finish();

    cleanup_benchmark_store(&redis_store);
}

criterion_group!(
    benches,
    bench_put_small_value,
    bench_get_small_value,
    bench_put_large_value,
    bench_get_large_value,
    bench_put_batch,
    bench_scan_prefix,
    bench_mixed_operations,
    bench_connection_pooling,
    bench_compression_thresholds,
    bench_memory_vs_redis
);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backend_switching_rocksdb() {
        // RocksDBバックエンドのテスト
        let config = StorageConfig {
            backend_type: BackendType::RocksDB,
            rocksdb_path: Some("./test_data".into()),
            ..Default::default()
        };

        let manager = StorageManager::new(config).await.unwrap();

        // バックエンドタイプが正しいか確認
        assert_eq!(manager.backend_type(), &BackendType::RocksDB);

        // 基本的な操作をテスト
        manager.put("test_key".to_string(), b"test_value".to_vec()).await.unwrap();
        let value = manager.get("test_key").await.unwrap();
        assert_eq!(value, Some(b"test_value".to_vec()));

        // クリーンアップ
        manager.clear().await.unwrap();
    }

    #[tokio::test]
    async fn test_backend_switching_redis() {
        // Redisが利用可能な場合のみテストを実行
        let redis_url = std::env::var("TEST_REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());

        let config = StorageConfig {
            backend_type: BackendType::Redis,
            redis_url: Some(redis_url.clone()),
            ..Default::default()
        };

        // Redis接続を試行
        match StorageManager::new(config).await {
            Ok(manager) => {
                // バックエンドタイプが正しいか確認
                assert_eq!(manager.backend_type(), &BackendType::Redis);

                // 基本的な操作をテスト
                manager.put("test_key".to_string(), b"test_value".to_vec()).await.unwrap();
                let value = manager.get("test_key").await.unwrap();
                assert_eq!(value, Some(b"test_value".to_vec()));

                // クリーンアップ
                manager.clear().await.unwrap();
            }
            Err(_) => {
                // Redisが利用できない場合はテストをスキップ
                println!("Redis not available, skipping Redis backend test");
            }
        }
    }

    #[tokio::test]
    async fn test_convenience_constructors() {
        // RocksDBの便利コンストラクタをテスト
        let manager = StorageManager::with_rocksdb("./test_data".into()).await.unwrap();
        assert_eq!(manager.backend_type(), &BackendType::RocksDB);

        // Redisの便利コンストラクタをテスト（利用可能な場合）
        let redis_url = "redis://localhost:6379".to_string();
        match StorageManager::with_upstash(redis_url).await {
            Ok(manager) => {
                assert_eq!(manager.backend_type(), &BackendType::Redis);
            }
            Err(_) => {
                println!("Redis not available, skipping convenience constructor test");
            }
        }
    }

    #[tokio::test]
    async fn test_storage_manager_interface_unified() {
        // 両バックエンドで同じインターフェースが使えることを確認
        let configs = vec![
            StorageConfig {
                backend_type: BackendType::RocksDB,
                rocksdb_path: Some("./test_data".into()),
                ..Default::default()
            },
            // Redisは利用可能な場合のみ
        ];

        for config in configs {
            match StorageManager::new(config).await {
                Ok(manager) => {
                    // 同じメソッドで操作可能
                    manager.put("unified_key".to_string(), b"unified_value".to_vec()).await.unwrap();
                    let exists = manager.exists("unified_key").await.unwrap();
                    assert!(exists);

                    let value = manager.get("unified_key").await.unwrap();
                    assert_eq!(value, Some(b"unified_value".to_vec()));

                    manager.delete("unified_key".to_string()).await.unwrap();
                    let exists_after_delete = manager.exists("unified_key").await.unwrap();
                    assert!(!exists_after_delete);

                    // クリーンアップ
                    manager.clear().await.unwrap();
                }
                Err(_) if config.backend_type == BackendType::RocksDB => {
                    panic!("RocksDB should always be available for testing");
                }
                Err(_) => {
                    // Redisが利用できない場合はスキップ
                    println!("Skipping test for unavailable backend: {:?}", config.backend_type);
                }
            }
        }
    }
}
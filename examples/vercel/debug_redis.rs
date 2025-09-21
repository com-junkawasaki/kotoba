//! Debug Redis connection and data storage

use kotoba_storage_redis::{RedisStore, RedisConfig};
use kotoba_storage::KeyValueStore;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("🔍 Debugging Redis connection and storage");

    // Create Redis store with same config as GraphQL API
    let config = RedisConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        key_prefix: "kotoba:graphql".to_string(),
        ..Default::default()
    };

    println!("📋 Config: {:?}", config);

    let store = Arc::new(RedisStore::new(config).await?);
    println!("✅ Redis store created successfully");

    // Test basic operations
    let test_key = b"debug:test:key";
    let test_value = b"{\"test\": \"data\"}";

    println!("💾 Testing PUT operation...");
    store.put(test_key, test_value).await?;
    println!("✅ PUT operation successful");

    println!("📖 Testing GET operation...");
    let retrieved = store.get(test_key).await?;
    match retrieved {
        Some(data) => {
            println!("✅ GET operation successful");
            println!("📄 Retrieved data: {:?}", String::from_utf8_lossy(&data));
        }
        None => println!("❌ GET operation failed - no data retrieved"),
    }

    // Check all keys with our prefix
    println!("🔍 Checking all keys with prefix 'kotoba:graphql:*'...");
    // Note: We can't directly scan in this trait, but we can check our test key
    let all_keys = vec![test_key];
    for key in all_keys {
        match store.get(key).await {
            Ok(Some(data)) => println!("🔑 Key {:?}: {:?}", key, String::from_utf8_lossy(&data)),
            Ok(None) => println!("🔑 Key {:?}: not found", key),
            Err(e) => println!("🔑 Key {:?}: error {:?}", key, e),
        }
    }

    // Test OCEL-like data storage
    println!("🏗️  Testing OCEL data storage...");
    let ocel_key = b"node:debug_ocel_order";
    let ocel_data = br#"{
        "ocel:type": "object",
        "ocel:oid": "debug_ocel_order",
        "ocel:object_type": "Order",
        "attributes": {
            "customer_id": "debug_customer",
            "amount": 100.0
        }
    }"#;

    store.put(ocel_key, ocel_data).await?;
    println!("✅ OCEL data stored");

    let retrieved_ocel = store.get(ocel_key).await?;
    match retrieved_ocel {
        Some(data) => println!("✅ OCEL data retrieved: {:?}", String::from_utf8_lossy(&data)),
        None => println!("❌ OCEL data not found"),
    }

    println!("🎉 Redis debugging complete!");
    Ok(())
}

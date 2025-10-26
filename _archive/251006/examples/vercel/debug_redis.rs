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

    // Test RedisGraphStore with proper GraphDB structures
    println!("🏗️  Testing RedisGraphStore with GraphDB structures...");
    use kotoba_storage_redis::{RedisStore, RedisConfig};
    use kotoba_storage::KeyValueStore;
    use kotoba_graphdb::{Node, Edge, PropertyValue};
    use std::collections::BTreeMap;
    use chrono::Utc;

    // Create RedisGraphStore-like key generation (simplified)
    let node_key = format!("nodes/{}", "test_order_001");
    let edge_key = format!("edges/{}", "test_event_001");

    // Test creating a GraphDB Node with proper key format
    let mut node_properties = BTreeMap::new();
    node_properties.insert("ocel:type".to_string(), PropertyValue::String("object".to_string()));
    node_properties.insert("ocel:oid".to_string(), PropertyValue::String("test_order_001".to_string()));
    node_properties.insert("ocel:object_type".to_string(), PropertyValue::String("Order".to_string()));
    node_properties.insert("customer_id".to_string(), PropertyValue::String("customer_123".to_string()));
    node_properties.insert("amount".to_string(), PropertyValue::Float(299.99));

    let test_node = Node {
        id: "test_order_001".to_string(),
        labels: vec!["Order".to_string(), "OCEL_Object".to_string()],
        properties: node_properties,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Test storing node with DAG-structured key
    let node_data = serde_json::to_string(&test_node)?;
    store.put(node_key.as_bytes(), node_data.as_bytes()).await?;
    println!("✅ GraphDB Node stored with DAG key: {}", node_key);

    // Test retrieving node
    let retrieved_node_data = store.get(node_key.as_bytes()).await?;
    match retrieved_node_data {
        Some(data) => {
            let retrieved_node: Node = serde_json::from_slice(&data)?;
            println!("✅ GraphDB Node retrieved: id={}, labels={:?}", retrieved_node.id, retrieved_node.labels);
        }
        None => println!("❌ GraphDB Node not found"),
    }

    // Test creating a GraphDB Edge
    let mut edge_properties = BTreeMap::new();
    edge_properties.insert("ocel:activity".to_string(), PropertyValue::String("Order Placed".to_string()));
    edge_properties.insert("ocel:timestamp".to_string(), PropertyValue::Date(Utc::now()));
    edge_properties.insert("user_agent".to_string(), PropertyValue::String("TestAgent/1.0".to_string()));

    let test_edge = Edge {
        id: "test_event_001".to_string(),
        from_node: "customer_123".to_string(),
        to_node: "test_order_001".to_string(),
        label: "PLACED_ORDER".to_string(),
        properties: edge_properties,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    // Test storing edge with DAG-structured key
    let edge_data = serde_json::to_string(&test_edge)?;
    store.put(edge_key.as_bytes(), edge_data.as_bytes()).await?;
    println!("✅ GraphDB Edge stored with DAG key: {}", edge_key);

    // Test retrieving edge
    let retrieved_edge_data = store.get(edge_key.as_bytes()).await?;
    match retrieved_edge_data {
        Some(data) => {
            let retrieved_edge: Edge = serde_json::from_slice(&data)?;
            println!("✅ GraphDB Edge retrieved: id={}, from={} to={}, label={}",
                     retrieved_edge.id, retrieved_edge.from_node, retrieved_edge.to_node, retrieved_edge.label);
        }
        None => println!("❌ GraphDB Edge not found"),
    }

    println!("🎉 Redis debugging complete!");
    Ok(())
}

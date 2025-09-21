//! Tests for Kotoba GraphQL API with OCEL evaluation

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_storage_redis::{RedisStore, RedisConfig};
    use kotoba_storage::KeyValueStore;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::test;

    async fn create_test_store() -> Arc<RedisStore> {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            key_prefix: "test:ocel".to_string(),
            ..Default::default()
        };

        Arc::new(RedisStore::new(config).await.unwrap())
    }

    async fn cleanup_test_data(store: &Arc<RedisStore>) {
        // Clean up test data
        let pattern = "test:ocel:*";
        // Note: In a real implementation, we'd need scan and delete
        // For now, we'll rely on Redis expiration or manual cleanup
    }

    #[test]
    async fn test_basic_crud_operations() {
        let store = create_test_store().await;

        // Test PUT operation
        let key = b"node:test_order_001";
        let value = br#"{"id": "test_order_001", "type": "order", "amount": 100.0}"#;

        store.put(key, value).await.expect("Failed to put data");

        // Test GET operation
        let retrieved = store.get(key).await.expect("Failed to get data");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), value);

        // Test DELETE operation
        store.delete(key).await.expect("Failed to delete data");

        // Verify deletion
        let retrieved_after_delete = store.get(key).await.expect("Failed to get after delete");
        assert!(retrieved_after_delete.is_none());

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_ocel_object_creation() {
        let store = create_test_store().await;

        // Create OCEL Object (Order)
        let order_id = "order_001";
        let order_data = json!({
            "ocel:type": "object",
            "ocel:oid": order_id,
            "ocel:object_type": "Order",
            "attributes": {
                "customer_id": "customer_123",
                "total_amount": 299.99,
                "currency": "USD",
                "status": "pending"
            },
            "timestamp": "2024-01-01T10:00:00Z"
        });

        let key = format!("object:{}", order_id);
        let serialized = serde_json::to_vec(&order_data).unwrap();

        store.put(key.as_bytes(), &serialized).await.expect("Failed to create OCEL object");

        // Verify creation
        let retrieved = store.get(key.as_bytes()).await.expect("Failed to retrieve OCEL object");
        assert!(retrieved.is_some());

        let retrieved_data: serde_json::Value = serde_json::from_slice(&retrieved.unwrap()).unwrap();
        assert_eq!(retrieved_data["ocel:oid"], order_id);
        assert_eq!(retrieved_data["ocel:object_type"], "Order");

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_ocel_event_creation() {
        let store = create_test_store().await;

        // Create OCEL Event (Order Placed)
        let event_id = "event_001";
        let event_data = json!({
            "ocel:type": "event",
            "ocel:eid": event_id,
            "ocel:activity": "Order Placed",
            "ocel:timestamp": "2024-01-01T10:00:00Z",
            "ocel:vmap": {
                "user_agent": "Mozilla/5.0",
                "ip_address": "192.168.1.1"
            },
            "ocel:omap": ["order_001", "customer_123"]
        });

        let key = format!("event:{}", event_id);
        let serialized = serde_json::to_vec(&event_data).unwrap();

        store.put(key.as_bytes(), &serialized).await.expect("Failed to create OCEL event");

        // Verify creation
        let retrieved = store.get(key.as_bytes()).await.expect("Failed to retrieve OCEL event");
        assert!(retrieved.is_some());

        let retrieved_data: serde_json::Value = serde_json::from_slice(&retrieved.unwrap()).unwrap();
        assert_eq!(retrieved_data["ocel:eid"], event_id);
        assert_eq!(retrieved_data["ocel:activity"], "Order Placed");
        assert_eq!(retrieved_data["ocel:omap"], json!(["order_001", "customer_123"]));

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_ocel_relationships() {
        let store = create_test_store().await;

        // Create Order Object
        let order_data = json!({
            "ocel:type": "object",
            "ocel:oid": "order_001",
            "ocel:object_type": "Order",
            "attributes": {
                "customer_id": "customer_123",
                "total_amount": 299.99
            }
        });
        store.put(b"object:order_001", &serde_json::to_vec(&order_data).unwrap()).await.unwrap();

        // Create Customer Object
        let customer_data = json!({
            "ocel:type": "object",
            "ocel:oid": "customer_123",
            "ocel:object_type": "Customer",
            "attributes": {
                "name": "John Doe",
                "email": "john@example.com"
            }
        });
        store.put(b"object:customer_123", &serde_json::to_vec(&customer_data).unwrap()).await.unwrap();

        // Create Order Placed Event connecting both objects
        let event_data = json!({
            "ocel:type": "event",
            "ocel:eid": "event_001",
            "ocel:activity": "Order Placed",
            "ocel:timestamp": "2024-01-01T10:00:00Z",
            "ocel:omap": ["order_001", "customer_123"]
        });
        store.put(b"event:event_001", &serde_json::to_vec(&event_data).unwrap()).await.unwrap();

        // Verify relationships
        let event_retrieved = store.get(b"event:event_001").await.unwrap().unwrap();
        let event_parsed: serde_json::Value = serde_json::from_slice(&event_retrieved).unwrap();
        let related_objects = event_parsed["ocel:omap"].as_array().unwrap();

        assert!(related_objects.contains(&json!("order_001")));
        assert!(related_objects.contains(&json!("customer_123")));

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_complex_ocel_process() {
        let store = create_test_store().await;

        // Create a complete OCEL process: Order → Payment → Shipment

        // Objects
        let objects = vec![
            ("order_001", "Order", json!({"amount": 299.99, "customer": "customer_123"})),
            ("customer_123", "Customer", json!({"name": "John Doe", "tier": "gold"})),
            ("payment_001", "Payment", json!({"method": "credit_card", "amount": 299.99})),
            ("shipment_001", "Shipment", json!({"carrier": "UPS", "tracking": "1Z999AA1234567890"})),
        ];

        for (oid, otype, attributes) in objects {
            let obj_data = json!({
                "ocel:type": "object",
                "ocel:oid": oid,
                "ocel:object_type": otype,
                "attributes": attributes
            });
            let key = format!("object:{}", oid);
            store.put(key.as_bytes(), &serde_json::to_vec(&obj_data).unwrap()).await.unwrap();
        }

        // Events
        let events = vec![
            ("event_001", "Order Created", "2024-01-01T10:00:00Z", vec!["order_001", "customer_123"]),
            ("event_002", "Payment Processed", "2024-01-01T10:05:00Z", vec!["order_001", "payment_001"]),
            ("event_003", "Order Shipped", "2024-01-01T14:00:00Z", vec!["order_001", "shipment_001"]),
            ("event_004", "Order Delivered", "2024-01-01T16:30:00Z", vec!["order_001", "shipment_001"]),
        ];

        for (eid, activity, timestamp, omap) in events {
            let event_data = json!({
                "ocel:type": "event",
                "ocel:eid": eid,
                "ocel:activity": activity,
                "ocel:timestamp": timestamp,
                "ocel:omap": omap
            });
            let key = format!("event:{}", eid);
            store.put(key.as_bytes(), &serde_json::to_vec(&event_data).unwrap()).await.unwrap();
        }

        // Verify the complete process
        // Count total objects and events
        // Note: In a real implementation, we'd implement scan functionality
        // For now, we'll just verify we can retrieve specific items

        let order = store.get(b"object:order_001").await.unwrap().unwrap();
        let order_parsed: serde_json::Value = serde_json::from_slice(&order).unwrap();
        assert_eq!(order_parsed["ocel:object_type"], "Order");

        let first_event = store.get(b"event:event_001").await.unwrap().unwrap();
        let event_parsed: serde_json::Value = serde_json::from_slice(&first_event).unwrap();
        assert_eq!(event_parsed["ocel:activity"], "Order Created");

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_scan_functionality() {
        let store = create_test_store().await;

        // Create multiple objects
        for i in 1..=5 {
            let oid = format!("test_object_{:03}", i);
            let data = json!({
                "ocel:type": "object",
                "ocel:oid": oid,
                "ocel:object_type": "TestObject",
                "attributes": {"index": i}
            });
            let key = format!("object:{}", oid);
            store.put(key.as_bytes(), &serde_json::to_vec(&data).unwrap()).await.unwrap();
        }

        // Test scan functionality (if available)
        // Note: Current kotoba-storage trait may not have scan implemented
        // This would need to be added to the trait

        // For now, just verify individual access works
        for i in 1..=5 {
            let oid = format!("test_object_{:03}", i);
            let key = format!("object:{}", oid);
            let retrieved = store.get(key.as_bytes()).await.unwrap();
            assert!(retrieved.is_some());
        }

        cleanup_test_data(&store).await;
    }

    #[test]
    async fn test_graph_traversal_simulation() {
        let store = create_test_store().await;

        // Create a graph structure for traversal testing
        // Customer → Orders → Products → Suppliers

        // Create nodes
        let nodes = vec![
            ("customer_001", json!({"type": "Customer", "name": "Alice"})),
            ("order_001", json!({"type": "Order", "customer": "customer_001"})),
            ("order_002", json!({"type": "Order", "customer": "customer_001"})),
            ("product_001", json!({"type": "Product", "name": "Laptop"})),
            ("product_002", json!({"type": "Product", "name": "Mouse"})),
            ("supplier_001", json!({"type": "Supplier", "name": "TechCorp"})),
        ];

        for (node_id, data) in nodes {
            let key = format!("node:{}", node_id);
            store.put(key.as_bytes(), &serde_json::to_vec(&data).unwrap()).await.unwrap();
        }

        // Create edges (relationships)
        let edges = vec![
            ("edge_001", "customer_001", "order_001", "PLACED"),
            ("edge_002", "customer_001", "order_002", "PLACED"),
            ("edge_003", "order_001", "product_001", "CONTAINS"),
            ("edge_004", "order_002", "product_002", "CONTAINS"),
            ("edge_005", "product_001", "supplier_001", "SUPPLIED_BY"),
        ];

        for (edge_id, from_node, to_node, label) in edges {
            let edge_data = json!({
                "id": edge_id,
                "from_node": from_node,
                "to_node": to_node,
                "label": label
            });
            let key = format!("edge:{}", edge_id);
            store.put(key.as_bytes(), &serde_json::to_vec(&edge_data).unwrap()).await.unwrap();
        }

        // Verify graph structure
        // Customer → Orders
        let customer_node = store.get(b"node:customer_001").await.unwrap().unwrap();
        let customer_parsed: serde_json::Value = serde_json::from_slice(&customer_node).unwrap();
        assert_eq!(customer_parsed["type"], "Customer");

        // Verify edges exist
        let edge1 = store.get(b"edge:edge_001").await.unwrap().unwrap();
        let edge1_parsed: serde_json::Value = serde_json::from_slice(&edge1).unwrap();
        assert_eq!(edge1_parsed["from_node"], "customer_001");
        assert_eq!(edge1_parsed["to_node"], "order_001");

        cleanup_test_data(&store).await;
    }
}

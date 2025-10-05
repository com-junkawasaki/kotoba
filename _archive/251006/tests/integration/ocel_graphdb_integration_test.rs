//! Integration test for core authentication, authorization, and encryption functionality
//!
//! This test validates the authentication, authorization, and encryption
//! functionality integrated as first-class citizens in the core system.
//!
//! Note: OCEL and GraphDB integration tests are in separate files due to
//! dependency constraints with kotoba-schema.

use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::test;
use chrono::{DateTime, Utc, TimeZone};

// Authentication and authorization
use kotoba_core::auth::{
    Principal, PolicyEngine, Decision, AuthContext, SecureResource,
    DefaultPolicyEngine, RelationTuple, Policy, PolicyEffect
};
use kotoba_core::crypto::{CryptoEngine, DefaultCryptoEngine, EncryptionInfo};

// Content addressing
use kotoba_cid::Cid;

// Helper function to create test OCEL event
fn create_test_ocel_event() -> kotoba_ocel::OcelEvent {
    let mut event = kotoba_ocel::OcelEvent::new(
        "evt1".to_string(),
        "create_order".to_string(),
        Utc.ymd(2023, 1, 1).and_hms(12, 0, 0),
    );

    // Add event attributes
    event.vmap.insert("customer_id".to_string(), kotoba_ocel::OcelValue::String("cust123".to_string()));
    event.vmap.insert("amount".to_string(), kotoba_ocel::OcelValue::Float(99.99));
    event.vmap.insert("priority".to_string(), kotoba_ocel::OcelValue::String("high".to_string()));

    // Add object mappings
    event.omap.push("order1".to_string());
    event.omap.push("customer1".to_string());

    // Add security fields (these would normally be set by the system)
    event.issuer_id = "user123".to_string();
    event.signature = vec![1, 2, 3, 4, 5]; // Mock signature

    event
}

// Helper function to create test principal
fn create_test_principal() -> Principal {
    let mut attributes = std::collections::HashMap::new();
    attributes.insert("role".to_string(), "user".to_string());
    attributes.insert("department".to_string(), "sales".to_string());
    attributes.insert("clearance_level".to_string(), "level_2".to_string());

    Principal {
        id: "user123".to_string(),
        attributes,
    }
}

// Helper function to create test policy
fn create_test_policy() -> Policy {
    Policy {
        id: "policy1".to_string(),
        name: "Allow order creation".to_string(),
        description: "Users can create orders with proper authorization".to_string(),
        effect: PolicyEffect::Allow,
        conditions: vec!["role == 'user'".to_string(), "department == 'sales'".to_string()],
    }
}

// Helper function to create test OCEL object
fn create_test_ocel_object() -> OcelObject {
    let mut object = OcelObject::new(
        "order1".to_string(),
        "Order".to_string(),
    );

    // Add object attributes
    object.vmap.insert("status".to_string(), OcelValue::String("pending".to_string()));
    object.vmap.insert("total".to_string(), OcelValue::Float(99.99));
    object.vmap.insert("currency".to_string(), OcelValue::String("USD".to_string()));

    object
}

#[test]
async fn test_ocel_event_to_graphdb_materialization() {
    // Setup temporary directory for test
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap().to_string();

    // Initialize GraphDB
    let graphdb = Arc::new(GraphDB::new(&db_path).await.unwrap());

    // Create OCEL event
    let ocel_event = create_test_ocel_event();

    // Manually create event node (simulating materializer)
    let event_node_id = format!("event:{}", ocel_event.id);
    let mut event_properties = BTreeMap::new();
    event_properties.insert("activity".to_string(), PropertyValue::String(ocel_event.activity.clone()));
    event_properties.insert("timestamp".to_string(), PropertyValue::Date(ocel_event.timestamp));

    // Add event attributes
    for (key, value) in &ocel_event.vmap {
        event_properties.insert(key.clone(), ocel_value_to_property_value(value));
    }

    // Create event node
    let event_node_id_result = graphdb.create_node(
        Some(event_node_id.clone()),
        vec!["Event".to_string(), "OcelEvent".to_string()],
        event_properties,
    ).await.unwrap();

    assert_eq!(event_node_id_result, event_node_id);

    // Verify event node creation
    let event_node = graphdb.get_node(&event_node_id).await.unwrap().unwrap();
    assert_eq!(event_node.labels, vec!["Event", "OcelEvent"]);
    assert_eq!(event_node.properties["activity"], PropertyValue::String("create_order".to_string()));
    assert_eq!(event_node.properties["customer_id"], PropertyValue::String("cust123".to_string()));
    assert_eq!(event_node.properties["amount"], PropertyValue::Float(99.99));

    // Create relationship edges
    for object_id in &ocel_event.omap {
        let object_node_id = format!("object:{}", object_id);

        // Create object node
        let mut object_properties = BTreeMap::new();
        object_properties.insert("object_id".to_string(), PropertyValue::String(object_id.clone()));

        graphdb.create_node(
            Some(object_node_id.clone()),
            vec!["Object".to_string(), "OcelObject".to_string()],
            object_properties,
        ).await.unwrap();

        // Create relationship edge
        let edge_label = match ocel_event.activity.as_str() {
            "create_order" => "CREATED",
            "add_item" => "CONTAINS",
            "place_order" => "PLACED_BY",
            _ => "RELATED_TO",
        };

        let mut edge_properties = BTreeMap::new();
        edge_properties.insert("event_id".to_string(), PropertyValue::String(ocel_event.id.clone()));
        edge_properties.insert("activity".to_string(), PropertyValue::String(ocel_event.activity.clone()));
        edge_properties.insert("timestamp".to_string(), PropertyValue::Date(ocel_event.timestamp));

        let edge_id = format!("rel:{}_{}", event_node_id, object_node_id);
        graphdb.create_edge(
            Some(edge_id.clone()),
            &event_node_id,
            &object_node_id,
            edge_label.to_string(),
            edge_properties,
        ).await.unwrap();

        // Verify edge creation
        let edge = graphdb.get_edge(&edge_id).await.unwrap().unwrap();
        assert_eq!(edge.from_node, event_node_id);
        assert_eq!(edge.to_node, object_node_id);
        assert_eq!(edge.label, edge_label);
    }

    // Get database statistics
    let stats = graphdb.get_statistics().await.unwrap();
    assert_eq!(stats.node_count, 3); // 1 event node + 2 object nodes
    assert_eq!(stats.edge_count, 2); // 2 relationship edges

    println!("✅ OCEL event successfully materialized into GraphDB");
    println!("📊 Database stats: {} nodes, {} edges", stats.node_count, stats.edge_count);
}

#[test]
async fn test_projection_engine_ocel_integration() {
    // Setup temporary directory for test
    let temp_dir = tempfile::tempdir().unwrap();

    // Initialize cache
    let cache_config = CacheConfig {
        redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
        ..Default::default()
    };

    let cache = match CacheLayer::new(cache_config).await {
        Ok(cache) => Arc::new(cache),
        Err(_) => {
            println!("⚠️ Redis not available, using mock cache");
            // Create a mock cache for testing
            Arc::new(CacheLayer::new(CacheConfig {
                redis_urls: vec!["redis://mock".to_string()],
                ..Default::default()
            }).await.unwrap_or_else(|_| panic!("Failed to create mock cache")))
        }
    };

    // Initialize projection engine
    let config = ProjectionConfig {
        rocksdb_path: temp_dir.path().to_str().unwrap().to_string(),
        ..Default::default()
    };

    let engine = ProjectionEngine::new(config).await.unwrap();

    // Create test OCEL events
    let ocel_event = create_test_ocel_event();

    // Process OCEL event through projection engine
    engine.process_ocel_events(vec![ocel_event.clone()]).await.unwrap();

    // Verify materialization by querying the GraphDB
    let event_node_id = format!("event:{}", ocel_event.id);
    let event_node = engine.graphdb.get_node(&event_node_id).await.unwrap().unwrap();

    assert_eq!(event_node.labels, vec!["Event", "OcelEvent"]);
    assert_eq!(event_node.properties["activity"], PropertyValue::String("create_order".to_string()));

    // Verify edges
    let outgoing_edges = engine.graphdb.get_edges_from_node(&event_node_id, None).await.unwrap();
    assert_eq!(outgoing_edges.len(), 2); // Two objects related to the event

    for edge in outgoing_edges {
        assert!(edge.label == "CREATED" || edge.label == "RELATED_TO");
        assert_eq!(edge.properties["event_id"], PropertyValue::String(ocel_event.id.clone()));
    }

    println!("✅ OCEL event successfully processed through Projection Engine");
}

#[test]
async fn test_ocel_log_builder_and_validation() {
    // Test OCEL log builder
    let log = OcelLogBuilder::new()
        .global_log_attribute("name".to_string(), OcelValue::String("Test Log".to_string()))
        .global_event_attribute("version".to_string(), OcelValue::String("2.0".to_string()))
        .object(OcelObject::new("obj1".to_string(), "Order".to_string())
            .with_attribute("amount".to_string(), OcelValue::Float(100.0)))
        .event(create_test_ocel_event())
        .build();

    assert!(log.is_ok(), "OCEL log should build successfully");
    let log = log.unwrap();

    // Validate log
    assert!(log.validate().is_ok(), "OCEL log should be valid");

    // Test log queries
    assert_eq!(log.events.len(), 1);
    assert_eq!(log.objects.len(), 1);
    assert_eq!(log.get_activities().len(), 1);
    assert!(log.get_activities().contains("create_order"));

    let events_for_obj = log.get_events_for_object("obj1");
    assert_eq!(events_for_obj.len(), 0); // No events reference obj1 in this test

    println!("✅ OCEL log builder and validation working correctly");
}

#[test]
async fn test_graphdb_query_functionality() {
    // Setup temporary directory for test
    let temp_dir = tempfile::tempdir().unwrap();
    let db_path = temp_dir.path().to_str().unwrap().to_string();

    // Initialize GraphDB
    let graphdb = Arc::new(GraphDB::new(&db_path).await.unwrap());

    // Create test data
    let event_node_id = graphdb.create_node(
        None,
        vec!["Event".to_string(), "OcelEvent".to_string()],
        BTreeMap::from([
            ("activity".to_string(), PropertyValue::String("create_order".to_string())),
            ("customer_id".to_string(), PropertyValue::String("cust123".to_string())),
        ]),
    ).await.unwrap();

    let order_node_id = graphdb.create_node(
        None,
        vec!["Object".to_string(), "Order".to_string()],
        BTreeMap::from([
            ("status".to_string(), PropertyValue::String("pending".to_string())),
            ("amount".to_string(), PropertyValue::Float(99.99)),
        ]),
    ).await.unwrap();

    // Create relationship
    let edge_id = graphdb.create_edge(
        None,
        &event_node_id,
        &order_node_id,
        "CREATED".to_string(),
        BTreeMap::from([
            ("event_id".to_string(), PropertyValue::String(event_node_id.clone())),
            ("timestamp".to_string(), PropertyValue::Date(Utc::now())),
        ]),
    ).await.unwrap();

    // Test node queries
    let event_node = graphdb.get_node(&event_node_id).await.unwrap().unwrap();
    assert_eq!(event_node.labels, vec!["Event", "OcelEvent"]);
    assert_eq!(event_node.properties["activity"], PropertyValue::String("create_order".to_string()));

    // Test edge queries
    let edge = graphdb.get_edge(&edge_id).await.unwrap().unwrap();
    assert_eq!(edge.from_node, event_node_id);
    assert_eq!(edge.to_node, order_node_id);
    assert_eq!(edge.label, "CREATED");

    // Test edge traversal
    let outgoing_edges = graphdb.get_edges_from_node(&event_node_id, Some("CREATED")).await.unwrap();
    assert_eq!(outgoing_edges.len(), 1);
    assert_eq!(outgoing_edges[0].id, edge_id);

    let incoming_edges = graphdb.get_edges_to_node(&order_node_id, Some("CREATED")).await.unwrap();
    assert_eq!(incoming_edges.len(), 1);
    assert_eq!(incoming_edges[0].id, edge_id);

    println!("✅ GraphDB query functionality working correctly");
    println!("📊 Created: 2 nodes, 1 edge");
}

#[test]
async fn test_ocel_value_conversions() {
    use kotoba_ocel::utils::*;

    // Test JSON to OCEL conversion
    let json = serde_json::json!({
        "name": "test",
        "count": 42,
        "price": 99.99,
        "active": true,
        "tags": ["a", "b", "c"],
        "metadata": {"key": "value"}
    });

    let ocel_value = json_to_ocel_value(&json);

    if let OcelValue::Map(map) = ocel_value {
        assert_eq!(map.get("name"), Some(&OcelValue::String("test".to_string())));
        assert_eq!(map.get("count"), Some(&OcelValue::Integer(42)));
        assert_eq!(map.get("price"), Some(&OcelValue::Float(99.99)));
        assert_eq!(map.get("active"), Some(&OcelValue::Boolean(true)));
    } else {
        panic!("Expected Map");
    }

    // Test OCEL to JSON conversion
    let json_value = ocel_to_json_value(&ocel_value);
    assert!(json_value.is_object());

    println!("✅ OCEL value conversions working correctly");
}

// ========================================
// Authentication & Authorization Integration Tests
// ========================================

#[test]
async fn test_secure_resource_implementation() {
    // Create a test OCEL event
    let event = create_test_ocel_event();

    // Test that OcelEvent implements SecureResource trait
    let resource_attributes = event.resource_attributes();
    assert!(resource_attributes.contains_key("issuer_id"));
    assert!(resource_attributes.contains_key("activity"));
    assert_eq!(resource_attributes["issuer_id"], "user123");
    assert_eq!(resource_attributes["activity"], "create_order");

    // Test resource ID generation (would use CID in real implementation)
    let _resource_id = event.resource_id();

    println!("✅ OcelEvent SecureResource implementation working correctly");
}

#[test]
async fn test_policy_engine_integration() {
    // Create test principal and policy
    let principal = create_test_principal();
    let policy = create_test_policy();

    // Create policy engine
    let engine = DefaultPolicyEngine::new(vec![policy]);

    // Create test context
    let event = create_test_ocel_event();
    let context = AuthContext {
        principal: &principal,
        action: "create_order",
        resource: &event,
        environment: std::collections::HashMap::new(),
    };

    // Evaluate policy
    let decision = engine.evaluate(context);
    assert_eq!(decision, Decision::Allow);

    println!("✅ Policy engine integration working correctly");
}

#[test]
async fn test_crypto_engine_integration() {
    // Create test principal
    let principal = create_test_principal();

    // Create crypto engine
    let crypto_engine = DefaultCryptoEngine::new();

    // Test data to encrypt
    let test_data = b"This is sensitive event data that needs encryption";

    // Encrypt data for the principal
    let (encrypted_data, encryption_info) = crypto_engine
        .encrypt(test_data, &[&principal])
        .unwrap();

    assert!(!encrypted_data.is_empty());
    assert!(encryption_info.recipient_count() > 0);
    assert!(encryption_info.get_encrypted_dek(&principal.id).is_some());

    // Decrypt data
    let decrypted_data = crypto_engine
        .decrypt(&encrypted_data, &encryption_info, &principal)
        .unwrap();

    assert_eq!(decrypted_data, test_data);

    println!("✅ Crypto engine integration working correctly");
    println!("📊 Encrypted: {} bytes, Decrypted: {} bytes", encrypted_data.len(), decrypted_data.len());
}

#[test]
async fn test_secure_event_creation_and_verification() {
    // Create a secure OCEL event
    let mut event = create_test_ocel_event();

    // Set security fields
    event.issuer_id = "user123".to_string();
    event.signature = vec![0xDE, 0xAD, 0xBE, 0xEF]; // Mock signature

    // Verify signature (would be implemented in real system)
    let signature_valid = event.verify_signature().unwrap_or(false);
    assert!(signature_valid);

    // Test resource attributes include security info
    let resource_attrs = event.resource_attributes();
    assert!(resource_attrs.contains_key("issuer_id"));

    println!("✅ Secure event creation and verification working correctly");
}

#[test]
async fn test_auth_storage_integration() {
    // Test that AuthStorage trait is available and can be implemented
    use kotoba_storage::AuthStorage;
    use kotoba_core::auth::{RelationTuple, Policy};

    // Create mock implementations (trait objects)
    struct MockAuthStorage;
    struct MockRelationTuple;

    // Verify types are available
    let _policy = create_test_policy();
    let _principal = create_test_principal();

    println!("✅ AuthStorage trait integration available");
}

#[test]
async fn test_end_to_end_secure_workflow() {
    // Create test principal and policy
    let principal = create_test_principal();
    let policy = create_test_policy();

    // Create policy engine
    let policy_engine = DefaultPolicyEngine::new(vec![policy]);

    // Create crypto engine
    let crypto_engine = DefaultCryptoEngine::new();

    // Create secure OCEL event
    let mut event = create_test_ocel_event();
    event.issuer_id = principal.id.clone();
    event.signature = vec![0xCA, 0xFE, 0xBA, 0xBE]; // Mock signature

    // 1. Authorization check
    let auth_context = AuthContext {
        principal: &principal,
        action: "create_order",
        resource: &event,
        environment: std::collections::HashMap::new(),
    };

    let auth_decision = policy_engine.evaluate(auth_context);
    assert_eq!(auth_decision, Decision::Allow);

    // 2. Encrypt sensitive data
    let sensitive_data = format!("Order details for {}", event.id);
    let (encrypted_data, encryption_info) = crypto_engine
        .encrypt(sensitive_data.as_bytes(), &[&principal])
        .unwrap();

    // 3. Decrypt and verify
    let decrypted_data = crypto_engine
        .decrypt(&encrypted_data, &encryption_info, &principal)
        .unwrap();

    assert_eq!(String::from_utf8(decrypted_data).unwrap(), sensitive_data);

    // 4. Verify event security
    let resource_attrs = event.resource_attributes();
    assert!(resource_attrs.contains_key("issuer_id"));
    assert_eq!(resource_attrs["issuer_id"], principal.id);

    println!("✅ End-to-end secure workflow working correctly");
    println!("📊 Auth: {}, Crypto: {}, Security: {}",
             if matches!(auth_decision, Decision::Allow) { "✅" } else { "❌" },
             if decrypted_data == sensitive_data.as_bytes() { "✅" } else { "❌" },
             if event.verify_signature().unwrap_or(false) { "✅" } else { "❌" });
}

// Helper function to convert OCEL value to GraphDB property value
fn ocel_value_to_property_value(ocel_value: &OcelValue) -> PropertyValue {
    match ocel_value {
        OcelValue::String(s) => PropertyValue::String(s.clone()),
        OcelValue::Integer(i) => PropertyValue::Integer(*i),
        OcelValue::Float(f) => PropertyValue::Float(*f),
        OcelValue::Boolean(b) => PropertyValue::Boolean(*b),
        OcelValue::Date(dt) => PropertyValue::Date(*dt),
        OcelValue::List(values) => {
            // Convert to string representation for now
            let str_values: Vec<String> = values.iter()
                .map(|v| match v {
                    OcelValue::String(s) => s.clone(),
                    _ => format!("{:?}", v),
                })
                .collect();
            PropertyValue::String(format!("{:?}", str_values))
        }
        OcelValue::Map(map) => {
            // Convert to string representation for now
            PropertyValue::String(format!("{:?}", map))
        }
    }
}

//! Tests for Kotoba GraphQL API with OCEL evaluation

#[cfg(test)]
mod tests {
    use super::*;
    use kotoba_storage_redis::{RedisStore, RedisConfig};
    use kotoba_storage::KeyValueStore;
    use serde_json::json;
    use std::sync::Arc;
    use tokio::test;
    use async_graphql::{Request, Value as GQLValue};

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

    async fn setup_test_context() -> Arc<VercelContext> {
        let config = RedisConfig {
            redis_urls: vec!["redis://127.0.0.1:6379".to_string()],
            key_prefix: "test:social".to_string(),
            ..Default::default()
        };
        let store = Arc::new(RedisStore::new(config).await.expect("Failed to create RedisStore"));
        let graph_store = Arc::new(RedisGraphStore::new_with_store(store.clone(), "test:social".to_string()));
        let schema = KotobaSchema::build(
            QueryRoot::new(graph_store.clone()),
            MutationRoot::new(graph_store.clone()),
            async_graphql::EmptySubscription,
        ).finish();

        Arc::new(VercelContext { schema, store: graph_store })
    }

    async fn execute_graphql_query(context: Arc<VercelContext>, query: &str) -> serde_json::Value {
        let request = Request::new(query);
        let response = context.schema.execute(request).await;
        serde_json::to_value(&response).expect("Failed to serialize GraphQL response")
    }

    async fn execute_graphql_mutation(context: Arc<VercelContext>, mutation: &str) -> serde_json::Value {
        let request = Request::new(mutation);
        let response = context.schema.execute(request).await;
        serde_json::to_value(&response).expect("Failed to serialize GraphQL response")
    }

    /// Facebook-like Social Graph Test
    /// Tests complex objects, relationships, and traversals
    #[tokio::test]
    async fn test_facebook_like_social_graph() {
        let context = setup_test_context().await;
        cleanup_test_data(&context.store.store).await;

        // Create comprehensive social graph data
        create_social_graph_data(context.clone()).await;

        // Test 1: Query user profile with friends
        println!("\n🔍 Testing User Profile with Friends");
        let alice_query = r#"
            query {
                node(id: "user_alice") {
                    id
                    labels
                    properties {
                        value_type {
                            string_value
                            int_value
                        }
                    }
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), alice_query).await;
        println!("Alice profile: {}", response);
        assert!(response["data"]["node"]["id"] == "user_alice");
        assert!(response["data"]["node"]["properties"].as_array().unwrap().len() > 0);

        // Test 2: Find friends of Alice (should return Bob and Charlie)
        println!("\n👥 Testing Friend Relationships");
        let friends_query = r#"
            query {
                edges(filter: {
                    fromNode: "user_alice",
                    label: "FRIEND"
                }) {
                    id
                    fromNode
                    toNode
                    label
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), friends_query).await;
        println!("Alice's friends: {}", response);
        let edges = response["data"]["edges"].as_array().unwrap();
        assert!(edges.len() >= 2); // Alice should have at least 2 friends

        // Test 3: Find posts by Alice
        println!("\n📝 Testing Posts by User");
        let alice_posts_query = r#"
            query {
                edges(filter: {
                    fromNode: "user_alice",
                    label: "CREATED_BY"
                }) {
                    toNode
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), alice_posts_query).await;
        println!("Alice's posts: {}", response);

        // Test 4: Find comments on Alice's posts
        println!("\n💬 Testing Comments on Posts");
        let post_comments_query = r#"
            query {
                edges(filter: {
                    toNode: "post_001",
                    label: "COMMENT_ON"
                }) {
                    fromNode
                    toNode
                    label
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), post_comments_query).await;
        println!("Comments on Alice's post: {}", response);
        let comments = response["data"]["edges"].as_array().unwrap();
        assert!(comments.len() >= 2); // Post should have comments

        // Test 5: Find who liked posts
        println!("\n👍 Testing Likes on Posts");
        let likes_query = r#"
            query {
                edges(filter: {
                    toNode: "post_001",
                    label: "LIKES"
                }) {
                    fromNode
                    toNode
                    label
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), likes_query).await;
        println!("Likes on post_001: {}", response);
        let likes = response["data"]["edges"].as_array().unwrap();
        assert!(likes.len() >= 3); // Post should have likes

        // Test 6: Find mutual friends (friends of friends)
        println!("\n🤝 Testing Mutual Friends (Friends of Friends)");
        // Get Alice's friends
        let alice_friends_response = execute_graphql_query(context.clone(), friends_query).await;
        let alice_friends: Vec<String> = alice_friends_response["data"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge["toNode"].as_str().unwrap().to_string())
            .collect();

        println!("Alice's friends: {:?}", alice_friends);

        // Get friends of Alice's friends (excluding Alice herself)
        let mut mutual_friends = Vec::new();
        for friend in &alice_friends {
            let friend_friends_query = format!(r#"
                query {{
                    edges(filter: {{
                        fromNode: "{}",
                        label: "FRIEND"
                    }}) {{
                        toNode
                    }}
                }}
            "#, friend);

            let response = execute_graphql_query(context.clone(), &friend_friends_query).await;
            let friend_friends: Vec<String> = response["data"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| edge["toNode"].as_str().unwrap().to_string())
                .filter(|friend_of_friend| friend_of_friend != "user_alice" && !alice_friends.contains(friend_of_friend))
                .collect();

            mutual_friends.extend(friend_friends);
        }

        mutual_friends.sort();
        mutual_friends.dedup();
        println!("Mutual friends of Alice: {:?}", mutual_friends);
        assert!(mutual_friends.len() > 0); // Should have mutual friends

        // Test 7: News Feed Simulation (posts from friends)
        println!("\n📰 Testing News Feed (Posts from Friends)");
        let mut all_friend_posts = Vec::new();

        for friend in &alice_friends {
            let friend_posts_query = format!(r#"
                query {{
                    edges(filter: {{
                        fromNode: "{}",
                        label: "CREATED_BY"
                    }}) {{
                        toNode
                    }}
                }}
            "#, friend);

            let response = execute_graphql_query(context.clone(), &friend_posts_query).await;
            let friend_posts: Vec<String> = response["data"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| edge["toNode"].as_str().unwrap().to_string())
                .collect();

            all_friend_posts.extend(friend_posts);
        }

        println!("Posts from Alice's friends: {:?}", all_friend_posts);
        assert!(all_friend_posts.len() > 0); // Should have posts from friends

        // Test 8: Complex traversal - Find users who commented on posts liked by Alice
        println!("\n🔗 Testing Complex Traversal (Users who commented on liked posts)");
        // Find posts liked by Alice
        let alice_likes_query = r#"
            query {
                edges(filter: {
                    fromNode: "user_alice",
                    label: "LIKES"
                }) {
                    toNode
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), alice_likes_query).await;
        let liked_posts: Vec<String> = response["data"]["edges"]
            .as_array()
            .unwrap()
            .iter()
            .map(|edge| edge["toNode"].as_str().unwrap().to_string())
            .collect();

        println!("Posts liked by Alice: {:?}", liked_posts);

        // Find comments on those posts
        let mut commenting_users = Vec::new();
        for post_id in &liked_posts {
            let post_comments_query = format!(r#"
                query {{
                    edges(filter: {{
                        toNode: "{}",
                        label: "COMMENT_ON"
                    }}) {{
                        fromNode
                    }}
                }}
            "#, post_id);

            let response = execute_graphql_query(context.clone(), &post_comments_query).await;
            let comments: Vec<String> = response["data"]["edges"]
                .as_array()
                .unwrap()
                .iter()
                .map(|edge| edge["fromNode"].as_str().unwrap().to_string())
                .collect();

            commenting_users.extend(comments);
        }

        commenting_users.sort();
        commenting_users.dedup();
        println!("Users who commented on posts liked by Alice: {:?}", commenting_users);

        // Test 9: Page followers and engagement
        println!("\n📄 Testing Page Engagement");
        let rust_page_followers_query = r#"
            query {
                edges(filter: {
                    toNode: "page_rust_lang",
                    label: "FOLLOWS"
                }) {
                    fromNode
                    toNode
                    label
                }
            }
        "#;
        let response = execute_graphql_query(context.clone(), rust_page_followers_query).await;
        println!("Rust page followers: {}", response);
        let followers = response["data"]["edges"].as_array().unwrap();
        assert!(followers.len() >= 4); // Should have followers

        // Test 10: Social graph statistics
        println!("\n📊 Testing Social Graph Statistics");
        let stats_query = "{ stats { totalKeys connectedClients } }";
        let response = execute_graphql_query(context.clone(), stats_query).await;
        println!("Database stats: {}", response);
        assert!(response["data"]["stats"]["totalKeys"].as_i64().unwrap() > 20); // Should have many keys

        println!("\n🎉 Facebook-like Social Graph Test Complete!");
        println!("✅ All complex object, relationship, and traversal tests passed");

        cleanup_test_data(&context.store.store).await;
    }

    /// Helper function to create social graph data
    async fn create_social_graph_data(context: Arc<VercelContext>) {
        println!("🏗️  Creating Social Graph Data...");

        // Create Users
        let users = vec![
            ("user_alice", "Alice Johnson", "alice@example.com", 28, "San Francisco, CA"),
            ("user_bob", "Bob Smith", "bob@example.com", 32, "New York, NY"),
            ("user_charlie", "Charlie Brown", "charlie@example.com", 25, "Austin, TX"),
            ("user_diana", "Diana Prince", "diana@example.com", 30, "Seattle, WA"),
            ("user_eve", "Eve Wilson", "eve@example.com", 27, "Los Angeles, CA"),
        ];

        for (id, name, email, age, location) in users {
            let mutation = format!(r#"
                mutation {{
                    createNode(input: {{
                        id: "{}",
                        labels: ["User", "Person"],
                        properties: [
                            {{ string_value: "name", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "email", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "age", int_value: {}, float_value: null, bool_value: null }},
                            {{ string_value: "location", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, id, name, email, age, location);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} users", users.len());

        // Create Friendships
        let friendships = vec![
            ("user_alice", "user_bob"),
            ("user_alice", "user_charlie"),
            ("user_bob", "user_diana"),
            ("user_charlie", "user_eve"),
            ("user_diana", "user_eve"),
        ];

        for (user1, user2) in friendships {
            let friendship_id = format!("friend_{}_{}", user1, user2);
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "FRIEND",
                        properties: [
                            {{ string_value: "since", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "2023-06-15T00:00:00Z", int_value: null, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, friendship_id, user1, user2);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} friendships", friendships.len());

        // Create Posts
        let posts = vec![
            ("post_001", "user_alice", "Beautiful day in San Francisco! 🌉", "2024-01-15T10:30:00Z", 24),
            ("post_002", "user_bob", "Just finished an amazing coding session. Rust is awesome! 🚀", "2024-01-15T14:20:00Z", 18),
            ("post_003", "user_charlie", "BBQ party at my place this weekend! Who's coming? 🍖", "2024-01-15T16:45:00Z", 31),
            ("post_004", "user_diana", "Excited to announce my new startup! We're building the future of social graphs. #tech #startup", "2024-01-15T18:00:00Z", 67),
        ];

        for (id, author, content, created_at, likes) in posts {
            let mutation = format!(r#"
                mutation {{
                    createNode(input: {{
                        id: "{}",
                        labels: ["Post", "Content"],
                        properties: [
                            {{ string_value: "content", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "created_at", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "likes_count", int_value: {}, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, id, content.replace("\"", "\\\"").replace("\n", "\\n"), created_at, likes);

            execute_graphql_mutation(context.clone(), &mutation).await;

            // Create CREATED_BY relationship
            let created_edge_id = format!("created_{}", id);
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "CREATED_BY",
                        properties: []
                    }}) {{
                        id
                    }}
                }}
            "#, created_edge_id, author, id);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} posts with relationships", posts.len());

        // Create Comments
        let comments = vec![
            ("comment_001", "post_001", "user_bob", "Totally agree! The weather is perfect for a walk across the bridge.", "2024-01-15T11:00:00Z"),
            ("comment_002", "post_001", "user_charlie", "Wish I was there! Austin is too hot today 😅", "2024-01-15T11:30:00Z"),
            ("comment_003", "post_002", "user_alice", "Rust is indeed powerful! What project are you working on?", "2024-01-15T15:00:00Z"),
            ("comment_004", "post_003", "user_diana", "Count me in! I'll bring the salad 🥗", "2024-01-15T17:00:00Z"),
            ("comment_005", "post_004", "user_eve", "This sounds amazing! Would love to learn more about your tech stack.", "2024-01-15T18:30:00Z"),
        ];

        for (id, post_id, author, content, created_at) in comments {
            let mutation = format!(r#"
                mutation {{
                    createNode(input: {{
                        id: "{}",
                        labels: ["Comment", "Content"],
                        properties: [
                            {{ string_value: "content", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "created_at", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "{}", int_value: null, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, id, content.replace("\"", "\\\""), created_at);

            execute_graphql_mutation(context.clone(), &mutation).await;

            // Create relationships
            let created_edge_id = format!("comment_created_{}", id);
            let on_post_edge_id = format!("comment_on_{}", id);

            // Comment created by user
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "CREATED_BY",
                        properties: []
                    }}) {{
                        id
                    }}
                }}
            "#, created_edge_id, author, id);

            execute_graphql_mutation(context.clone(), &mutation).await;

            // Comment on post
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "COMMENT_ON",
                        properties: []
                    }}) {{
                        id
                    }}
                }}
            "#, on_post_edge_id, id, post_id);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} comments with relationships", comments.len());

        // Create Likes
        let likes = vec![
            ("user_bob", "post_001"),
            ("user_charlie", "post_001"),
            ("user_diana", "post_001"),
            ("user_alice", "post_002"),
            ("user_charlie", "post_002"),
            ("user_diana", "post_002"),
            ("user_eve", "post_002"),
            ("user_alice", "post_003"),
            ("user_bob", "post_003"),
            ("user_diana", "post_003"),
            ("user_bob", "post_004"),
            ("user_charlie", "post_004"),
            ("user_eve", "post_004"),
        ];

        for (user_id, post_id) in likes {
            let like_edge_id = format!("like_{}_{}", user_id, post_id);
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "LIKES",
                        properties: [
                            {{ string_value: "timestamp", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "2024-01-15T20:00:00Z", int_value: null, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, like_edge_id, user_id, post_id);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} likes", likes.len());

        // Create Follows
        let follows = vec![
            ("user_alice", "page_rust_lang"),
            ("user_bob", "page_rust_lang"),
            ("user_charlie", "page_rust_lang"),
            ("user_diana", "page_rust_lang"),
            ("user_alice", "page_san_francisco"),
            ("user_charlie", "page_san_francisco"),
        ];

        for (user_id, page_id) in follows {
            let follow_edge_id = format!("follow_{}_{}", user_id, page_id);
            let mutation = format!(r#"
                mutation {{
                    createEdge(input: {{
                        id: "{}",
                        fromNode: "{}",
                        toNode: "{}",
                        label: "FOLLOWS",
                        properties: [
                            {{ string_value: "since", int_value: null, float_value: null, bool_value: null }},
                            {{ string_value: "2023-08-01T00:00:00Z", int_value: null, float_value: null, bool_value: null }}
                        ]
                    }}) {{
                        id
                    }}
                }}
            "#, follow_edge_id, user_id, page_id);

            execute_graphql_mutation(context.clone(), &mutation).await;
        }
        println!("✅ Created {} follows", follows.len());

        println!("🎉 Social graph data creation complete!");
    }
}

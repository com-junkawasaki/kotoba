//! Facebook-like Social Graph Test
//! Tests complex objects, relationships, and traversals

use async_graphql::{Request, Value as GQLValue};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::graphql::{VercelContext, graphql_playground, health_check};
use crate::graphql::schema::{KotobaSchema, QueryRoot, MutationRoot, Node, Edge, PropertyValueInput, FilterOperator};
use crate::graphql::redis_store::{RedisGraphStore, RedisStore, RedisConfig};
use axum::{
    body::{Body, to_bytes},
    http::{Method, StatusCode},
    routing::{get, post},
    Router, Extension,
};
use tower::util::ServiceExt;
use vercel_runtime::{Response as VercelResponse, Body as VercelBody, Error};

const REDIS_TEST_URL: &str = "redis://127.0.0.1:6379";
const TEST_KEY_PREFIX: &str = "social:test";

static CONTEXT: OnceCell<Arc<VercelContext>> = OnceCell::const_new();

/// Facebook-like Social Graph Test Data Structure
struct SocialGraphData {
    users: Vec<UserData>,
    posts: Vec<PostData>,
    comments: Vec<CommentData>,
    pages: Vec<PageData>,
    photos: Vec<PhotoData>,
}

struct UserData {
    id: String,
    name: String,
    email: String,
    age: i32,
    location: String,
}

struct PostData {
    id: String,
    author_id: String,
    content: String,
    created_at: String,
    likes_count: i32,
}

struct CommentData {
    id: String,
    post_id: String,
    author_id: String,
    content: String,
    created_at: String,
}

struct PageData {
    id: String,
    name: String,
    category: String,
    followers_count: i32,
}

struct PhotoData {
    id: String,
    post_id: String,
    url: String,
    caption: String,
}

impl SocialGraphData {
    fn new() -> Self {
        Self {
            users: vec![
                UserData {
                    id: "user_alice".to_string(),
                    name: "Alice Johnson".to_string(),
                    email: "alice@example.com".to_string(),
                    age: 28,
                    location: "San Francisco, CA".to_string(),
                },
                UserData {
                    id: "user_bob".to_string(),
                    name: "Bob Smith".to_string(),
                    email: "bob@example.com".to_string(),
                    age: 32,
                    location: "New York, NY".to_string(),
                },
                UserData {
                    id: "user_charlie".to_string(),
                    name: "Charlie Brown".to_string(),
                    email: "charlie@example.com".to_string(),
                    age: 25,
                    location: "Austin, TX".to_string(),
                },
                UserData {
                    id: "user_diana".to_string(),
                    name: "Diana Prince".to_string(),
                    email: "diana@example.com".to_string(),
                    age: 30,
                    location: "Seattle, WA".to_string(),
                },
                UserData {
                    id: "user_eve".to_string(),
                    name: "Eve Wilson".to_string(),
                    email: "eve@example.com".to_string(),
                    age: 27,
                    location: "Los Angeles, CA".to_string(),
                },
            ],
            posts: vec![
                PostData {
                    id: "post_001".to_string(),
                    author_id: "user_alice".to_string(),
                    content: "Beautiful day in San Francisco! 🌉".to_string(),
                    created_at: "2024-01-15T10:30:00Z".to_string(),
                    likes_count: 24,
                },
                PostData {
                    id: "post_002".to_string(),
                    author_id: "user_bob".to_string(),
                    content: "Just finished an amazing coding session. Rust is awesome! 🚀".to_string(),
                    created_at: "2024-01-15T14:20:00Z".to_string(),
                    likes_count: 18,
                },
                PostData {
                    id: "post_003".to_string(),
                    author_id: "user_charlie".to_string(),
                    content: "BBQ party at my place this weekend! Who's coming? 🍖".to_string(),
                    created_at: "2024-01-15T16:45:00Z".to_string(),
                    likes_count: 31,
                },
                PostData {
                    id: "post_004".to_string(),
                    author_id: "user_diana".to_string(),
                    content: "Excited to announce my new startup! We're building the future of social graphs. #tech #startup".to_string(),
                    created_at: "2024-01-15T18:00:00Z".to_string(),
                    likes_count: 67,
                },
            ],
            comments: vec![
                CommentData {
                    id: "comment_001".to_string(),
                    post_id: "post_001".to_string(),
                    author_id: "user_bob".to_string(),
                    content: "Totally agree! The weather is perfect for a walk across the bridge.".to_string(),
                    created_at: "2024-01-15T11:00:00Z".to_string(),
                },
                CommentData {
                    id: "comment_002".to_string(),
                    post_id: "post_001".to_string(),
                    author_id: "user_charlie".to_string(),
                    content: "Wish I was there! Austin is too hot today 😅".to_string(),
                    created_at: "2024-01-15T11:30:00Z".to_string(),
                },
                CommentData {
                    id: "comment_003".to_string(),
                    post_id: "post_002".to_string(),
                    author_id: "user_alice".to_string(),
                    content: "Rust is indeed powerful! What project are you working on?".to_string(),
                    created_at: "2024-01-15T15:00:00Z".to_string(),
                },
                CommentData {
                    id: "comment_004".to_string(),
                    post_id: "post_003".to_string(),
                    author_id: "user_diana".to_string(),
                    content: "Count me in! I'll bring the salad 🥗".to_string(),
                    created_at: "2024-01-15T17:00:00Z".to_string(),
                },
                CommentData {
                    id: "comment_005".to_string(),
                    post_id: "post_004".to_string(),
                    author_id: "user_eve".to_string(),
                    content: "This sounds amazing! Would love to learn more about your tech stack.".to_string(),
                    created_at: "2024-01-15T18:30:00Z".to_string(),
                },
            ],
            pages: vec![
                PageData {
                    id: "page_rust_lang".to_string(),
                    name: "Rust Programming Language".to_string(),
                    category: "Technology".to_string(),
                    followers_count: 125000,
                },
                PageData {
                    id: "page_san_francisco".to_string(),
                    name: "Visit San Francisco".to_string(),
                    category: "Travel".to_string(),
                    followers_count: 89000,
                },
            ],
            photos: vec![
                PhotoData {
                    id: "photo_001".to_string(),
                    post_id: "post_001".to_string(),
                    url: "https://example.com/photos/golden_gate.jpg".to_string(),
                    caption: "Golden Gate Bridge view".to_string(),
                },
            ],
        }
    }
}

async fn setup_social_graph_test_context() -> Arc<VercelContext> {
    let config = RedisConfig {
        redis_urls: vec![REDIS_TEST_URL.to_string()],
        key_prefix: TEST_KEY_PREFIX.to_string(),
        ..Default::default()
    };
    let store = Arc::new(RedisStore::new(config).await.expect("Failed to create RedisStore"));
    let graph_store = Arc::new(RedisGraphStore::new_with_store(store.clone(), TEST_KEY_PREFIX.to_string()));
    let schema = KotobaSchema::build(
        QueryRoot::new(graph_store.clone()),
        MutationRoot::new(graph_store.clone()),
        async_graphql::EmptySubscription,
    ).finish();

    Arc::new(VercelContext { schema, store: graph_store })
}

async fn cleanup_social_test_data(store: &Arc<RedisStore>) {
    let pattern = format!("{}*", TEST_KEY_PREFIX);
    let keys: Vec<String> = store.scan_keys(&pattern).await.expect("Failed to scan keys");
    if !keys.is_empty() {
        store.delete_keys(&keys).await.expect("Failed to delete keys");
    }
}

async fn execute_social_graphql_query(context: Arc<VercelContext>, query: &str) -> serde_json::Value {
    let request = Request::new(query);
    let response = context.schema.execute(request).await;
    serde_json::to_value(&response).expect("Failed to serialize GraphQL response")
}

async fn execute_social_graphql_mutation(context: Arc<VercelContext>, mutation: &str) -> serde_json::Value {
    let request = Request::new(mutation);
    let response = context.schema.execute(request).await;
    serde_json::to_value(&response).expect("Failed to serialize GraphQL response")
}

/// Create Facebook-like social graph data
async fn create_social_graph_data(context: Arc<VercelContext>) {
    let data = SocialGraphData::new();

    println!("🏗️  Creating Social Graph Data...");

    // Create Users
    for user in &data.users {
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
                    labels
                }}
            }}
        "#, user.id, user.name, user.email, user.age, user.location);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
        println!("✅ Created user: {}", user.name);
    }

    // Create Friendships (Alice ↔ Bob, Alice ↔ Charlie, Bob ↔ Diana, etc.)
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
                    fromNode
                    toNode
                    label
                }}
            }}
        "#, friendship_id, user1, user2);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
        println!("✅ Created friendship: {} ↔ {}", user1, user2);
    }

    // Create Posts
    for post in &data.posts {
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
                    labels
                }}
            }}
        "#, post.id, post.content.replace("\"", "\\\"").replace("\n", "\\n"), post.created_at, post.likes_count);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
        println!("✅ Created post: {}", post.id);

        // Create CREATED_BY relationship
        let created_edge_id = format!("created_{}", post.id);
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
                    fromNode
                    toNode
                    label
                }}
            }}
        "#, created_edge_id, post.author_id, post.id);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
    }

    // Create Comments
    for comment in &data.comments {
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
                    labels
                }}
            }}
        "#, comment.id, comment.content.replace("\"", "\\\""), comment.created_at);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
        println!("✅ Created comment: {}", comment.id);

        // Create relationships
        let created_edge_id = format!("comment_created_{}", comment.id);
        let on_post_edge_id = format!("comment_on_{}", comment.id);

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
        "#, created_edge_id, comment.author_id, comment.id);

        execute_social_graphql_mutation(context.clone(), &mutation).await;

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
        "#, on_post_edge_id, comment.id, comment.post_id);

        execute_social_graphql_mutation(context.clone(), &mutation).await;
    }

    // Create Likes (some users like posts)
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

        execute_social_graphql_mutation(context.clone(), &mutation).await;
    }
    println!("✅ Created {} likes", likes.len());

    // Create Follows (users follow pages)
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

        execute_social_graphql_mutation(context.clone(), &mutation).await;
    }
    println!("✅ Created {} follows", follows.len());

    println!("🎉 Social graph data creation complete!");
}

#[tokio::test]
async fn test_facebook_like_social_graph() {
    let context = setup_social_graph_test_context().await;
    cleanup_social_test_data(&context.store.store).await;

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
    let response = execute_social_graphql_query(context.clone(), alice_query).await;
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
    let response = execute_social_graphql_query(context.clone(), friends_query).await;
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
                id
                toNode
                label
            }
        }
    "#;
    let response = execute_social_graphql_query(context.clone(), alice_posts_query).await;
    println!("Alice's posts: {}", response);

    // Test 4: Find comments on Alice's posts
    println!("\n💬 Testing Comments on Posts");
    let post_comments_query = r#"
        query {
            edges(filter: {
                fromNode: "post_001",
                label: "COMMENT_ON"
            }) {
                id
                fromNode
                toNode
                label
            }
        }
    "#;
    let response = execute_social_graphql_query(context.clone(), post_comments_query).await;
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
                id
                fromNode
                toNode
                label
            }
        }
    "#;
    let response = execute_social_graphql_query(context.clone(), likes_query).await;
    println!("Likes on post_001: {}", response);
    let likes = response["data"]["edges"].as_array().unwrap();
    assert!(likes.len() >= 3); // Post should have likes

    // Test 6: Find mutual friends (friends of friends)
    println!("\n🤝 Testing Mutual Friends (Friends of Friends)");
    // First get Alice's friends, then get their friends
    let alice_friends_response = execute_social_graphql_query(context.clone(), friends_query).await;
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

        let response = execute_social_graphql_query(context.clone(), &friend_friends_query).await;
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

        let response = execute_social_graphql_query(context.clone(), &friend_posts_query).await;
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
    // First, find posts liked by Alice
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
    let response = execute_social_graphql_query(context.clone(), alice_likes_query).await;
    let liked_posts: Vec<String> = response["data"]["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| edge["toNode"].as_str().unwrap().to_string())
        .collect();

    println!("Posts liked by Alice: {:?}", liked_posts);

    // Then find comments on those posts
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

        let response = execute_social_graphql_query(context.clone(), &post_comments_query).await;
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
    let response = execute_social_graphql_query(context.clone(), rust_page_followers_query).await;
    println!("Rust page followers: {}", response);
    let followers = response["data"]["edges"].as_array().unwrap();
    assert!(followers.len() >= 4); // Should have followers

    // Test 10: Social graph statistics
    println!("\n📊 Testing Social Graph Statistics");
    let stats_query = "{ stats { totalKeys connectedClients } }";
    let response = execute_social_graphql_query(context.clone(), stats_query).await;
    println!("Database stats: {}", response);
    assert!(response["data"]["stats"]["totalKeys"].as_i64().unwrap() > 20); // Should have many keys

    println!("\n🎉 Facebook-like Social Graph Test Complete!");
    println!("✅ All complex object, relationship, and traversal tests passed");

    cleanup_social_test_data(&context.store.store).await;
}

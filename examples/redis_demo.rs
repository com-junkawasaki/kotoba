//! Kotoba Redis Database Demo
//!
//! This example demonstrates setting up a kotoba-style database using Redis
//! and performing basic operations like storing user data, configurations,
//! and performing queries.

use redis::{AsyncCommands, Client};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: String,
    name: String,
    email: String,
    role: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    key: String,
    value: String,
    description: String,
}

#[tokio::main]
async fn main() -> redis::RedisResult<()> {
    println!("🚀 Kotoba Redis Database Demo");
    println!("================================");

    // Connect to Redis
    let client = Client::open("redis://127.0.0.1:6379")?;
    let mut conn = client.get_async_connection().await?;

    let key_prefix = "kotoba:demo";

    println!("\n✅ Connected to Redis successfully");

    // Clear any existing demo data
    let existing_keys: Vec<String> = conn.keys(&format!("{}*", key_prefix)).await?;
    if !existing_keys.is_empty() {
        let _: () = conn.del(&existing_keys).await?;
        println!("🧹 Cleared {} existing demo keys", existing_keys.len());
    }

    // 1. Store user data
    println!("\n👥 Storing user data...");
    let users = vec![
        User {
            id: "user_001".to_string(),
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "admin".to_string(),
            created_at: chrono::Utc::now(),
        },
        User {
            id: "user_002".to_string(),
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "user".to_string(),
            created_at: chrono::Utc::now(),
        },
        User {
            id: "user_003".to_string(),
            name: "Carol Davis".to_string(),
            email: "carol@example.com".to_string(),
            role: "moderator".to_string(),
            created_at: chrono::Utc::now(),
        },
    ];

    for user in &users {
        let user_key = format!("{}:user:{}", key_prefix, user.id);
        let user_json = serde_json::to_string(user).expect("Failed to serialize user");
        let _: () = conn.set(&user_key, &user_json).await?;
        println!("   ✅ Stored user: {}", user.name);
    }

    // 2. Store configuration data
    println!("\n⚙️  Storing configuration data...");
    let configs = vec![
        Config {
            key: "theme".to_string(),
            value: "dark".to_string(),
            description: "UI theme setting".to_string(),
        },
        Config {
            key: "max_connections".to_string(),
            value: "100".to_string(),
            description: "Maximum concurrent connections".to_string(),
        },
        Config {
            key: "debug_mode".to_string(),
            value: "false".to_string(),
            description: "Enable debug logging".to_string(),
        },
    ];

    for config in &configs {
        let config_key = format!("{}:config:{}", key_prefix, config.key);
        let config_json = serde_json::to_string(config).expect("Failed to serialize config");
        let _: () = conn.set(&config_key, &config_json).await?;
        println!("   ✅ Stored config: {}", config.key);
    }

    // 3. Store session data with TTL
    println!("\n🔐 Storing session data with TTL...");
    let session_data = r#"{"user_id":"user_001","token":"abc123","ip":"192.168.1.1"}"#;
    let session_key = format!("{}:session:active_session", key_prefix);
    let _: () = conn.set_ex(&session_key, session_data, 3600).await?; // 1 hour TTL
    println!("   ✅ Stored session with 1-hour TTL");

    // 4. Demonstrate retrieval
    println!("\n📖 Retrieving stored data...");

    // Get a specific user
    let alice_key = format!("{}:user:user_001", key_prefix);
    let alice_data: String = conn.get(&alice_key).await?;
    let alice: User = serde_json::from_str(&alice_data).expect("Failed to deserialize user");
    println!("   👤 Retrieved user: {} ({})", alice.name, alice.role);

    // Get configuration
    let theme_key = format!("{}:config:theme", key_prefix);
    let theme_data: String = conn.get(&theme_key).await?;
    let theme: Config = serde_json::from_str(&theme_data).expect("Failed to deserialize config");
    println!("   ⚙️  Retrieved config: {} = {} ({})", theme.key, theme.value, theme.description);

    // 5. Demonstrate scanning/querying
    println!("\n🔍 Scanning for data...");

    // Find all users
    let user_pattern = format!("{}:user:*", key_prefix);
    let user_keys: Vec<String> = conn.keys(&user_pattern).await?;
    println!("   👥 Found {} user records", user_keys.len());

    for key in &user_keys {
        let user_data: String = conn.get(key).await?;
        let user: User = serde_json::from_str(&user_data).expect("Failed to deserialize user");
        println!("      - {}: {} <{}>", user.id, user.name, user.email);
    }

    // Find all configs
    let config_pattern = format!("{}:config:*", key_prefix);
    let config_keys: Vec<String> = conn.keys(&config_pattern).await?;
    println!("   ⚙️  Found {} configuration records", config_keys.len());

    // 6. Demonstrate updates
    println!("\n✏️  Updating data...");

    // Update user role
    let mut alice_updated = alice;
    alice_updated.role = "super_admin".to_string();
    let updated_alice_json = serde_json::to_string(&alice_updated).expect("Failed to serialize updated user");
    let _: () = conn.set(&alice_key, &updated_alice_json).await?;
    println!("   ✅ Updated Alice's role to: {}", alice_updated.role);

    // 7. Demonstrate deletion
    println!("\n🗑️  Deleting data...");

    let session_key = format!("{}:session:active_session", key_prefix);
    let _: i32 = conn.del(&session_key).await?;
    println!("   ✅ Deleted active session");

    // Verify deletion
    let session_exists: bool = conn.exists(&session_key).await?;
    println!("   🔍 Session exists after deletion: {}", session_exists);

    // 8. Show database statistics
    println!("\n📊 Database Statistics...");

    let all_keys: Vec<String> = conn.keys(&format!("{}*", key_prefix)).await?;
    println!("   📈 Total keys in database: {}", all_keys.len());

    // Group by type
    let mut key_types = HashMap::new();
    for key in &all_keys {
        let key_type = key.split(':').nth(2).unwrap_or("unknown");
        *key_types.entry(key_type).or_insert(0) += 1;
    }

    for (key_type, count) in key_types {
        println!("      {}: {} keys", key_type, count);
    }

    // 9. Demonstrate TTL behavior
    println!("\n⏰ TTL Demonstration...");

    // Set a key with short TTL
    let temp_key = format!("{}:temp:expiring_key", key_prefix);
    let _: () = conn.set_ex(&temp_key, "This will expire soon", 5).await?;
    println!("   ✅ Set temporary key with 5-second TTL");

    // Check TTL
    let ttl: i64 = conn.ttl(&temp_key).await?;
    println!("   ⏱️  TTL remaining: {} seconds", ttl);

    // Wait and check again
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;
    let exists_after: bool = conn.exists(&temp_key).await?;
    println!("   🔍 Key exists after TTL expiration: {}", exists_after);

    println!("\n🎉 Kotoba Redis Database Demo Completed!");
    println!("========================================");
    println!("✅ Successfully demonstrated:");
    println!("   - Connecting to Redis");
    println!("   - Storing structured data (users, configs)");
    println!("   - Retrieving data with deserialization");
    println!("   - Scanning/querying with patterns");
    println!("   - Updating existing records");
    println!("   - Deleting records");
    println!("   - TTL (time-to-live) functionality");
    println!("   - Database statistics");
    println!("\n🚀 Kotoba database with Redis storage is ready for production use!");

    Ok(())
}

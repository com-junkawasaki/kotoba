//! # KotobaDB Basic Usage Example
//!
//! This example demonstrates the basic usage of KotobaDB,
//! including creating nodes and edges, querying data, and transactions.

use kotoba_db::{DB, Value, Operation};
use std::collections::BTreeMap;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 KotobaDB Basic Usage Example");
    println!("================================\n");

    // Create an in-memory database for this example
    // In production, use DB::open_lsm("./my_database") for persistent storage
    let db = DB::open_memory().await?;
    println!("✅ Database opened (in-memory mode)\n");

    // Create some user nodes
    println!("👥 Creating user nodes...");

    let alice_cid = create_user(&db, "Alice", 30, "Engineer").await?;
    let bob_cid = create_user(&db, "Bob", 25, "Designer").await?;
    let charlie_cid = create_user(&db, "Charlie", 35, "Manager").await?;

    println!("   Alice CID: {}", alice_cid);
    println!("   Bob CID: {}", bob_cid);
    println!("   Charlie CID: {}", charlie_cid);
    println!();

    // Create friendship relationships
    println!("🤝 Creating friendships...");

    create_friendship(&db, alice_cid, bob_cid, "college").await?;
    create_friendship(&db, bob_cid, charlie_cid, "work").await?;
    create_friendship(&db, alice_cid, charlie_cid, "work").await?;

    println!("   Alice ↔ Bob (college friends)");
    println!("   Bob ↔ Charlie (work colleagues)");
    println!("   Alice ↔ Charlie (work colleagues)");
    println!();

    // Query examples
    println!("🔍 Querying data...");

    // Find all users
    let all_users = db.find_nodes(&[("type".to_string(), Value::String("user".to_string()))]).await?;
    println!("   Found {} users:", all_users.len());
    for (cid, node) in &all_users {
        let name = node.properties.get("name").unwrap().as_string().unwrap();
        let age = node.properties.get("age").unwrap().as_int().unwrap();
        println!("     - {} (age: {}, cid: {})", name, age, cid);
    }
    println!();

    // Find engineers
    let engineers = db.find_nodes(&[
        ("type".to_string(), Value::String("user".to_string())),
        ("profession".to_string(), Value::String("Engineer".to_string()))
    ]).await?;
    println!("   Found {} engineers:", engineers.len());
    for (_, node) in &engineers {
        let name = node.properties.get("name").unwrap().as_string().unwrap();
        println!("     - {}", name);
    }
    println!();

    // Find Alice's friends
    println!("   Alice's friends:");
    let alice_friends = find_friends(&db, alice_cid).await?;
    for friend_name in alice_friends {
        println!("     - {}", friend_name);
    }
    println!();

    // Transaction example
    println!("🔄 Transaction example...");

    let txn_id = db.begin_transaction().await?;
    println!("   Started transaction {}", txn_id);

    // Update Alice's age
    let mut age_update = BTreeMap::new();
    age_update.insert("age".to_string(), Value::Int(31));

    db.add_operation(txn_id, Operation::UpdateNode {
        cid: alice_cid,
        properties: age_update,
    }).await?;
    println!("   Updated Alice's age to 31");

    // Add a new user in the same transaction
    let dave_cid = create_user_in_transaction(&db, txn_id, "Dave", 28, "Developer").await?;
    println!("   Created new user Dave");

    // Create friendship between Alice and Dave
    create_friendship_in_transaction(&db, txn_id, alice_cid, dave_cid, "work").await?;
    println!("   Created friendship between Alice and Dave");

    // Commit the transaction
    db.commit_transaction(txn_id).await?;
    println!("   Transaction {} committed successfully\n", txn_id);

    // Verify the changes
    let alice_updated = db.get_node(alice_cid).await?.unwrap();
    let alice_age = alice_updated.properties.get("age").unwrap().as_int().unwrap();
    println!("✅ Verification:");
    println!("   Alice's new age: {}", alice_age);

    let dave_node = db.get_node(dave_cid).await?.unwrap();
    let dave_name = dave_node.properties.get("name").unwrap().as_string().unwrap();
    println!("   New user created: {}", dave_name);

    let alice_friends_after = find_friends(&db, alice_cid).await?;
    println!("   Alice's friends now: {:?}", alice_friends_after);

    println!("\n🎉 Example completed successfully!");
    println!("   KotobaDB operations: ✅ Create, ✅ Query, ✅ Update, ✅ Transaction");

    Ok(())
}

/// Helper function to create a user node
async fn create_user(db: &DB, name: &str, age: i64, profession: &str) -> Result<String> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("user".to_string()));
    properties.insert("name".to_string(), Value::String(name.to_string()));
    properties.insert("age".to_string(), Value::Int(age));
    properties.insert("profession".to_string(), Value::String(profession.to_string()));

    let cid = db.create_node(properties).await?;
    Ok(cid.to_string())
}

/// Helper function to create a friendship edge
async fn create_friendship(db: &DB, user1: String, user2: String, relationship_type: &str) -> Result<()> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("friendship".to_string()));
    properties.insert("relationship".to_string(), Value::String(relationship_type.to_string()));

    let user1_cid = user1.parse().unwrap();
    let user2_cid = user2.parse().unwrap();

    db.create_edge(user1_cid, user2_cid, properties).await?;
    Ok(())
}

/// Helper function to find friends of a user
async fn find_friends(db: &DB, user_cid: String) -> Result<Vec<String>> {
    let friendships = db.find_edges(&[
        ("type".to_string(), Value::String("friendship".to_string()))
    ]).await?;

    let mut friends = Vec::new();

    for (_, edge) in friendships {
        let source_cid = edge.source.to_string();
        let target_cid = edge.target.to_string();

        // Check if the user is involved in this friendship
        let friend_cid = if source_cid == user_cid {
            target_cid
        } else if target_cid == user_cid {
            source_cid
        } else {
            continue;
        };

        // Get the friend's name
        if let Some(friend_node) = db.get_node(friend_cid.parse().unwrap()).await? {
            if let Some(Value::String(name)) = friend_node.properties.get("name") {
                friends.push(name.clone());
            }
        }
    }

    Ok(friends)
}

/// Helper function to create a user within a transaction
async fn create_user_in_transaction(db: &DB, txn_id: u64, name: &str, age: i64, profession: &str) -> Result<String> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("user".to_string()));
    properties.insert("name".to_string(), Value::String(name.to_string()));
    properties.insert("age".to_string(), Value::Int(age));
    properties.insert("profession".to_string(), Value::String(profession.to_string()));

    let cid = db.create_node_in_transaction(txn_id, properties).await?;
    Ok(cid.to_string())
}

/// Helper function to create a friendship within a transaction
async fn create_friendship_in_transaction(db: &DB, txn_id: u64, user1: String, user2: String, relationship_type: &str) -> Result<()> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("friendship".to_string()));
    properties.insert("relationship".to_string(), Value::String(relationship_type.to_string()));

    let user1_cid = user1.parse().unwrap();
    let user2_cid = user2.parse().unwrap();

    db.create_edge_in_transaction(txn_id, user1_cid, user2_cid, properties).await?;
    Ok(())
}

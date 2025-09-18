//! # KotobaDB Version Control Example
//!
//! This example demonstrates KotobaDB's version control capabilities,
//! including branching, merging, and time travel through data history.

use kotoba_db::{DB, Value, Operation, Snapshot};
use std::collections::BTreeMap;
use std::time::{Duration, SystemTime};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔄 KotobaDB Version Control Example");
    println!("===================================\n");

    // Create an in-memory database
    let db = DB::open_memory().await?;
    println!("✅ Database opened (in-memory mode)\n");

    // Create initial dataset
    println!("📝 Creating initial dataset...");

    let project_cid = create_project(&db, "WebApp", "A web application project").await?;
    let user1_cid = create_user(&db, "Alice", "Developer").await?;
    let user2_cid = create_user(&db, "Bob", "Designer").await?;

    assign_user_to_project(&db, user1_cid, project_cid, "lead").await?;
    assign_user_to_project(&db, user2_cid, project_cid, "contributor").await?;

    println!("   Project: WebApp");
    println!("   Team: Alice (lead), Bob (contributor)");
    println!();

    // Create a snapshot of the initial state
    let initial_snapshot = db.create_snapshot("initial-setup").await?;
    println!("📸 Created snapshot: {}", initial_snapshot.id);
    println!();

    // Simulate development workflow
    println!("🚀 Development workflow simulation...");

    // Branch: feature/auth
    let auth_branch = db.create_branch("feature/auth", "main").await?;
    println!("🌿 Created branch: {}", auth_branch);

    db.checkout_branch(auth_branch).await?;
    println!("   Switched to branch: feature/auth");

    // Add authentication features
    let auth_cid = add_feature_to_project(&db, project_cid, "Authentication", "User login/logout system").await?;
    assign_user_to_project(&db, user1_cid, auth_cid, "implementer").await?;
    println!("   Added authentication feature");

    let auth_snapshot = db.create_snapshot("auth-implemented").await?;
    println!("   📸 Created snapshot: {}", auth_snapshot.id);

    // Branch: feature/ui
    let ui_branch = db.create_branch("feature/ui", "main").await?;
    println!("🌿 Created branch: {}", ui_branch);

    db.checkout_branch(ui_branch).await?;
    println!("   Switched to branch: feature/ui");

    // Add UI improvements
    let ui_cid = add_feature_to_project(&db, project_cid, "UI Redesign", "Modern responsive design").await?;
    assign_user_to_project(&db, user2_cid, ui_cid, "designer").await?;
    println!("   Added UI redesign feature");

    let ui_snapshot = db.create_snapshot("ui-redesign").await?;
    println!("   📸 Created snapshot: {}", ui_snapshot.id);

    // Merge auth branch back to main
    println!("\n🔀 Merging branches...");

    db.checkout_branch("main".to_string()).await?;
    println!("   Switched to branch: main");

    db.merge_branch(auth_branch, "main").await?;
    println!("   ✅ Merged feature/auth into main");

    // Check current state
    let current_features = get_project_features(&db, project_cid).await?;
    println!("   Current features on main: {:?}", current_features);

    // Merge UI branch (with potential conflicts)
    db.merge_branch(ui_branch, "main").await?;
    println!("   ✅ Merged feature/ui into main");

    let final_features = get_project_features(&db, project_cid).await?;
    println!("   Final features on main: {:?}", final_features);
    println!();

    // Time travel demonstration
    println!("⏰ Time travel demonstration...");

    // Go back to initial state
    db.restore_snapshot(&initial_snapshot.id).await?;
    println!("   Restored to snapshot: {}", initial_snapshot.id);

    let initial_features = get_project_features(&db, project_cid).await?;
    println!("   Features at initial state: {:?}", initial_features);

    // Go back to auth implementation state
    db.restore_snapshot(&auth_snapshot.id).await?;
    println!("   Restored to snapshot: {}", auth_snapshot.id);

    let auth_features = get_project_features(&db, project_cid).await?;
    println!("   Features at auth state: {:?}", auth_features);

    // Go to latest state
    db.restore_snapshot("HEAD").await?;
    println!("   Restored to latest state (HEAD)");

    let latest_features = get_project_features(&db, project_cid).await?;
    println!("   Features at latest state: {:?}", latest_features);
    println!();

    // Query historical data
    println!("📊 Historical data analysis...");

    let history = db.get_history(project_cid.parse().unwrap()).await?;
    println!("   Project history has {} versions", history.len());

    for (i, (timestamp, cid, node)) in history.iter().enumerate() {
        let name = node.properties.get("name").unwrap().as_string().unwrap();
        println!("   Version {}: {} (CID: {}, Time: {:?})",
                i + 1, name, cid, timestamp);
    }
    println!();

    // Branch analysis
    println!("🌿 Branch analysis...");

    let branches = db.list_branches().await?;
    println!("   Total branches: {}", branches.len());

    for branch in branches {
        let commit_count = db.get_branch_commits(&branch).await?.len();
        println!("   Branch '{}': {} commits", branch, commit_count);
    }
    println!();

    println!("🎉 Version control example completed!");
    println!("   Features demonstrated:");
    println!("   ✅ Branching and merging");
    println!("   ✅ Snapshots and time travel");
    println!("   ✅ Conflict-free version control");
    println!("   ✅ Historical data analysis");

    Ok(())
}

/// Helper function to create a project
async fn create_project(db: &DB, name: &str, description: &str) -> Result<String> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("project".to_string()));
    properties.insert("name".to_string(), Value::String(name.to_string()));
    properties.insert("description".to_string(), Value::String(description.to_string()));
    properties.insert("status".to_string(), Value::String("active".to_string()));

    let cid = db.create_node(properties).await?;
    Ok(cid.to_string())
}

/// Helper function to create a user
async fn create_user(db: &DB, name: &str, role: &str) -> Result<String> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("user".to_string()));
    properties.insert("name".to_string(), Value::String(name.to_string()));
    properties.insert("role".to_string(), Value::String(role.to_string()));

    let cid = db.create_node(properties).await?;
    Ok(cid.to_string())
}

/// Helper function to assign a user to a project
async fn assign_user_to_project(db: &DB, user_cid: String, project_cid: String, role: &str) -> Result<()> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("assignment".to_string()));
    properties.insert("role".to_string(), Value::String(role.to_string()));

    let user_cid_parsed = user_cid.parse().unwrap();
    let project_cid_parsed = project_cid.parse().unwrap();

    db.create_edge(user_cid_parsed, project_cid_parsed, properties).await?;
    Ok(())
}

/// Helper function to add a feature to a project
async fn add_feature_to_project(db: &DB, project_cid: String, feature_name: &str, description: &str) -> Result<String> {
    let mut properties = BTreeMap::new();
    properties.insert("type".to_string(), Value::String("feature".to_string()));
    properties.insert("name".to_string(), Value::String(feature_name.to_string()));
    properties.insert("description".to_string(), Value::String(description.to_string()));
    properties.insert("status".to_string(), Value::String("planned".to_string()));

    let feature_cid = db.create_node(properties).await?;

    // Link feature to project
    let mut link_properties = BTreeMap::new();
    link_properties.insert("type".to_string(), Value::String("belongs_to".to_string()));

    let project_cid_parsed = project_cid.parse().unwrap();
    let feature_cid_parsed = feature_cid.clone();

    db.create_edge(feature_cid_parsed, project_cid_parsed, link_properties).await?;

    Ok(feature_cid.to_string())
}

/// Helper function to get all features of a project
async fn get_project_features(db: &DB, project_cid: String) -> Result<Vec<String>> {
    let project_cid_parsed = project_cid.parse().unwrap();

    // Find all features linked to this project
    let links = db.find_edges(&[
        ("type".to_string(), Value::String("belongs_to".to_string()))
    ]).await?;

    let mut features = Vec::new();

    for (_, edge) in links {
        // Check if this edge targets our project
        if edge.target.to_string() == project_cid {
            // Get the feature node
            if let Some(feature_node) = db.get_node(edge.source).await? {
                if let Some(Value::String(name)) = feature_node.properties.get("name") {
                    features.push(name.clone());
                }
            }
        }
    }

    Ok(features)
}

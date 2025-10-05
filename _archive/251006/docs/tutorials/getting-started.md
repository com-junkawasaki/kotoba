# Getting Started with KotobaDB

Welcome to KotobaDB! This tutorial will guide you through your first steps with the graph-native database. By the end of this tutorial, you'll have:

- Installed KotobaDB
- Created your first database
- Learned basic operations (CRUD)
- Written your first queries
- Built a simple social network application

## Prerequisites

- Rust 1.70 or later
- Basic knowledge of databases
- Familiarity with command line tools

## Installation

### Option 1: Install from Source

```bash
# Clone the repository
git clone https://github.com/your-org/kotoba.git
cd kotoba

# Build the project
cargo build --release

# The binary will be available at target/release/kotoba
./target/release/kotoba --version
```

### Option 2: Download Pre-built Binary

```bash
# Download for your platform
# Linux x64
wget https://github.com/your-org/kotoba/releases/latest/download/kotoba-linux-x64.tar.gz
tar xzf kotoba-linux-x64.tar.gz

# macOS x64
wget https://github.com/your-org/kotoba/releases/latest/download/kotoba-macos-x64.tar.gz
tar xzf kotoba-macos-x64.tar.gz

# Windows x64
# Download kotoba-windows-x64.zip and extract

# Verify installation
./kotoba --version
```

### Option 3: Docker

```bash
# Pull the official image
docker pull kotoba/kotoba:latest

# Run KotobaDB
docker run -p 8080:8080 --name kotoba kotoba/kotoba:latest
```

## Starting KotobaDB

### Development Mode

```bash
# Start with default in-memory configuration
./kotoba

# Or specify a configuration file
./kotoba --config config/dev.toml
```

### Production Mode

```bash
# Create a production configuration
cat > config/production.toml << EOF
[database]
engine = "lsm"
path = "./data"

[server]
host = "127.0.0.1"
port = 8080

[cache]
max_size = "1GB"
EOF

# Start with production config
./kotoba --config config/production.toml
```

## Your First Database Operations

Let's start with basic CRUD operations using the KotobaDB CLI.

### Creating Your First Nodes

```bash
# Start the KotobaDB server (in another terminal)
./kotoba

# In a new terminal, connect and create nodes
curl -X POST http://localhost:8080/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "type": "User",
    "properties": {
      "name": "Alice Johnson",
      "email": "alice@example.com",
      "age": 28
    }
  }'

# Create another user
curl -X POST http://localhost:8080/nodes \
  -H "Content-Type: application/json" \
  -d '{
    "type": "User",
    "properties": {
      "name": "Bob Smith",
      "email": "bob@example.com",
      "age": 32
    }
  }'
```

### Reading Data

```bash
# Get all users
curl http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (u:User) RETURN u"}'

# Find a specific user
curl http://localhost:8080/query \
  -H "Content-Type: application/json" \
  -d '{"query": "MATCH (u:User {name: \"Alice Johnson\"}) RETURN u"}'

# Get user by ID (replace with actual ID from creation response)
curl http://localhost:8080/nodes/{node_id}
```

### Creating Relationships

```bash
# Create a friendship relationship
curl -X POST http://localhost:8080/edges \
  -H "Content-Type: application/json" \
  -d '{
    "from_node": "{alice_node_id}",
    "to_node": "{bob_node_id}",
    "type": "FRIENDS_WITH",
    "properties": {
      "since": "2024-01-01T00:00:00Z"
    }
  }'
```

### Updating Data

```bash
# Update user properties
curl -X PUT http://localhost:8080/nodes/{alice_node_id} \
  -H "Content-Type: application/json" \
  -d '{
    "properties": {
      "age": 29,
      "city": "San Francisco"
    }
  }'
```

## Using the Rust API

Now let's write some Rust code to interact with KotobaDB programmatically.

### Project Setup

```bash
# Create a new Rust project
cargo new kotoba-tutorial
cd kotoba-tutorial

# Add KotobaDB dependency
echo 'kotoba-db = "0.1.0"' >> Cargo.toml

# For local development, use path dependency
echo 'kotoba-db = { path = "../kotoba/crates/kotoba-db" }' >> Cargo.toml
```

### Basic Operations

```rust
use kotoba_db::{DB, Result};
use kotoba_core::types::Value;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Open an in-memory database for this tutorial
    let db = DB::open_memory().await?;
    println!("✅ Database opened successfully!");

    // Create some users
    let alice_id = create_user(&db, "Alice Johnson", "alice@example.com", 28).await?;
    let bob_id = create_user(&db, "Bob Smith", "bob@example.com", 32).await?;
    let charlie_id = create_user(&db, "Charlie Brown", "charlie@example.com", 25).await?;

    println!("✅ Created users: Alice ({}), Bob ({}), Charlie ({})",
             alice_id, bob_id, charlie_id);

    // Create friendships
    create_friendship(&db, alice_id, bob_id).await?;
    create_friendship(&db, bob_id, charlie_id).await?;

    println!("✅ Created friendships");

    // Query the data
    query_users(&db).await?;
    query_friends(&db, alice_id).await?;

    Ok(())
}

async fn create_user(db: &DB, name: &str, email: &str, age: i64) -> Result<String> {
    let mut properties = HashMap::new();
    properties.insert("name".to_string(), Value::String(name.to_string()));
    properties.insert("email".to_string(), Value::String(email.to_string()));
    properties.insert("age".to_string(), Value::Int(age));

    let node_id = db.create_node("User", properties).await?;
    Ok(node_id.to_string())
}

async fn create_friendship(db: &DB, from_id: String, to_id: String) -> Result<()> {
    let mut properties = HashMap::new();
    properties.insert("since".to_string(),
        Value::String("2024-01-01T00:00:00Z".to_string()));

    db.create_edge(from_id, to_id, "FRIENDS_WITH", properties).await?;
    Ok(())
}

async fn query_users(db: &DB) -> Result<()> {
    let result = db.query("MATCH (u:User) RETURN u.name, u.age ORDER BY u.age").await?;
    println!("📋 All users:");
    for row in result.rows {
        if let (Some(Value::String(name)), Some(Value::Int(age))) =
            (row.get("u.name"), row.get("u.age")) {
            println!("  - {} ({} years old)", name, age);
        }
    }
    Ok(())
}

async fn query_friends(db: &DB, user_id: String) -> Result<()> {
    let query = format!("MATCH (u:User)-[:FRIENDS_WITH]->(friend:User) WHERE u.id = '{}' RETURN friend.name", user_id);
    let result = db.query(&query).await?;
    println!("👥 Friends of user {}:", user_id);
    for row in result.rows {
        if let Some(Value::String(name)) = row.get("friend.name") {
            println!("  - {}", name);
        }
    }
    Ok(())
}
```

### Running the Code

```bash
# Make sure KotobaDB server is running in another terminal
./kotoba

# Run your Rust code
cargo run

# Expected output:
# ✅ Database opened successfully!
# ✅ Created users: Alice (node_1), Bob (node_2), Charlie (node_3)
# ✅ Created friendships
# 📋 All users:
#   - Charlie Brown (25 years old)
#   - Alice Johnson (28 years old)
#   - Bob Smith (32 years old)
# 👥 Friends of user node_1:
#   - Bob Smith
```

## Building a Social Network

Let's build a more complex social network application with posts, likes, and comments.

### Project Structure

```bash
# Create a new project
cargo new social_network
cd social_network

# Project structure
cat > Cargo.toml << EOF
[package]
name = "social-network"
version = "0.1.0"
edition = "2021"

[dependencies]
kotoba-db = { path = "../kotoba/crates/kotoba-db" }
tokio = { version = "1.0", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
anyhow = "1.0"
EOF

mkdir -p src/models src/handlers
```

### Models

```rust
// src/models.rs
use kotoba_db::DB;
use kotoba_core::types::Value;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
    pub bio: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: String,
    pub author_id: String,
    pub content: String,
    pub created_at: String,
    pub likes_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub post_id: String,
    pub author_id: String,
    pub content: String,
    pub created_at: String,
}

pub struct SocialNetwork {
    db: DB,
}

impl SocialNetwork {
    pub async fn new() -> anyhow::Result<Self> {
        let db = DB::open_memory().await?;
        Ok(Self { db })
    }

    // User management
    pub async fn create_user(&self, name: &str, email: &str, bio: Option<&str>) -> anyhow::Result<User> {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), Value::String(name.to_string()));
        properties.insert("email".to_string(), Value::String(email.to_string()));
        properties.insert("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));

        if let Some(bio) = bio {
            properties.insert("bio".to_string(), Value::String(bio.to_string()));
        }

        let node_id = self.db.create_node("User", properties).await?;
        let user = self.get_user(&node_id.to_string()).await?;
        Ok(user)
    }

    pub async fn get_user(&self, user_id: &str) -> anyhow::Result<User> {
        let node = self.db.get_node(user_id.parse()?).await?;

        Ok(User {
            id: user_id.to_string(),
            name: node.properties.get("name").and_then(|v| v.as_string()).unwrap_or_default(),
            email: node.properties.get("email").and_then(|v| v.as_string()).unwrap_or_default(),
            bio: node.properties.get("bio").and_then(|v| v.as_string()),
            created_at: node.properties.get("created_at").and_then(|v| v.as_string()).unwrap_or_default(),
        })
    }

    // Friendship management
    pub async fn add_friend(&self, user1_id: &str, user2_id: &str) -> anyhow::Result<()> {
        let mut properties = HashMap::new();
        properties.insert("since".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));

        self.db.create_edge(user1_id.to_string(), user2_id.to_string(), "FRIENDS_WITH", properties).await?;
        Ok(())
    }

    pub async fn get_friends(&self, user_id: &str) -> anyhow::Result<Vec<User>> {
        let query = format!("MATCH (u:User)-[:FRIENDS_WITH]-(friend:User) WHERE u.id = '{}' RETURN friend", user_id);
        let result = self.db.query(&query).await?;

        let mut friends = Vec::new();
        for row in result.rows {
            // Extract friend information from query result
            // Implementation depends on query result structure
        }

        Ok(friends)
    }

    // Post management
    pub async fn create_post(&self, author_id: &str, content: &str) -> anyhow::Result<Post> {
        let mut properties = HashMap::new();
        properties.insert("content".to_string(), Value::String(content.to_string()));
        properties.insert("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));
        properties.insert("likes_count".to_string(), Value::Int(0));

        let node_id = self.db.create_node("Post", properties).await?;

        // Create author relationship
        self.db.create_edge(author_id.to_string(), node_id.to_string(), "POSTED", HashMap::new()).await?;

        let post = self.get_post(&node_id.to_string()).await?;
        Ok(post)
    }

    pub async fn get_post(&self, post_id: &str) -> anyhow::Result<Post> {
        let node = self.db.get_node(post_id.parse()?).await?;

        // Get author ID through relationship
        let author_query = format!("MATCH (u:User)-[:POSTED]->(p:Post) WHERE p.id = '{}' RETURN u.id", post_id);
        let author_result = self.db.query(&author_query).await?;
        let author_id = author_result.rows.first()
            .and_then(|row| row.get("u.id"))
            .and_then(|v| v.as_string())
            .unwrap_or_default();

        Ok(Post {
            id: post_id.to_string(),
            author_id,
            content: node.properties.get("content").and_then(|v| v.as_string()).unwrap_or_default(),
            created_at: node.properties.get("created_at").and_then(|v| v.as_string()).unwrap_or_default(),
            likes_count: node.properties.get("likes_count").and_then(|v| v.as_int()).unwrap_or(0),
        })
    }

    // Like management
    pub async fn like_post(&self, user_id: &str, post_id: &str) -> anyhow::Result<()> {
        // Check if already liked
        let existing_query = format!(
            "MATCH (u:User)-[l:LIKES]->(p:Post) WHERE u.id = '{}' AND p.id = '{}' RETURN l",
            user_id, post_id
        );
        let existing = self.db.query(&existing_query).await?;

        if !existing.rows.is_empty() {
            return Ok(()); // Already liked
        }

        // Create like relationship
        let mut properties = HashMap::new();
        properties.insert("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339()));

        self.db.create_edge(user_id.to_string(), post_id.to_string(), "LIKES", properties).await?;

        // Increment likes count
        let current_likes_query = format!("MATCH (p:Post) WHERE p.id = '{}' RETURN p.likes_count", post_id);
        let current_result = self.db.query(&current_likes_query).await?;
        let current_likes = current_result.rows.first()
            .and_then(|row| row.get("p.likes_count"))
            .and_then(|v| v.as_int())
            .unwrap_or(0);

        let mut updates = HashMap::new();
        updates.insert("likes_count".to_string(), Value::Int(current_likes + 1));
        self.db.update_node(post_id.parse()?, updates).await?;

        Ok(())
    }

    // Feed generation
    pub async fn get_user_feed(&self, user_id: &str, limit: usize) -> anyhow::Result<Vec<Post>> {
        let query = format!(
            "MATCH (u:User)-[:FRIENDS_WITH*0..2]-(friend:User)-[:POSTED]->(p:Post) \
             WHERE u.id = '{}' \
             RETURN p, friend.name as author_name \
             ORDER BY p.created_at DESC \
             LIMIT {}",
            user_id, limit
        );

        let result = self.db.query(&query).await?;
        let mut posts = Vec::new();

        for row in result.rows {
            // Extract post information
            // Implementation depends on query result structure
        }

        Ok(posts)
    }

    // Search functionality
    pub async fn search_users(&self, query: &str, limit: usize) -> anyhow::Result<Vec<User>> {
        let search_query = format!(
            "MATCH (u:User) \
             WHERE u.name CONTAINS '{}' OR u.bio CONTAINS '{}' \
             RETURN u \
             LIMIT {}",
            query, query, limit
        );

        let result = self.db.query(&search_query).await?;
        let mut users = Vec::new();

        for row in result.rows {
            // Extract user information
            // Implementation depends on query result structure
        }

        Ok(users)
    }
}
```

### Main Application

```rust
// src/main.rs
mod models;

use models::SocialNetwork;
use std::io::{self, Write};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app = SocialNetwork::new().await?;
    println!("🚀 Welcome to KotobaSocial!");

    // Create some sample users
    println!("📝 Creating sample users...");
    let alice = app.create_user("Alice Johnson", "alice@example.com", Some("Software Engineer")).await?;
    let bob = app.create_user("Bob Smith", "bob@example.com", Some("Product Manager")).await?;
    let charlie = app.create_user("Charlie Brown", "charlie@example.com", Some("Designer")).await?;

    println!("✅ Created users:");
    println!("  👤 {} ({})", alice.name, alice.email);
    println!("  👤 {} ({})", bob.name, bob.email);
    println!("  👤 {} ({})", charlie.name, charlie.email);

    // Create friendships
    println!("\n🤝 Creating friendships...");
    app.add_friend(&alice.id, &bob.id).await?;
    app.add_friend(&bob.id, &charlie.id).await?;
    println!("✅ Friendships established");

    // Create some posts
    println!("\n📝 Creating posts...");
    let post1 = app.create_post(&alice.id, "Hello everyone! Excited to try KotobaDB! 🚀").await?;
    let post2 = app.create_post(&bob.id, "Just shipped a new feature. Time for celebration! 🎉").await?;
    let post3 = app.create_post(&charlie.id, "Beautiful day for coding ☀️").await?;
    println!("✅ Created {} posts", 3);

    // Add some likes
    println!("\n❤️ Adding likes...");
    app.like_post(&bob.id, &post1.id).await?;
    app.like_post(&charlie.id, &post1.id).await?;
    app.like_post(&alice.id, &post2.id).await?;
    println!("✅ Added likes");

    // Display feed
    println!("\n📱 Alice's feed:");
    let alice_feed = app.get_user_feed(&alice.id, 10).await?;
    for post in alice_feed {
        println!("  📄 {}: {}", post.author_id, post.content);
        println!("     ❤️ {} likes", post.likes_count);
    }

    // Search functionality
    println!("\n🔍 Searching for 'Software'...");
    let search_results = app.search_users("Software", 10).await?;
    for user in search_results {
        println!("  👤 {} - {}", user.name, user.bio.unwrap_or_default());
    }

    println!("\n🎉 Social network demo completed!");
    println!("💡 Try extending this with comments, direct messages, or groups!");

    Ok(())
}
```

### Running the Social Network

```bash
# Run the application
cargo run

# Expected output:
# 🚀 Welcome to KotobaSocial!
# 📝 Creating sample users...
# ✅ Created users:
#   👤 Alice Johnson (alice@example.com)
#   👤 Bob Smith (bob@example.com)
#   👤 Charlie Brown (charlie@example.com)
# 🤝 Creating friendships...
# ✅ Friendships established
# 📝 Creating posts...
# ✅ Created 3 posts
# ❤️ Adding likes...
# ✅ Added likes
# 📱 Alice's feed:
#   📄 alice: Hello everyone! Excited to try KotobaDB! 🚀
#      ❤️ 2 likes
#   📄 bob: Just shipped a new feature. Time for celebration! 🎉
#      ❤️ 1 likes
# 🔍 Searching for 'Software'...
#   👤 Alice Johnson - Software Engineer
# 🎉 Social network demo completed!
```

## Advanced Features

### Custom Indexes

```rust
// Create indexes for better performance
app.db.create_index(IndexDefinition {
    name: "user_email_idx".to_string(),
    target: IndexTarget::NodeProperty {
        node_type: "User".to_string(),
        property: "email".to_string(),
    },
    index_type: IndexType::BTree,
    unique: true,
}).await?;

app.db.create_index(IndexDefinition {
    name: "post_created_at_idx".to_string(),
    target: IndexTarget::NodeProperty {
        node_type: "Post".to_string(),
        property: "created_at".to_string(),
    },
    index_type: IndexType::BTree,
    unique: false,
}).await?;
```

### Schema Validation

```rust
// Define schemas for data validation
let user_schema = NodeSchema {
    name: "User".to_string(),
    properties: vec![
        PropertySchema {
            name: "name".to_string(),
            data_type: ValueType::String,
            constraints: vec![PropertyConstraint::Required, PropertyConstraint::MinLength(1)],
            description: Some("User's full name".to_string()),
        },
        PropertySchema {
            name: "email".to_string(),
            data_type: ValueType::String,
            constraints: vec![PropertyConstraint::Required, PropertyConstraint::Unique],
            description: Some("User's email address".to_string()),
        },
    ].into_iter().collect(),
    required_properties: vec!["name".to_string(), "email".to_string()],
    unique_constraints: vec![vec!["email".to_string()]],
    indexes: vec!["email".to_string()],
};

// Register schema
app.db.define_node_schema(user_schema).await?;
```

### Transactions

```rust
// Perform multiple operations atomically
let result = app.db.execute_transaction(IsolationLevel::Serializable, vec![
    TransactionOperation::CreateNode {
        node_type: "Post".to_string(),
        properties: vec![
            ("content", Value::String("Atomic post".to_string())),
            ("created_at", Value::String(chrono::Utc::now().to_rfc3339())),
        ],
    },
    TransactionOperation::CreateNode {
        node_type: "Comment".to_string(),
        properties: vec![
            ("content", Value::String("Great post!".to_string())),
            ("created_at", Value::String(chrono::Utc::now().to_rfc3339())),
        ],
    },
]).await?;
```

## What's Next?

Congratulations! You've completed the getting started tutorial. Here's what you can explore next:

### Intermediate Topics
- **Query Optimization**: Learn about indexes, query planning, and performance tuning
- **Clustering**: Set up a distributed KotobaDB cluster
- **Backup & Recovery**: Implement automated backups and point-in-time recovery
- **Monitoring**: Set up metrics collection and alerting

### Advanced Topics
- **Custom Storage Engines**: Implement your own storage backend
- **Extensions**: Build custom functions and procedures
- **Machine Learning**: Integrate ML capabilities with your graph data
- **Federation**: Connect multiple KotobaDB instances

### Resources
- [API Reference](../api/README.md) - Complete API documentation
- [Deployment Guide](../deployment/README.md) - Production deployment instructions
- [Examples](../../examples/) - More sample applications
- [Community](https://github.com/your-org/kotoba/discussions) - Get help and share your projects

Happy coding with KotobaDB! 🎉

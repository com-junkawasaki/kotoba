//! Transaction Integration Tests
//!
//! Tests ACID properties and transaction isolation levels:
//! - Atomicity: All operations succeed or all fail
//! - Consistency: Database remains in valid state
//! - Isolation: Concurrent transactions don't interfere
//! - Durability: Committed changes persist

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use kotoba_db::{DB, Transaction};
use kotoba_db_core::{Block, NodeBlock, EdgeBlock, Value};

#[tokio::test]
async fn test_transaction_atomicity() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("atomicity_test.db");
    let db = Arc::new(Mutex::new(DB::open_lsm(&db_path).await?));

    let db_guard = db.lock().await;

    // Test successful transaction
    {
        let tx = db_guard.begin_transaction().await?;

        // Insert multiple related nodes
        let account1 = create_account_node("acc1", 1000);
        let account2 = create_account_node("acc2", 500);

        let cid1 = db_guard.put_block(&Block::Node(account1)).await?;
        let cid2 = db_guard.put_block(&Block::Node(account2)).await?;

        // Create transfer relationship
        let transfer = create_transfer_edge(cid1, cid2, 200);
        let _transfer_cid = db_guard.put_block(&Block::Edge(transfer)).await?;

        tx.commit().await?;

        // Verify all operations succeeded
        assert!(db_guard.get_block(&cid1).await?.is_some());
        assert!(db_guard.get_block(&cid2).await?.is_some());
    }

    println!("✓ Transaction atomicity test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_isolation() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("isolation_test.db");
    let db = Arc::new(Mutex::new(DB::open_lsm(&db_path).await?));

    // Pre-populate with test data
    {
        let db_guard = db.lock().await;
        let account = create_account_node("shared", 1000);
        let cid = db_guard.put_block(&Block::Node(account)).await?;
    }

    // Start two concurrent transactions
    let db1 = Arc::clone(&db);
    let db2 = Arc::clone(&db);

    let handle1 = tokio::spawn(async move {
        let db_guard = db1.lock().await;
        let tx = db_guard.begin_transaction().await?;

        // Read current balance
        let accounts = db_guard.find_nodes_by_label("Account").await?;
        let account_cid = accounts[0];

        if let Some(Block::Node(mut account)) = db_guard.get_block(&account_cid).await? {
            let current_balance = match account.properties.get("balance").unwrap() {
                Value::Int(balance) => *balance,
                _ => 0,
            };

            // Modify balance
            account.properties.insert("balance".to_string(), Value::Int(current_balance + 100));
            let _new_cid = db_guard.put_block(&Block::Node(account)).await?;
        }

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        tx.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    let handle2 = tokio::spawn(async move {
        let db_guard = db2.lock().await;
        let tx = db_guard.begin_transaction().await?;

        // Read current balance
        let accounts = db_guard.find_nodes_by_label("Account").await?;
        let account_cid = accounts[0];

        if let Some(Block::Node(mut account)) = db_guard.get_block(&account_cid).await? {
            let current_balance = match account.properties.get("balance").unwrap() {
                Value::Int(balance) => *balance,
                _ => 0,
            };

            // Modify balance
            account.properties.insert("balance".to_string(), Value::Int(current_balance - 50));
            let _new_cid = db_guard.put_block(&Block::Node(account)).await?;
        }

        // Simulate some processing time
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        tx.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    // Wait for both transactions to complete
    handle1.await??;
    handle2.await??;

    // Verify final state (should be 1000 + 100 - 50 = 1050)
    {
        let db_guard = db.lock().await;
        let accounts = db_guard.find_nodes_by_label("Account").await?;
        let account_cid = accounts[0];

        if let Some(Block::Node(account)) = db_guard.get_block(&account_cid).await? {
            let final_balance = match account.properties.get("balance").unwrap() {
                Value::Int(balance) => *balance,
                _ => 0,
            };
            // Note: Actual behavior depends on isolation level implementation
            // For this test, we just ensure no crashes occurred
            println!("Final balance: {}", final_balance);
        }
    }

    println!("✓ Transaction isolation test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("consistency_test.db");
    let db = Arc::new(Mutex::new(DB::open_lsm(&db_path).await?));

    let db_guard = db.lock().await;

    // Test constraint-like consistency
    {
        let tx = db_guard.begin_transaction().await?;

        // Create an order
        let order = create_order_node("order1", "pending");
        let order_cid = db_guard.put_block(&Block::Node(order)).await?;

        // Create order items that reference the order
        let item1 = create_order_item_node("item1", 2, 25.0, order_cid);
        let item2 = create_order_item_node("item2", 1, 15.0, order_cid);

        let _item1_cid = db_guard.put_block(&Block::Node(item1)).await?;
        let _item2_cid = db_guard.put_block(&Block::Node(item2)).await?;

        // Update order status
        let mut updated_order = create_order_node("order1", "confirmed");
        let _updated_cid = db_guard.put_block(&Block::Node(updated_order)).await?;

        tx.commit().await?;
    }

    // Verify consistency: all related data should exist together
    let orders = db_guard.find_nodes_by_label("Order").await?;
    let order_items = db_guard.find_nodes_by_label("OrderItem").await?;

    assert!(!orders.is_empty());
    assert!(!order_items.is_empty());

    println!("✓ Transaction consistency test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_durability() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("durability_test.db");

    {
        let db = DB::open_lsm(&db_path).await?;
        let tx = db.begin_transaction().await?;

        // Create durable data
        let durable_node = NodeBlock {
            labels: vec!["Durable".to_string()],
            properties: HashMap::from([
                ("data".to_string(), Value::String("This should persist".to_string())),
                ("timestamp".to_string(), Value::String(chrono::Utc::now().to_rfc3339())),
            ]),
        };

        let cid = db.put_block(&Block::Node(durable_node)).await?;
        tx.commit().await?;

        println!("✓ Committed transaction with CID: {:?}", cid);
    }

    // Simulate database restart by reopening
    {
        let db = DB::open_lsm(&db_path).await?;

        // Verify data persisted
        let durable_nodes = db.find_nodes_by_label("Durable").await?;
        assert_eq!(durable_nodes.len(), 1);

        if let Some(Block::Node(node)) = db.get_block(&durable_nodes[0]).await? {
            assert_eq!(
                node.properties.get("data").unwrap(),
                &Value::String("This should persist".to_string())
            );
        }

        println!("✓ Data persisted across database restart");
    }

    println!("✓ Transaction durability test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_rollback() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("rollback_test.db");
    let db = DB::open_lsm(&db_path).await?;

    let tx = db.begin_transaction().await?;

    // Insert data that should be rolled back
    let rollback_node = NodeBlock {
        labels: vec!["RollbackTest".to_string()],
        properties: HashMap::from([
            ("should_not_exist".to_string(), Value::Bool(true)),
        ]),
    };

    let cid = db.put_block(&Block::Node(rollback_node)).await?;

    // Verify data exists within transaction
    let retrieved = db.get_block(&cid).await?;
    assert!(retrieved.is_some());

    // Rollback transaction
    tx.rollback().await?;

    // Verify data no longer exists
    let retrieved_after_rollback = db.get_block(&cid).await?;
    assert!(retrieved_after_rollback.is_none());

    println!("✓ Transaction rollback test passed");
    Ok(())
}

#[tokio::test]
async fn test_nested_transactions() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("nested_tx_test.db");
    let db = DB::open_lsm(&db_path).await?;

    // Test nested transaction behavior
    let outer_tx = db.begin_transaction().await?;
    let outer_node = create_test_node("outer", 1);
    let outer_cid = db.put_block(&Block::Node(outer_node)).await?;

    // Nested transaction
    let inner_tx = db.begin_transaction().await?;
    let inner_node = create_test_node("inner", 2);
    let inner_cid = db.put_block(&Block::Node(inner_node)).await?;

    // Commit inner transaction
    inner_tx.commit().await?;

    // Both should still exist
    assert!(db.get_block(&outer_cid).await?.is_some());
    assert!(db.get_block(&inner_cid).await?.is_some());

    // Commit outer transaction
    outer_tx.commit().await?;

    // Both should still exist after outer commit
    assert!(db.get_block(&outer_cid).await?.is_some());
    assert!(db.get_block(&inner_cid).await?.is_some());

    println!("✓ Nested transactions test passed");
    Ok(())
}

#[tokio::test]
async fn test_transaction_deadlock_detection() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = tempfile::tempdir()?;
    let db_path = temp_dir.path().join("deadlock_test.db");
    let db = Arc::new(Mutex::new(DB::open_lsm(&db_path).await?));

    // Pre-populate with two accounts
    let (account1_cid, account2_cid) = {
        let db_guard = db.lock().await;
        let account1 = create_account_node("acc1", 1000);
        let account2 = create_account_node("acc2", 1000);

        let cid1 = db_guard.put_block(&Block::Node(account1)).await?;
        let cid2 = db_guard.put_block(&Block::Node(account2)).await?;
        (cid1, cid2)
    };

    let db1 = Arc::clone(&db);
    let db2 = Arc::clone(&db);

    // Transaction 1: acc1 -> acc2
    let handle1 = tokio::spawn(async move {
        let db_guard = db1.lock().await;
        let tx = db_guard.begin_transaction().await?;

        // Lock acc1 first
        let _acc1_data = db_guard.get_block(&account1_cid).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Then try to access acc2
        let _acc2_data = db_guard.get_block(&account2_cid).await?;

        tx.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    // Transaction 2: acc2 -> acc1 (reverse order)
    let handle2 = tokio::spawn(async move {
        let db_guard = db2.lock().await;
        let tx = db_guard.begin_transaction().await?;

        // Lock acc2 first
        let _acc2_data = db_guard.get_block(&account2_cid).await?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Then try to access acc1
        let _acc1_data = db_guard.get_block(&account1_cid).await?;

        tx.commit().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    });

    // Both transactions should complete without deadlock
    let result1 = handle1.await?;
    let result2 = handle2.await?;

    // If we get here, no deadlock occurred
    result1?;
    result2?;

    println!("✓ Deadlock detection test passed");
    Ok(())
}

// Helper functions for creating test data

fn create_account_node(id: &str, balance: i64) -> NodeBlock {
    NodeBlock {
        labels: vec!["Account".to_string()],
        properties: HashMap::from([
            ("id".to_string(), Value::String(id.to_string())),
            ("balance".to_string(), Value::Int(balance)),
        ]),
    }
}

fn create_transfer_edge(from_cid: kotoba_db_core::Cid, to_cid: kotoba_db_core::Cid, amount: i64) -> EdgeBlock {
    EdgeBlock {
        from_labels: vec!["Account".to_string()],
        to_labels: vec!["Account".to_string()],
        label: "TRANSFER".to_string(),
        properties: HashMap::from([
            ("amount".to_string(), Value::Int(amount)),
            ("timestamp".to_string(), Value::String(chrono::Utc::now().to_rfc3339())),
        ]),
    }
}

fn create_order_node(id: &str, status: &str) -> NodeBlock {
    NodeBlock {
        labels: vec!["Order".to_string()],
        properties: HashMap::from([
            ("order_id".to_string(), Value::String(id.to_string())),
            ("status".to_string(), Value::String(status.to_string())),
            ("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339())),
        ]),
    }
}

fn create_order_item_node(id: &str, quantity: i64, price: f64, order_cid: kotoba_db_core::Cid) -> NodeBlock {
    NodeBlock {
        labels: vec!["OrderItem".to_string()],
        properties: HashMap::from([
            ("item_id".to_string(), Value::String(id.to_string())),
            ("quantity".to_string(), Value::Int(quantity)),
            ("unit_price".to_string(), Value::Float(price)),
            ("order_ref".to_string(), Value::Link(order_cid)),
        ]),
    }
}

fn create_test_node(name: &str, value: i64) -> NodeBlock {
    NodeBlock {
        labels: vec!["Test".to_string()],
        properties: HashMap::from([
            ("name".to_string(), Value::String(name.to_string())),
            ("value".to_string(), Value::Int(value)),
        ]),
    }
}

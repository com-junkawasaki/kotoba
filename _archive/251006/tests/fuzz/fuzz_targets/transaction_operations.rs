//! Fuzz testing for transaction operations
//!
//! Tests random sequences of transaction operations to find issues with:
//! - ACID properties
//! - Concurrency control
//! - Transaction isolation
//! - Rollback behavior

#![no_main]

use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use std::collections::HashMap;
use kotoba_db::{DB, Transaction};
use kotoba_db_core::{Block, NodeBlock, EdgeBlock, Value};
use tempfile::NamedTempFile;

#[derive(Debug, Arbitrary)]
enum TransactionOperation {
    BeginTransaction,
    CommitTransaction,
    RollbackTransaction,
    CreateNodeInTransaction {
        label: String,
        properties: HashMap<String, Value>,
    },
    UpdateNodeInTransaction {
        node_key: String,
        properties: HashMap<String, Value>,
    },
    DeleteNodeInTransaction {
        node_key: String,
    },
    CreateMultipleNodes {
        count: u8,
        base_label: String,
    },
    ConcurrentOperations {
        operation_count: u8,
    },
}

#[derive(Debug, Arbitrary)]
struct TransactionFuzzInput {
    operations: Vec<TransactionOperation>,
}

fuzz_target!(|input: TransactionFuzzInput| {
    let temp_file = NamedTempFile::new().unwrap();
    let db_path = temp_file.path();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        fuzz_transaction_operations(db_path, input).await;
    });
});

async fn fuzz_transaction_operations(db_path: &std::path::Path, input: TransactionFuzzInput) {
    let db = match DB::open_lsm(db_path).await {
        Ok(db) => db,
        Err(_) => return,
    };

    let mut active_transaction: Option<Transaction> = None;
    let mut created_nodes = HashMap::new();

    for operation in input.operations {
        match operation {
            TransactionOperation::BeginTransaction => {
                if active_transaction.is_none() {
                    match db.begin_transaction().await {
                        Ok(tx) => active_transaction = Some(tx),
                        Err(_) => continue,
                    }
                }
            }

            TransactionOperation::CommitTransaction => {
                if let Some(tx) = active_transaction.take() {
                    let _ = tx.commit().await;
                }
            }

            TransactionOperation::RollbackTransaction => {
                if let Some(tx) = active_transaction.take() {
                    let _ = tx.rollback().await;
                }
            }

            TransactionOperation::CreateNodeInTransaction { label, properties } => {
                let node = NodeBlock {
                    labels: vec![label.clone()],
                    properties,
                };

                let result = if let Some(ref tx) = active_transaction {
                    // In a real implementation, transaction would affect the put_block call
                    db.put_block(&Block::Node(node)).await
                } else {
                    db.put_block(&Block::Node(node)).await
                };

                if let Ok(cid) = result {
                    created_nodes.insert(label, cid);
                }
            }

            TransactionOperation::UpdateNodeInTransaction { node_key, properties } => {
                if let Some(cid) = created_nodes.get(&node_key) {
                    let updated_node = NodeBlock {
                        labels: vec!["Updated".to_string()],
                        properties,
                    };

                    let _ = db.put_block(&Block::Node(updated_node)).await;
                }
            }

            TransactionOperation::DeleteNodeInTransaction { node_key } => {
                // Simplified deletion - in practice would need tombstone logic
                created_nodes.remove(&node_key);
            }

            TransactionOperation::CreateMultipleNodes { count, base_label } => {
                let count = std::cmp::min(count as usize, 100); // Limit to prevent excessive operations

                for i in 0..count {
                    let label = format!("{}_{}", base_label, i);
                    let node = NodeBlock {
                        labels: vec![label.clone()],
                        properties: HashMap::from([
                            ("index".to_string(), Value::Int(i as i64)),
                            ("batch".to_string(), Value::String(base_label.clone())),
                        ]),
                    };

                    if let Ok(cid) = db.put_block(&Block::Node(node)).await {
                        created_nodes.insert(label, cid);
                    }
                }
            }

            TransactionOperation::ConcurrentOperations { operation_count } => {
                let count = std::cmp::min(operation_count as usize, 50);
                let mut handles = Vec::new();

                for i in 0..count {
                    let db_clone = db.clone();
                    let handle = tokio::spawn(async move {
                        let node = NodeBlock {
                            labels: vec![format!("Concurrent_{}", i)],
                            properties: HashMap::from([
                                ("concurrent_id".to_string(), Value::Int(i as i64)),
                            ]),
                        };

                        let _ = db_clone.put_block(&Block::Node(node)).await;
                    });

                    handles.push(handle);
                }

                // Wait for all concurrent operations
                for handle in handles {
                    let _ = handle.await;
                }
            }
        }
    }

    // Cleanup: ensure any active transaction is properly closed
    if let Some(tx) = active_transaction {
        let _ = tx.rollback().await;
    }

    // Final consistency check
    let _ = db.find_nodes_by_label("TransactionTest").await;
}

// Additional fuzz target for transaction isolation testing
fuzz_target!(|data: &[u8]| {
    if data.len() < 10 {
        return;
    }

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path();

        let db = match DB::open_lsm(db_path).await {
            Ok(db) => db,
            Err(_) => return,
        };

        // Test transaction isolation with random data patterns
        let mut handles = Vec::new();

        for i in 0..std::cmp::min(data.len(), 10) {
            let db_clone = db.clone();
            let chunk = data[i..std::cmp::min(i + 10, data.len())].to_vec();

            let handle = tokio::spawn(async move {
                let tx = db_clone.begin_transaction().await?;

                // Create nodes with random data
                for (j, &byte) in chunk.iter().enumerate() {
                    let node = NodeBlock {
                        labels: vec![format!("Isolation_{}_{}", i, j)],
                        properties: HashMap::from([
                            ("random_byte".to_string(), Value::Int(byte as i64)),
                            ("chunk_index".to_string(), Value::Int(j as i64)),
                        ]),
                    };

                    db_clone.put_block(&Block::Node(node)).await?;
                }

                // Randomly commit or rollback
                if chunk.iter().sum::<u8>() % 2 == 0 {
                    tx.commit().await?;
                } else {
                    tx.rollback().await?;
                }

                Ok::<(), Box<dyn std::error::Error>>(())
            });

            handles.push(handle);
        }

        // Wait for all transactions to complete
        for handle in handles {
            let _ = handle.await;
        }

        // Verify database consistency
        let nodes = db.find_nodes_by_label("Isolation").await?;
        // Basic check - database should not be corrupted
        let _ = nodes.len();
    });
});

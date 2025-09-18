//! Predefined Benchmark Workloads
//!
//! Ready-to-use benchmark implementations for common database operations

use crate::{Benchmark, BenchmarkConfig, BenchmarkResult, utils};
use kotoba_db::DB;
use kotoba_db_core::{Block, NodeBlock, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// CRUD Operations Benchmark
pub struct CrudBenchmark {
    db: Arc<DB>,
    record_count: usize,
    operation_mix: CrudOperationMix,
}

#[derive(Debug, Clone)]
pub struct CrudOperationMix {
    pub create_percent: f64,
    pub read_percent: f64,
    pub update_percent: f64,
    pub delete_percent: f64,
}

impl Default for CrudOperationMix {
    fn default() -> Self {
        Self {
            create_percent: 0.25,
            read_percent: 0.50,
            update_percent: 0.20,
            delete_percent: 0.05,
        }
    }
}

impl CrudBenchmark {
    pub fn new(db: Arc<DB>, record_count: usize) -> Self {
        Self {
            db,
            record_count,
            operation_mix: CrudOperationMix::default(),
        }
    }

    pub fn with_operation_mix(mut self, mix: CrudOperationMix) -> Self {
        self.operation_mix = mix;
        self
    }

    fn generate_operation(&self, operation_count: u64) -> crate::Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val = rng.gen::<f64>();

        let record_id = (operation_count % self.record_count as u64) as usize;

        if rand_val < self.operation_mix.create_percent {
            // Create
            let key = format!("crud_record_{}_create_{}", record_id, operation_count);
            let value = format!("value_{}_{}", record_id, operation_count).into_bytes();
            crate::Operation::Insert { key: key.into_bytes(), value }
        } else if rand_val < self.operation_mix.create_percent + self.operation_mix.read_percent {
            // Read
            let key = format!("crud_record_{}", record_id);
            crate::Operation::Read { key: key.into_bytes() }
        } else if rand_val < self.operation_mix.create_percent + self.operation_mix.read_percent + self.operation_mix.update_percent {
            // Update
            let key = format!("crud_record_{}", record_id);
            let value = format!("updated_value_{}_{}", record_id, operation_count).into_bytes();
            crate::Operation::Update { key: key.into_bytes(), value }
        } else {
            // Delete
            let key = format!("crud_record_{}", record_id);
            crate::Operation::Delete { key: key.into_bytes() }
        }
    }
}

#[async_trait::async_trait]
impl Benchmark for CrudBenchmark {
    fn name(&self) -> &str {
        "CRUD Operations"
    }

    async fn setup(&mut self, _config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Pre-populate some initial data
        for i in 0..std::cmp::min(self.record_count, 1000) {
            let node = NodeBlock {
                labels: vec!["CrudRecord".to_string()],
                properties: HashMap::from([
                    ("record_id".to_string(), Value::Int(i as i64)),
                    ("data".to_string(), Value::String(format!("initial_data_{}", i))),
                    ("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339())),
                ]),
            };

            self.db.put_block(&Block::Node(node)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::{runner::BenchmarkRunner, LatencyPercentiles};

        let runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Cleanup is handled by temp directory
        Ok(())
    }
}

impl crate::BenchmarkExt for CrudBenchmark {
    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        let operation = self.generate_operation(operation_count);

        match operation {
            crate::Operation::Insert { key, value } => {
                let node = NodeBlock {
                    labels: vec!["CrudRecord".to_string()],
                    properties: HashMap::from([
                        ("key".to_string(), Value::String(String::from_utf8_lossy(&key).to_string())),
                        ("value".to_string(), Value::String(String::from_utf8_lossy(&value).to_string())),
                    ]),
                };
                self.db.put_block(&Block::Node(node)).await?;
            }
            crate::Operation::Read { key } => {
                // Try to find a node with this key pattern
                let nodes = self.db.find_nodes_by_label("CrudRecord").await?;
                if let Some(node_cid) = nodes.first() {
                    let _ = self.db.get_block(node_cid).await?;
                }
            }
            crate::Operation::Update { key, value } => {
                // Simplified update - create new version
                let node = NodeBlock {
                    labels: vec!["CrudRecord".to_string()],
                    properties: HashMap::from([
                        ("key".to_string(), Value::String(String::from_utf8_lossy(&key).to_string())),
                        ("value".to_string(), Value::String(String::from_utf8_lossy(&value).to_string())),
                        ("updated".to_string(), Value::Bool(true)),
                    ]),
                };
                self.db.put_block(&Block::Node(node)).await?;
            }
            crate::Operation::Delete { key } => {
                // Simplified delete - just mark as deleted
                let node = NodeBlock {
                    labels: vec!["CrudRecord".to_string()],
                    properties: HashMap::from([
                        ("key".to_string(), Value::String(String::from_utf8_lossy(&key).to_string())),
                        ("deleted".to_string(), Value::Bool(true)),
                    ]),
                };
                self.db.put_block(&Block::Node(node)).await?;
            }
            _ => {}
        }

        Ok(())
    }
}

/// Query Performance Benchmark
pub struct QueryBenchmark {
    db: Arc<DB>,
    record_count: usize,
}

impl QueryBenchmark {
    pub fn new(db: Arc<DB>, record_count: usize) -> Self {
        Self { db, record_count }
    }
}

#[async_trait::async_trait]
impl Benchmark for QueryBenchmark {
    fn name(&self) -> &str {
        "Query Performance"
    }

    async fn setup(&mut self, _config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Create test data with various properties for querying
        for i in 0..std::cmp::min(self.record_count, 10000) {
            let node = NodeBlock {
                labels: vec![format!("QueryTest{}", i % 10)], // 10 different labels
                properties: HashMap::from([
                    ("id".to_string(), Value::Int(i as i64)),
                    ("category".to_string(), Value::String(format!("cat_{}", i % 5))),
                    ("score".to_string(), Value::Int((i % 100) as i64)),
                    ("active".to_string(), Value::Bool(i % 2 == 0)),
                    ("tags".to_string(), Value::Array(vec![
                        Value::String(format!("tag{}", i % 3)),
                        Value::String(format!("group{}", i % 4)),
                    ])),
                ]),
            };

            self.db.put_block(&Block::Node(node)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl crate::BenchmarkExt for QueryBenchmark {
    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let query_type = rng.gen_range(0..5);

        match query_type {
            0 => {
                // Label-based query
                let label = format!("QueryTest{}", rng.gen_range(0..10));
                let _ = self.db.find_nodes_by_label(&label).await?;
            }
            1 => {
                // Property-based query
                let score = rng.gen_range(0..100) as i64;
                let _ = self.db.find_nodes_by_property("score", &Value::Int(score)).await?;
            }
            2 => {
                // Range query simulation
                let min_score = rng.gen_range(0..50) as i64;
                let max_score = min_score + rng.gen_range(10..50) as i64;
                // Note: Actual range queries would depend on implementation
                let _ = self.db.find_nodes_by_property_range("score", &Value::Int(min_score), &Value::Int(max_score)).await?;
            }
            3 => {
                // Complex query (multiple conditions)
                let category = format!("cat_{}", rng.gen_range(0..5));
                let active = rng.gen_bool(0.5);
                let _ = self.db.find_nodes_by_properties(
                    &[
                        ("category".to_string(), Value::String(category)),
                        ("active".to_string(), Value::Bool(active)),
                    ],
                    Some("QueryTest0")
                ).await?;
            }
            _ => {
                // Scan operation
                let _ = self.db.find_nodes_by_label("QueryTest0").await?;
            }
        }

        Ok(())
    }
}

/// Transaction Throughput Benchmark
pub struct TransactionBenchmark {
    db: Arc<DB>,
    transaction_size: usize,
}

impl TransactionBenchmark {
    pub fn new(db: Arc<DB>, transaction_size: usize) -> Self {
        Self { db, transaction_size }
    }
}

#[async_trait::async_trait]
impl Benchmark for TransactionBenchmark {
    fn name(&self) -> &str {
        "Transaction Throughput"
    }

    async fn setup(&mut self, _config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        // Pre-populate accounts for transfer operations
        for i in 0..100 {
            let account = NodeBlock {
                labels: vec!["Account".to_string()],
                properties: HashMap::from([
                    ("account_id".to_string(), Value::String(format!("acc_{}", i))),
                    ("balance".to_string(), Value::Int(1000)),
                ]),
            };

            self.db.put_block(&Block::Node(account)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl crate::BenchmarkExt for TransactionBenchmark {
    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let tx = self.db.begin_transaction().await?;

        // Perform multiple operations in a transaction
        for i in 0..self.transaction_size {
            let account_id = rng.gen_range(0..100);
            let operation_type = rng.gen_range(0..3);

            match operation_type {
                0 => {
                    // Read balance
                    // Simplified - just create a read operation
                    let _accounts = self.db.find_nodes_by_label("Account").await?;
                }
                1 => {
                    // Update balance
                    let new_balance = rng.gen_range(500..1500);
                    let account = NodeBlock {
                        labels: vec!["Account".to_string()],
                        properties: HashMap::from([
                            ("account_id".to_string(), Value::String(format!("acc_{}", account_id))),
                            ("balance".to_string(), Value::Int(new_balance)),
                            ("last_tx".to_string(), Value::Int(operation_count as i64)),
                        ]),
                    };
                    self.db.put_block(&Block::Node(account)).await?;
                }
                _ => {
                    // Transfer between accounts
                    let from_id = rng.gen_range(0..100);
                    let to_id = rng.gen_range(0..100);
                    let amount = rng.gen_range(1..100);

                    // Simplified transfer - just log the operation
                    let transfer = NodeBlock {
                        labels: vec!["Transfer".to_string()],
                        properties: HashMap::from([
                            ("from".to_string(), Value::String(format!("acc_{}", from_id))),
                            ("to".to_string(), Value::String(format!("acc_{}", to_id))),
                            ("amount".to_string(), Value::Int(amount)),
                            ("tx_id".to_string(), Value::Int(operation_count as i64)),
                        ]),
                    };
                    self.db.put_block(&Block::Node(transfer)).await?;
                }
            }
        }

        // Randomly commit or rollback
        if rng.gen_bool(0.9) { // 90% success rate
            tx.commit().await?;
        } else {
            tx.rollback().await?;
        }

        Ok(())
    }
}

/// Memory Usage Benchmark
pub struct MemoryBenchmark {
    db: Arc<DB>,
    data_size: usize,
}

impl MemoryBenchmark {
    pub fn new(db: Arc<DB>, data_size: usize) -> Self {
        Self { db, data_size }
    }
}

#[async_trait::async_trait]
impl Benchmark for MemoryBenchmark {
    fn name(&self) -> &str {
        "Memory Usage"
    }

    async fn setup(&mut self, _config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let runner = crate::runner::BenchmarkRunner::new(config.clone()).with_metrics_collection();
        runner.run_benchmark(self).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl crate::BenchmarkExt for MemoryBenchmark {
    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Create data that consumes memory
        let large_data = vec![b'A'; self.data_size];

        let node = NodeBlock {
            labels: vec!["MemoryTest".to_string()],
            properties: HashMap::from([
                ("id".to_string(), Value::Int(operation_count as i64)),
                ("data".to_string(), Value::String(String::from_utf8_lossy(&large_data).to_string())),
                ("size".to_string(), Value::Int(self.data_size as i64)),
            ]),
        };

        self.db.put_block(&Block::Node(node)).await?;

        // Simulate some memory operations
        let _processed_data = large_data.into_iter().map(|b| b as char).collect::<String>();

        Ok(())
    }
}

/// Storage Operations Benchmark
pub struct StorageBenchmark {
    db: Arc<DB>,
    operation_count: usize,
}

impl StorageBenchmark {
    pub fn new(db: Arc<DB>, operation_count: usize) -> Self {
        Self { db, operation_count }
    }
}

#[async_trait::async_trait]
impl Benchmark for StorageBenchmark {
    fn name(&self) -> &str {
        "Storage Operations"
    }

    async fn setup(&mut self, _config: &BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let runner = crate::runner::BenchmarkRunner::new(config.clone()).with_metrics_collection();
        runner.run_benchmark(self).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}

impl crate::BenchmarkExt for StorageBenchmark {
    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        match rng.gen_range(0..4) {
            0 => {
                // Sequential write
                let node = NodeBlock {
                    labels: vec!["StorageTest".to_string()],
                    properties: HashMap::from([
                        ("seq_id".to_string(), Value::Int(operation_count as i64)),
                        ("data".to_string(), Value::String(format!("sequential_data_{}", operation_count))),
                    ]),
                };
                self.db.put_block(&Block::Node(node)).await?;
            }
            1 => {
                // Random read
                let target_id = rng.gen_range(0..std::cmp::max(1, operation_count as usize)) as i64;
                // Simplified read operation
                let _ = self.db.find_nodes_by_label("StorageTest").await?;
            }
            2 => {
                // Batch write simulation
                for i in 0..5 {
                    let node = NodeBlock {
                        labels: vec!["BatchStorageTest".to_string()],
                        properties: HashMap::from([
                            ("batch_id".to_string(), Value::Int(operation_count as i64)),
                            ("item_id".to_string(), Value::Int(i)),
                            ("data".to_string(), Value::String(format!("batch_data_{}_{}", operation_count, i))),
                        ]),
                    };
                    self.db.put_block(&Block::Node(node)).await?;
                }
            }
            _ => {
                // Large object write
                let large_data = (0..1000).map(|i| format!("large_data_item_{}_", i)).collect::<String>();
                let node = NodeBlock {
                    labels: vec!["LargeStorageTest".to_string()],
                    properties: HashMap::from([
                        ("id".to_string(), Value::Int(operation_count as i64)),
                        ("large_data".to_string(), Value::String(large_data)),
                    ]),
                };
                self.db.put_block(&Block::Node(node)).await?;
            }
        }

        Ok(())
    }
}

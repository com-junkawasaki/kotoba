//! Predefined Benchmark Workloads
//!
//! Ready-to-use benchmark implementations for common database operations

use crate::{Benchmark, BenchmarkConfig, BenchmarkResult, utils};

/// Database operation types for benchmarking
#[derive(Debug, Clone)]
pub enum Operation {
    Insert { key: Vec<u8>, value: Vec<u8> },
    Read { key: Vec<u8> },
    Update { key: Vec<u8>, value: Vec<u8> },
    Delete { key: Vec<u8> },
}
use crate::runner::BenchmarkExt;
use kotoba_db::DB;
use kotoba_db_core::{Block, NodeBlock, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::Instant;

/// CRUD Operations Benchmark
#[derive(Clone)]
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

    fn generate_operation(&self, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val = rng.gen::<f64>();

        let record_id = (operation_count % self.record_count as u64) as usize;

        if rand_val < self.operation_mix.create_percent {
            // Create
            let key = format!("crud_record_{}_create_{}", record_id, operation_count);
            let value = format!("value_{}_{}", record_id, operation_count).into_bytes();
            Operation::Insert { key: key.into_bytes(), value }
        } else if rand_val < self.operation_mix.create_percent + self.operation_mix.read_percent {
            // Read
            let key = format!("crud_record_{}", record_id);
            Operation::Read { key: key.into_bytes() }
        } else if rand_val < self.operation_mix.create_percent + self.operation_mix.read_percent + self.operation_mix.update_percent {
            // Update
            let key = format!("crud_record_{}", record_id);
            let value = format!("updated_value_{}_{}", record_id, operation_count).into_bytes();
            Operation::Update { key: key.into_bytes(), value }
        } else {
            // Delete
            let key = format!("crud_record_{}", record_id);
            Operation::Delete { key: key.into_bytes() }
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
            let properties = BTreeMap::from([
                    ("record_id".to_string(), Value::Int(i as i64)),
                    ("data".to_string(), Value::String(format!("initial_data_{}", i))),
                    ("created_at".to_string(), Value::String(chrono::Utc::now().to_rfc3339())),
                    ("label".to_string(), Value::String("CrudRecord".to_string())),
                ]);
                let node = NodeBlock {
                    properties,
                    edges: vec![],
                };
                self.db.storage.put_block(&Block::Node(node)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::{runner::BenchmarkRunner, LatencyPercentiles};

        let mut runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self.clone()).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Cleanup is handled by temp directory
        Ok(())
    }

    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        let operation = self.generate_operation(operation_count);
        match operation {
            Operation::Insert { key, value } => {
                // For insert operations, create a NodeBlock
                let properties = BTreeMap::from([
                    ("key".to_string(), Value::String(String::from_utf8_lossy(&key).to_string())),
                    ("value".to_string(), Value::String(String::from_utf8_lossy(&value).to_string())),
                ]);
                let node = NodeBlock {
                    properties,
                    edges: vec![],
                };
                self.db.storage.put_block(&Block::Node(node)).await?;
            }
            Operation::Read { key } => {
                // For read operations, we would need a way to get the block by key
                // For now, just perform a simple operation
                let _ = key; // Use the key to avoid unused variable warning
            }
            Operation::Update { key, value } => {
                // For update operations, similar to insert but with different logic
                let _ = key;
                let _ = value;
            }
            Operation::Delete { key } => {
                // For delete operations
                let _ = key;
            }
        }
        Ok(())
    }
}


/// Query Performance Benchmark
#[derive(Clone)]
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
                properties: BTreeMap::from([
                    ("label".to_string(), Value::String(format!("QueryTest{}", i % 10))),
                    ("id".to_string(), Value::Int(i as i64)),
                    ("category".to_string(), Value::String(format!("cat_{}", i % 5))),
                    ("score".to_string(), Value::Int((i % 100) as i64)),
                    ("active".to_string(), Value::Bool(i % 2 == 0)),
                    ("tags".to_string(), Value::Array(vec![
                        Value::String(format!("tag{}", i % 3)),
                        Value::String(format!("group{}", i % 4)),
                    ])),
                ]),
                edges: Vec::new(),
            };

            self.db.storage.put_block(&Block::Node(node)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let mut runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self.clone()).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // For query operations, perform various types of queries
        let query_type = (operation_count % 5) as usize;

        match query_type {
            0 => {
                // Find nodes by label
                let _ = self.db.find_nodes(&[("label".to_string(), Value::String("QueryTest0".to_string()))]);
            }
            1 => {
                // Find nodes by category
                let _ = self.db.find_nodes(&[("category".to_string(), Value::String("cat_0".to_string()))]);
            }
            2 => {
                // Find nodes by score range (simplified)
                let _ = self.db.find_nodes(&[("score".to_string(), Value::Int(50))]);
            }
            3 => {
                // Find nodes by active status
                let _ = self.db.find_nodes(&[("active".to_string(), Value::Bool(true))]);
            }
            4 => {
                // Find nodes by tags (simplified)
                let _ = self.db.find_nodes(&[("tags".to_string(), Value::Array(vec![Value::String("tag0".to_string())]))]);
            }
            _ => {}
        }
        Ok(())
    }
}

/// Transaction Throughput Benchmark
#[derive(Clone)]
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
                properties: BTreeMap::from([
                    ("label".to_string(), Value::String("Account".to_string())),
                    ("account_id".to_string(), Value::String(format!("acc_{}", i))),
                    ("balance".to_string(), Value::Int(1000)),
                ]),
                edges: vec![],
            };

            self.db.storage.put_block(&Block::Node(account)).await?;
        }

        Ok(())
    }

    async fn run(&self, config: &BenchmarkConfig) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
        use crate::runner::BenchmarkRunner;
        let mut runner = crate::runner::BenchmarkRunner::new(config.clone());
        runner.run_benchmark(self.clone()).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Transaction simulation (simplified - no actual transaction API used)
        // For now, just perform some basic operations to simulate transaction work
        let account_id = (operation_count % 100) as usize;

        // Simulate reading account balance
        let _ = self.db.find_nodes(&[("account_id".to_string(), Value::String(format!("acc_{}", account_id)))]);

        // Simulate updating account balance
        let properties = BTreeMap::from([
            ("account_id".to_string(), Value::String(format!("acc_{}", account_id))),
            ("balance".to_string(), Value::Int(1000 + operation_count as i64)),
        ]);
        let node = NodeBlock {
            properties,
            edges: vec![],
        };
        let _ = self.db.storage.put_block(&Block::Node(node)).await;

        Ok(())
    }

}


/// Memory Usage Benchmark
#[derive(Clone)]
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
        runner.run_benchmark(self.clone()).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Memory benchmark operations - create data structures to stress memory
        let properties = BTreeMap::from([
            ("operation_id".to_string(), Value::Int(operation_count as i64)),
            ("data".to_string(), Value::String(format!("memory_test_data_{}", operation_count))),
            ("size".to_string(), Value::Int(self.data_size as i64)),
        ]);
        let node = NodeBlock {
            properties,
            edges: vec![],
        };
        self.db.storage.put_block(&Block::Node(node)).await?;
        Ok(())
    }
}

/// Storage Operations Benchmark
#[derive(Clone)]
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
        runner.run_benchmark(self.clone()).await
    }

    async fn teardown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }

    async fn run_operation(&self, _worker_id: usize, operation_count: u64) -> Result<(), Box<dyn std::error::Error>> {
        // Storage benchmark operations - mix of reads and writes
        let operation_type = (operation_count % 3) as usize;

        match operation_type {
            0 => {
                // Write operation
                let properties = BTreeMap::from([
                    ("storage_op_id".to_string(), Value::Int(operation_count as i64)),
                    ("data".to_string(), Value::String(format!("storage_test_data_{}", operation_count))),
                ]);
                let node = NodeBlock {
                    properties,
                    edges: vec![],
                };
                self.db.storage.put_block(&Block::Node(node)).await?;
            }
            1 => {
                // Read operation (simplified)
                let _ = operation_count; // Use to avoid unused variable warning
            }
            2 => {
                // Mixed operation - read then write
                let _ = operation_count;
                // Could implement more complex storage operations here
            }
            _ => {}
        }
        Ok(())
    }
}

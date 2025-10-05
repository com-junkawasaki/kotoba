//! Workload Generators for Load Testing
//!
//! Implements various workload patterns including:
//! - YCSB workloads (A-F)
//! - Custom workload patterns
//! - Realistic application workloads

use crate::{Operation, LoadTestConfig};
use async_trait::async_trait;
use rand::prelude::*;
use std::collections::HashMap;

/// Workload generator trait
#[async_trait]
pub trait WorkloadGenerator: Send + Sync {
    async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation;
    fn clone_box(&self) -> Box<dyn WorkloadGenerator>;
}

/// YCSB (Yahoo! Cloud Serving Benchmark) workloads
pub mod ycsb {
    use super::*;

    /// YCSB Workload A: 50% reads, 50% updates
    pub struct WorkloadA {
        key_range: u64,
        value_size: usize,
        rng: Mutex<ThreadRng>,
    }

    impl WorkloadA {
        pub fn new(key_range: u64, value_size: usize) -> Self {
            Self {
                key_range,
                value_size,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for WorkloadA {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;
            let key = format!("user{:010}", rng.gen_range(0..self.key_range));
            let value = format!("value_{:020}", rng.gen::<u64>()).into_bytes();

            // 50% reads, 50% updates
            if rng.gen_bool(0.5) {
                Operation::Read { key: key.into_bytes() }
            } else {
                Operation::Update { key: key.into_bytes(), value }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(WorkloadA {
                key_range: self.key_range,
                value_size: self.value_size,
                rng: Mutex::new(thread_rng()),
            })
        }
    }

    /// YCSB Workload B: 95% reads, 5% updates
    pub struct WorkloadB {
        key_range: u64,
        value_size: usize,
        rng: Mutex<ThreadRng>,
    }

    impl WorkloadB {
        pub fn new(key_range: u64, value_size: usize) -> Self {
            Self {
                key_range,
                value_size,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for WorkloadB {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;
            let key = format!("user{:010}", rng.gen_range(0..self.key_range));
            let value = format!("value_{:020}", rng.gen::<u64>()).into_bytes();

            // 95% reads, 5% updates
            if rng.gen_bool(0.95) {
                Operation::Read { key: key.into_bytes() }
            } else {
                Operation::Update { key: key.into_bytes(), value }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(WorkloadB {
                key_range: self.key_range,
                value_size: self.value_size,
                rng: Mutex::new(thread_rng()),
            })
        }
    }

    /// YCSB Workload C: 100% reads
    pub struct WorkloadC {
        key_range: u64,
        rng: Mutex<ThreadRng>,
    }

    impl WorkloadC {
        pub fn new(key_range: u64) -> Self {
            Self {
                key_range,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for WorkloadC {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;
            let key = format!("user{:010}", rng.gen_range(0..self.key_range));

            Operation::Read { key: key.into_bytes() }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(WorkloadC {
                key_range: self.key_range,
                rng: Mutex::new(thread_rng()),
            })
        }
    }

    /// YCSB Workload D: 95% reads, 5% inserts
    pub struct WorkloadD {
        key_range: u64,
        value_size: usize,
        rng: Mutex<ThreadRng>,
        next_key: Mutex<u64>,
    }

    impl WorkloadD {
        pub fn new(key_range: u64, value_size: usize) -> Self {
            Self {
                key_range,
                value_size,
                rng: Mutex::new(thread_rng()),
                next_key: Mutex::new(key_range),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for WorkloadD {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            // 95% reads, 5% inserts
            if rng.gen_bool(0.95) {
                let key = format!("user{:010}", rng.gen_range(0..self.key_range));
                Operation::Read { key: key.into_bytes() }
            } else {
                let mut next_key = self.next_key.lock().await;
                let key = format!("user{:010}", *next_key);
                *next_key += 1;
                let value = format!("value_{:020}", rng.gen::<u64>()).into_bytes();
                Operation::Insert { key: key.into_bytes(), value }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(WorkloadD {
                key_range: self.key_range,
                value_size: self.value_size,
                rng: Mutex::new(thread_rng()),
                next_key: Mutex::new(0), // Reset for cloned workload
            })
        }
    }
}

/// Custom application-specific workloads
pub mod custom {
    use super::*;

    /// Social network workload (reads, writes, updates)
    pub struct SocialNetworkWorkload {
        user_count: u64,
        post_count: u64,
        rng: Mutex<ThreadRng>,
    }

    impl SocialNetworkWorkload {
        pub fn new(user_count: u64, post_count: u64) -> Self {
            Self {
                user_count,
                post_count,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for SocialNetworkWorkload {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            // Simulate social network operations:
            // 70% reads (user profiles, posts, timelines)
            // 20% writes (new posts, comments)
            // 10% updates (profile updates, likes)

            let operation_type = rng.gen_range(0..100);

            match operation_type {
                0..=69 => {
                    // Read operations
                    match rng.gen_range(0..3) {
                        0 => {
                            // Read user profile
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("user:profile:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                        1 => {
                            // Read user posts
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("user:posts:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                        _ => {
                            // Read timeline
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("user:timeline:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                    }
                }
                70..=89 => {
                    // Write operations
                    match rng.gen_range(0..2) {
                        0 => {
                            // New post
                            let post_id = rng.gen_range(1..=self.post_count);
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("post:{}", post_id);
                            let value = format!("Post content by user {} at {}", user_id, chrono::Utc::now()).into_bytes();
                            Operation::Insert { key: key.into_bytes(), value }
                        }
                        _ => {
                            // New comment
                            let comment_id = rng.gen::<u64>();
                            let post_id = rng.gen_range(1..=self.post_count);
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("comment:{}:post:{}", comment_id, post_id);
                            let value = format!("Comment by user {} on post {}", user_id, post_id).into_bytes();
                            Operation::Insert { key: key.into_bytes(), value }
                        }
                    }
                }
                _ => {
                    // Update operations
                    match rng.gen_range(0..3) {
                        0 => {
                            // Update profile
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("user:profile:{}", user_id);
                            let value = format!("Updated profile for user {} at {}", user_id, chrono::Utc::now()).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                        1 => {
                            // Add like
                            let post_id = rng.gen_range(1..=self.post_count);
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("post:{}:likes", post_id);
                            let value = format!("user_{}", user_id).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                        _ => {
                            // Update follower count
                            let user_id = rng.gen_range(1..=self.user_count);
                            let key = format!("user:{}:follower_count", user_id);
                            let value = format!("{}", rng.gen_range(0..10000)).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                    }
                }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(SocialNetworkWorkload {
                user_count: self.user_count,
                post_count: self.post_count,
                rng: Mutex::new(thread_rng()),
            })
        }
    }

    /// E-commerce workload
    pub struct EcommerceWorkload {
        product_count: u64,
        user_count: u64,
        rng: Mutex<ThreadRng>,
    }

    impl EcommerceWorkload {
        pub fn new(product_count: u64, user_count: u64) -> Self {
            Self {
                product_count,
                user_count,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for EcommerceWorkload {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            // E-commerce patterns:
            // 60% product catalog reads
            // 25% user session/cart operations
            // 10% order processing
            // 5% inventory updates

            let operation_type = rng.gen_range(0..100);

            match operation_type {
                0..=59 => {
                    // Product catalog operations
                    let product_id = rng.gen_range(1..=self.product_count);
                    let key = format!("product:{}", product_id);
                    Operation::Read { key: key.into_bytes() }
                }
                60..=84 => {
                    // User session/cart operations
                    let user_id = rng.gen_range(1..=self.user_count);
                    match rng.gen_range(0..3) {
                        0 => {
                            // Read cart
                            let key = format!("cart:user:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                        1 => {
                            // Update cart
                            let key = format!("cart:user:{}", user_id);
                            let value = format!("product_{},product_{},product_{}",
                                              rng.gen_range(1..=self.product_count),
                                              rng.gen_range(1..=self.product_count),
                                              rng.gen_range(1..=self.product_count)).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                        _ => {
                            // User session
                            let key = format!("session:user:{}", user_id);
                            let value = format!("session_data_{}", rng.gen::<u64>()).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                    }
                }
                85..=94 => {
                    // Order processing
                    let order_id = rng.gen::<u64>();
                    let user_id = rng.gen_range(1..=self.user_count);
                    let key = format!("order:{}:user:{}", order_id, user_id);
                    let value = format!("order_details_{}", chrono::Utc::now().timestamp()).into_bytes();
                    Operation::Insert { key: key.into_bytes(), value }
                }
                _ => {
                    // Inventory updates
                    let product_id = rng.gen_range(1..=self.product_count);
                    let key = format!("inventory:product:{}", product_id);
                    let value = format!("{}", rng.gen_range(0..1000)).into_bytes();
                    Operation::Update { key: key.into_bytes(), value }
                }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(EcommerceWorkload {
                product_count: self.product_count,
                user_count: self.user_count,
                rng: Mutex::new(thread_rng()),
            })
        }
    }
}

/// Stress testing workloads
pub mod stress {
    use super::*;

    /// High contention workload (hotspot)
    pub struct HotspotWorkload {
        key_range: u64,
        hotspot_keys: Vec<String>,
        hotspot_probability: f64,
        rng: Mutex<ThreadRng>,
    }

    impl HotspotWorkload {
        pub fn new(key_range: u64, hotspot_count: usize, hotspot_probability: f64) -> Self {
            let hotspot_keys = (0..hotspot_count)
                .map(|i| format!("hotspot_key_{:05}", i))
                .collect();

            Self {
                key_range,
                hotspot_keys,
                hotspot_probability,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for HotspotWorkload {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            let key = if rng.gen_bool(self.hotspot_probability) {
                // Access hotspot key
                self.hotspot_keys[rng.gen_range(0..self.hotspot_keys.len())].clone()
            } else {
                // Access regular key
                format!("regular_key_{:010}", rng.gen_range(0..self.key_range))
            };

            let value = format!("value_{:020}", rng.gen::<u64>()).into_bytes();

            // Mix of reads and writes to create contention
            if rng.gen_bool(0.7) {
                Operation::Read { key: key.into_bytes() }
            } else {
                Operation::Update { key: key.into_bytes(), value }
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(HotspotWorkload {
                key_range: self.key_range,
                hotspot_keys: self.hotspot_keys.clone(),
                hotspot_probability: self.hotspot_probability,
                rng: Mutex::new(thread_rng()),
            })
        }
    }

    /// Large value workload for memory/disk pressure testing
    pub struct LargeValueWorkload {
        key_count: u64,
        value_size_kb: usize,
        rng: Mutex<ThreadRng>,
    }

    impl LargeValueWorkload {
        pub fn new(key_count: u64, value_size_kb: usize) -> Self {
            Self {
                key_count,
                value_size_kb,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl WorkloadGenerator for LargeValueWorkload {
        async fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            let key = format!("large_key_{:010}", rng.gen_range(0..self.key_count));
            let value = vec![b'A'; self.value_size_kb * 1024]; // Large value

            match rng.gen_range(0..3) {
                0 => Operation::Read { key: key.into_bytes() },
                1 => Operation::Insert { key: key.into_bytes(), value },
                _ => Operation::Update { key: key.into_bytes(), value },
            }
        }

        fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
            Box::new(LargeValueWorkload {
                key_count: self.key_count,
                value_size_kb: self.value_size_kb,
                rng: Mutex::new(thread_rng()),
            })
        }
    }
}

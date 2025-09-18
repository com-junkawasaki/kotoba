//! Load Generator
//!
//! Advanced load generation capabilities including:
//! - Realistic workload patterns
//! - Time-based load variations
//! - Burst and spike generation
//! - Custom load profiles

use crate::{BenchmarkConfig, Operation};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use rand::prelude::*;
use async_trait::async_trait;

/// Load generator trait for creating various load patterns
#[async_trait]
pub trait LoadGenerator {
    /// Generate the next operation based on current state
    async fn generate_operation(&mut self, worker_id: usize, operation_count: u64) -> Operation;

    /// Get current load profile description
    fn current_profile(&self) -> LoadProfile;

    /// Reset generator state
    fn reset(&mut self);
}

/// Load profile describing current load characteristics
#[derive(Debug, Clone)]
pub struct LoadProfile {
    pub target_throughput: Option<f64>,
    pub pattern: LoadPattern,
    pub burst_probability: f64,
    pub spike_probability: f64,
    pub description: String,
}

/// Load pattern types
#[derive(Debug, Clone)]
pub enum LoadPattern {
    Constant,
    RampUp { initial_ops: f64, final_ops: f64 },
    RampDown { initial_ops: f64, final_ops: f64 },
    Sinusoidal { amplitude: f64, frequency: f64, baseline: f64 },
    Step { steps: Vec<(Duration, f64)> },
    Bursty { burst_duration: Duration, cooldown_duration: Duration, burst_multiplier: f64 },
    Custom { profile: Vec<(f64, f64)> }, // (time_offset, throughput)
}

/// Standard workload generators
pub mod workloads {
    use super::*;

    /// YCSB Workload A: 50% reads, 50% updates
    pub struct YcsbWorkloadA {
        key_range: u64,
        value_size: usize,
        rng: Mutex<ThreadRng>,
    }

    impl YcsbWorkloadA {
        pub fn new(key_range: u64, value_size: usize) -> Self {
            Self {
                key_range,
                value_size,
                rng: Mutex::new(thread_rng()),
            }
        }
    }

    #[async_trait]
    impl LoadGenerator for YcsbWorkloadA {
        async fn generate_operation(&mut self, _worker_id: usize, _operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;
            let key = format!("user{:010}", rng.gen_range(0..self.key_range));
            let value = format!("value_{:020}", rng.gen::<u64>()).into_bytes();

            if rng.gen_bool(0.5) {
                Operation::Read { key: key.into_bytes() }
            } else {
                Operation::Update { key: key.into_bytes(), value }
            }
        }

        fn current_profile(&self) -> LoadProfile {
            LoadProfile {
                target_throughput: None,
                pattern: LoadPattern::Constant,
                burst_probability: 0.0,
                spike_probability: 0.0,
                description: "YCSB Workload A: 50% reads, 50% updates".to_string(),
            }
        }

        fn reset(&mut self) {
            *self.rng.lock().unwrap() = thread_rng();
        }
    }

    /// Social network workload
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
    impl LoadGenerator for SocialNetworkWorkload {
        async fn generate_operation(&mut self, _worker_id: usize, _operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            let operation_type = rng.gen_range(0..100);

            match operation_type {
                0..=59 => {
                    // Product catalog operations
                    let user_id = rng.gen_range(1..=self.user_count);
                    let key = format!("user:timeline:{}", user_id);
                    Operation::Read { key: key.into_bytes() }
                }
                60..=84 => {
                    // User session/cart operations
                    let user_id = rng.gen_range(1..=self.user_count);
                    match rng.gen_range(0..3) {
                        0 => {
                            let key = format!("user:profile:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                        1 => {
                            let key = format!("user:posts:{}", user_id);
                            let value = format!("post content {}", rng.gen::<u64>()).into_bytes();
                            Operation::Insert { key: key.into_bytes(), value }
                        }
                        _ => {
                            let post_id = rng.gen_range(1..=self.post_count);
                            let key = format!("post:likes:{}", post_id);
                            let value = format!("user_{}", user_id).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                    }
                }
                _ => {
                    // Order processing
                    let comment_id = rng.gen::<u64>();
                    let post_id = rng.gen_range(1..=self.post_count);
                    let user_id = rng.gen_range(1..=self.user_count);
                    let key = format!("comment:{}:post:{}", comment_id, post_id);
                    let value = format!("Comment by user {} on post {}", user_id, post_id).into_bytes();
                    Operation::Insert { key: key.into_bytes(), value }
                }
            }
        }

        fn current_profile(&self) -> LoadProfile {
            LoadProfile {
                target_throughput: None,
                pattern: LoadPattern::Constant,
                burst_probability: 0.1,
                spike_probability: 0.01,
                description: "Social Network: reads, posts, comments, likes".to_string(),
            }
        }

        fn reset(&mut self) {
            *self.rng.lock().unwrap() = thread_rng();
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
    impl LoadGenerator for EcommerceWorkload {
        async fn generate_operation(&mut self, _worker_id: usize, _operation_count: u64) -> Operation {
            let mut rng = self.rng.lock().await;

            let operation_type = rng.gen_range(0..100);

            match operation_type {
                0..=69 => {
                    // Product catalog operations
                    let product_id = rng.gen_range(1..=self.product_count);
                    let key = format!("product:{}", product_id);
                    Operation::Read { key: key.into_bytes() }
                }
                70..=89 => {
                    // Shopping operations
                    let user_id = rng.gen_range(1..=self.user_count);
                    match rng.gen_range(0..3) {
                        0 => {
                            let key = format!("cart:user:{}", user_id);
                            Operation::Read { key: key.into_bytes() }
                        }
                        1 => {
                            let key = format!("cart:user:{}", user_id);
                            let product_ids: Vec<u64> = (0..rng.gen_range(1..5))
                                .map(|_| rng.gen_range(1..=self.product_count))
                                .collect();
                            let value = serde_json::to_string(&product_ids).unwrap().into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                        _ => {
                            let key = format!("wishlist:user:{}", user_id);
                            let product_id = rng.gen_range(1..=self.product_count);
                            let value = format!("product_{}", product_id).into_bytes();
                            Operation::Update { key: key.into_bytes(), value }
                        }
                    }
                }
                _ => {
                    // Order operations
                    let order_id = rng.gen::<u64>();
                    let user_id = rng.gen_range(1..=self.user_count);
                    let key = format!("order:{}:user:{}", order_id, user_id);
                    let value = format!("order_details_{}", chrono::Utc::now().timestamp()).into_bytes();
                    Operation::Insert { key: key.into_bytes(), value }
                }
            }
        }

        fn current_profile(&self) -> LoadProfile {
            LoadProfile {
                target_throughput: None,
                pattern: LoadPattern::Constant,
                burst_probability: 0.05,
                spike_probability: 0.02,
                description: "E-commerce: catalog, cart, orders".to_string(),
            }
        }

        fn reset(&mut self) {
            *self.rng.lock().unwrap() = thread_rng();
        }
    }
}

/// Advanced load patterns
pub mod patterns {
    use super::*;

    /// Ramp up load pattern
    pub struct RampUpLoadGenerator<G: LoadGenerator> {
        inner: G,
        start_time: Instant,
        initial_ops_per_sec: f64,
        final_ops_per_sec: f64,
        ramp_duration: Duration,
    }

    impl<G: LoadGenerator> RampUpLoadGenerator<G> {
        pub fn new(
            inner: G,
            initial_ops_per_sec: f64,
            final_ops_per_sec: f64,
            ramp_duration: Duration,
        ) -> Self {
            Self {
                inner,
                start_time: Instant::now(),
                initial_ops_per_sec,
                final_ops_per_sec,
                ramp_duration,
            }
        }

        fn current_target_throughput(&self) -> f64 {
            let elapsed = self.start_time.elapsed();
            if elapsed >= self.ramp_duration {
                return self.final_ops_per_sec;
            }

            let progress = elapsed.as_secs_f64() / self.ramp_duration.as_secs_f64();
            self.initial_ops_per_sec + (self.final_ops_per_sec - self.initial_ops_per_sec) * progress
        }
    }

    #[async_trait]
    impl<G: LoadGenerator> LoadGenerator for RampUpLoadGenerator<G> {
        async fn generate_operation(&mut self, worker_id: usize, operation_count: u64) -> Operation {
            self.inner.generate_operation(worker_id, operation_count).await
        }

        fn current_profile(&self) -> LoadProfile {
            let target = self.current_target_throughput();
            LoadProfile {
                target_throughput: Some(target),
                pattern: LoadPattern::RampUp {
                    initial_ops: self.initial_ops_per_sec,
                    final_ops: self.final_ops_per_sec,
                },
                burst_probability: 0.0,
                spike_probability: 0.0,
                description: format!("Ramp up from {:.0} to {:.0} ops/sec over {:?}",
                    self.initial_ops_per_sec, self.final_ops_per_sec, self.ramp_duration),
            }
        }

        fn reset(&mut self) {
            self.start_time = Instant::now();
            self.inner.reset();
        }
    }

    /// Bursty load pattern
    pub struct BurstyLoadGenerator<G: LoadGenerator> {
        inner: G,
        burst_duration: Duration,
        cooldown_duration: Duration,
        burst_multiplier: f64,
        cycle_start: Instant,
        rng: Mutex<ThreadRng>,
    }

    impl<G: LoadGenerator> BurstyLoadGenerator<G> {
        pub fn new(
            inner: G,
            burst_duration: Duration,
            cooldown_duration: Duration,
            burst_multiplier: f64,
        ) -> Self {
            Self {
                inner,
                burst_duration,
                cooldown_duration,
                burst_multiplier,
                cycle_start: Instant::now(),
                rng: Mutex::new(thread_rng()),
            }
        }

        fn is_in_burst(&self) -> bool {
            let elapsed = self.cycle_start.elapsed();
            let cycle_duration = self.burst_duration + self.cooldown_duration;
            let position_in_cycle = elapsed.as_nanos() % cycle_duration.as_nanos();

            position_in_cycle < self.burst_duration.as_nanos()
        }

        fn current_burst_multiplier(&self) -> f64 {
            if self.is_in_burst() {
                self.burst_multiplier
            } else {
                1.0
            }
        }
    }

    #[async_trait]
    impl<G: LoadGenerator> LoadGenerator for BurstyLoadGenerator<G> {
        async fn generate_operation(&mut self, worker_id: usize, operation_count: u64) -> Operation {
            self.inner.generate_operation(worker_id, operation_count).await
        }

        fn current_profile(&self) -> LoadProfile {
            let multiplier = self.current_burst_multiplier();
            let in_burst = self.is_in_burst();

            LoadProfile {
                target_throughput: None,
                pattern: LoadPattern::Bursty {
                    burst_duration: self.burst_duration,
                    cooldown_duration: self.cooldown_duration,
                    burst_multiplier: self.burst_multiplier,
                },
                burst_probability: if in_burst { 1.0 } else { 0.0 },
                spike_probability: 0.0,
                description: format!("Bursty load: {}x during bursts, current multiplier: {:.1}x",
                    self.burst_multiplier, multiplier),
            }
        }

        fn reset(&mut self) {
            self.cycle_start = Instant::now();
            self.inner.reset();
        }
    }

    /// Spike load generator for sudden load increases
    pub struct SpikeLoadGenerator<G: LoadGenerator> {
        inner: G,
        base_throughput: f64,
        spike_multiplier: f64,
        spike_probability: f64,
        spike_duration: Duration,
        last_spike: Mutex<Option<Instant>>,
        rng: Mutex<ThreadRng>,
    }

    impl<G: LoadGenerator> SpikeLoadGenerator<G> {
        pub fn new(
            inner: G,
            base_throughput: f64,
            spike_multiplier: f64,
            spike_probability: f64,
            spike_duration: Duration,
        ) -> Self {
            Self {
                inner,
                base_throughput,
                spike_multiplier,
                spike_probability,
                spike_duration,
                last_spike: Mutex::new(None),
                rng: Mutex::new(thread_rng()),
            }
        }

        fn should_spike(&self) -> bool {
            let mut rng = self.rng.lock().unwrap();

            // Check if we're currently in a spike
            if let Some(spike_start) = *self.last_spike.lock().unwrap() {
                if spike_start.elapsed() < self.spike_duration {
                    return true; // Continue existing spike
                }
            }

            // Check if we should start a new spike
            if rng.gen_bool(self.spike_probability) {
                *self.last_spike.lock().unwrap() = Some(Instant::now());
                return true;
            }

            false
        }

        fn current_multiplier(&self) -> f64 {
            if self.should_spike() {
                self.spike_multiplier
            } else {
                1.0
            }
        }
    }

    #[async_trait]
    impl<G: LoadGenerator> LoadGenerator for SpikeLoadGenerator<G> {
        async fn generate_operation(&mut self, worker_id: usize, operation_count: u64) -> Operation {
            self.inner.generate_operation(worker_id, operation_count).await
        }

        fn current_profile(&self) -> LoadProfile {
            let multiplier = self.current_multiplier();
            let is_spiking = multiplier > 1.0;

            LoadProfile {
                target_throughput: Some(self.base_throughput * multiplier),
                pattern: LoadPattern::Constant, // Base pattern is constant
                burst_probability: 0.0,
                spike_probability: if is_spiking { 1.0 } else { self.spike_probability },
                description: format!("Spike load: base {:.0} ops/sec, current multiplier: {:.1}x",
                    self.base_throughput, multiplier),
            }
        }

        fn reset(&mut self) {
            *self.last_spike.lock().unwrap() = None;
            self.inner.reset();
        }
    }
}

/// Load profile builder for creating custom load patterns
pub struct LoadProfileBuilder {
    pattern: LoadPattern,
    burst_probability: f64,
    spike_probability: f64,
    description: String,
}

impl LoadProfileBuilder {
    pub fn new() -> Self {
        Self {
            pattern: LoadPattern::Constant,
            burst_probability: 0.0,
            spike_probability: 0.0,
            description: "Custom load profile".to_string(),
        }
    }

    pub fn constant(mut self) -> Self {
        self.pattern = LoadPattern::Constant;
        self
    }

    pub fn ramp_up(mut self, initial: f64, final_: f64) -> Self {
        self.pattern = LoadPattern::RampUp {
            initial_ops: initial,
            final_ops: final_,
        };
        self
    }

    pub fn bursty(mut self, burst_duration: Duration, cooldown: Duration, multiplier: f64) -> Self {
        self.pattern = LoadPattern::Bursty {
            burst_duration,
            cooldown_duration: cooldown,
            burst_multiplier: multiplier,
        };
        self
    }

    pub fn with_burst_probability(mut self, probability: f64) -> Self {
        self.burst_probability = probability;
        self
    }

    pub fn with_spike_probability(mut self, probability: f64) -> Self {
        self.spike_probability = probability;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn build(self) -> LoadProfile {
        LoadProfile {
            target_throughput: None,
            pattern: self.pattern,
            burst_probability: self.burst_probability,
            spike_probability: self.spike_probability,
            description: self.description,
        }
    }
}

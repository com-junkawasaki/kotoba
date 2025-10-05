//! Application-Specific Load Testing
//!
//! This module provides load testing scenarios based on real-world applications,
//! including social networks, e-commerce platforms, content management systems,
//! and other common application patterns.

use crate::runner::KotobaDBRunner;
use crate::workload::WorkloadGenerator;
use crate::{LoadTestConfig, LoadTestResult, run_load_test, Operation};
use std::time::Duration;
use std::collections::HashMap;

/// Social network application workload
pub async fn social_network(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📱 Running Social Network Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 50, // High concurrency for social features
        warmup_duration: Duration::from_secs(15),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = SocialNetworkWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("Social Network", &result);
    Ok(())
}

/// E-commerce platform workload
pub async fn ecommerce(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🛒 Running E-Commerce Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 40,
        warmup_duration: Duration::from_secs(15),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = EcommerceWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("E-Commerce", &result);
    Ok(())
}

/// Content management system workload
pub async fn cms(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📝 Running CMS Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 30,
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = CMSWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("Content Management", &result);
    Ok(())
}

/// Analytics platform workload
pub async fn analytics(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📊 Running Analytics Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 20, // Lower concurrency, complex queries
        warmup_duration: Duration::from_secs(20),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = AnalyticsWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("Analytics", &result);
    Ok(())
}

/// IoT platform workload
pub async fn iot(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Running IoT Platform Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 100, // High concurrency for sensor data
        warmup_duration: Duration::from_secs(10),
        operations_per_second: Some(10000), // Rate limited for IoT scenarios
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = IoTWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("IoT Platform", &result);
    Ok(())
}

/// Financial services workload
pub async fn financial(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("💰 Running Financial Services Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 25, // Moderate concurrency, high consistency requirements
        warmup_duration: Duration::from_secs(15),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = FinancialWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("Financial Services", &result);
    Ok(())
}

/// Gaming platform workload
pub async fn gaming(runner: KotobaDBRunner, duration_secs: u64) -> Result<(), Box<dyn std::error::Error>> {
    println!("🎮 Running Gaming Platform Workload Simulation");

    let config = LoadTestConfig {
        duration: Duration::from_secs(duration_secs),
        concurrency: 60, // High concurrency for multiplayer features
        warmup_duration: Duration::from_secs(10),
        operations_per_second: None,
        enable_metrics: true,
        collect_detailed_latency: true,
    };

    let workload = GamingWorkload::new();
    let result = run_load_test(runner, Box::new(workload), config).await?;

    print_application_results("Gaming Platform", &result);
    Ok(())
}

/// Print application workload results
fn print_application_results(app_name: &str, result: &LoadTestResult) {
    println!("📊 {} Results:", app_name);
    println!("   Throughput: {:.1} ops/sec", result.operations_per_second);
    println!("   Total Operations: {}", result.total_operations);
    println!("   Duration: {:.2}s", result.duration.as_secs_f64());
    println!("   Latency (μs): P50={}, P95={}, P99={}, Max={}",
             result.latency_percentiles.p50,
             result.latency_percentiles.p95,
             result.latency_percentiles.p99,
             result.latency_percentiles.max);
    println!("   Error Rate: {:.3}%", result.error_rate * 100.0);
    println!();
}

// Application workload implementations

/// Social network workload simulation
pub struct SocialNetworkWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl SocialNetworkWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("view_profile".to_string(), 35.0),      // View user profiles
                ("post_status".to_string(), 15.0),       // Create posts
                ("view_feed".to_string(), 25.0),         // View news feed
                ("like_post".to_string(), 10.0),         // Like/interact with posts
                ("search_users".to_string(), 8.0),       // Search for users
                ("send_message".to_string(), 5.0),       // Private messaging
                ("follow_user".to_string(), 2.0),        // Follow/unfollow users
            ],
        }
    }
}

impl WorkloadGenerator for SocialNetworkWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "view_profile" => {
                let user_id = rng.gen_range(1..100000);
                let key = format!("user_profile_{}", user_id);
                Operation::Read { key: key.into_bytes() }
            }
            "post_status" => {
                let post_id = format!("post_{}_{}", worker_id, operation_count);
                let content = format!("Status update from user {} at {}", worker_id, chrono::Utc::now());
                Operation::Insert {
                    key: post_id.into_bytes(),
                    value: content.into_bytes(),
                }
            }
            "view_feed" => {
                let user_id = rng.gen_range(1..100000);
                let key = format!("user_feed_{}", user_id);
                Operation::Read { key: key.into_bytes() }
            }
            "like_post" => {
                let post_id = rng.gen_range(1..1000000);
                let key = format!("post_likes_{}", post_id);
                let value = format!("like_{}_{}", worker_id, operation_count);
                Operation::Update {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                }
            }
            "search_users" => {
                let query = format!("search_users_{}", rng.gen_range(1..1000));
                Operation::Scan {
                    start_key: query.into_bytes(),
                    limit: rng.gen_range(10..50),
                }
            }
            "send_message" => {
                let conversation_id = rng.gen_range(1..50000);
                let key = format!("conversation_{}_messages", conversation_id);
                let message = format!("Message from user {}: {}", worker_id, operation_count);
                Operation::Update {
                    key: key.into_bytes(),
                    value: message.into_bytes(),
                }
            }
            "follow_user" => {
                let target_user = rng.gen_range(1..100000);
                let key = format!("user_{}_following", worker_id);
                let value = format!("follow_{}", target_user);
                Operation::Update {
                    key: key.into_bytes(),
                    value: value.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(SocialNetworkWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// E-commerce workload simulation
pub struct EcommerceWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl EcommerceWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("browse_products".to_string(), 40.0),    // Browse product catalog
                ("view_product".to_string(), 25.0),       // View product details
                ("search_products".to_string(), 15.0),    // Search products
                ("add_to_cart".to_string(), 8.0),         // Add items to cart
                ("checkout".to_string(), 5.0),            // Process orders
                ("update_inventory".to_string(), 5.0),    // Update stock levels
                ("user_login".to_string(), 2.0),          // User authentication
            ],
        }
    }
}

impl WorkloadGenerator for EcommerceWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "browse_products" => {
                let category = rng.gen_range(1..100);
                let key = format!("category_{}_products", category);
                Operation::Read { key: key.into_bytes() }
            }
            "view_product" => {
                let product_id = rng.gen_range(1..100000);
                let key = format!("product_{}", product_id);
                Operation::Read { key: key.into_bytes() }
            }
            "search_products" => {
                let query = format!("search_{}", rng.gen_range(1..1000));
                Operation::Scan {
                    start_key: query.into_bytes(),
                    limit: rng.gen_range(20..100),
                }
            }
            "add_to_cart" => {
                let cart_id = format!("cart_{}", worker_id);
                let product_id = rng.gen_range(1..100000);
                let item = format!("product_{}_qty_{}", product_id, rng.gen_range(1..5));
                Operation::Update {
                    key: cart_id.into_bytes(),
                    value: item.into_bytes(),
                }
            }
            "checkout" => {
                let order_id = format!("order_{}_{}", worker_id, operation_count);
                let order_data = format!("checkout_{}_{}", worker_id, chrono::Utc::now());
                Operation::Insert {
                    key: order_id.into_bytes(),
                    value: order_data.into_bytes(),
                }
            }
            "update_inventory" => {
                let product_id = rng.gen_range(1..100000);
                let key = format!("inventory_{}", product_id);
                let stock_level = rng.gen_range(0..1000);
                Operation::Update {
                    key: key.into_bytes(),
                    value: stock_level.to_string().into_bytes(),
                }
            }
            "user_login" => {
                let session_id = format!("session_{}", worker_id);
                let session_data = format!("login_{}", chrono::Utc::now());
                Operation::Update {
                    key: session_id.into_bytes(),
                    value: session_data.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(EcommerceWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// Content Management System workload
pub struct CMSWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl CMSWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("view_page".to_string(), 50.0),          // View content pages
                ("edit_content".to_string(), 15.0),       // Edit content
                ("search_content".to_string(), 15.0),     // Search content
                ("publish_content".to_string(), 8.0),     // Publish new content
                ("user_management".to_string(), 7.0),     // User management
                ("media_upload".to_string(), 3.0),        // Media file handling
                ("comment_system".to_string(), 2.0),      // Comments and interactions
            ],
        }
    }
}

impl WorkloadGenerator for CMSWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "view_page" => {
                let page_id = rng.gen_range(1..50000);
                let key = format!("page_{}", page_id);
                Operation::Read { key: key.into_bytes() }
            }
            "edit_content" => {
                let content_id = rng.gen_range(1..50000);
                let key = format!("content_{}", content_id);
                let content = format!("Updated content by user {} at {}", worker_id, chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: content.into_bytes(),
                }
            }
            "search_content" => {
                let query = format!("search_{}", rng.gen_range(1..1000));
                Operation::Scan {
                    start_key: query.into_bytes(),
                    limit: rng.gen_range(10..50),
                }
            }
            "publish_content" => {
                let article_id = format!("article_{}_{}", worker_id, operation_count);
                let article_data = format!("New article published at {}", chrono::Utc::now());
                Operation::Insert {
                    key: article_id.into_bytes(),
                    value: article_data.into_bytes(),
                }
            }
            "user_management" => {
                let user_id = rng.gen_range(1..10000);
                let key = format!("user_profile_{}", user_id);
                let profile_data = format!("Updated profile for user {}", user_id);
                Operation::Update {
                    key: key.into_bytes(),
                    value: profile_data.into_bytes(),
                }
            }
            "media_upload" => {
                let media_id = format!("media_{}_{}", worker_id, operation_count);
                let media_data = format!("Media file uploaded by user {}", worker_id);
                Operation::Insert {
                    key: media_id.into_bytes(),
                    value: media_data.into_bytes(),
                }
            }
            "comment_system" => {
                let post_id = rng.gen_range(1..50000);
                let key = format!("comments_{}", post_id);
                let comment = format!("Comment by user {}: {}", worker_id, operation_count);
                Operation::Update {
                    key: key.into_bytes(),
                    value: comment.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(CMSWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// Analytics platform workload (simplified)
pub struct AnalyticsWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl AnalyticsWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("query_metrics".to_string(), 40.0),      // Query performance metrics
                ("ingest_data".to_string(), 30.0),        // Ingest analytics data
                ("aggregate_data".to_string(), 15.0),     // Data aggregation queries
                ("export_reports".to_string(), 10.0),     // Export analytics reports
                ("update_dashboards".to_string(), 5.0),   // Update dashboard configurations
            ],
        }
    }
}

impl WorkloadGenerator for AnalyticsWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "query_metrics" => {
                let metric_type = rng.gen_range(1..100);
                let key = format!("metrics_{}", metric_type);
                Operation::Read { key: key.into_bytes() }
            }
            "ingest_data" => {
                let event_id = format!("event_{}_{}", worker_id, operation_count);
                let event_data = format!("Analytics event at {}", chrono::Utc::now());
                Operation::Insert {
                    key: event_id.into_bytes(),
                    value: event_data.into_bytes(),
                }
            }
            "aggregate_data" => {
                let query = format!("aggregate_{}", rng.gen_range(1..50));
                Operation::Scan {
                    start_key: query.into_bytes(),
                    limit: rng.gen_range(100..1000),
                }
            }
            "export_reports" => {
                let report_id = rng.gen_range(1..1000);
                let key = format!("report_{}", report_id);
                Operation::Read { key: key.into_bytes() }
            }
            "update_dashboards" => {
                let dashboard_id = rng.gen_range(1..100);
                let key = format!("dashboard_{}", dashboard_id);
                let config = format!("Updated dashboard config {}", chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: config.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(AnalyticsWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// IoT platform workload (simplified)
pub struct IoTWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl IoTWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("sensor_reading".to_string(), 85.0),     // Sensor data ingestion
                ("device_status".to_string(), 10.0),      // Device status updates
                ("query_readings".to_string(), 4.0),      // Query historical data
                ("alert_processing".to_string(), 1.0),    // Process alerts
            ],
        }
    }
}

impl WorkloadGenerator for IoTWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "sensor_reading" => {
                let device_id = worker_id % 10000; // Simulate 10k devices
                let reading_id = format!("sensor_{}_{}", device_id, operation_count);
                let reading_data = format!("{{\"temperature\": {:.1}, \"humidity\": {:.1}, \"timestamp\": \"{}\"}}",
                    rng.gen_range(20.0..30.0),
                    rng.gen_range(40.0..80.0),
                    chrono::Utc::now().to_rfc3339());
                Operation::Insert {
                    key: reading_id.into_bytes(),
                    value: reading_data.into_bytes(),
                }
            }
            "device_status" => {
                let device_id = worker_id % 10000;
                let key = format!("device_status_{}", device_id);
                let status = format!("Online - Last seen: {}", chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: status.into_bytes(),
                }
            }
            "query_readings" => {
                let device_id = rng.gen_range(1..10000);
                let key = format!("sensor_{}_{}", device_id, rng.gen_range(1..1000));
                Operation::Read { key: key.into_bytes() }
            }
            "alert_processing" => {
                let alert_id = format!("alert_{}_{}", worker_id, operation_count);
                let alert_data = format!("Temperature alert at {}", chrono::Utc::now());
                Operation::Insert {
                    key: alert_id.into_bytes(),
                    value: alert_data.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(IoTWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// Financial services workload (simplified)
pub struct FinancialWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl FinancialWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("balance_inquiry".to_string(), 40.0),    // Check account balances
                ("funds_transfer".to_string(), 25.0),     // Transfer money
                ("transaction_history".to_string(), 20.0), // View transaction history
                ("payment_processing".to_string(), 10.0),  // Process payments
                ("account_update".to_string(), 5.0),      // Update account info
            ],
        }
    }
}

impl WorkloadGenerator for FinancialWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "balance_inquiry" => {
                let account_id = rng.gen_range(1..100000);
                let key = format!("account_balance_{}", account_id);
                Operation::Read { key: key.into_bytes() }
            }
            "funds_transfer" => {
                let transfer_id = format!("transfer_{}_{}", worker_id, operation_count);
                let transfer_data = format!("Transfer ${:.2} from account {} to account {} at {}",
                    rng.gen_range(10.0..1000.0),
                    rng.gen_range(1..100000),
                    rng.gen_range(1..100000),
                    chrono::Utc::now());
                Operation::Insert {
                    key: transfer_id.into_bytes(),
                    value: transfer_data.into_bytes(),
                }
            }
            "transaction_history" => {
                let account_id = rng.gen_range(1..100000);
                let key = format!("transactions_{}", account_id);
                Operation::Read { key: key.into_bytes() }
            }
            "payment_processing" => {
                let payment_id = format!("payment_{}_{}", worker_id, operation_count);
                let payment_data = format!("Payment ${:.2} processed at {}",
                    rng.gen_range(5.0..500.0),
                    chrono::Utc::now());
                Operation::Insert {
                    key: payment_id.into_bytes(),
                    value: payment_data.into_bytes(),
                }
            }
            "account_update" => {
                let account_id = rng.gen_range(1..100000);
                let key = format!("account_info_{}", account_id);
                let update_data = format!("Account updated at {}", chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: update_data.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(FinancialWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

/// Gaming platform workload (simplified)
pub struct GamingWorkload {
    operation_weights: Vec<(String, f64)>,
}

impl GamingWorkload {
    pub fn new() -> Self {
        Self {
            operation_weights: vec![
                ("player_stats".to_string(), 30.0),       // Update/view player statistics
                ("game_session".to_string(), 25.0),       // Game session data
                ("leaderboard".to_string(), 20.0),        // Leaderboard queries
                ("inventory_update".to_string(), 10.0),   // Update player inventory
                ("social_features".to_string(), 10.0),    // Friend lists, chat, etc.
                ("achievement_check".to_string(), 5.0),   // Achievement processing
            ],
        }
    }
}

impl WorkloadGenerator for GamingWorkload {
    fn generate_operation(&self, worker_id: usize, operation_count: u64) -> Operation {
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let rand_val: f64 = rng.gen();

        let mut cumulative_prob = 0.0;
        let operation_type = self.operation_weights.iter()
            .find(|(_, prob)| {
                cumulative_prob += prob / 100.0;
                rand_val <= cumulative_prob
            })
            .map(|(op, _)| op)
            .unwrap_or(&self.operation_weights[0].0);

        match operation_type.as_str() {
            "player_stats" => {
                let player_id = worker_id % 100000; // Simulate 100k players
                let key = format!("player_stats_{}", player_id);
                let stats = format!("Level: {}, XP: {}, Last played: {}",
                    rng.gen_range(1..100),
                    rng.gen_range(1000..100000),
                    chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: stats.into_bytes(),
                }
            }
            "game_session" => {
                let session_id = format!("session_{}_{}", worker_id, operation_count % 100);
                let session_data = format!("Game session data for player {} at {}",
                    worker_id, chrono::Utc::now());
                Operation::Update {
                    key: session_id.into_bytes(),
                    value: session_data.into_bytes(),
                }
            }
            "leaderboard" => {
                let game_mode = rng.gen_range(1..10);
                let key = format!("leaderboard_mode_{}", game_mode);
                Operation::Read { key: key.into_bytes() }
            }
            "inventory_update" => {
                let player_id = worker_id % 100000;
                let key = format!("inventory_{}", player_id);
                let item = format!("Added item_{} at {}", rng.gen_range(1..1000), chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: item.into_bytes(),
                }
            }
            "social_features" => {
                let feature_type = rng.gen_range(0..3);
                match feature_type {
                    0 => { // Friend list
                        let player_id = worker_id % 100000;
                        let key = format!("friends_{}", player_id);
                        Operation::Read { key: key.into_bytes() }
                    }
                    1 => { // Chat message
                        let chat_id = rng.gen_range(1..10000);
                        let key = format!("chat_{}", chat_id);
                        let message = format!("Message from player {}: {}", worker_id, operation_count);
                        Operation::Update {
                            key: key.into_bytes(),
                            value: message.into_bytes(),
                        }
                    }
                    _ => { // Guild info
                        let guild_id = rng.gen_range(1..1000);
                        let key = format!("guild_{}", guild_id);
                        Operation::Read { key: key.into_bytes() }
                    }
                }
            }
            "achievement_check" => {
                let player_id = worker_id % 100000;
                let key = format!("achievements_{}", player_id);
                let achievement = format!("Unlocked achievement_{} at {}", rng.gen_range(1..100), chrono::Utc::now());
                Operation::Update {
                    key: key.into_bytes(),
                    value: achievement.into_bytes(),
                }
            }
            _ => {
                let key = format!("default_{}_{}", worker_id, operation_count);
                Operation::Read { key: key.into_bytes() }
            }
        }
    }

    fn clone_box(&self) -> Box<dyn WorkloadGenerator> {
        Box::new(GamingWorkload {
            operation_weights: self.operation_weights.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_social_network_workload() {
        let workload = SocialNetworkWorkload::new();

        // Generate a few operations to test
        for i in 0..5 {
            let op = workload.generate_operation(1, i);
            match op {
                Operation::Read { .. } | Operation::Insert { .. } | Operation::Update { .. } | Operation::Scan { .. } => {}
                _ => panic!("Unexpected operation type"),
            }
        }
    }

    #[test]
    fn test_ecommerce_workload() {
        let workload = EcommerceWorkload::new();

        let op = workload.generate_operation(1, 1);
        // Should generate a valid operation
        match op {
            Operation::Read { .. } | Operation::Insert { .. } | Operation::Update { .. } | Operation::Scan { .. } => {}
            _ => panic!("Unexpected operation type"),
        }
    }
}

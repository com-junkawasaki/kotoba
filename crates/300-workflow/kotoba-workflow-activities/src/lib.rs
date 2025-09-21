//! # Kotoba Workflow Activities Library
//!
//! Pre-built activity implementations for common workflow patterns.
//! This library provides ready-to-use activities for HTTP, database, messaging, cloud services, and more.

pub mod http;
pub mod database;
pub mod cache;
pub mod messaging;
pub mod cloud;
pub mod email;
pub mod file;
pub mod transform;
pub mod timer;
pub mod validation;
pub mod notification;
pub mod integration;
pub mod ai;

use kotoba_workflow::Activity;
use std::collections::HashMap;
use serde_json::Value;

/// Activity registry for easy access to all pre-built activities
pub struct ActivityLibrary {
    activities: HashMap<String, Box<dyn Activity + Send + Sync>>,
}

impl ActivityLibrary {
    pub fn new() -> Self {
        Self {
            activities: HashMap::new(),
        }
    }

    /// Register a custom activity
    pub fn register(&mut self, name: String, activity: Box<dyn Activity + Send + Sync>) {
        self.activities.insert(name, activity);
    }

    /// Get an activity by name
    pub fn get(&self, name: &str) -> Option<&Box<dyn Activity + Send + Sync>> {
        self.activities.get(name)
    }

    /// List all registered activities
    pub fn list(&self) -> Vec<String> {
        self.activities.keys().cloned().collect()
    }
}

/// Standard activity library with pre-built activities
pub fn create_standard_library() -> ActivityLibrary {
    let mut library = ActivityLibrary::new();

    // HTTP Activities
    library.register("http_get".to_string(), Box::new(http::HttpGetActivity::default()));
    library.register("http_post".to_string(), Box::new(http::HttpPostActivity::default()));
    library.register("http_put".to_string(), Box::new(http::HttpPutActivity::default()));
    library.register("http_delete".to_string(), Box::new(http::HttpDeleteActivity::default()));
    library.register("http_patch".to_string(), Box::new(http::HttpPatchActivity::default()));

    // Database Activities
    library.register("postgres_query".to_string(), Box::new(database::PostgresQueryActivity::default()));
    library.register("mysql_query".to_string(), Box::new(database::MySqlQueryActivity::default()));
    library.register("sqlite_query".to_string(), Box::new(database::SqliteQueryActivity::default()));

    // Cache Activities (TODO: Implement)
    // library.register("redis_get".to_string(), Box::new(cache::RedisGetActivity::default()));
    // library.register("redis_set".to_string(), Box::new(cache::RedisSetActivity::default()));
    // library.register("redis_delete".to_string(), Box::new(cache::RedisDeleteActivity::default()));

    // Messaging Activities (TODO: Implement)
    // library.register("rabbitmq_publish".to_string(), Box::new(messaging::RabbitMqPublishActivity::default()));
    // library.register("rabbitmq_consume".to_string(), Box::new(messaging::RabbitMqConsumeActivity::default()));

    // Cloud Storage Activities (TODO: Implement)
    // library.register("s3_upload".to_string(), Box::new(cloud::S3UploadActivity::default()));
    // library.register("s3_download".to_string(), Box::new(cloud::S3DownloadActivity::default()));
    // library.register("s3_delete".to_string(), Box::new(cloud::S3DeleteActivity::default()));

    // Email Activities (TODO: Implement)
    // library.register("smtp_send".to_string(), Box::new(email::SmtpSendActivity::default()));

    // File Activities (TODO: Implement)
    // library.register("file_read".to_string(), Box::new(file::FileReadActivity::default()));
    // library.register("file_write".to_string(), Box::new(file::FileWriteActivity::default()));
    // library.register("file_copy".to_string(), Box::new(file::FileCopyActivity::default()));
    // library.register("csv_parse".to_string(), Box::new(file::CsvParseActivity::default()));
    // library.register("zip_create".to_string(), Box::new(file::ZipCreateActivity::default()));

    // Transform Activities (TODO: Implement)
    // library.register("json_transform".to_string(), Box::new(transform::JsonTransformActivity::default()));
    // library.register("string_replace".to_string(), Box::new(transform::StringReplaceActivity::default()));
    // library.register("base64_encode".to_string(), Box::new(transform::Base64EncodeActivity::default()));
    // library.register("base64_decode".to_string(), Box::new(transform::Base64DecodeActivity::default()));

    // Timer Activities (TODO: Implement)
    // library.register("timer_wait".to_string(), Box::new(timer::TimerWaitActivity::default()));
    // library.register("timer_schedule".to_string(), Box::new(timer::TimerScheduleActivity::default()));

    // Validation Activities (TODO: Implement)
    // library.register("json_validate".to_string(), Box::new(validation::JsonValidateActivity::default()));
    // library.register("regex_match".to_string(), Box::new(validation::RegexMatchActivity::default()));
    // library.register("schema_validate".to_string(), Box::new(validation::SchemaValidateActivity::default()));

    // Notification Activities (TODO: Implement)
    // library.register("webhook_notify".to_string(), Box::new(notification::WebhookNotifyActivity::default()));
    // library.register("slack_notify".to_string(), Box::new(notification::SlackNotifyActivity::default()));

    library
}

/// Configuration for activity initialization
#[derive(Debug, Clone)]
pub struct ActivityConfig {
    pub name: String,
    pub config: HashMap<String, Value>,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            config: HashMap::new(),
        }
    }
}

/// Activity factory for creating configured activities
pub struct ActivityFactory;

impl ActivityFactory {
    pub fn create_http_get(config: ActivityConfig) -> Box<dyn Activity + Send + Sync> {
        Box::new(http::HttpGetActivity::with_config(config))
    }

    pub fn create_database_query(config: ActivityConfig, db_type: &str) -> Box<dyn Activity + Send + Sync> {
        match db_type {
            "postgres" => Box::new(database::PostgresQueryActivity::with_config(config)),
            "mysql" => Box::new(database::MySqlQueryActivity::with_config(config)),
            "sqlite" => Box::new(database::SqliteQueryActivity::with_config(config)),
            _ => panic!("Unsupported database type: {}", db_type),
        }
    }

    pub fn create_s3_upload(config: ActivityConfig) -> Box<dyn Activity + Send + Sync> {
        // TODO: Implement
        todo!("S3 upload activity not implemented")
    }

    pub fn create_email_send(config: ActivityConfig) -> Box<dyn Activity + Send + Sync> {
        // TODO: Implement
        todo!("Email send activity not implemented")
    }
}

/// Activity categories for organization
#[derive(Debug, Clone)]
pub enum ActivityCategory {
    Http,
    Database,
    Cache,
    Messaging,
    Cloud,
    Email,
    File,
    Transform,
    Timer,
    Validation,
    Notification,
    Ai,
    Custom,
}

impl ActivityCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActivityCategory::Http => "http",
            ActivityCategory::Database => "database",
            ActivityCategory::Cache => "cache",
            ActivityCategory::Messaging => "messaging",
            ActivityCategory::Cloud => "cloud",
            ActivityCategory::Email => "email",
            ActivityCategory::File => "file",
            ActivityCategory::Transform => "transform",
            ActivityCategory::Timer => "timer",
            ActivityCategory::Validation => "validation",
            ActivityCategory::Notification => "notification",
            ActivityCategory::Ai => "ai",
            ActivityCategory::Custom => "custom",
        }
    }
}

/// Activity metadata for discovery and documentation
#[derive(Debug, Clone)]
pub struct ActivityMetadata {
    pub name: String,
    pub category: ActivityCategory,
    pub description: String,
    pub inputs: Vec<ActivityParam>,
    pub outputs: Vec<ActivityParam>,
    pub config_params: Vec<ActivityParam>,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ActivityParam {
    pub name: String,
    pub param_type: String,
    pub required: bool,
    pub description: String,
    pub default_value: Option<Value>,
}

/// Activity registry with metadata
pub struct ActivityRegistry {
    activities: HashMap<String, Box<dyn Activity + Send + Sync>>,
    metadata: HashMap<String, ActivityMetadata>,
}

impl ActivityRegistry {
    pub fn new() -> Self {
        Self {
            activities: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn register(&mut self, name: String, activity: Box<dyn Activity + Send + Sync>, metadata: ActivityMetadata) {
        self.activities.insert(name.clone(), activity);
        self.metadata.insert(name, metadata);
    }

    pub fn get_activity(&self, name: &str) -> Option<&Box<dyn Activity + Send + Sync>> {
        self.activities.get(name)
    }

    pub fn get_metadata(&self, name: &str) -> Option<&ActivityMetadata> {
        self.metadata.get(name)
    }

    pub fn list_activities(&self) -> Vec<String> {
        self.activities.keys().cloned().collect()
    }

    pub fn list_by_category(&self, category: &ActivityCategory) -> Vec<String> {
        self.metadata.iter()
            .filter(|(_, metadata)| std::mem::discriminant(&metadata.category) == std::mem::discriminant(category))
            .map(|(name, _)| name.clone())
            .collect()
    }
}

/// Utility functions for activity development
pub mod utils {
    use super::*;

    /// Validate activity inputs against expected parameters
    pub fn validate_inputs(inputs: &HashMap<String, Value>, params: &[ActivityParam]) -> Result<(), String> {
        for param in params {
            if param.required {
                if !inputs.contains_key(&param.name) {
                    return Err(format!("Required parameter '{}' is missing", param.name));
                }
            }

            if let Some(value) = inputs.get(&param.name) {
                if !validate_param_type(value, &param.param_type) {
                    return Err(format!("Parameter '{}' has invalid type. Expected {}", param.name, param.param_type));
                }
            }
        }
        Ok(())
    }

    /// Validate parameter type
    pub fn validate_param_type(value: &Value, expected_type: &str) -> bool {
        match expected_type {
            "string" => value.is_string(),
            "number" => value.is_number(),
            "boolean" => value.is_boolean(),
            "object" => value.is_object(),
            "array" => value.is_array(),
            _ => true, // Allow unknown types
        }
    }

    /// Extract parameter value with default
    pub fn extract_param<'a>(inputs: &'a HashMap<String, Value>, name: &str, default: Option<&'a Value>) -> &'a Value {
        inputs.get(name).unwrap_or(default.unwrap_or(&Value::Null))
    }

    /// Convert parameter to string
    pub fn param_as_string(inputs: &HashMap<String, Value>, name: &str) -> Option<String> {
        inputs.get(name).and_then(|v| v.as_str()).map(|s| s.to_string())
    }

    /// Convert parameter to number
    pub fn param_as_number(inputs: &HashMap<String, Value>, name: &str) -> Option<f64> {
        inputs.get(name).and_then(|v| v.as_f64())
    }

    /// Convert parameter to boolean
    pub fn param_as_bool(inputs: &HashMap<String, Value>, name: &str) -> Option<bool> {
        inputs.get(name).and_then(|v| v.as_bool())
    }
}

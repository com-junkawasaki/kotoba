//! General configuration management for Kotoba applications

use crate::{KotobaNetError, Result};
use kotoba_jsonnet::JsonnetValue;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,
    pub database: DatabaseConfig,
    pub cache: CacheConfig,
    pub messaging: MessagingConfig,
    pub external: ExternalServicesConfig,
    pub features: FeatureFlags,
    pub custom: serde_json::Value,
}

/// Application settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub name: String,
    pub version: String,
    pub environment: String,
    pub debug: bool,
    pub log_level: String,
    pub timezone: String,
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub enabled: bool,
    pub driver: DatabaseDriver,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: Option<String>, // Should be loaded from secrets
    pub connection_pool: ConnectionPoolConfig,
    pub ssl: bool,
    pub migrations: MigrationConfig,
}

/// Database driver
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(PartialEq)]
pub enum DatabaseDriver {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    Custom(String),
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout_seconds: u32,
    pub idle_timeout_seconds: u32,
    pub max_lifetime_seconds: u32,
}

/// Migration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationConfig {
    pub enabled: bool,
    pub directory: String,
    pub auto_run: bool,
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub enabled: bool,
    pub driver: CacheDriver,
    pub host: String,
    pub port: u16,
    pub ttl_seconds: u32,
    pub max_memory_mb: u32,
}

/// Cache driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheDriver {
    Redis,
    Memcached,
    InMemory,
    Custom(String),
}

/// Messaging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagingConfig {
    pub enabled: bool,
    pub driver: MessagingDriver,
    pub host: String,
    pub port: u16,
    pub queues: HashMap<String, QueueConfig>,
    pub topics: HashMap<String, TopicConfig>,
}

/// Messaging driver
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessagingDriver {
    RabbitMQ,
    Kafka,
    SQS,
    PubSub,
    Custom(String),
}

/// Queue configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    pub name: String,
    pub durable: bool,
    pub auto_delete: bool,
    pub max_length: Option<u32>,
}

/// Topic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicConfig {
    pub name: String,
    pub partitions: u32,
    pub replication_factor: u32,
}

/// External services configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalServicesConfig {
    pub apis: HashMap<String, ApiConfig>,
    pub webhooks: Vec<WebhookConfig>,
    pub integrations: HashMap<String, IntegrationConfig>,
}

/// API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiConfig {
    pub name: String,
    pub base_url: String,
    pub timeout_seconds: u32,
    pub retry_count: u32,
    pub headers: HashMap<String, String>,
}

/// Webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    pub name: String,
    pub url: String,
    pub events: Vec<String>,
    pub secret: Option<String>,
    pub retry_count: u32,
}

/// Integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub name: String,
    pub provider: String,
    pub config: serde_json::Value,
    pub enabled: bool,
}

/// Feature flags
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    pub flags: HashMap<String, bool>,
}

/// Configuration parser for general application settings
#[derive(Debug)]
pub struct ConfigParser;

impl ConfigParser {
    /// Parse application configuration from Jsonnet
    pub fn parse(content: &str) -> Result<AppConfig> {
        let evaluated = crate::evaluate_kotoba(content)?;
        Self::jsonnet_value_to_app_config(&evaluated)
    }

    /// Parse config from file
    pub fn parse_file<P: AsRef<std::path::Path>>(path: P) -> Result<AppConfig> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Convert JsonnetValue to AppConfig
    fn jsonnet_value_to_app_config(value: &JsonnetValue) -> Result<AppConfig> {
        match value {
            JsonnetValue::Object(obj) => {
                let app = Self::extract_app_settings(obj)?;
                let database = Self::extract_database_config(obj)?;
                let cache = Self::extract_cache_config(obj)?;
                let messaging = Self::extract_messaging_config(obj)?;
                let external = Self::extract_external_services(obj)?;
                let features = Self::extract_feature_flags(obj)?;
                let custom = Self::extract_custom_config(obj)?;

                Ok(AppConfig {
                    app,
                    database,
                    cache,
                    messaging,
                    external,
                    features,
                    custom,
                })
            }
            _ => Err(KotobaNetError::Config(
                "Configuration must be an object".to_string(),
            )),
        }
    }

    /// Extract application settings
    fn extract_app_settings(obj: &HashMap<String, JsonnetValue>) -> Result<AppSettings> {
        if let Some(JsonnetValue::Object(app_obj)) = obj.get("app") {
            let name = Self::extract_string(app_obj, "name")?;
            let version = Self::extract_string(app_obj, "version")?;
            let environment = Self::extract_string(app_obj, "environment")
                .unwrap_or_else(|_| "development".to_string());
            let debug = Self::extract_bool(app_obj, "debug").unwrap_or(false);
            let log_level = Self::extract_string(app_obj, "logLevel")
                .unwrap_or_else(|_| "info".to_string());
            let timezone = Self::extract_string(app_obj, "timezone")
                .unwrap_or_else(|_| "UTC".to_string());

            Ok(AppSettings {
                name,
                version,
                environment,
                debug,
                log_level,
                timezone,
            })
        } else {
            // Default app settings
            Ok(AppSettings {
                name: "Kotoba App".to_string(),
                version: "1.0.0".to_string(),
                environment: "development".to_string(),
                debug: false,
                log_level: "info".to_string(),
                timezone: "UTC".to_string(),
            })
        }
    }

    /// Extract database configuration
    fn extract_database_config(obj: &HashMap<String, JsonnetValue>) -> Result<DatabaseConfig> {
        if let Some(JsonnetValue::Object(db_obj)) = obj.get("database") {
            let enabled = Self::extract_bool(db_obj, "enabled").unwrap_or(true);
            let driver = Self::extract_database_driver(db_obj)?;
            let host = Self::extract_string(db_obj, "host")?;
            let port = Self::extract_number(db_obj, "port").unwrap_or(5432.0) as u16;
            let database = Self::extract_string(db_obj, "database")?;
            let username = Self::extract_string(db_obj, "username")?;
            let password = Self::extract_string(db_obj, "password").ok();
            let connection_pool = Self::extract_connection_pool(db_obj)?;
            let ssl = Self::extract_bool(db_obj, "ssl").unwrap_or(true);
            let migrations = Self::extract_migration_config(db_obj)?;

            Ok(DatabaseConfig {
                enabled,
                driver,
                host,
                port,
                database,
                username,
                password,
                connection_pool,
                ssl,
                migrations,
            })
        } else {
            Ok(DatabaseConfig {
                enabled: false,
                driver: DatabaseDriver::PostgreSQL,
                host: "localhost".to_string(),
                port: 5432,
                database: "kotoba".to_string(),
                username: "user".to_string(),
                password: None,
                connection_pool: ConnectionPoolConfig {
                    max_connections: 10,
                    min_connections: 1,
                    acquire_timeout_seconds: 30,
                    idle_timeout_seconds: 300,
                    max_lifetime_seconds: 3600,
                },
                ssl: false,
                migrations: MigrationConfig {
                    enabled: true,
                    directory: "migrations".to_string(),
                    auto_run: true,
                },
            })
        }
    }

    /// Extract database driver
    fn extract_database_driver(obj: &HashMap<String, JsonnetValue>) -> Result<DatabaseDriver> {
        let driver_str = Self::extract_string(obj, "driver")
            .unwrap_or_else(|_| "PostgreSQL".to_string());

        match driver_str.as_str() {
            "PostgreSQL" => Ok(DatabaseDriver::PostgreSQL),
            "MySQL" => Ok(DatabaseDriver::MySQL),
            "SQLite" => Ok(DatabaseDriver::SQLite),
            "MongoDB" => Ok(DatabaseDriver::MongoDB),
            "Redis" => Ok(DatabaseDriver::Redis),
            custom => Ok(DatabaseDriver::Custom(custom.to_string())),
        }
    }

    /// Extract connection pool configuration
    fn extract_connection_pool(obj: &HashMap<String, JsonnetValue>) -> Result<ConnectionPoolConfig> {
        if let Some(JsonnetValue::Object(pool_obj)) = obj.get("connectionPool") {
            let max_connections = Self::extract_number(pool_obj, "maxConnections").unwrap_or(10.0) as u32;
            let min_connections = Self::extract_number(pool_obj, "minConnections").unwrap_or(1.0) as u32;
            let acquire_timeout_seconds = Self::extract_number(pool_obj, "acquireTimeoutSeconds").unwrap_or(30.0) as u32;
            let idle_timeout_seconds = Self::extract_number(pool_obj, "idleTimeoutSeconds").unwrap_or(300.0) as u32;
            let max_lifetime_seconds = Self::extract_number(pool_obj, "maxLifetimeSeconds").unwrap_or(3600.0) as u32;

            Ok(ConnectionPoolConfig {
                max_connections,
                min_connections,
                acquire_timeout_seconds,
                idle_timeout_seconds,
                max_lifetime_seconds,
            })
        } else {
            Ok(ConnectionPoolConfig {
                max_connections: 10,
                min_connections: 1,
                acquire_timeout_seconds: 30,
                idle_timeout_seconds: 300,
                max_lifetime_seconds: 3600,
            })
        }
    }

    /// Extract migration configuration
    fn extract_migration_config(obj: &HashMap<String, JsonnetValue>) -> Result<MigrationConfig> {
        if let Some(JsonnetValue::Object(mig_obj)) = obj.get("migrations") {
            let enabled = Self::extract_bool(mig_obj, "enabled").unwrap_or(true);
            let directory = Self::extract_string(mig_obj, "directory")
                .unwrap_or_else(|_| "migrations".to_string());
            let auto_run = Self::extract_bool(mig_obj, "autoRun").unwrap_or(true);

            Ok(MigrationConfig {
                enabled,
                directory,
                auto_run,
            })
        } else {
            Ok(MigrationConfig {
                enabled: true,
                directory: "migrations".to_string(),
                auto_run: true,
            })
        }
    }

    /// Extract cache configuration
    fn extract_cache_config(obj: &HashMap<String, JsonnetValue>) -> Result<CacheConfig> {
        if let Some(JsonnetValue::Object(cache_obj)) = obj.get("cache") {
            let enabled = Self::extract_bool(cache_obj, "enabled").unwrap_or(true);
            let driver = Self::extract_cache_driver(cache_obj)?;
            let host = Self::extract_string(cache_obj, "host")
                .unwrap_or_else(|_| "localhost".to_string());
            let port = Self::extract_number(cache_obj, "port").unwrap_or(6379.0) as u16;
            let ttl_seconds = Self::extract_number(cache_obj, "ttlSeconds").unwrap_or(3600.0) as u32;
            let max_memory_mb = Self::extract_number(cache_obj, "maxMemoryMb").unwrap_or(512.0) as u32;

            Ok(CacheConfig {
                enabled,
                driver,
                host,
                port,
                ttl_seconds,
                max_memory_mb,
            })
        } else {
            Ok(CacheConfig {
                enabled: false,
                driver: CacheDriver::InMemory,
                host: "localhost".to_string(),
                port: 6379,
                ttl_seconds: 3600,
                max_memory_mb: 512,
            })
        }
    }

    /// Extract cache driver
    fn extract_cache_driver(obj: &HashMap<String, JsonnetValue>) -> Result<CacheDriver> {
        let driver_str = Self::extract_string(obj, "driver")
            .unwrap_or_else(|_| "InMemory".to_string());

        match driver_str.as_str() {
            "Redis" => Ok(CacheDriver::Redis),
            "Memcached" => Ok(CacheDriver::Memcached),
            "InMemory" => Ok(CacheDriver::InMemory),
            custom => Ok(CacheDriver::Custom(custom.to_string())),
        }
    }

    /// Extract messaging configuration
    fn extract_messaging_config(obj: &HashMap<String, JsonnetValue>) -> Result<MessagingConfig> {
        if let Some(JsonnetValue::Object(msg_obj)) = obj.get("messaging") {
            let enabled = Self::extract_bool(msg_obj, "enabled").unwrap_or(false);
            let driver = Self::extract_messaging_driver(msg_obj)?;
            let host = Self::extract_string(msg_obj, "host")
                .unwrap_or_else(|_| "localhost".to_string());
            let port = Self::extract_number(msg_obj, "port").unwrap_or(5672.0) as u16;
            let queues = Self::extract_queues(msg_obj)?;
            let topics = Self::extract_topics(msg_obj)?;

            Ok(MessagingConfig {
                enabled,
                driver,
                host,
                port,
                queues,
                topics,
            })
        } else {
            Ok(MessagingConfig {
                enabled: false,
                driver: MessagingDriver::RabbitMQ,
                host: "localhost".to_string(),
                port: 5672,
                queues: HashMap::new(),
                topics: HashMap::new(),
            })
        }
    }

    /// Extract messaging driver
    fn extract_messaging_driver(obj: &HashMap<String, JsonnetValue>) -> Result<MessagingDriver> {
        let driver_str = Self::extract_string(obj, "driver")
            .unwrap_or_else(|_| "RabbitMQ".to_string());

        match driver_str.as_str() {
            "RabbitMQ" => Ok(MessagingDriver::RabbitMQ),
            "Kafka" => Ok(MessagingDriver::Kafka),
            "SQS" => Ok(MessagingDriver::SQS),
            "PubSub" => Ok(MessagingDriver::PubSub),
            custom => Ok(MessagingDriver::Custom(custom.to_string())),
        }
    }

    /// Extract queues
    fn extract_queues(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, QueueConfig>> {
        let mut queues = HashMap::new();

        if let Some(JsonnetValue::Object(queues_obj)) = obj.get("queues") {
            for (name, config) in queues_obj {
                if let JsonnetValue::Object(config_obj) = config {
                    let queue_config = Self::parse_queue_config(name, config_obj)?;
                    queues.insert(name.clone(), queue_config);
                }
            }
        }

        Ok(queues)
    }

    /// Parse queue configuration
    fn parse_queue_config(name: &str, obj: &HashMap<String, JsonnetValue>) -> Result<QueueConfig> {
        let durable = Self::extract_bool(obj, "durable").unwrap_or(true);
        let auto_delete = Self::extract_bool(obj, "autoDelete").unwrap_or(false);
        let max_length = Self::extract_number(obj, "maxLength").map(|n| n as u32).ok();

        Ok(QueueConfig {
            name: name.to_string(),
            durable,
            auto_delete,
            max_length,
        })
    }

    /// Extract topics
    fn extract_topics(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, TopicConfig>> {
        let mut topics = HashMap::new();

        if let Some(JsonnetValue::Object(topics_obj)) = obj.get("topics") {
            for (name, config) in topics_obj {
                if let JsonnetValue::Object(config_obj) = config {
                    let topic_config = Self::parse_topic_config(name, config_obj)?;
                    topics.insert(name.clone(), topic_config);
                }
            }
        }

        Ok(topics)
    }

    /// Parse topic configuration
    fn parse_topic_config(name: &str, obj: &HashMap<String, JsonnetValue>) -> Result<TopicConfig> {
        let partitions = Self::extract_number(obj, "partitions").unwrap_or(1.0) as u32;
        let replication_factor = Self::extract_number(obj, "replicationFactor").unwrap_or(1.0) as u32;

        Ok(TopicConfig {
            name: name.to_string(),
            partitions,
            replication_factor,
        })
    }

    /// Extract external services configuration
    fn extract_external_services(obj: &HashMap<String, JsonnetValue>) -> Result<ExternalServicesConfig> {
        let apis = Self::extract_apis(obj)?;
        let webhooks = Self::extract_webhooks(obj)?;
        let integrations = Self::extract_integrations(obj)?;

        Ok(ExternalServicesConfig {
            apis,
            webhooks,
            integrations,
        })
    }

    /// Extract APIs
    fn extract_apis(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, ApiConfig>> {
        let mut apis = HashMap::new();

        if let Some(JsonnetValue::Object(apis_obj)) = obj.get("apis") {
            for (name, config) in apis_obj {
                if let JsonnetValue::Object(config_obj) = config {
                    let api_config = Self::parse_api_config(name, config_obj)?;
                    apis.insert(name.clone(), api_config);
                }
            }
        }

        Ok(apis)
    }

    /// Parse API configuration
    fn parse_api_config(name: &str, obj: &HashMap<String, JsonnetValue>) -> Result<ApiConfig> {
        let base_url = Self::extract_string(obj, "baseUrl")?;
        let timeout_seconds = Self::extract_number(obj, "timeoutSeconds").unwrap_or(30.0) as u32;
        let retry_count = Self::extract_number(obj, "retryCount").unwrap_or(3.0) as u32;
        let headers = Self::extract_string_map(obj, "headers")?;

        Ok(ApiConfig {
            name: name.to_string(),
            base_url,
            timeout_seconds,
            retry_count,
            headers,
        })
    }

    /// Extract webhooks
    fn extract_webhooks(obj: &HashMap<String, JsonnetValue>) -> Result<Vec<WebhookConfig>> {
        let mut webhooks = Vec::new();

        if let Some(JsonnetValue::Array(webhook_array)) = obj.get("webhooks") {
            for webhook_value in webhook_array {
                if let JsonnetValue::Object(webhook_obj) = webhook_value {
                    let webhook = Self::parse_webhook_config(webhook_obj)?;
                    webhooks.push(webhook);
                }
            }
        }

        Ok(webhooks)
    }

    /// Parse webhook configuration
    fn parse_webhook_config(obj: &HashMap<String, JsonnetValue>) -> Result<WebhookConfig> {
        let name = Self::extract_string(obj, "name")?;
        let url = Self::extract_string(obj, "url")?;
        let events = Self::extract_string_array(obj, "events")?;
        let secret = Self::extract_string(obj, "secret").ok();
        let retry_count = Self::extract_number(obj, "retryCount").unwrap_or(3.0) as u32;

        Ok(WebhookConfig {
            name,
            url,
            events,
            secret,
            retry_count,
        })
    }

    /// Extract integrations
    fn extract_integrations(obj: &HashMap<String, JsonnetValue>) -> Result<HashMap<String, IntegrationConfig>> {
        let mut integrations = HashMap::new();

        if let Some(JsonnetValue::Object(int_obj)) = obj.get("integrations") {
            for (name, config) in int_obj {
                if let JsonnetValue::Object(config_obj) = config {
                    let integration_config = Self::parse_integration_config(name, config_obj)?;
                    integrations.insert(name.clone(), integration_config);
                }
            }
        }

        Ok(integrations)
    }

    /// Parse integration configuration
    fn parse_integration_config(name: &str, obj: &HashMap<String, JsonnetValue>) -> Result<IntegrationConfig> {
        let provider = Self::extract_string(obj, "provider")?;
        let config = Self::jsonnet_object_to_hashmap(obj)?;
        let enabled = Self::extract_bool(obj, "enabled").unwrap_or(true);

        Ok(IntegrationConfig {
            name: name.to_string(),
            provider,
            config,
            enabled,
        })
    }

    /// Extract feature flags
    fn extract_feature_flags(obj: &HashMap<String, JsonnetValue>) -> Result<FeatureFlags> {
        let mut flags = HashMap::new();

        if let Some(JsonnetValue::Object(flags_obj)) = obj.get("features") {
            for (flag_name, flag_value) in flags_obj {
                if let JsonnetValue::Boolean(enabled) = flag_value {
                    flags.insert(flag_name.clone(), *enabled);
                }
            }
        }

        Ok(FeatureFlags { flags })
    }

    /// Extract custom configuration
    fn extract_custom_config(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        if let Some(JsonnetValue::Object(custom_obj)) = obj.get("custom") {
            Self::jsonnet_object_to_json_value(custom_obj)
        } else {
            Ok(serde_json::Value::Object(serde_json::Map::new()))
        }
    }

    fn jsonnet_object_to_json_value(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        let mut map = serde_json::Map::new();
        for (key, value) in obj {
            let json_value = Self::jsonnet_value_to_json_value(value)?;
            map.insert(key.clone(), json_value);
        }
        Ok(serde_json::Value::Object(map))
    }

    fn jsonnet_value_to_json_value(value: &JsonnetValue) -> Result<serde_json::Value> {
        match value {
            JsonnetValue::Null => Ok(serde_json::Value::Null),
            JsonnetValue::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
            JsonnetValue::Number(n) => Ok(serde_json::Value::Number(serde_json::Number::from_f64(*n).unwrap())),
            JsonnetValue::String(s) => Ok(serde_json::Value::String(s.clone())),
            JsonnetValue::Array(arr) => {
                let mut json_arr = Vec::new();
                for item in arr {
                    json_arr.push(Self::jsonnet_value_to_json_value(item)?);
                }
                Ok(serde_json::Value::Array(json_arr))
            }
            JsonnetValue::Object(obj) => Self::jsonnet_object_to_json_value(obj),
            JsonnetValue::Function(_) => Err(KotobaNetError::Config("Functions cannot be converted to JSON".to_string())),
        }
    }

    // Helper methods

    fn extract_string(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<String> {
        match obj.get(key) {
            Some(JsonnetValue::String(s)) => Ok(s.clone()),
            _ => Err(KotobaNetError::Config(format!("Expected string for key '{}'", key))),
        }
    }

    fn extract_bool(obj: &HashMap<String, JsonnetValue>, key: &str) -> Option<bool> {
        match obj.get(key) {
            Some(JsonnetValue::Boolean(b)) => Some(*b),
            _ => None,
        }
    }

    fn extract_number(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<f64> {
        match obj.get(key) {
            Some(JsonnetValue::Number(n)) => Ok(*n),
            _ => Err(KotobaNetError::Config(format!("Expected number for key '{}'", key))),
        }
    }

    fn extract_string_array(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<Vec<String>> {
        match obj.get(key) {
            Some(JsonnetValue::Array(arr)) => {
                let mut strings = Vec::new();
                for item in arr {
                    if let JsonnetValue::String(s) = item {
                        strings.push(s.clone());
                    }
                }
                Ok(strings)
            }
            _ => Ok(Vec::new()),
        }
    }

    fn extract_string_map(obj: &HashMap<String, JsonnetValue>, key: &str) -> Result<HashMap<String, String>> {
        match obj.get(key) {
            Some(JsonnetValue::Object(map_obj)) => {
                let mut result = HashMap::new();
                for (k, v) in map_obj {
                    if let JsonnetValue::String(s) = v {
                        result.insert(k.clone(), s.clone());
                    }
                }
                Ok(result)
            }
            _ => Ok(HashMap::new()),
        }
    }

    fn jsonnet_object_to_hashmap(obj: &HashMap<String, JsonnetValue>) -> Result<serde_json::Value> {
        Self::jsonnet_object_to_json_value(obj)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_app_config() {
        let config = r#"
        {
            app: {
                name: "MyApp",
                version: "1.0.0",
                environment: "production",
                debug: false,
            },
            database: {
                enabled: true,
                driver: "PostgreSQL",
                host: "localhost",
                port: 5432,
                database: "myapp",
                username: "user",
            },
            features: {
                newFeature: true,
                experimentalFeature: false,
            }
        }
        "#;

        let result = ConfigParser::parse(config);
        assert!(result.is_ok());

        let app_config = result.unwrap();
        assert_eq!(app_config.app.name, "MyApp");
        assert_eq!(app_config.app.version, "1.0.0");
        assert!(app_config.database.enabled);
        assert_eq!(app_config.database.driver, DatabaseDriver::PostgreSQL);
        assert!(app_config.features.flags.get("newFeature").unwrap());
        assert!(!app_config.features.flags.get("experimentalFeature").unwrap());
    }
}

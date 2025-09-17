//! # Kotoba Cloud Integrations
//!
//! Cloud-native integrations for AWS, GCP, and Azure services.
//!
//! ## Features
//!
//! - **AWS Integration**: S3, Lambda, SQS, SNS, DynamoDB, EC2, ECS, EKS, IAM, CloudWatch
//! - **GCP Integration**: Cloud Storage, Pub/Sub, BigQuery, Cloud Functions, Cloud Logging
//! - **Azure Integration**: Blob Storage, Service Bus, Functions, Monitor
//! - **Unified API**: Consistent interface across all cloud providers
//! - **Auto-discovery**: Automatic resource discovery and configuration
//! - **Error Handling**: Comprehensive error handling and retry logic
//! - **Monitoring**: Integration with cloud monitoring services

pub mod aws;
pub mod gcp;
pub mod azure;
pub mod common;
pub mod discovery;
pub mod monitoring;

use kotoba_workflow::{Activity, ActivityError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Cloud provider enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
}

/// Cloud region/location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudRegion {
    pub provider: CloudProvider,
    pub region: String,
    pub display_name: String,
    pub available_services: Vec<String>,
}

/// Cloud resource representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudResource {
    pub id: String,
    pub name: String,
    pub resource_type: String,
    pub provider: CloudProvider,
    pub region: String,
    pub arn: Option<String>,
    pub tags: HashMap<String, String>,
    pub status: ResourceStatus,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Resource status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceStatus {
    Creating,
    Available,
    Updating,
    Deleting,
    Deleted,
    Error,
}

/// Cloud credentials
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudCredentials {
    pub provider: CloudProvider,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
    pub service_account_key: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub tenant_id: Option<String>,
    pub subscription_id: Option<String>,
}

/// Cloud service client
#[async_trait::async_trait]
pub trait CloudService: Send + Sync {
    /// Get service name
    fn service_name(&self) -> &str;

    /// Get cloud provider
    fn provider(&self) -> CloudProvider;

    /// Health check
    async fn health_check(&self) -> Result<(), CloudError>;

    /// List resources
    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError>;

    /// Get resource details
    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError>;

    /// Create resource
    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError>;

    /// Update resource
    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError>;

    /// Delete resource
    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError>;
}

/// Cloud integration manager
pub struct CloudIntegrationManager {
    services: HashMap<String, Arc<dyn CloudService>>,
    credentials: HashMap<CloudProvider, CloudCredentials>,
    regions: Vec<CloudRegion>,
}

impl CloudIntegrationManager {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            credentials: HashMap::new(),
            regions: Vec::new(),
        }
    }

    /// Add cloud credentials
    pub fn add_credentials(&mut self, credentials: CloudCredentials) {
        self.credentials.insert(credentials.provider.clone(), credentials);
    }

    /// Register cloud service
    pub fn register_service(&mut self, name: String, service: Arc<dyn CloudService>) {
        self.services.insert(name, service);
    }

    /// Get cloud service
    pub fn get_service(&self, name: &str) -> Option<&Arc<dyn CloudService>> {
        self.services.get(name)
    }

    /// List all services
    pub fn list_services(&self) -> Vec<String> {
        self.services.keys().cloned().collect()
    }

    /// Initialize AWS services
    #[cfg(feature = "aws")]
    pub async fn init_aws_services(&mut self) -> Result<(), CloudError> {
        if let Some(creds) = self.credentials.get(&CloudProvider::AWS) {
            // Initialize AWS SDK
            let config = aws_config::load_from_env().await;

            // Register AWS services
            let s3_service = Arc::new(aws::S3Service::new(config.clone()).await?);
            self.register_service("aws_s3".to_string(), s3_service);

            let lambda_service = Arc::new(aws::LambdaService::new(config.clone()).await?);
            self.register_service("aws_lambda".to_string(), lambda_service);

            let sqs_service = Arc::new(aws::SQSService::new(config.clone()).await?);
            self.register_service("aws_sqs".to_string(), sqs_service);

            let sns_service = Arc::new(aws::SNSService::new(config.clone()).await?);
            self.register_service("aws_sns".to_string(), sns_service);

            let dynamodb_service = Arc::new(aws::DynamoDBService::new(config.clone()).await?);
            self.register_service("aws_dynamodb".to_string(), dynamodb_service);

            let cloudwatch_service = Arc::new(aws::CloudWatchService::new(config).await?);
            self.register_service("aws_cloudwatch".to_string(), cloudwatch_service);
        }

        Ok(())
    }

    /// Initialize GCP services
    #[cfg(feature = "gcp")]
    pub async fn init_gcp_services(&mut self) -> Result<(), CloudError> {
        if let Some(creds) = self.credentials.get(&CloudProvider::GCP) {
            // Initialize GCP services
            let storage_service = Arc::new(gcp::CloudStorageService::new(creds.clone()).await?);
            self.register_service("gcp_storage".to_string(), storage_service);

            let pubsub_service = Arc::new(gcp::PubSubService::new(creds.clone()).await?);
            self.register_service("gcp_pubsub".to_string(), pubsub_service);

            let bigquery_service = Arc::new(gcp::BigQueryService::new(creds.clone()).await?);
            self.register_service("gcp_bigquery".to_string(), bigquery_service);

            let functions_service = Arc::new(gcp::CloudFunctionsService::new(creds.clone()).await?);
            self.register_service("gcp_functions".to_string(), functions_service);
        }

        Ok(())
    }

    /// Initialize Azure services
    #[cfg(feature = "azure")]
    pub async fn init_azure_services(&mut self) -> Result<(), CloudError> {
        if let Some(creds) = self.credentials.get(&CloudProvider::Azure) {
            // Initialize Azure services
            let storage_service = Arc::new(azure::BlobStorageService::new(creds.clone()).await?);
            self.register_service("azure_storage".to_string(), storage_service);
        }

        Ok(())
    }

    /// Initialize all cloud services
    pub async fn init_all_services(&mut self) -> Result<(), CloudError> {
        #[cfg(feature = "aws")]
        self.init_aws_services().await?;

        #[cfg(feature = "gcp")]
        self.init_gcp_services().await?;

        #[cfg(feature = "azure")]
        self.init_azure_services().await?;

        Ok(())
    }

    /// Discover cloud resources
    pub async fn discover_resources(&self, provider: CloudProvider, service: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let mut all_resources = Vec::new();

        for (service_name, service_impl) in &self.services {
            if service_impl.provider() == provider {
                if let Some(target_service) = service {
                    if service_name.contains(target_service) {
                        let resources = service_impl.list_resources(None).await?;
                        all_resources.extend(resources);
                    }
                } else {
                    let resources = service_impl.list_resources(None).await?;
                    all_resources.extend(resources);
                }
            }
        }

        Ok(all_resources)
    }

    /// Health check all services
    pub async fn health_check_all(&self) -> HashMap<String, Result<(), CloudError>> {
        let mut results = HashMap::new();

        for (name, service) in &self.services {
            let result = service.health_check().await;
            results.insert(name.clone(), result);
        }

        results
    }
}

/// Cloud activity base trait
#[async_trait::async_trait]
pub trait CloudActivity: Activity {
    /// Get cloud provider
    fn cloud_provider(&self) -> CloudProvider;

    /// Get service name
    fn service_name(&self) -> &str;

    /// Validate cloud credentials
    async fn validate_credentials(&self) -> Result<(), CloudError>;

    /// Get cloud resource information
    async fn get_resource_info(&self, resource_id: &str) -> Result<CloudResource, CloudError>;
}

/// Cloud error types
#[derive(Debug, thiserror::Error)]
pub enum CloudError {
    #[error("AWS SDK error: {0}")]
    AwsError(String),
    #[error("GCP SDK error: {0}")]
    GcpError(String),
    #[error("Azure SDK error: {0}")]
    AzureError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
    #[error("Authentication error: {0}")]
    AuthError(String),
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    #[error("Operation timeout: {0}")]
    Timeout(String),
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

/// Cloud operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudOperationResult {
    pub operation_id: String,
    pub status: OperationStatus,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub start_time: chrono::DateTime<chrono::Utc>,
    pub end_time: Option<chrono::DateTime<chrono::Utc>>,
    pub duration_ms: Option<u64>,
}

/// Operation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// Cloud metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudMetrics {
    pub provider: CloudProvider,
    pub service: String,
    pub operation: String,
    pub success_count: u64,
    pub error_count: u64,
    pub average_latency_ms: f64,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl CloudMetrics {
    pub fn new(provider: CloudProvider, service: &str, operation: &str) -> Self {
        Self {
            provider,
            service: service.to_string(),
            operation: operation.to_string(),
            success_count: 0,
            error_count: 0,
            average_latency_ms: 0.0,
            last_updated: chrono::Utc::now(),
        }
    }

    pub fn record_success(&mut self, latency_ms: u64) {
        self.success_count += 1;
        self.average_latency_ms = (self.average_latency_ms + latency_ms as f64) / 2.0;
        self.last_updated = chrono::Utc::now();
    }

    pub fn record_error(&mut self) {
        self.error_count += 1;
        self.last_updated = chrono::Utc::now();
    }

    pub fn success_rate(&self) -> f64 {
        if self.success_count + self.error_count == 0 {
            0.0
        } else {
            self.success_count as f64 / (self.success_count + self.error_count) as f64
        }
    }
}

/// Cloud resource discovery
pub struct ResourceDiscovery {
    manager: Arc<CloudIntegrationManager>,
}

impl ResourceDiscovery {
    pub fn new(manager: Arc<CloudIntegrationManager>) -> Self {
        Self { manager }
    }

    /// Discover all resources for a provider
    pub async fn discover_all(&self, provider: CloudProvider) -> Result<Vec<CloudResource>, CloudError> {
        self.manager.discover_resources(provider, None).await
    }

    /// Discover resources by type
    pub async fn discover_by_type(&self, provider: CloudProvider, resource_type: &str) -> Result<Vec<CloudResource>, CloudError> {
        let all_resources = self.manager.discover_resources(provider, None).await?;
        Ok(all_resources.into_iter()
            .filter(|r| r.resource_type == resource_type)
            .collect())
    }

    /// Discover resources by tags
    pub async fn discover_by_tags(&self, provider: CloudProvider, tags: HashMap<String, String>) -> Result<Vec<CloudResource>, CloudError> {
        let all_resources = self.manager.discover_resources(provider, None).await?;
        Ok(all_resources.into_iter()
            .filter(|r| {
                tags.iter().all(|(key, value)| {
                    r.tags.get(key).map_or(false, |v| v == value)
                })
            })
            .collect())
    }

    /// Get resource hierarchy
    pub async fn get_resource_hierarchy(&self, provider: CloudProvider) -> Result<HashMap<String, Vec<CloudResource>>, CloudError> {
        let resources = self.manager.discover_resources(provider, None).await?;
        let mut hierarchy = HashMap::new();

        for resource in resources {
            hierarchy.entry(resource.resource_type.clone())
                .or_insert_with(Vec::new)
                .push(resource);
        }

        Ok(hierarchy)
    }
}

/// Cloud monitoring integration
pub struct CloudMonitoring {
    manager: Arc<CloudIntegrationManager>,
    metrics: HashMap<String, CloudMetrics>,
}

impl CloudMonitoring {
    pub fn new(manager: Arc<CloudIntegrationManager>) -> Self {
        Self {
            manager,
            metrics: HashMap::new(),
        }
    }

    /// Record operation metrics
    pub async fn record_operation(&mut self, provider: CloudProvider, service: &str, operation: &str, success: bool, latency_ms: u64) {
        let key = format!("{}:{}:{}", provider_as_str(&provider), service, operation);
        let metrics = self.metrics.entry(key).or_insert_with(|| {
            CloudMetrics::new(provider, service, operation)
        });

        if success {
            metrics.record_success(latency_ms);
        } else {
            metrics.record_error();
        }
    }

    /// Get metrics for service
    pub fn get_service_metrics(&self, provider: CloudProvider, service: &str) -> Vec<&CloudMetrics> {
        let prefix = format!("{}:{}:", provider_as_str(&provider), service);
        self.metrics.values()
            .filter(|m| m.service.starts_with(service))
            .collect()
    }

    /// Get all metrics
    pub fn get_all_metrics(&self) -> Vec<&CloudMetrics> {
        self.metrics.values().collect()
    }

    /// Generate health report
    pub async fn generate_health_report(&self) -> CloudHealthReport {
        let service_health = self.manager.health_check_all().await;
        let mut healthy_services = 0;
        let mut unhealthy_services = 0;

        for result in service_health.values() {
            match result {
                Ok(_) => healthy_services += 1,
                Err(_) => unhealthy_services += 1,
            }
        }

        let mut provider_metrics = HashMap::new();
        for metrics in self.metrics.values() {
            let provider_key = provider_as_str(&metrics.provider);
            let entry = provider_metrics.entry(provider_key.to_string())
                .or_insert_with(|| ProviderMetrics::default());

            entry.total_operations += metrics.success_count + metrics.error_count;
            entry.success_rate = (entry.success_rate + metrics.success_rate()) / 2.0;
            entry.avg_latency = (entry.avg_latency + metrics.average_latency_ms) / 2.0;
        }

        CloudHealthReport {
            timestamp: chrono::Utc::now(),
            healthy_services,
            unhealthy_services,
            provider_metrics,
        }
    }
}

fn provider_as_str(provider: &CloudProvider) -> &str {
    match provider {
        CloudProvider::AWS => "aws",
        CloudProvider::GCP => "gcp",
        CloudProvider::Azure => "azure",
    }
}

/// Cloud health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudHealthReport {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub healthy_services: u32,
    pub unhealthy_services: u32,
    pub provider_metrics: HashMap<String, ProviderMetrics>,
}

/// Provider-specific metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProviderMetrics {
    pub total_operations: u64,
    pub success_rate: f64,
    pub avg_latency: f64,
}

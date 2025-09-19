//! AWS Cloud Integrations
//!
//! Comprehensive AWS service integrations including S3, Lambda, SQS, SNS, DynamoDB, and more.

use async_trait::async_trait;
use kotoba_workflow::{Activity, ActivityError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::{CloudProvider, CloudService, CloudError, CloudResource, CloudActivity, ResourceStatus};

/// AWS S3 Service Integration
#[derive(Debug)]
pub struct S3Service {
    client: aws_sdk_s3::Client,
    config: aws_config::SdkConfig,
}

impl S3Service {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_s3::Client::new(&config);
        Ok(Self { client, config })
    }

    /// List S3 buckets
    pub async fn list_buckets(&self) -> Result<Vec<String>, CloudError> {
        let response = self.client.list_buckets()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to list buckets: {}", e)))?;

        let buckets = response.buckets()
            .iter()
            .filter_map(|bucket| bucket.name().map(|s| s.to_string()))
            .collect();

        Ok(buckets)
    }

    /// Create S3 bucket
    pub async fn create_bucket(&self, bucket_name: &str, region: Option<&str>) -> Result<(), CloudError> {
        let mut request = self.client.create_bucket()
            .bucket(bucket_name);

        if let Some(region) = region {
            if region != "us-east-1" {
                request = request.create_bucket_configuration(
                    aws_sdk_s3::types::CreateBucketConfiguration::builder()
                        .location_constraint(aws_sdk_s3::types::BucketLocationConstraint::from(region))
                        .build()
                );
            }
        }

        request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to create bucket: {}", e)))?;

        Ok(())
    }

    /// Upload object to S3
    pub async fn upload_object(&self, bucket: &str, key: &str, data: Vec<u8>, content_type: Option<&str>) -> Result<String, CloudError> {
        let mut request = self.client.put_object()
            .bucket(bucket)
            .key(key)
            .body(data.into());

        if let Some(content_type) = content_type {
            request = request.content_type(content_type);
        }

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to upload object: {}", e)))?;

        Ok(response.e_tag().unwrap_or("").to_string())
    }

    /// Download object from S3
    pub async fn download_object(&self, bucket: &str, key: &str) -> Result<Vec<u8>, CloudError> {
        let response = self.client.get_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to download object: {}", e)))?;

        let data = response.body.collect()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to read object data: {}", e)))?;

        Ok(data.into_bytes().to_vec())
    }

    /// Delete object from S3
    pub async fn delete_object(&self, bucket: &str, key: &str) -> Result<(), CloudError> {
        self.client.delete_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete object: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl CloudService for S3Service {
    fn service_name(&self) -> &str {
        "s3"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        // Try to list buckets to verify connectivity
        self.list_buckets().await?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let buckets = self.list_buckets().await?;

        let resources = buckets.into_iter()
            .filter(|bucket| {
                if let Some(rt) = resource_type {
                    rt == "bucket"
                } else {
                    true
                }
            })
            .map(|bucket| CloudResource {
                id: bucket.clone(),
                name: bucket.clone(),
                resource_type: "bucket".to_string(),
                provider: CloudProvider::AWS,
                region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                arn: Some(format!("arn:aws:s3:::{}", bucket)),
                tags: HashMap::new(),
                status: ResourceStatus::Available,
                created_at: None,
                metadata: HashMap::new(),
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        let buckets = self.list_buckets().await?;

        if buckets.contains(&resource_id.to_string()) {
            Ok(Some(CloudResource {
                id: resource_id.to_string(),
                name: resource_id.to_string(),
                resource_type: "bucket".to_string(),
                provider: CloudProvider::AWS,
                region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                arn: Some(format!("arn:aws:s3:::{}", resource_id)),
                tags: HashMap::new(),
                status: ResourceStatus::Available,
                created_at: None,
                metadata: HashMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        let bucket_name = config.get("bucket_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CloudError::InvalidRequest("bucket_name is required".to_string()))?;

        let region = config.get("region")
            .and_then(|v| v.as_str());

        self.create_bucket(bucket_name, region).await?;
        Ok(bucket_name.to_string())
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // S3 buckets have limited update capabilities
        // TODO: Implement tagging, versioning, etc.
        Err(CloudError::InvalidRequest("S3 bucket updates not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        // TODO: Implement bucket deletion (requires emptying bucket first)
        Err(CloudError::InvalidRequest("S3 bucket deletion not implemented".to_string()))
    }
}

/// AWS Lambda Service Integration
#[derive(Debug)]
pub struct LambdaService {
    client: aws_sdk_lambda::Client,
    config: aws_config::SdkConfig,
}

impl LambdaService {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_lambda::Client::new(&config);
        Ok(Self { client, config })
    }

    /// List Lambda functions
    pub async fn list_functions(&self) -> Result<Vec<aws_sdk_lambda::types::FunctionConfiguration>, CloudError> {
        let response = self.client.list_functions()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to list functions: {}", e)))?;

        Ok(response.functions().to_vec())
    }

    /// Invoke Lambda function
    pub async fn invoke_function(&self, function_name: &str, payload: Option<&str>) -> Result<String, CloudError> {
        let mut request = self.client.invoke()
            .function_name(function_name);

        if let Some(payload) = payload {
            request = request.payload(aws_sdk_lambda::primitives::Blob::new(payload));
        }

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to invoke function: {}", e)))?;

        let result = if let Some(payload) = response.payload() {
            String::from_utf8(payload.as_ref().to_vec())
                .map_err(|e| CloudError::AwsError(format!("Invalid response payload: {}", e)))?
        } else {
            "".to_string()
        };

        Ok(result)
    }

    /// Create Lambda function
    pub async fn create_function(&self, config: CreateFunctionConfig) -> Result<String, CloudError> {
        let request = self.client.create_function()
            .function_name(&config.function_name)
            .runtime(config.runtime)
            .role(&config.role_arn)
            .handler(&config.handler)
            .code(aws_sdk_lambda::types::FunctionCode::builder()
                .zip_file(config.code)
                .build())
            .description(&config.description)
            .timeout(config.timeout.unwrap_or(30))
            .memory_size(config.memory_size.unwrap_or(128));

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to create function: {}", e)))?;

        Ok(response.function_name().unwrap_or("").to_string())
    }
}

#[derive(Debug)]
pub struct CreateFunctionConfig {
    pub function_name: String,
    pub runtime: aws_sdk_lambda::types::Runtime,
    pub role_arn: String,
    pub handler: String,
    pub code: aws_sdk_lambda::primitives::Blob,
    pub description: String,
    pub timeout: Option<i32>,
    pub memory_size: Option<i32>,
}

#[async_trait]
impl CloudService for LambdaService {
    fn service_name(&self) -> &str {
        "lambda"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        self.list_functions().await?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let functions = self.list_functions().await?;

        let resources = functions.into_iter()
            .filter(|function| {
                if let Some(rt) = resource_type {
                    rt == "function"
                } else {
                    true
                }
            })
            .filter_map(|function| {
                let name = function.function_name()?;
                Some(CloudResource {
                    id: name.to_string(),
                    name: name.to_string(),
                    resource_type: "function".to_string(),
                    provider: CloudProvider::AWS,
                    region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                    arn: function.function_arn().map(|s| s.to_string()),
                    tags: HashMap::new(), // TODO: Get function tags
                    status: ResourceStatus::Available,
                    created_at: function.last_modified().and_then(|s| {
                        chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.into())
                    }),
                    metadata: {
                        let mut meta = HashMap::new();
                        if let Some(runtime) = function.runtime() {
                            meta.insert("runtime".to_string(), serde_json::json!(runtime.as_str()));
                        }
                        if let Some(memory) = function.memory_size() {
                            meta.insert("memory_size".to_string(), serde_json::json!(memory));
                        }
                        meta
                    },
                })
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        let functions = self.list_functions().await?;
        let function = functions.into_iter()
            .find(|f| f.function_name() == Some(resource_id));

        Ok(function.map(|f| {
            let name = f.function_name().unwrap_or("").to_string();
            CloudResource {
                id: name.clone(),
                name,
                resource_type: "function".to_string(),
                provider: CloudProvider::AWS,
                region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                arn: f.function_arn().map(|s| s.to_string()),
                tags: HashMap::new(),
                status: ResourceStatus::Available,
                created_at: f.last_modified().and_then(|s| {
                    chrono::DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.into())
                }),
                metadata: HashMap::new(),
            }
        }))
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        // TODO: Parse config and create function
        Err(CloudError::InvalidRequest("Lambda function creation not fully implemented".to_string()))
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // TODO: Implement function update
        Err(CloudError::InvalidRequest("Lambda function update not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        self.client.delete_function()
            .function_name(resource_id)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete function: {}", e)))?;

        Ok(())
    }
}

/// AWS SQS Service Integration
#[derive(Debug)]
pub struct SQSService {
    client: aws_sdk_sqs::Client,
    config: aws_config::SdkConfig,
}

impl SQSService {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_sqs::Client::new(&config);
        Ok(Self { client, config })
    }

    /// Send message to SQS queue
    pub async fn send_message(&self, queue_url: &str, message_body: &str, attributes: Option<HashMap<String, String>>) -> Result<String, CloudError> {
        let mut request = self.client.send_message()
            .queue_url(queue_url)
            .message_body(message_body);

        if let Some(attrs) = attributes {
            for (key, value) in attrs {
                request = request.message_attributes(
                    &key,
                    aws_sdk_sqs::types::MessageAttributeValue::builder()
                        .data_type("String")
                        .string_value(value)
                        .build()
                );
            }
        }

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to send message: {}", e)))?;

        Ok(response.message_id().unwrap_or("").to_string())
    }

    /// Receive messages from SQS queue
    pub async fn receive_messages(&self, queue_url: &str, max_messages: Option<i32>) -> Result<Vec<aws_sdk_sqs::types::Message>, CloudError> {
        let response = self.client.receive_message()
            .queue_url(queue_url)
            .max_number_of_messages(max_messages.unwrap_or(1))
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to receive messages: {}", e)))?;

        Ok(response.messages().to_vec())
    }

    /// Delete message from SQS queue
    pub async fn delete_message(&self, queue_url: &str, receipt_handle: &str) -> Result<(), CloudError> {
        self.client.delete_message()
            .queue_url(queue_url)
            .receipt_handle(receipt_handle)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete message: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl CloudService for SQSService {
    fn service_name(&self) -> &str {
        "sqs"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        // Try to list queues
        self.client.list_queues()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("SQS health check failed: {}", e)))?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let response = self.client.list_queues()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to list queues: {}", e)))?;

        let resources = response.queue_urls()
            .iter()
            .filter(|_| {
                if let Some(rt) = resource_type {
                    rt == "queue"
                } else {
                    true
                }
            })
            .map(|queue_url| {
                let queue_name = queue_url.split('/').last().unwrap_or(queue_url);
                CloudResource {
                    id: queue_url.to_string(),
                    name: queue_name.to_string(),
                    resource_type: "queue".to_string(),
                    provider: CloudProvider::AWS,
                    region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                    arn: Some(format!("arn:aws:sqs:{}:*", self.config.region().unwrap_or_default())),
                    tags: HashMap::new(),
                    status: ResourceStatus::Available,
                    created_at: None,
                    metadata: HashMap::new(),
                }
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        // For SQS, resource_id should be the queue URL
        // TODO: Implement queue attributes retrieval
        Ok(Some(CloudResource {
            id: resource_id.to_string(),
            name: resource_id.split('/').last().unwrap_or(resource_id).to_string(),
            resource_type: "queue".to_string(),
            provider: CloudProvider::AWS,
            region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
            arn: Some(format!("arn:aws:sqs:{}:*", self.config.region().unwrap_or_default())),
            tags: HashMap::new(),
            status: ResourceStatus::Available,
            created_at: None,
            metadata: HashMap::new(),
        }))
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        let queue_name = config.get("queue_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CloudError::InvalidRequest("queue_name is required".to_string()))?;

        let response = self.client.create_queue()
            .queue_name(queue_name)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to create queue: {}", e)))?;

        Ok(response.queue_url().unwrap_or("").to_string())
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // TODO: Implement queue attribute updates
        Err(CloudError::InvalidRequest("SQS queue update not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        self.client.delete_queue()
            .queue_url(resource_id)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete queue: {}", e)))?;

        Ok(())
    }
}

/// AWS SNS Service Integration
#[derive(Debug)]
pub struct SNSService {
    client: aws_sdk_sns::Client,
    config: aws_config::SdkConfig,
}

impl SNSService {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_sns::Client::new(&config);
        Ok(Self { client, config })
    }

    /// Publish message to SNS topic
    pub async fn publish_message(&self, topic_arn: &str, message: &str, subject: Option<&str>) -> Result<String, CloudError> {
        let mut request = self.client.publish()
            .topic_arn(topic_arn)
            .message(message);

        if let Some(subject) = subject {
            request = request.subject(subject);
        }

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to publish message: {}", e)))?;

        Ok(response.message_id().unwrap_or("").to_string())
    }

    /// Subscribe to SNS topic
    pub async fn subscribe(&self, topic_arn: &str, protocol: &str, endpoint: &str) -> Result<String, CloudError> {
        let response = self.client.subscribe()
            .topic_arn(topic_arn)
            .protocol(protocol)
            .endpoint(endpoint)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to subscribe: {}", e)))?;

        Ok(response.subscription_arn().unwrap_or("").to_string())
    }
}

#[async_trait]
impl CloudService for SNSService {
    fn service_name(&self) -> &str {
        "sns"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        // Try to list topics
        self.client.list_topics()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("SNS health check failed: {}", e)))?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let response = self.client.list_topics()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to list topics: {}", e)))?;

        let resources = response.topics()
            .iter()
            .filter(|_| {
                if let Some(rt) = resource_type {
                    rt == "topic"
                } else {
                    true
                }
            })
            .filter_map(|topic| {
                let topic_arn = topic.topic_arn()?;
                let topic_name = topic_arn.split(':').last().unwrap_or(topic_arn);
                Some(CloudResource {
                    id: topic_arn.to_string(),
                    name: topic_name.to_string(),
                    resource_type: "topic".to_string(),
                    provider: CloudProvider::AWS,
                    region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                    arn: Some(topic_arn.to_string()),
                    tags: HashMap::new(),
                    status: ResourceStatus::Available,
                    created_at: None,
                    metadata: HashMap::new(),
                })
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        // For SNS, resource_id should be the topic ARN
        let topic_name = resource_id.split(':').last().unwrap_or(resource_id);
        Ok(Some(CloudResource {
            id: resource_id.to_string(),
            name: topic_name.to_string(),
            resource_type: "topic".to_string(),
            provider: CloudProvider::AWS,
            region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
            arn: Some(resource_id.to_string()),
            tags: HashMap::new(),
            status: ResourceStatus::Available,
            created_at: None,
            metadata: HashMap::new(),
        }))
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        let name = config.get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| CloudError::InvalidRequest("name is required".to_string()))?;

        let response = self.client.create_topic()
            .name(name)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to create topic: {}", e)))?;

        Ok(response.topic_arn().unwrap_or("").to_string())
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // TODO: Implement topic attribute updates
        Err(CloudError::InvalidRequest("SNS topic update not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        self.client.delete_topic()
            .topic_arn(resource_id)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete topic: {}", e)))?;

        Ok(())
    }
}

/// AWS DynamoDB Service Integration
#[derive(Debug)]
pub struct DynamoDBService {
    client: aws_sdk_dynamodb::Client,
    config: aws_config::SdkConfig,
}

impl DynamoDBService {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_dynamodb::Client::new(&config);
        Ok(Self { client, config })
    }

    /// Put item in DynamoDB table
    pub async fn put_item(&self, table_name: &str, item: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> Result<(), CloudError> {
        self.client.put_item()
            .table_name(table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to put item: {}", e)))?;

        Ok(())
    }

    /// Get item from DynamoDB table
    pub async fn get_item(&self, table_name: &str, key: HashMap<String, aws_sdk_dynamodb::types::AttributeValue>) -> Result<Option<HashMap<String, aws_sdk_dynamodb::types::AttributeValue>>, CloudError> {
        let response = self.client.get_item()
            .table_name(table_name)
            .set_key(Some(key))
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to get item: {}", e)))?;

        Ok(response.item)
    }

    /// Query DynamoDB table
    pub async fn query(&self, table_name: &str, key_condition_expression: &str, expression_attribute_values: Option<HashMap<String, aws_sdk_dynamodb::types::AttributeValue>>) -> Result<Vec<HashMap<String, aws_sdk_dynamodb::types::AttributeValue>>, CloudError> {
        let mut request = self.client.query()
            .table_name(table_name)
            .key_condition_expression(key_condition_expression);

        if let Some(values) = expression_attribute_values {
            request = request.set_expression_attribute_values(Some(values));
        }

        let response = request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to query: {}", e)))?;

        Ok(response.items().to_vec())
    }
}

#[async_trait]
impl CloudService for DynamoDBService {
    fn service_name(&self) -> &str {
        "dynamodb"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        // Try to list tables
        self.client.list_tables()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("DynamoDB health check failed: {}", e)))?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        let response = self.client.list_tables()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to list tables: {}", e)))?;

        let resources = response.table_names()
            .iter()
            .filter(|_| {
                if let Some(rt) = resource_type {
                    rt == "table"
                } else {
                    true
                }
            })
            .map(|table_name| CloudResource {
                id: table_name.to_string(),
                name: table_name.to_string(),
                resource_type: "table".to_string(),
                provider: CloudProvider::AWS,
                region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                arn: Some(format!("arn:aws:dynamodb:{}:*", self.config.region().unwrap_or_default())),
                tags: HashMap::new(),
                status: ResourceStatus::Available,
                created_at: None,
                metadata: HashMap::new(),
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        let response = self.client.describe_table()
            .table_name(resource_id)
            .send()
            .await;

        match response {
            Ok(table_desc) => {
                if let Some(table) = table_desc.table {
                    Ok(Some(CloudResource {
                        id: resource_id.to_string(),
                        name: table.table_name().unwrap_or(resource_id).to_string(),
                        resource_type: "table".to_string(),
                        provider: CloudProvider::AWS,
                        region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                        arn: table.table_arn().map(|s| s.to_string()),
                        tags: HashMap::new(),
                        status: match table.table_status() {
                            Some(aws_sdk_dynamodb::types::TableStatus::Creating) => ResourceStatus::Creating,
                            Some(aws_sdk_dynamodb::types::TableStatus::Updating) => ResourceStatus::Updating,
                            Some(aws_sdk_dynamodb::types::TableStatus::Deleting) => ResourceStatus::Deleting,
                            Some(aws_sdk_dynamodb::types::TableStatus::Active) => ResourceStatus::Available,
                            _ => ResourceStatus::Available,
                        },
                        created_at: table.creation_date_time().map(|dt| dt.into()),
                        metadata: HashMap::new(),
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        // TODO: Implement table creation
        Err(CloudError::InvalidRequest("DynamoDB table creation not implemented".to_string()))
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // TODO: Implement table updates
        Err(CloudError::InvalidRequest("DynamoDB table update not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        self.client.delete_table()
            .table_name(resource_id)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to delete table: {}", e)))?;

        Ok(())
    }
}

/// AWS CloudWatch Service Integration
#[derive(Debug)]
pub struct CloudWatchService {
    client: aws_sdk_cloudwatch::Client,
    config: aws_config::SdkConfig,
}

impl CloudWatchService {
    pub async fn new(config: aws_config::SdkConfig) -> Result<Self, CloudError> {
        let client = aws_sdk_cloudwatch::Client::new(&config);
        Ok(Self { client, config })
    }

    /// Put metric data
    pub async fn put_metric_data(&self, namespace: &str, metric_name: &str, value: f64, dimensions: Option<Vec<aws_sdk_cloudwatch::types::Dimension>>) -> Result<(), CloudError> {
        let mut request = self.client.put_metric_data()
            .namespace(namespace)
            .metric_data(
                aws_sdk_cloudwatch::types::MetricDatum::builder()
                    .metric_name(metric_name)
                    .value(value)
                    .timestamp(chrono::Utc::now().into())
                    .set_dimensions(dimensions)
                    .unit(aws_sdk_cloudwatch::types::StandardUnit::Count)
                    .build()
            );

        request.send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to put metric data: {}", e)))?;

        Ok(())
    }

    /// Get metric statistics
    pub async fn get_metric_statistics(&self, namespace: &str, metric_name: &str, start_time: chrono::DateTime<chrono::Utc>, end_time: chrono::DateTime<chrono::Utc>) -> Result<Vec<aws_sdk_cloudwatch::types::Datapoint>, CloudError> {
        let response = self.client.get_metric_statistics()
            .namespace(namespace)
            .metric_name(metric_name)
            .start_time(start_time.into())
            .end_time(end_time.into())
            .period(300) // 5 minutes
            .statistics(aws_sdk_cloudwatch::types::Statistic::Average)
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to get metric statistics: {}", e)))?;

        Ok(response.datapoints().to_vec())
    }
}

#[async_trait]
impl CloudService for CloudWatchService {
    fn service_name(&self) -> &str {
        "cloudwatch"
    }

    fn provider(&self) -> CloudProvider {
        CloudProvider::AWS
    }

    async fn health_check(&self) -> Result<(), CloudError> {
        // Try to list metrics
        self.client.list_metrics()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("CloudWatch health check failed: {}", e)))?;
        Ok(())
    }

    async fn list_resources(&self, resource_type: Option<&str>) -> Result<Vec<CloudResource>, CloudError> {
        // CloudWatch doesn't have traditional resources, but we can list alarms
        let response = self.client.describe_alarms()
            .send()
            .await
            .map_err(|e| CloudError::AwsError(format!("Failed to describe alarms: {}", e)))?;

        let resources = response.metric_alarms()
            .iter()
            .filter(|_| {
                if let Some(rt) = resource_type {
                    rt == "alarm"
                } else {
                    true
                }
            })
            .filter_map(|alarm| {
                let name = alarm.alarm_name()?;
                Some(CloudResource {
                    id: name.to_string(),
                    name: name.to_string(),
                    resource_type: "alarm".to_string(),
                    provider: CloudProvider::AWS,
                    region: self.config.region().map(|r| r.to_string()).unwrap_or_default(),
                    arn: alarm.alarm_arn().map(|s| s.to_string()),
                    tags: HashMap::new(),
                    status: ResourceStatus::Available,
                    created_at: None,
                    metadata: HashMap::new(),
                })
            })
            .collect();

        Ok(resources)
    }

    async fn get_resource(&self, resource_id: &str) -> Result<Option<CloudResource>, CloudError> {
        // TODO: Implement getting specific alarm
        Err(CloudError::InvalidRequest("CloudWatch resource retrieval not implemented".to_string()))
    }

    async fn create_resource(&self, config: HashMap<String, serde_json::Value>) -> Result<String, CloudError> {
        // TODO: Implement alarm creation
        Err(CloudError::InvalidRequest("CloudWatch alarm creation not implemented".to_string()))
    }

    async fn update_resource(&self, resource_id: &str, config: HashMap<String, serde_json::Value>) -> Result<(), CloudError> {
        // TODO: Implement alarm updates
        Err(CloudError::InvalidRequest("CloudWatch alarm update not implemented".to_string()))
    }

    async fn delete_resource(&self, resource_id: &str) -> Result<(), CloudError> {
        // TODO: Implement alarm deletion
        Err(CloudError::InvalidRequest("CloudWatch alarm deletion not implemented".to_string()))
    }
}

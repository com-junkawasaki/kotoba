//! External System Integrations - Phase 3
//!
//! 外部システムとの統合を提供します。
//! HTTP, データベース, メッセージング, クラウドサービスなどの統合。

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

/// 統合設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    pub name: String,
    pub integration_type: IntegrationType,
    pub config: HashMap<String, serde_json::Value>,
    pub timeout: Option<std::time::Duration>,
    pub retry_config: Option<RetryConfig>,
}

/// 統合種別
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntegrationType {
    Http,
    Database,
    MessageQueue,
    CloudStorage,
    Email,
    SMS,
    Webhook,
    GraphQL,
    REST,
}

/// リトライ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub initial_delay: std::time::Duration,
    pub max_delay: std::time::Duration,
    pub backoff_multiplier: f64,
}

/// 統合マネージャー
pub struct IntegrationManager {
    integrations: HashMap<String, Box<dyn Integration>>,
}

impl IntegrationManager {
    pub fn new() -> Self {
        Self {
            integrations: HashMap::new(),
        }
    }

    /// 統合を登録
    pub fn register_integration(&mut self, name: &str, integration: Box<dyn Integration>) {
        self.integrations.insert(name.to_string(), integration);
    }

    /// 統合を取得
    pub fn get_integration(&self, name: &str) -> Option<&Box<dyn Integration>> {
        self.integrations.get(name)
    }

    /// 統合を実行
    pub async fn execute_integration(
        &self,
        name: &str,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        if let Some(integration) = self.integrations.get(name) {
            integration.execute(operation, params).await
        } else {
            Err(IntegrationError::IntegrationNotFound(name.to_string()))
        }
    }
}

/// 統合インターフェース
#[async_trait]
pub trait Integration: Send + Sync {
    /// 統合を実行
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError>;

    /// 統合のヘルスチェック
    async fn health_check(&self) -> Result<(), IntegrationError>;

    /// 統合の種類を取得
    fn integration_type(&self) -> IntegrationType;
}

/// HTTP統合
#[cfg(feature = "activities-http")]
pub struct HttpIntegration {
    client: reqwest::Client,
    base_url: String,
    headers: HashMap<String, String>,
    timeout: std::time::Duration,
}

#[cfg(feature = "activities-http")]
impl HttpIntegration {
    pub fn new(base_url: &str, timeout: std::time::Duration) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
            headers: HashMap::new(),
            timeout,
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_bearer_token(mut self, token: &str) -> Self {
        self.headers.insert("Authorization".to_string(), format!("Bearer {}", token));
        self
    }
}

#[cfg(feature = "activities-http")]
#[async_trait]
impl Integration for HttpIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), operation.trim_start_matches('/'));

        let mut request = self.client.post(&url).timeout(self.timeout);

        // ヘッダーを設定
        for (key, value) in &self.headers {
            request = request.header(key, value);
        }

        // JSONボディを設定
        request = request.json(&params);

        let response = request.send().await
            .map_err(|e| IntegrationError::HttpError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(IntegrationError::HttpError(format!("HTTP {}: {}", response.status(), response.text().await.unwrap_or_default())));
        }

        let json_response = response.json().await
            .map_err(|e| IntegrationError::ParseError(e.to_string()))?;

        Ok(json_response)
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        let response = self.client.get(&self.base_url).timeout(std::time::Duration::from_secs(5)).send().await
            .map_err(|e| IntegrationError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(IntegrationError::HealthCheckFailed)
        }
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::Http
    }
}

/// データベース統合
#[cfg(feature = "activities-db")]
pub struct DatabaseIntegration {
    connection_string: String,
    pool: Option<sqlx::PgPool>, // PostgreSQLを例として使用
}

#[cfg(feature = "activities-db")]
impl DatabaseIntegration {
    pub fn new(connection_string: &str) -> Self {
        Self {
            connection_string: connection_string.to_string(),
            pool: None,
        }
    }

    /// データベース接続を初期化
    pub async fn initialize(&mut self) -> Result<(), IntegrationError> {
        self.pool = Some(sqlx::PgPool::connect(&self.connection_string).await
            .map_err(|e| IntegrationError::DatabaseError(e.to_string()))?);
        Ok(())
    }
}

#[cfg(feature = "activities-db")]
#[async_trait]
impl Integration for DatabaseIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        let pool = self.pool.as_ref().ok_or(IntegrationError::DatabaseError("Not initialized".to_string()))?;

        match operation {
            #[cfg(feature = "activities-db")]
            "query" => {
                let sql = params.get("sql").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let rows = sqlx::query(sql).fetch_all(pool).await
                    .map_err(|e| IntegrationError::DatabaseError(e.to_string()))?;

                let result: Vec<HashMap<String, serde_json::Value>> = rows.iter().map(|row| {
                    // TODO: 実際の行データをJSONに変換
                    HashMap::new()
                }).collect();

                Ok(serde_json::json!(result))
            }
            #[cfg(feature = "activities-db")]
            "execute" => {
                let sql = params.get("sql").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let result = sqlx::query(sql).execute(pool).await
                    .map_err(|e| IntegrationError::DatabaseError(e.to_string()))?;

                Ok(serde_json::json!({
                    "rows_affected": result.rows_affected()
                }))
            }
            _ => Err(IntegrationError::UnsupportedOperation(operation.to_string())),
        }
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        #[cfg(feature = "activities-db")]
        {
            let pool = self.pool.as_ref().ok_or(IntegrationError::DatabaseError("Not initialized".to_string()))?;

            sqlx::query("SELECT 1").fetch_one(pool).await
                .map_err(|e| IntegrationError::DatabaseError(e.to_string()))?;
        }

        Ok(())
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::Database
    }
}

/// メッセージキュー統合
#[cfg(feature = "activities-db")]
pub struct MessageQueueIntegration {
    broker_url: String,
    client: Option<lapin::Connection>,
}

#[cfg(feature = "activities-db")]
impl MessageQueueIntegration {
    pub fn new(broker_url: &str) -> Self {
        Self {
            broker_url: broker_url.to_string(),
            client: None,
        }
    }

    /// メッセージキュー接続を初期化
    pub async fn initialize(&mut self) -> Result<(), IntegrationError> {
        let connection = lapin::Connection::connect(&self.broker_url, lapin::ConnectionProperties::default()).await
            .map_err(|e| IntegrationError::MessageQueueError(e.to_string()))?;
        self.client = Some(connection);
        Ok(())
    }
}

#[cfg(feature = "activities-db")]
#[async_trait]
impl Integration for MessageQueueIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        let connection = self.client.as_ref().ok_or(IntegrationError::MessageQueueError("Not initialized".to_string()))?;
        let channel = connection.create_channel().await
            .map_err(|e| IntegrationError::MessageQueueError(e.to_string()))?;

        match operation {
            "publish" => {
                let queue = params.get("queue").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let message = params.get("message").ok_or(IntegrationError::InvalidParams)?;

                channel.queue_declare(queue, Default::default(), Default::default()).await
                    .map_err(|e| IntegrationError::MessageQueueError(e.to_string()))?;

                channel.basic_publish(
                    "",
                    queue,
                    Default::default(),
                    &serde_json::to_vec(message).unwrap_or_default(),
                    Default::default(),
                ).await
                    .map_err(|e| IntegrationError::MessageQueueError(e.to_string()))?;

                Ok(serde_json::json!({"status": "published"}))
            }
            "consume" => {
                let queue = params.get("queue").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;

                channel.queue_declare(queue, Default::default(), Default::default()).await
                    .map_err(|e| IntegrationError::MessageQueueError(e.to_string()))?;

                // TODO: メッセージ消費の実装
                Ok(serde_json::json!({"status": "consuming"}))
            }
            _ => Err(IntegrationError::UnsupportedOperation(operation.to_string())),
        }
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        let connection = self.client.as_ref().ok_or(IntegrationError::MessageQueueError("Not initialized".to_string()))?;

        if connection.status().connected() {
            Ok(())
        } else {
            Err(IntegrationError::HealthCheckFailed)
        }
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::MessageQueue
    }
}

/// クラウドストレージ統合
pub struct CloudStorageIntegration {
    provider: CloudProvider,
    bucket_name: String,
    credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudProvider {
    AWS,
    GCP,
    Azure,
}

impl CloudStorageIntegration {
    pub fn new(provider: CloudProvider, bucket_name: &str, credentials: HashMap<String, String>) -> Self {
        Self {
            provider,
            bucket_name: bucket_name.to_string(),
            credentials,
        }
    }
}

#[async_trait]
impl Integration for CloudStorageIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        match operation {
            "upload" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let data = params.get("data").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;

                // TODO: 実際のクラウドストレージアップロード実装
                println!("Uploading {} to {} in {}", key, self.bucket_name, format!("{:?}", self.provider));

                Ok(serde_json::json!({
                    "status": "uploaded",
                    "key": key,
                    "bucket": self.bucket_name
                }))
            }
            "download" => {
                let key = params.get("key").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;

                // TODO: 実際のクラウドストレージダウンロード実装
                println!("Downloading {} from {}", key, self.bucket_name);

                Ok(serde_json::json!({
                    "status": "downloaded",
                    "key": key,
                    "data": "downloaded_content"
                }))
            }
            _ => Err(IntegrationError::UnsupportedOperation(operation.to_string())),
        }
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        // TODO: 実際のヘルスチェック実装
        Ok(())
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::CloudStorage
    }
}

/// Email統合
pub struct EmailIntegration {
    smtp_server: String,
    smtp_port: u16,
    username: String,
    password: String,
    from_email: String,
}

impl EmailIntegration {
    pub fn new(smtp_server: &str, smtp_port: u16, username: &str, password: &str, from_email: &str) -> Self {
        Self {
            smtp_server: smtp_server.to_string(),
            smtp_port,
            username: username.to_string(),
            password: password.to_string(),
            from_email: from_email.to_string(),
        }
    }
}

#[async_trait]
impl Integration for EmailIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        match operation {
            "send" => {
                let to = params.get("to").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let subject = params.get("subject").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let body = params.get("body").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;

                // TODO: 実際のEmail送信実装
                println!("Sending email to {} with subject: {}", to, subject);

                Ok(serde_json::json!({
                    "status": "sent",
                    "to": to,
                    "subject": subject
                }))
            }
            _ => Err(IntegrationError::UnsupportedOperation(operation.to_string())),
        }
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        // TODO: SMTP接続テスト
        Ok(())
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::Email
    }
}

/// Webhook統合
#[cfg(feature = "activities-http")]
pub struct WebhookIntegration {
    client: reqwest::Client,
    timeout: std::time::Duration,
}

#[cfg(feature = "activities-http")]
impl WebhookIntegration {
    pub fn new(timeout: std::time::Duration) -> Self {
        Self {
            client: reqwest::Client::new(),
            timeout,
        }
    }
}

#[cfg(feature = "activities-http")]
#[async_trait]
impl Integration for WebhookIntegration {
    async fn execute(
        &self,
        operation: &str,
        params: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, IntegrationError> {
        match operation {
            "post" => {
                let url = params.get("url").and_then(|v| v.as_str()).ok_or(IntegrationError::InvalidParams)?;
                let payload = params.get("payload").ok_or(IntegrationError::InvalidParams)?;

                let response = self.client.post(url)
                    .timeout(self.timeout)
                    .json(payload)
                    .send().await
                    .map_err(|e| IntegrationError::HttpError(e.to_string()))?;

                if response.status().is_success() {
                    let result = response.json().await
                        .map_err(|e| IntegrationError::ParseError(e.to_string()))?;
                    Ok(result)
                } else {
                    Err(IntegrationError::HttpError(format!("Webhook failed with status: {}", response.status())))
                }
            }
            _ => Err(IntegrationError::UnsupportedOperation(operation.to_string())),
        }
    }

    async fn health_check(&self) -> Result<(), IntegrationError> {
        Ok(())
    }

    fn integration_type(&self) -> IntegrationType {
        IntegrationType::Webhook
    }
}

#[derive(Debug, thiserror::Error)]
pub enum IntegrationError {
    #[error("Integration not found: {0}")]
    IntegrationNotFound(String),
    #[error("HTTP error: {0}")]
    HttpError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Message queue error: {0}")]
    MessageQueueError(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Invalid parameters")]
    InvalidParams,
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),
    #[error("Health check failed")]
    HealthCheckFailed,
}

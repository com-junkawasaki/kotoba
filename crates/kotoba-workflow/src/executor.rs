//! WorkflowExecutor - Temporalベースワークフロー実行器
//!
//! 拡張されたStrategyIRを解釈し、Temporal風のワークフロー実行を
//! 実現します。Activity実行、並列実行、Sagaパターンなどをサポート。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::timeout;

use kotoba_core::types::*;
use crate::ir::*;

/// Activity実行インターフェース
#[async_trait]
pub trait Activity: Send + Sync {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> std::result::Result<HashMap<String, serde_json::Value>, ActivityError>;
    fn name(&self) -> &str;
    fn timeout(&self) -> Option<Duration> { None }
    fn retry_policy(&self) -> Option<RetryPolicy> { None }
}

/// Activity実行エラー
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity not found: {0}")]
    NotFound(String),
    #[error("Activity execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Activity timeout")]
    Timeout,
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// リトライポリシー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub initial_interval: Duration,
    pub backoff_coefficient: f64,
    pub maximum_interval: Option<Duration>,
    pub maximum_attempts: u32,
    pub non_retryable_errors: Vec<String>,
}

/// Activity実行結果
#[derive(Debug, Clone)]
pub struct ActivityResult {
    pub activity_name: String,
    pub status: ActivityStatus,
    pub outputs: Option<HashMap<String, serde_json::Value>>,
    pub error: Option<String>,
    pub execution_time: Duration,
    pub attempt_count: u32,
}

/// Activity実行状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityStatus {
    Scheduled,
    Started,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

/// Activityレジストリ
pub struct ActivityRegistry {
    activities: tokio::sync::RwLock<HashMap<String, Arc<dyn Activity>>>,
}

impl ActivityRegistry {
    pub fn new() -> Self {
        Self {
            activities: tokio::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Activityを登録
    pub async fn register(&self, activity: Arc<dyn Activity>) {
        let mut activities = self.activities.write().await;
        activities.insert(activity.name().to_string(), activity);
    }

    /// Activityを取得
    pub async fn get(&self, name: &str) -> Option<Arc<dyn Activity>> {
        let activities = self.activities.read().await;
        activities.get(name).cloned()
    }

    /// Activityを実行
    pub async fn execute(
        &self,
        name: &str,
        inputs: HashMap<String, serde_json::Value>,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        let start_time = std::time::Instant::now();
        let activity = self.get(name).await
            .ok_or(ActivityError::NotFound(name.to_string()))?;

        let mut attempt_count = 0;
        let retry_policy = activity.retry_policy();

        // リトライロジック
        if let Some(retry_policy) = retry_policy {
            self.execute_with_retry(&*activity, inputs, retry_policy, start_time).await
        } else {
            self.execute_once(&*activity, inputs, start_time, 1).await
        }
    }

    async fn execute_with_retry(
        &self,
        activity: &dyn Activity,
        inputs: HashMap<String, serde_json::Value>,
        retry_policy: RetryPolicy,
        start_time: std::time::Instant,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        let mut attempt_count = 0;
        let mut current_interval = retry_policy.initial_interval;

        loop {
            attempt_count += 1;

            match self.execute_once(activity, inputs.clone(), start_time, attempt_count).await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    // リトライ不可エラーのチェック
                    if retry_policy.non_retryable_errors.iter().any(|err| e.to_string().contains(err)) {
                        return Err(e);
                    }

                    // 最大試行回数チェック
                    if attempt_count >= retry_policy.maximum_attempts {
                        return Err(e);
                    }

                    // リトライ待機
                    tokio::time::sleep(current_interval).await;

                    // インターバル更新
                    current_interval = std::cmp::min(
                        current_interval.mul_f64(retry_policy.backoff_coefficient),
                        retry_policy.maximum_interval.unwrap_or(Duration::from_secs(300)),
                    );
                }
            }
        }
    }

    async fn execute_once(
        &self,
        activity: &dyn Activity,
        inputs: HashMap<String, serde_json::Value>,
        start_time: std::time::Instant,
        attempt_count: u32,
    ) -> std::result::Result<ActivityResult, ActivityError> {
        // タイムアウト設定を考慮した実行
        let result = if let Some(timeout_duration) = activity.timeout() {
            match timeout(timeout_duration, activity.execute(inputs)).await {
                Ok(result) => result,
                Err(_) => return Err(ActivityError::Timeout),
            }
        } else {
            activity.execute(inputs).await
        };

        let execution_time = start_time.elapsed();

        match result {
            Ok(outputs) => Ok(ActivityResult {
                activity_name: activity.name().to_string(),
                status: ActivityStatus::Completed,
                outputs: Some(outputs),
                error: None,
                execution_time,
                attempt_count,
            }),
            Err(e) => Ok(ActivityResult {
                activity_name: activity.name().to_string(),
                status: ActivityStatus::Failed,
                outputs: None,
                error: Some(e.to_string()),
                execution_time,
                attempt_count,
            }),
        }
    }

    /// 登録されているActivity一覧を取得
    pub async fn list_activities(&self) -> Vec<String> {
        let activities = self.activities.read().await;
        activities.keys().cloned().collect()
    }
}

/// ワークフロー実行エラー
#[derive(Debug, thiserror::Error)]
pub enum WorkflowError {
    #[error("Workflow not found: {0}")]
    WorkflowNotFound(String),
    #[error("Activity execution failed: {0}")]
    ActivityFailed(#[from] ActivityError),
    #[error("Invalid strategy: {0}")]
    InvalidStrategy(String),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Compensation failed: {0}")]
    CompensationFailed(String),
    #[error("Graph operation failed: {0}")]
    GraphError(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

// TODO: Implement workflow execution engine
// For now, this module provides basic activity execution framework

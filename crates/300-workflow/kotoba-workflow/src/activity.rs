//! Activity System - ワークフローActivity実行フレームワーク
//!
//! TemporalのActivityに相当する実行可能タスクの定義と管理。

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;


/// Activity実行インターフェース
#[async_trait]
pub trait Activity: Send + Sync {
    /// Activityを実行
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError>;

    /// Activity名を取得
    fn name(&self) -> &str;

    /// タイムアウト設定を取得
    fn timeout(&self) -> Option<Duration> { None }

    /// リトライポリシーを取得
    fn retry_policy(&self) -> Option<RetryPolicy> { None }
}

/// Activity実行エラー
#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Activity execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Timeout exceeded")]
    Timeout,
    #[error("Activity not found: {0}")]
    NotFound(String),
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
    ) -> Result<ActivityResult, ActivityError> {
        let start_time = std::time::Instant::now();
        let activity = self.get(name).await
            .ok_or(ActivityError::NotFound(name.to_string()))?;

        let _attempt_count = 0;
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
    ) -> Result<ActivityResult, ActivityError> {
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
    ) -> Result<ActivityResult, ActivityError> {
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

/// 標準Activity実装

/// HTTP Activity - HTTPリクエストを実行
pub struct HttpActivity {
    name: String,
    url: String,
    method: String,
    headers: HashMap<String, String>,
    timeout: Option<Duration>,
}

impl HttpActivity {
    pub fn new(name: &str, url: &str, method: &str) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            headers: HashMap::new(),
            timeout: Some(Duration::from_secs(30)),
        }
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Self {
        self.headers.insert(key.to_string(), value.to_string());
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[async_trait]
impl Activity for HttpActivity {
    async fn execute(&self, _inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        // TODO: HTTPクライアント実装
        // 実際の実装ではreqwestなどを使用
        println!("Executing HTTP activity: {} {} -> {}", self.method, self.url, self.name);

        // ダミー実装
        let mut outputs = HashMap::new();
        outputs.insert("status".to_string(), serde_json::json!(200));
        outputs.insert("response".to_string(), serde_json::json!({"ok": true}));

        Ok(outputs)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// Database Activity - データベース操作を実行
pub struct DatabaseActivity {
    name: String,
    query: String,
    timeout: Option<Duration>,
}

impl DatabaseActivity {
    pub fn new(name: &str, query: &str) -> Self {
        Self {
            name: name.to_string(),
            query: query.to_string(),
            timeout: Some(Duration::from_secs(30)),
        }
    }
}

#[async_trait]
impl Activity for DatabaseActivity {
    async fn execute(&self, _inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        // TODO: データベースクライアント実装
        println!("Executing DB activity: {} -> {}", self.query, self.name);

        // ダミー実装
        let mut outputs = HashMap::new();
        outputs.insert("rows_affected".to_string(), serde_json::json!(1));
        outputs.insert("result".to_string(), serde_json::json!({"success": true}));

        Ok(outputs)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// Function Activity - Rust関数を実行
pub struct FunctionActivity {
    name: String,
    function: Arc<dyn Fn(HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> + Send + Sync>,
    timeout: Option<Duration>,
}

impl FunctionActivity {
    pub fn new<F>(name: &str, function: F) -> Self
    where
        F: Fn(HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> + Send + Sync + 'static,
    {
        Self {
            name: name.to_string(),
            function: Arc::new(function),
            timeout: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }
}

#[async_trait]
impl Activity for FunctionActivity {
    async fn execute(&self, inputs: HashMap<String, serde_json::Value>) -> Result<HashMap<String, serde_json::Value>, ActivityError> {
        // 同期関数を非同期で実行
        let function = Arc::clone(&self.function);
        let inputs_clone = inputs.clone();
        tokio::task::spawn_blocking(move || {
            function(inputs_clone)
        })
        .await
        .map_err(|e| ActivityError::ExecutionFailed(format!("Task join error: {}", e)))?
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn timeout(&self) -> Option<Duration> {
        self.timeout
    }
}

/// Activityビルダー
pub struct ActivityBuilder {
    name: String,
    activity_type: ActivityType,
}

#[derive(Debug)]
pub enum ActivityType {
    Http { url: String, method: String },
    Database { query: String },
    Function { function_name: String },
}

impl ActivityBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            activity_type: ActivityType::Http { url: String::new(), method: "GET".to_string() },
        }
    }

    pub fn http(mut self, url: &str, method: &str) -> Self {
        self.activity_type = ActivityType::Http {
            url: url.to_string(),
            method: method.to_string(),
        };
        self
    }

    pub fn database(mut self, query: &str) -> Self {
        self.activity_type = ActivityType::Database {
            query: query.to_string(),
        };
        self
    }

    pub fn function(mut self, function_name: &str) -> Self {
        self.activity_type = ActivityType::Function {
            function_name: function_name.to_string(),
        };
        self
    }

    pub fn build(self) -> Arc<dyn Activity> {
        match self.activity_type {
            ActivityType::Http { url, method } => Arc::new(HttpActivity::new(&self.name, &url, &method)),
            ActivityType::Database { query } => Arc::new(DatabaseActivity::new(&self.name, &query)),
            ActivityType::Function { function_name: _ } => {
                // TODO: 関数レジストリから取得
                panic!("Function activities not yet implemented")
            }
        }
    }
}

/// Activityユーティリティ関数
pub mod prelude {
    pub use super::{
        Activity, ActivityRegistry, ActivityResult, ActivityStatus, ActivityError,
        HttpActivity, DatabaseActivity, FunctionActivity, ActivityBuilder,
        RetryPolicy,
    };
}

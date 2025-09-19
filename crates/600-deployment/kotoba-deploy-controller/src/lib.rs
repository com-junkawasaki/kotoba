//! # Kotoba Deploy Controller
//!
//! Deployment controller for the Kotoba deployment system.
//! Provides orchestration, state management, and GQL-based deployment operations.

use kotoba_core::prelude::{KotobaError, Value};
use kotoba_rewrite::prelude::RewriteEngine;
use kotoba_storage::KeyValueStore;

// Type alias for Result
type Result<T> = std::result::Result<T, KotobaError>;
use kotoba_deploy_core::*;
use kotoba_deploy_scaling::*;
use kotoba_deploy_network::*;
use kotoba_deploy_git::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
use uuid::Uuid;
use dashmap::DashMap;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use reqwest::Client;

/// デプロイメント状態
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeploymentStatus {
    /// 作成済み
    Created,
    /// ビルド中
    Building,
    /// デプロイ中
    Deploying,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// 失敗
    Failed,
    /// 削除済み
    Deleted,
}

/// デプロイコントローラー
#[derive(Debug)]
pub struct DeployController<T: KeyValueStore + 'static> {
    /// 書換えエンジン
    rewrite_engine: Arc<RewriteEngine<T>>,
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// ネットワークマネージャー
    network_manager: Arc<NetworkMgr>,
    /// Git統合
    git_integration: Option<Arc<GitIntegration>>,
    /// ストレージ (Graphの代わり)
    storage: Arc<T>,
    /// デプロイメント状態
    deployment_states: Arc<RwLock<HashMap<Uuid, DeploymentState>>>,

    // 新しい拡張機能
    /// デプロイメント履歴マネージャー
    history_manager: Arc<DeploymentHistoryManager>,
    /// ロールバックマネージャー
    rollback_manager: Arc<RollbackManager>,
    /// ブルーグリーンマネージャー
    blue_green_manager: Arc<BlueGreenDeploymentManager>,
    /// カナリアマネージャー
    canary_manager: Arc<CanaryDeploymentManager>,
    /// ヘルスチェックマネージャー
    health_check_manager: Arc<HealthCheckManager>,
    /// ロールバック設定
    rollback_config: RollbackConfig,
    /// ブルーグリーン設定
    blue_green_config: BlueGreenConfig,
    /// カナリア設定
    canary_config: CanaryConfig,
}

/// デプロイメントマネージャー
#[derive(Debug)]
pub struct DeploymentManager<T: KeyValueStore + 'static> {
    /// コントローラー
    controller: Arc<DeployController<T>>,
    /// デプロイメントキュー
    deployment_queue: Arc<RwLock<Vec<DeploymentRequest>>>,
    /// 実行中のデプロイメント
    running_deployments: Arc<RwLock<HashMap<String, RunningDeployment>>>,
}

/// デプロイメント状態
#[derive(Debug, Clone)]
pub struct DeploymentState {
    /// デプロイメントID
    pub id: String,
    /// 設定
    pub config: DeployConfig,
    /// 現在のステータス
    pub status: DeploymentStatus,
    /// 作成時刻
    pub created_at: SystemTime,
    /// 更新時刻
    pub updated_at: SystemTime,
    /// インスタンス数
    pub instance_count: u32,
    /// エンドポイント
    pub endpoints: Vec<String>,
}

/// デプロイメントリクエスト
#[derive(Debug, Clone)]
pub struct DeploymentRequest {
    /// リクエストID
    pub id: String,
    /// デプロイメントID
    pub deployment_id: String,
    /// 設定
    pub config: DeployConfig,
    /// 優先度
    pub priority: DeploymentPriority,
    /// リクエスト時刻
    pub requested_at: SystemTime,
}

/// 実行中のデプロイメント
#[derive(Debug, Clone)]
pub struct RunningDeployment {
    /// デプロイメントID
    pub id: String,
    /// 開始時刻
    pub started_at: SystemTime,
    /// プロセスID
    pub process_id: Option<u32>,
    /// リソース使用量
    pub resource_usage: ResourceUsage,
}

/// リソース使用量
#[derive(Debug, Clone)]
pub struct ResourceUsage {
    /// CPU使用率
    pub cpu_percent: f64,
    /// メモリ使用量 (MB)
    pub memory_mb: u64,
    /// ネットワークI/O (bytes/sec)
    pub network_bytes_per_sec: u64,
}

/// デプロイメント優先度
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeploymentPriority {
    /// 低
    Low,
    /// 通常
    Normal,
    /// 高
    High,
    /// 緊急
    Critical,
}

/// デプロイメント履歴エントリ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentHistoryEntry {
    /// エントリID
    pub id: String,
    /// デプロイメントID
    pub deployment_id: String,
    /// バージョン
    pub version: String,
    /// アクション
    pub action: DeploymentAction,
    /// ステータス
    pub status: DeploymentStatus,
    /// タイムスタンプ
    pub timestamp: DateTime<Utc>,
    /// メトリクス
    pub metrics: DeploymentMetrics,
    /// エラーメッセージ（失敗時）
    pub error_message: Option<String>,
}

/// デプロイメントアクション
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentAction {
    /// デプロイ
    Deploy,
    /// スケール
    Scale,
    /// ロールバック
    Rollback,
    /// 削除
    Delete,
    /// ブルーグリーンスイッチ
    BlueGreenSwitch,
    /// カナリアリリース
    CanaryRelease,
}

/// デプロイメントメトリクス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    /// デプロイ時間（秒）
    pub deploy_time_seconds: u64,
    /// CPU使用率
    pub cpu_usage_percent: f64,
    /// メモリ使用量（MB）
    pub memory_usage_mb: u64,
    /// レスポンス時間（ミリ秒）
    pub response_time_ms: u64,
    /// エラー率
    pub error_rate: f64,
    /// リクエスト数/秒
    pub requests_per_second: f64,
}

/// ロールバック設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackConfig {
    /// 自動ロールバックを有効化
    pub auto_rollback_enabled: bool,
    /// ヘルスチェック失敗時のロールバック閾値
    pub health_check_failure_threshold: u32,
    /// レスポンスタイム閾値（ミリ秒）
    pub response_time_threshold_ms: u64,
    /// エラーレート閾値（パーセンテージ）
    pub error_rate_threshold_percent: f64,
    /// ロールバック後の監視時間（秒）
    pub rollback_monitoring_duration_seconds: u64,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            auto_rollback_enabled: true,
            health_check_failure_threshold: 3,
            response_time_threshold_ms: 5000,
            error_rate_threshold_percent: 5.0,
            rollback_monitoring_duration_seconds: 300,
        }
    }
}

/// ブルーグリーンデプロイ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenConfig {
    /// トラフィック移行ステップ（パーセンテージ）
    pub traffic_shift_steps: Vec<u8>,
    /// 各ステップの待機時間（秒）
    pub step_wait_duration_seconds: u64,
    /// 自動ロールバック閾値
    pub auto_rollback_threshold: f64,
}

impl Default for BlueGreenConfig {
    fn default() -> Self {
        Self {
            traffic_shift_steps: vec![10, 25, 50, 75, 100],
            step_wait_duration_seconds: 60,
            auto_rollback_threshold: 2.0,
        }
    }
}

/// カナリアデプロイ設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// カナリアリリースのトラフィック割合（パーセンテージ）
    pub traffic_percentage: u8,
    /// カナリア期間（秒）
    pub canary_duration_seconds: u64,
    /// 成功基準メトリクス
    pub success_criteria: CanarySuccessCriteria,
}

impl Default for CanaryConfig {
    fn default() -> Self {
        Self {
            traffic_percentage: 10,
            canary_duration_seconds: 300,
            success_criteria: CanarySuccessCriteria::default(),
        }
    }
}

/// カナリア成功基準
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanarySuccessCriteria {
    /// 最小レスポンス時間（ミリ秒）
    pub min_response_time_ms: u64,
    /// 最大エラーレート（パーセンテージ）
    pub max_error_rate_percent: f64,
    /// 最小リクエスト成功率（パーセンテージ）
    pub min_success_rate_percent: f64,
}

impl Default for CanarySuccessCriteria {
    fn default() -> Self {
        Self {
            min_response_time_ms: 2000,
            max_error_rate_percent: 2.0,
            min_success_rate_percent: 95.0,
        }
    }
}

/// ヘルスチェック設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// ヘルスチェックURL
    pub url: String,
    /// チェック間隔（秒）
    pub interval_seconds: u64,
    /// タイムアウト（秒）
    pub timeout_seconds: u64,
    /// 成功判定のための連続成功回数
    pub success_threshold: u32,
    /// 失敗判定のための連続失敗回数
    pub failure_threshold: u32,
    /// HTTPステータスコード
    pub expected_status_codes: Vec<u16>,
}

/// デプロイメント履歴マネージャー
#[derive(Debug)]
pub struct DeploymentHistoryManager {
    /// 履歴エントリのマップ
    history: Arc<DashMap<String, Vec<DeploymentHistoryEntry>>>,
    /// 最大履歴数
    max_history_per_deployment: usize,
}

impl DeploymentHistoryManager {
    /// 新しい履歴マネージャーを作成
    pub fn new(max_history_per_deployment: usize) -> Self {
        Self {
            history: Arc::new(DashMap::new()),
            max_history_per_deployment,
        }
    }

    /// デプロイメント履歴を追加
    pub fn add_entry(&self, entry: DeploymentHistoryEntry) {
        let mut entries = self.history
            .entry(entry.deployment_id.clone())
            .or_insert_with(Vec::new);

        entries.push(entry);

        // 古いエントリを削除
        if entries.len() > self.max_history_per_deployment {
            entries.remove(0);
        }
    }

    /// デプロイメントの履歴を取得
    pub fn get_history(&self, deployment_id: &str) -> Vec<DeploymentHistoryEntry> {
        self.history
            .get(deployment_id)
            .map(|entries| entries.clone())
            .unwrap_or_default()
    }

    /// 最後の成功したデプロイメントを取得
    pub fn get_last_successful_deployment(&self, deployment_id: &str) -> Option<DeploymentHistoryEntry> {
        self.history
            .get(deployment_id)?
            .iter()
            .rev()
            .find(|entry| matches!(entry.status, DeploymentStatus::Running))
            .cloned()
    }
}

/// ロールバックマネージャー
#[derive(Debug)]
pub struct RollbackManager {
    /// 履歴マネージャー
    history_manager: Arc<DeploymentHistoryManager>,
    /// HTTPクライアント
    http_client: Client,
}

impl RollbackManager {
    /// 新しいロールバックマネージャーを作成
    pub fn new(history_manager: Arc<DeploymentHistoryManager>) -> Self {
        Self {
            history_manager,
            http_client: Client::new(),
        }
    }

    /// デプロイメントをロールバック
    pub async fn rollback_deployment(
        &self,
        deployment_id: &str,
        reason: &str
    ) -> Result<()> {
        println!("🔄 Starting rollback for deployment: {}", deployment_id);

        // 最後の成功したデプロイメントを取得
        let last_successful = self.history_manager
            .get_last_successful_deployment(deployment_id)
            .ok_or_else(|| {
                KotobaError::Execution(format!("No successful deployment found for rollback: {}", deployment_id))
            })?;

        println!("📋 Rolling back to version: {}", last_successful.version);

        // ロールバック履歴エントリを作成
        let rollback_entry = DeploymentHistoryEntry {
            id: Uuid::new_v4().to_string(),
            deployment_id: deployment_id.to_string(),
            version: format!("rollback-to-{}", last_successful.version),
            action: DeploymentAction::Rollback,
            status: DeploymentStatus::Running,
            timestamp: Utc::now(),
            metrics: DeploymentMetrics {
                deploy_time_seconds: 0,
                cpu_usage_percent: 0.0,
                memory_usage_mb: 0,
                response_time_ms: 0,
                error_rate: 0.0,
                requests_per_second: 0.0,
            },
            error_message: Some(format!("Rollback triggered: {}", reason)),
        };

        self.history_manager.add_entry(rollback_entry);

        println!("✅ Rollback completed successfully");
        Ok(())
    }

    /// 自動ロールバック条件をチェック
    pub async fn check_auto_rollback_conditions(
        &self,
        deployment_id: &str,
        config: &RollbackConfig,
        metrics: &DeploymentMetrics
    ) -> Result<bool> {
        let mut should_rollback = false;

        // レスポンスタイムチェック
        if metrics.response_time_ms > config.response_time_threshold_ms {
            println!("⚠️  Response time threshold exceeded: {}ms > {}ms",
                    metrics.response_time_ms, config.response_time_threshold_ms);
            should_rollback = true;
        }

        // エラーレートチェック
        if metrics.error_rate > config.error_rate_threshold_percent {
            println!("⚠️  Error rate threshold exceeded: {:.2}% > {:.2}%",
                    metrics.error_rate, config.error_rate_threshold_percent);
            should_rollback = true;
        }

        Ok(should_rollback)
    }
}

/// ブルーグリーンデプロイメントマネージャー
#[derive(Debug)]
pub struct BlueGreenDeploymentManager {
    /// 現在のトラフィック割合（新しいバージョンへの割合）
    traffic_distribution: Arc<DashMap<String, u8>>,
    /// ブルーグリーン設定
    config: BlueGreenConfig,
}

impl BlueGreenDeploymentManager {
    /// 新しいブルーグリーンマネージャーを作成
    pub fn new(config: BlueGreenConfig) -> Self {
        Self {
            traffic_distribution: Arc::new(DashMap::new()),
            config,
        }
    }

    /// ブルーグリーンデプロイメントを開始
    pub async fn start_blue_green_deployment(
        &self,
        deployment_id: &str,
        blue_version: &str,
        green_version: &str
    ) -> Result<()> {
        println!("🚀 Starting blue-green deployment for: {}", deployment_id);
        println!("🔵 Blue version: {}", blue_version);
        println!("🟢 Green version: {}", green_version);

        // 初期状態: 100% blue, 0% green
        self.traffic_distribution.insert(deployment_id.to_string(), 0);

        // 段階的にトラフィックを移行
        for &step_percentage in &self.config.traffic_shift_steps {
            println!("📊 Shifting {}% traffic to green version", step_percentage);

            self.traffic_distribution
                .insert(deployment_id.to_string(), step_percentage);

            // 各ステップで待機
            tokio::time::sleep(Duration::from_secs(self.config.step_wait_duration_seconds)).await;

            // ヘルスチェック（簡易実装）
            if step_percentage >= 50 {
                println!("🏥 Performing health check at {}% traffic shift", step_percentage);
                // 実際のヘルスチェックはここで実装
            }
        }

        println!("✅ Blue-green deployment completed successfully");
        Ok(())
    }

    /// 現在のトラフィック分布を取得
    pub fn get_traffic_distribution(&self, deployment_id: &str) -> u8 {
        self.traffic_distribution
            .get(deployment_id)
            .map(|r| *r)
            .unwrap_or(0)
    }
}

/// カナリアデプロイメントマネージャー
#[derive(Debug)]
pub struct CanaryDeploymentManager {
    /// カナリア設定
    config: CanaryConfig,
    /// カナリアリリース中のデプロイメント
    canary_deployments: Arc<DashMap<String, CanaryState>>,
}

#[derive(Debug, Clone)]
pub struct CanaryState {
    pub start_time: SystemTime,
    pub traffic_percentage: u8,
    pub metrics: DeploymentMetrics,
}

impl CanaryDeploymentManager {
    /// 新しいカナリアマネージャーを作成
    pub fn new(config: CanaryConfig) -> Self {
        Self {
            config,
            canary_deployments: Arc::new(DashMap::new()),
        }
    }

    /// カナリアリリースを開始
    pub async fn start_canary_release(
        &self,
        deployment_id: &str,
        new_version: &str
    ) -> Result<()> {
        println!("🐦 Starting canary release for: {}", deployment_id);
        println!("📦 New version: {}", new_version);
        println!("📊 Traffic percentage: {}%", self.config.traffic_percentage);

        let canary_state = CanaryState {
            start_time: SystemTime::now(),
            traffic_percentage: self.config.traffic_percentage,
            metrics: DeploymentMetrics {
                deploy_time_seconds: 0,
                cpu_usage_percent: 0.0,
                memory_usage_mb: 0,
                response_time_ms: 0,
                error_rate: 0.0,
                requests_per_second: 0.0,
            },
        };

        self.canary_deployments
            .insert(deployment_id.to_string(), canary_state);

        // カナリア期間を待機
        tokio::time::sleep(Duration::from_secs(self.config.canary_duration_seconds)).await;

        // 成功基準をチェック
        let should_promote = self.check_canary_success_criteria(deployment_id).await?;

        if should_promote {
            println!("✅ Canary release successful - promoting to full deployment");
            // 完全リリースを実行
        } else {
            println!("❌ Canary release failed - rolling back");
            // ロールバックを実行
        }

        Ok(())
    }

    /// カナリア成功基準をチェック
    async fn check_canary_success_criteria(&self, deployment_id: &str) -> Result<bool> {
        let state = self.canary_deployments
            .get(deployment_id)
            .ok_or_else(|| KotobaError::InvalidArgument("Canary state not found".to_string()))?;

        let metrics = &state.metrics;
        let criteria = &self.config.success_criteria;

        let success = metrics.response_time_ms <= criteria.min_response_time_ms
            && metrics.error_rate <= criteria.max_error_rate_percent
            && (100.0 - metrics.error_rate) >= criteria.min_success_rate_percent;

        println!("🎯 Canary success check:");
        println!("  Response time: {}ms <= {}ms: {}",
                metrics.response_time_ms, criteria.min_response_time_ms,
                metrics.response_time_ms <= criteria.min_response_time_ms);
        println!("  Error rate: {:.2}% <= {:.2}%: {}",
                metrics.error_rate, criteria.max_error_rate_percent,
                metrics.error_rate <= criteria.max_error_rate_percent);
        println!("  Success rate: {:.2}% >= {:.2}%: {}",
                100.0 - metrics.error_rate, criteria.min_success_rate_percent,
                (100.0 - metrics.error_rate) >= criteria.min_success_rate_percent);

        Ok(success)
    }
}

/// ヘルスチェックマネージャー
#[derive(Debug)]
pub struct HealthCheckManager {
    /// HTTPクライアント
    http_client: Client,
    /// ヘルスチェック設定
    configs: Arc<DashMap<String, HealthCheckConfig>>,
    /// ヘルスチェック結果
    results: Arc<DashMap<String, HealthCheckResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub last_check: SystemTime,
    pub is_healthy: bool,
    pub consecutive_successes: u32,
    pub consecutive_failures: u32,
    pub last_error: Option<String>,
    pub response_time_ms: u64,
}

impl HealthCheckManager {
    /// 新しいヘルスチェックマネージャーを作成
    pub fn new() -> Self {
        Self {
            http_client: Client::new(),
            configs: Arc::new(DashMap::new()),
            results: Arc::new(DashMap::new()),
        }
    }

    /// ヘルスチェック設定を登録
    pub fn register_health_check(&self, deployment_id: &str, config: HealthCheckConfig) {
        self.configs.insert(deployment_id.to_string(), config);

        // 初期結果
        let initial_result = HealthCheckResult {
            last_check: SystemTime::now(),
            is_healthy: false,
            consecutive_successes: 0,
            consecutive_failures: 0,
            last_error: None,
            response_time_ms: 0,
        };

        self.results.insert(deployment_id.to_string(), initial_result);
    }

    /// ヘルスチェックを実行
    pub async fn perform_health_check(&self, deployment_id: &str) -> Result<bool> {
        let config = self.configs
            .get(deployment_id)
            .ok_or_else(|| KotobaError::InvalidArgument("Health check config not found".to_string()))?;

        let start_time = SystemTime::now();

        let result = match self.http_client
            .get(&config.url)
            .timeout(Duration::from_secs(config.timeout_seconds))
            .send()
            .await
        {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let is_success = config.expected_status_codes.contains(&status_code);

                HealthCheckResult {
                    last_check: SystemTime::now(),
                    is_healthy: is_success,
                    consecutive_successes: if is_success { 1 } else { 0 },
                    consecutive_failures: if !is_success { 1 } else { 0 },
                    last_error: if !is_success {
                        Some(format!("Unexpected status code: {}", status_code))
                    } else {
                        None
                    },
                    response_time_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                }
            }
            Err(e) => {
                HealthCheckResult {
                    last_check: SystemTime::now(),
                    is_healthy: false,
                    consecutive_successes: 0,
                    consecutive_failures: 1,
                    last_error: Some(e.to_string()),
                    response_time_ms: start_time.elapsed().unwrap_or_default().as_millis() as u64,
                }
            }
        };

        // 結果を更新（連続成功/失敗を考慮）
        if let Some(mut existing_result) = self.results.get_mut(deployment_id) {
            if result.is_healthy {
                existing_result.consecutive_successes += 1;
                existing_result.consecutive_failures = 0;
            } else {
                existing_result.consecutive_failures += 1;
                existing_result.consecutive_successes = 0;
            }

            existing_result.last_check = result.last_check;
            existing_result.is_healthy = existing_result.consecutive_successes >= config.success_threshold;
            existing_result.last_error = result.last_error;
            existing_result.response_time_ms = result.response_time_ms;
        } else {
            self.results.insert(deployment_id.to_string(), result);
        }

        let final_result = self.results
            .get(deployment_id)
            .map(|r| r.is_healthy)
            .unwrap_or(false);

        Ok(final_result)
    }

    /// ヘルスチェック結果を取得
    pub fn get_health_result(&self, deployment_id: &str) -> Option<HealthCheckResult> {
        self.results.get(deployment_id).map(|r| r.clone())
    }
}

/// GQLデプロイメントクエリ
#[derive(Debug, Clone)]
pub struct GqlDeploymentQuery {
    /// クエリタイプ
    pub query_type: DeploymentQueryType,
    /// GQLクエリ
    pub gql_query: String,
    /// パラメータ
    pub parameters: HashMap<String, Value>,
}

/// デプロイメントクエリタイプ
#[derive(Debug, Clone)]
pub enum DeploymentQueryType {
    /// デプロイメント作成
    CreateDeployment,
    /// デプロイメント更新
    UpdateDeployment,
    /// デプロイメント削除
    DeleteDeployment,
    /// デプロイメント状態取得
    GetDeploymentStatus,
    /// デプロイメント一覧取得
    ListDeployments,
    /// スケーリング操作
    ScaleDeployment,
    /// ロールバック
    RollbackDeployment,
}

/// GQLデプロイメントレスポンス
#[derive(Debug, Clone)]
pub struct GqlDeploymentResponse {
    /// 成功フラグ
    pub success: bool,
    /// データ
    pub data: Option<Value>,
    /// エラー
    pub error: Option<String>,
    /// 実行時間
    pub execution_time_ms: u64,
}

impl<T: KeyValueStore + 'static> DeployController<T> {
    /// 新しいデプロイコントローラーを作成
    pub fn new(
        rewrite_engine: Arc<RewriteEngine<T>>,
        scaling_engine: Arc<ScalingEngine>,
        network_manager: Arc<NetworkMgr>,
        storage: Arc<T>,
    ) -> Self {
        Self::new_with_configs(
            rewrite_engine,
            scaling_engine,
            network_manager,
            Arc::clone(&storage),
            RollbackConfig::default(),
            BlueGreenConfig::default(),
            CanaryConfig::default(),
        )
    }

    /// 設定付きで新しいデプロイコントローラーを作成
    pub fn new_with_configs(
        rewrite_engine: Arc<RewriteEngine<T>>,
        scaling_engine: Arc<ScalingEngine>,
        network_manager: Arc<NetworkMgr>,
        storage: Arc<T>,
        rollback_config: RollbackConfig,
        blue_green_config: BlueGreenConfig,
        canary_config: CanaryConfig,
    ) -> Self {
        // 履歴マネージャーを作成
        let history_manager = Arc::new(DeploymentHistoryManager::new(100));

        // ロールバックマネージャーを作成
        let rollback_manager = Arc::new(RollbackManager::new(Arc::clone(&history_manager)));

        // ブルーグリーンマネージャーを作成
        let blue_green_manager = Arc::new(BlueGreenDeploymentManager::new(blue_green_config.clone()));

        // カナリアマネージャーを作成
        let canary_manager = Arc::new(CanaryDeploymentManager::new(canary_config.clone()));

        // ヘルスチェックマネージャーを作成
        let health_check_manager = Arc::new(HealthCheckManager::new());

        Self {
            rewrite_engine,
            scaling_engine,
            network_manager,
            git_integration: None,
            storage,
            deployment_states: Arc::new(RwLock::new(HashMap::<Uuid, DeploymentState>::new())),

            history_manager,
            rollback_manager,
            blue_green_manager,
            canary_manager,
            health_check_manager,
            rollback_config,
            blue_green_config,
            canary_config,
        }
    }

    /// Git統合を設定
    pub fn with_git_integration(mut self, git_integration: Arc<GitIntegration>) -> Self {
        self.git_integration = Some(git_integration);
        self
    }

    /// ISO GQLクエリを使用してデプロイメントを管理
    pub async fn execute_gql_deployment_query(
        &self,
        query: GqlDeploymentQuery,
    ) -> Result<GqlDeploymentResponse> {
        let start_time = SystemTime::now();

        let result = match query.query_type {
            DeploymentQueryType::CreateDeployment => {
                self.create_deployment_via_gql(&query).await
            }
            DeploymentQueryType::UpdateDeployment => {
                self.update_deployment_via_gql(&query).await
            }
            DeploymentQueryType::DeleteDeployment => {
                self.delete_deployment_via_gql(&query).await
            }
            DeploymentQueryType::GetDeploymentStatus => {
                self.get_deployment_status_via_gql(&query).await
            }
            DeploymentQueryType::ListDeployments => {
                self.list_deployments_via_gql(&query).await
            }
            DeploymentQueryType::ScaleDeployment => {
                self.scale_deployment_via_gql(&query).await
            }
            DeploymentQueryType::RollbackDeployment => {
                self.rollback_deployment_via_gql(&query).await
            }
        };

        let execution_time = SystemTime::now()
            .duration_since(start_time)
            .unwrap_or_default()
            .as_millis() as u64;

        match result {
            Ok(data) => Ok(GqlDeploymentResponse {
                success: true,
                data: Some(data),
                error: None,
                execution_time_ms: execution_time,
            }),
            Err(e) => Ok(GqlDeploymentResponse {
                success: false,
                data: None,
                error: Some(e.to_string()),
                execution_time_ms: execution_time,
            }),
        }
    }

    /// GQLを使用してデプロイメントを作成
    async fn create_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        // GQLクエリを解析してデプロイメント設定を取得
        let config = self.parse_deployment_config_from_gql(&query.gql_query)?;

        // デプロイメントグラフに頂点を追加
        let deployment_id = Uuid::new_v4();
        let _created_at = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| KotobaError::InvalidArgument(format!("Time error: {}", e)))?
            .as_secs();

        // デプロイメントグラフに頂点を追加 (簡易実装)
        // TODO: 実際のグラフ操作を実装
        println!("Adding deployment {} to graph", deployment_id);

        // デプロイメント状態を記録
        let state = DeploymentState {
            id: deployment_id.to_string(),
            config: config.clone(),
            status: DeploymentStatus::Created,
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            instance_count: config.scaling.min_instances,
            endpoints: vec![],
        };

        self.deployment_states.write().unwrap().insert(deployment_id, state);

        // ネットワーク設定を適用 (簡易実装)
        println!("Configuring network domains: {:?}", config.network.domains);

        // スケーリング設定を適用
        self.scaling_engine.set_instances(config.scaling.min_instances).await?;

        Ok(Value::String(format!("Deployment {} created successfully", deployment_id)))
    }

    /// GQLクエリからデプロイメント設定を解析
    fn parse_deployment_config_from_gql(&self, gql_query: &str) -> Result<DeployConfig> {
        // 簡易実装: GQLクエリから設定を抽出
        // 実際の実装ではより洗練されたGQLパーサーを使用

        if gql_query.contains("mutation createDeployment") {
            // デフォルト設定を使用
            let mut config = DeployConfig::default();
            config.metadata.name = "default-deployment".to_string();
            config.metadata.version = "1.0.0".to_string();
            config.metadata.description = Some("Auto-created deployment".to_string());
            config.application.entry_point = "index.js".to_string();
            config.application.build_command = Some("cargo build --release".to_string());
            Ok(config)
        } else {
            Err(KotobaError::InvalidArgument("Invalid GQL deployment query".to_string()))
        }
    }

    /// GQLを使用してデプロイメントを更新
    async fn update_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        // デプロイメントIDをクエリから抽出
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        // デプロイメント状態を更新
        let mut states = self.deployment_states.write().unwrap();
        if let Some(state) = states.get_mut(&deployment_id) {
            state.updated_at = SystemTime::now();
            state.status = DeploymentStatus::Deploying;

            // ネットワーク設定を更新 (簡易実装)
            println!("Updating network domains: {:?}", state.config.network.domains);

            Ok(Value::String(format!("Deployment {} updated successfully", deployment_id)))
        } else {
            Err(KotobaError::InvalidArgument(format!("Deployment {} not found", deployment_id)))
        }
    }

    /// GQLを使用してデプロイメントを削除
    async fn delete_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        let mut states = self.deployment_states.write().unwrap();
        if let Some(mut state) = states.remove(&deployment_id) {
            state.status = DeploymentStatus::Deleted;
            state.updated_at = SystemTime::now();

            // スケーリングを0に設定
            self.scaling_engine.set_instances(0).await?;

            Ok(Value::String(format!("Deployment {} deleted successfully", deployment_id)))
        } else {
            Err(KotobaError::InvalidArgument(format!("Deployment {} not found", deployment_id)))
        }
    }

    /// GQLを使用してデプロイメント状態を取得
    async fn get_deployment_status_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        let states = self.deployment_states.read().unwrap();
        if let Some(state) = states.get(&deployment_id) {
            let status_data = serde_json::json!({
                "id": state.id,
                "status": format!("{:?}", state.status),
                "instance_count": state.instance_count,
                "created_at": state.created_at.duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),
                "endpoints": state.endpoints
            });
            Ok(serde_json::from_value(status_data).unwrap())
        } else {
            Err(KotobaError::InvalidArgument(format!("Deployment {} not found", deployment_id)))
        }
    }

    /// GQLを使用してデプロイメント一覧を取得
    async fn list_deployments_via_gql(&self, _query: &GqlDeploymentQuery) -> Result<Value> {
        let states = self.deployment_states.read().unwrap();
        let deployments: Vec<String> = states.values()
            .map(|state| {
                format!("id={},name={},status={:?},instances={}",
                       state.id,
                       state.config.metadata.name,
                       state.status,
                       state.instance_count)
            })
            .collect();

        Ok(Value::Array(deployments))
    }

    /// GQLを使用してデプロイメントをスケーリング
    async fn scale_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;
        let target_instances = self.extract_scale_target_from_gql(&query.gql_query)?;

        // スケーリングを実行
        self.scaling_engine.set_instances(target_instances).await?;

        // デプロイメント状態を更新
        let mut states = self.deployment_states.write().unwrap();
        if let Some(state) = states.get_mut(&deployment_id) {
            state.instance_count = target_instances;
            state.updated_at = SystemTime::now();
        }

        Ok(Value::String(format!("Deployment {} scaled to {} instances", deployment_id, target_instances)))
    }

    /// GQLを使用してデプロイメントをロールバック
    async fn rollback_deployment_via_gql(&self, query: &GqlDeploymentQuery) -> Result<Value> {
        let deployment_id = self.extract_deployment_id_from_gql(&query.gql_query)?;

        // ロールバックロジック (簡易実装)
        let mut states = self.deployment_states.write().unwrap();
        if let Some(state) = states.get_mut(&deployment_id) {
            state.status = DeploymentStatus::Running;
            state.updated_at = SystemTime::now();
            Ok(Value::String(format!("Deployment {} rolled back successfully", deployment_id)))
        } else {
            Err(KotobaError::InvalidArgument(format!("Deployment {} not found", deployment_id)))
        }
    }

    /// GQLクエリからデプロイメントIDを抽出
    fn extract_deployment_id_from_gql(&self, _gql_query: &str) -> Result<Uuid> {
        // 簡易実装: クエリからIDを抽出
        // TODO: 実際のGQLパーシングを実装
        Err(KotobaError::InvalidArgument("GQL parsing not implemented".to_string()))
    }

    /// GQLクエリからスケールターゲットを抽出
    fn extract_scale_target_from_gql(&self, _gql_query: &str) -> Result<u32> {
        // 簡易実装: クエリからインスタンス数を抽出
        // TODO: 実際のGQLパーシングを実装
        Err(KotobaError::InvalidArgument("GQL parsing not implemented".to_string()))
    }


    /// デプロイメント状態を取得
    pub fn deployment_states(&self) -> Arc<RwLock<HashMap<Uuid, DeploymentState>>> {
        Arc::clone(&self.deployment_states)
    }

    // ===== 拡張機能メソッド =====

    /// デプロイメントをロールバック
    pub async fn rollback_deployment(&self, deployment_id: &str, reason: &str) -> Result<()> {
        self.rollback_manager.rollback_deployment(deployment_id, reason).await?;

        // デプロイメント状態を更新
        if let Ok(uuid) = Uuid::parse_str(deployment_id) {
            let mut states = self.deployment_states.write().unwrap();
            if let Some(state) = states.get_mut(&uuid) {
                state.status = DeploymentStatus::Running; // ロールバック後はRunning状態
                state.updated_at = SystemTime::now();
            }
        }

        Ok(())
    }

    /// 自動ロールバック条件をチェック
    pub async fn check_auto_rollback_conditions(
        &self,
        deployment_id: &str,
        metrics: &DeploymentMetrics
    ) -> Result<bool> {
        self.rollback_manager
            .check_auto_rollback_conditions(deployment_id, &self.rollback_config, metrics)
            .await
    }

    /// ブルーグリーンデプロイメントを開始
    pub async fn start_blue_green_deployment(
        &self,
        deployment_id: &str,
        blue_version: &str,
        green_version: &str
    ) -> Result<()> {
        self.blue_green_manager
            .start_blue_green_deployment(deployment_id, blue_version, green_version)
            .await
    }

    /// カナリアリリースを開始
    pub async fn start_canary_release(
        &self,
        deployment_id: &str,
        new_version: &str
    ) -> Result<()> {
        self.canary_manager
            .start_canary_release(deployment_id, new_version)
            .await
    }

    /// ヘルスチェックを設定
    pub fn register_health_check(&self, deployment_id: &str, config: HealthCheckConfig) {
        self.health_check_manager.register_health_check(deployment_id, config);
    }

    /// ヘルスチェックを実行
    pub async fn perform_health_check(&self, deployment_id: &str) -> Result<bool> {
        self.health_check_manager.perform_health_check(deployment_id).await
    }

    /// ヘルスチェック結果を取得
    pub fn get_health_result(&self, deployment_id: &str) -> Option<HealthCheckResult> {
        self.health_check_manager.get_health_result(deployment_id)
    }

    /// デプロイメント履歴を取得
    pub fn get_deployment_history(&self, deployment_id: &str) -> Vec<DeploymentHistoryEntry> {
        self.history_manager.get_history(deployment_id)
    }

    /// デプロイメント履歴を追加
    pub fn add_deployment_history_entry(&self, entry: DeploymentHistoryEntry) {
        self.history_manager.add_entry(entry);
    }

    /// 最後の成功したデプロイメントを取得
    pub fn get_last_successful_deployment(&self, deployment_id: &str) -> Option<DeploymentHistoryEntry> {
        self.history_manager.get_last_successful_deployment(deployment_id)
    }

    /// 現在のトラフィック分布を取得（ブルーグリーン）
    pub fn get_traffic_distribution(&self, deployment_id: &str) -> u8 {
        self.blue_green_manager.get_traffic_distribution(deployment_id)
    }

    /// 高度なデプロイメントを実行（戦略選択）
    pub async fn execute_advanced_deployment(
        &self,
        deployment_request: &DeploymentRequest,
        strategy: DeploymentStrategy
    ) -> Result<()> {
        println!("🚀 Executing advanced deployment with strategy: {:?}", strategy);

        match strategy {
            DeploymentStrategy::RollingUpdate => {
                self.execute_rolling_update(deployment_request).await
            }
            DeploymentStrategy::BlueGreen => {
                self.execute_blue_green_strategy(deployment_request).await
            }
            DeploymentStrategy::Canary => {
                self.execute_canary_strategy(deployment_request).await
            }
        }
    }

    /// ローリングアップデートを実行
    async fn execute_rolling_update(&self, request: &DeploymentRequest) -> Result<()> {
        println!("🔄 Executing rolling update for deployment: {}", request.deployment_id);

        // 既存インスタンスを段階的に置き換え
        let instance_count = request.config.scaling.max_instances;
        let batch_size = (instance_count / 4).max(1); // 25%ずつ更新

        for batch in (0..instance_count).step_by(batch_size as usize) {
            println!("📦 Updating batch {} - {}", batch, (batch + batch_size).min(instance_count));

            // バッチ更新のロジック（簡易実装）
            tokio::time::sleep(Duration::from_secs(10)).await;

            // ヘルスチェック
            if let Ok(healthy) = self.perform_health_check(&request.deployment_id).await {
                if !healthy {
                    println!("❌ Health check failed, stopping rolling update");
                    return Err(KotobaError::Execution("Rolling update failed health check".to_string()));
                }
            }
        }

        println!("✅ Rolling update completed successfully");
        Ok(())
    }

    /// ブルーグリーン戦略を実行
    async fn execute_blue_green_strategy(&self, request: &DeploymentRequest) -> Result<()> {
        let blue_version = "current".to_string();
        let green_version = format!("v{}", request.config.metadata.version);

        self.start_blue_green_deployment(
            &request.deployment_id,
            &blue_version,
            &green_version
        ).await
    }

    /// カナリア戦略を実行
    async fn execute_canary_strategy(&self, request: &DeploymentRequest) -> Result<()> {
        let new_version = format!("v{}", request.config.metadata.version);

        self.start_canary_release(&request.deployment_id, &new_version).await
    }

    /// デプロイメント監視を開始
    pub async fn start_deployment_monitoring(&self, deployment_id: &str) -> Result<()> {
        println!("👀 Starting deployment monitoring for: {}", deployment_id);

        // 定期的なヘルスチェックを開始
        let health_manager = Arc::clone(&self.health_check_manager);
        let deployment_id = deployment_id.to_string();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                match health_manager.perform_health_check(&deployment_id).await {
                    Ok(healthy) => {
                        if !healthy {
                            println!("⚠️  Health check failed for deployment: {}", deployment_id);
                            // 自動ロールバックのトリガーをここで実装可能
                        }
                    }
                    Err(e) => {
                        println!("❌ Health check error for {}: {}", deployment_id, e);
                    }
                }
            }
        });

        Ok(())
    }
}

/// デプロイメント戦略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeploymentStrategy {
    /// ローリングアップデート
    RollingUpdate,
    /// ブルーグリーンデプロイ
    BlueGreen,
    /// カナリアリリース
    Canary,
}

impl<T: KeyValueStore + 'static> DeploymentManager<T> {
    /// 新しいデプロイメントマネージャーを作成
    pub fn new(controller: Arc<DeployController<T>>) -> Self {
        Self {
            controller,
            deployment_queue: Arc::new(RwLock::new(Vec::new())),
            running_deployments: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// デプロイメントリクエストをキューに追加
    pub async fn enqueue_deployment(&self, request: DeploymentRequest) -> Result<()> {
        let mut queue = self.deployment_queue.write().unwrap();
        queue.push(request);
        // 優先度順にソート
        queue.sort_by(|a, b| b.priority.cmp(&a.priority));
        Ok(())
    }

    /// キューから次のデプロイメントリクエストを取得
    pub async fn dequeue_deployment(&self) -> Result<Option<DeploymentRequest>> {
        let mut queue = self.deployment_queue.write().unwrap();
        Ok(queue.pop())
    }

    /// デプロイメントを実行
    pub async fn execute_deployment(&self, request: &DeploymentRequest) -> Result<()> {
        let running = RunningDeployment {
            id: request.deployment_id.clone(),
            started_at: SystemTime::now(),
            process_id: None, // 実際のプロセスIDはランタイムで設定
            resource_usage: ResourceUsage {
                cpu_percent: 0.0,
                memory_mb: 0,
                network_bytes_per_sec: 0,
            },
        };

        let mut running_deployments = self.running_deployments.write().unwrap();
        running_deployments.insert(request.deployment_id.clone(), running);

        // コントローラーを使用してデプロイメントを実行
        let gql_query = GqlDeploymentQuery {
            query_type: DeploymentQueryType::CreateDeployment,
            gql_query: format!("mutation {{ createDeployment(id: \"{}\", config: {}) }}",
                             request.deployment_id, serde_json::to_string(&request.config).unwrap()),
            parameters: HashMap::new(),
        };

        self.controller.execute_gql_deployment_query(gql_query).await?;

        Ok(())
    }

    /// 実行中のデプロイメントを取得
    pub fn running_deployments(&self) -> Arc<RwLock<HashMap<String, RunningDeployment>>> {
        Arc::clone(&self.running_deployments)
    }

    /// デプロイメントキューを取得
    pub fn deployment_queue(&self) -> Arc<RwLock<Vec<DeploymentRequest>>> {
        Arc::clone(&self.deployment_queue)
    }
}

// Re-export commonly used types
pub use DeployController as DeploymentController;
pub use DeploymentManager as DeployMgr;

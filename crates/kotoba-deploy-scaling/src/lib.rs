//! # Kotoba Deploy Scaling
//!
//! Auto-scaling engine for the Kotoba deployment system.
//! Provides metrics collection, load balancing, and automatic scaling capabilities.

use kotoba_core::types::Result;
use kotoba_core::prelude::KotobaError;
use kotoba_deploy_core::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, Duration};
use tokio::time::interval;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use reqwest::Client;
use std::cmp::Ordering;

// Advanced analytics imports

/// スケーリングエンジン
#[derive(Debug)]
pub struct ScalingEngine {
    /// 設定
    config: ScalingConfig,
    /// 現在のインスタンス数
    current_instances: Arc<RwLock<u32>>,
    /// メトリクス収集器
    metrics_collector: MetricsCollector,
    /// スケーリング履歴
    scaling_history: Arc<RwLock<Vec<ScalingEvent>>>,
    /// 最後のスケーリング時刻
    last_scaling_time: Arc<RwLock<SystemTime>>,
}

/// メトリクス収集器
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    /// CPU使用率メトリクス
    cpu_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// メモリ使用率メトリクス
    memory_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// リクエスト数メトリクス
    request_metrics: Arc<RwLock<Vec<MetricPoint>>>,
    /// 応答時間メトリクス
    response_time_metrics: Arc<RwLock<Vec<MetricPoint>>>,
}

/// メトリクスポイント
#[derive(Debug, Clone)]
pub struct MetricPoint {
    /// タイムスタンプ
    pub timestamp: SystemTime,
    /// 値
    pub value: f64,
    /// インスタンスID
    pub instance_id: String,
}

/// スケーリングイベント
#[derive(Debug, Clone)]
pub struct ScalingEvent {
    /// イベント時刻
    pub timestamp: SystemTime,
    /// イベントタイプ
    pub event_type: ScalingEventType,
    /// 前のインスタンス数
    pub previous_instances: u32,
    /// 新しいインスタンス数
    pub new_instances: u32,
    /// トリガー理由
    pub reason: String,
}

/// スケーリングイベントタイプ
#[derive(Debug, Clone)]
pub enum ScalingEventType {
    /// スケールアップ
    ScaleUp,
    /// スケールダウン
    ScaleDown,
    /// 手動スケーリング
    Manual,
}

/// ロードバランサー
#[derive(Debug)]
pub struct LoadBalancer {
    /// インスタンスプール
    instance_pool: Arc<RwLock<HashMap<String, InstanceInfo>>>,
    /// 負荷分散アルゴリズム
    algorithm: LoadBalancingAlgorithm,
}

/// インスタンス情報
#[derive(Debug, Clone)]
pub struct InstanceInfo {
    /// インスタンスID
    pub id: String,
    /// ホスト名/IP
    pub address: String,
    /// ポート
    pub port: u16,
    /// 状態
    pub status: InstanceStatus,
    /// 最後のヘルスチェック時刻
    pub last_health_check: SystemTime,
    /// CPU使用率
    pub cpu_usage: f64,
    /// メモリ使用率
    pub memory_usage: f64,
    /// アクティブな接続数
    pub active_connections: u32,
}

/// インスタンス状態
#[derive(Debug, Clone, PartialEq)]
pub enum InstanceStatus {
    /// 起動中
    Starting,
    /// 実行中
    Running,
    /// 停止中
    Stopping,
    /// 停止済み
    Stopped,
    /// エラー
    Error,
}

/// 負荷分散アルゴリズム
#[derive(Debug, Clone)]
pub enum LoadBalancingAlgorithm {
    /// ラウンドロビン
    RoundRobin,
    /// 最小接続数
    LeastConnections,
    /// 最小応答時間
    LeastResponseTime,
    /// IPハッシュ
    IpHash,
}

/// オートスケーラー
#[derive(Debug)]
pub struct AutoScaler {
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// メトリクス収集間隔（秒）
    metrics_interval: u64,
    /// スケーリング評価間隔（秒）
    scaling_interval: u64,
    /// スケーリングタスクハンドル
    scaling_task: Option<tokio::task::JoinHandle<()>>,
}

/// 予測スケーリングエンジン
#[derive(Debug)]
pub struct PredictiveScaler {
    /// 履歴データストレージ
    historical_data: Arc<RwLock<Vec<TimeSeriesData>>>,
    /// 予測モデル
    prediction_model: Option<PredictionModel>,
    /// 予測期間（分）
    prediction_window_minutes: u32,
    /// 信頼区間
    confidence_interval: f64,
}

/// 時系列データ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    /// タイムスタンプ
    pub timestamp: DateTime<Utc>,
    /// CPU使用率
    pub cpu_usage: f64,
    /// メモリ使用率
    pub memory_usage: f64,
    /// リクエスト数
    pub request_count: f64,
    /// 応答時間
    pub response_time: f64,
    /// インスタンス数
    pub instance_count: u32,
}

/// 予測モデル
#[derive(Debug)]
pub enum PredictionModel {
    /// 線形回帰
    LinearRegression(LinearRegressionModel),
    /// 指数平滑法
    ExponentialSmoothing(ExponentialSmoothingModel),
    /// ARIMAモデル
    Arima(ArimaModel),
}

/// 線形回帰モデル
#[derive(Debug)]
pub struct LinearRegressionModel {
    /// 傾き
    slope: f64,
    /// 切片
    intercept: f64,
    /// R²値
    r_squared: f64,
}

/// 指数平滑法モデル
#[derive(Debug)]
pub struct ExponentialSmoothingModel {
    /// 平滑定数
    alpha: f64,
    /// 最新の予測値
    last_forecast: f64,
}

/// ARIMAモデル（簡易実装）
#[derive(Debug)]
pub struct ArimaModel {
    /// 自己回帰パラメータ
    ar_params: Vec<f64>,
    /// 移動平均パラメータ
    ma_params: Vec<f64>,
    /// 差分次数
    d: usize,
}

/// コスト最適化エンジン
#[derive(Debug)]
pub struct CostOptimizer {
    /// インスタンスタイプとコストのマッピング
    instance_costs: HashMap<String, InstanceCost>,
    /// 現在のリージョン
    region: String,
    /// コスト最適化設定
    optimization_config: CostOptimizationConfig,
    /// HTTPクライアント
    http_client: Client,
}

/// インスタンスコスト情報
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceCost {
    /// インスタンスタイプ
    pub instance_type: String,
    /// 時間あたりのコスト
    pub hourly_cost: f64,
    /// vCPU数
    pub vcpu_count: u32,
    /// メモリ（GB）
    pub memory_gb: f64,
    /// ネットワーク性能
    pub network_performance: String,
}

/// コスト最適化設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationConfig {
    /// コスト予算（時間あたり）
    pub max_hourly_budget: Option<f64>,
    /// 優先するメトリクス
    pub priority_metric: CostPriorityMetric,
    /// 最適化頻度（分）
    pub optimization_interval_minutes: u32,
    /// 自動最適化有効化
    pub auto_optimization_enabled: bool,
}

/// コスト優先メトリクス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CostPriorityMetric {
    /// コスト最小化
    CostMinimization,
    /// パフォーマンス最大化
    PerformanceMaximization,
    /// コストパフォーマンスバランス
    CostPerformanceBalance,
}

/// 高度なメトリクスアナライザー
#[derive(Debug)]
pub struct AdvancedMetricsAnalyzer {
    /// メトリクスストレージ
    metrics_storage: Arc<RwLock<HashMap<String, Vec<MetricPoint>>>>,
    /// 異常検知設定
    anomaly_config: AnomalyDetectionConfig,
    /// 統計分析器
    statistical_analyzer: StatisticalAnalyzer,
}

/// 異常検知設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionConfig {
    /// 標準偏差の閾値
    pub z_score_threshold: f64,
    /// 移動平均ウィンドウサイズ
    pub moving_average_window: usize,
    /// 最小データポイント数
    pub min_data_points: usize,
    /// 異常検知有効化
    pub enabled: bool,
}

/// 統計分析器
#[derive(Debug)]
pub struct StatisticalAnalyzer {
    /// 計算された統計値のキャッシュ
    statistics_cache: Arc<RwLock<HashMap<String, StatisticalSummary>>>,
}

/// 統計サマリー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatisticalSummary {
    /// 平均値
    pub mean: f64,
    /// 標準偏差
    pub std_dev: f64,
    /// 最小値
    pub min: f64,
    /// 最大値
    pub max: f64,
    /// 中央値
    pub median: f64,
    /// パーセンタイル
    pub percentiles: HashMap<String, f64>,
    /// データポイント数
    pub count: usize,
    /// 最終更新時刻
    pub last_updated: DateTime<Utc>,
}

/// 統合スケーリングマネージャー
#[derive(Debug)]
pub struct IntegratedScalingManager {
    /// 基本スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// 予測スケーリングエンジン
    predictive_scaler: PredictiveScaler,
    /// コスト最適化エンジン
    cost_optimizer: CostOptimizer,
    /// 高度なメトリクスアナライザー
    metrics_analyzer: AdvancedMetricsAnalyzer,
    /// オートスケーラー
    auto_scaler: AutoScaler,
    /// HTTPクライアント
    http_client: Client,
}

impl ScalingEngine {
    /// 新しいスケーリングエンジンを作成
    pub fn new(config: ScalingConfig) -> Self {
        let current_instances = Arc::new(RwLock::new(config.min_instances));

        Self {
            config,
            current_instances,
            metrics_collector: MetricsCollector::new(),
            scaling_history: Arc::new(RwLock::new(Vec::new())),
            last_scaling_time: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// 現在のインスタンス数を取得
    pub fn get_current_instances(&self) -> u32 {
        *self.current_instances.read().unwrap()
    }

    /// インスタンス数を設定
    pub async fn set_instances(&self, count: u32) -> Result<()> {
        let mut current = self.current_instances.write().unwrap();
        let previous = *current;

        // 範囲チェック
        let count = count.clamp(self.config.min_instances, self.config.max_instances);

        if count != previous {
            *current = count;

            // スケーリングイベントを記録
            let event = ScalingEvent {
                timestamp: SystemTime::now(),
                event_type: if count > previous { ScalingEventType::ScaleUp } else { ScalingEventType::ScaleDown },
                previous_instances: previous,
                new_instances: count,
                reason: "Manual scaling".to_string(),
            };

            let mut history = self.scaling_history.write().unwrap();
            history.push(event);

            // 最後のスケーリング時刻を更新
            let mut last_time = self.last_scaling_time.write().unwrap();
            *last_time = SystemTime::now();
        }

        Ok(())
    }

    /// スケールアップ
    pub async fn scale_up(&self) -> Result<()> {
        let current = self.get_current_instances();
        if current < self.config.max_instances {
            self.set_instances(current + 1).await?;
        }
        Ok(())
    }

    /// スケールダウン
    pub async fn scale_down(&self) -> Result<()> {
        let current = self.get_current_instances();
        if current > self.config.min_instances {
            self.set_instances(current - 1).await?;
        }
        Ok(())
    }

    /// メトリクスを収集
    pub async fn collect_metrics(&self, instance_id: &str, cpu: f64, memory: f64, requests: f64, response_time: f64) -> Result<()> {
        self.metrics_collector.add_cpu_metric(instance_id, cpu).await?;
        self.metrics_collector.add_memory_metric(instance_id, memory).await?;
        self.metrics_collector.add_request_metric(instance_id, requests).await?;
        self.metrics_collector.add_response_time_metric(instance_id, response_time).await?;
        Ok(())
    }

    /// スケーリングが必要かを判定
    pub async fn should_scale(&self) -> Result<Option<ScalingDecision>> {
        let current_instances = self.get_current_instances();

        // CPU使用率の平均を計算
        let avg_cpu = self.metrics_collector.get_average_cpu_usage().await?;
        let avg_memory = self.metrics_collector.get_average_memory_usage().await?;

        // スケールアップ条件
        if avg_cpu > self.config.cpu_threshold || avg_memory > self.config.memory_threshold {
            if current_instances < self.config.max_instances {
                return Ok(Some(ScalingDecision::ScaleUp));
            }
        }

        // スケールダウン条件
        if avg_cpu < self.config.cpu_threshold * 0.5 && avg_memory < self.config.memory_threshold * 0.5 {
            if current_instances > self.config.min_instances {
                return Ok(Some(ScalingDecision::ScaleDown));
            }
        }

        Ok(None)
    }

    /// 設定を取得
    pub fn config(&self) -> &ScalingConfig {
        &self.config
    }

    /// メトリクス収集器を取得
    pub fn metrics_collector(&self) -> &MetricsCollector {
        &self.metrics_collector
    }

    /// スケーリング履歴を取得
    pub fn scaling_history(&self) -> Arc<RwLock<Vec<ScalingEvent>>> {
        Arc::clone(&self.scaling_history)
    }
}

/// スケーリング判定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScalingDecision {
    /// スケールアップ
    ScaleUp,
    /// スケールダウン
    ScaleDown,
}

impl MetricsCollector {
    /// 新しいメトリクス収集器を作成
    pub fn new() -> Self {
        Self {
            cpu_metrics: Arc::new(RwLock::new(Vec::new())),
            memory_metrics: Arc::new(RwLock::new(Vec::new())),
            request_metrics: Arc::new(RwLock::new(Vec::new())),
            response_time_metrics: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// CPUメトリクスを追加
    pub async fn add_cpu_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.cpu_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除（保持期間: 1時間）
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// メモリメトリクスを追加
    pub async fn add_memory_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.memory_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// リクエストメトリクスを追加
    pub async fn add_request_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.request_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// 応答時間メトリクスを追加
    pub async fn add_response_time_metric(&self, instance_id: &str, value: f64) -> Result<()> {
        let metric = MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: instance_id.to_string(),
        };

        let mut metrics = self.response_time_metrics.write().unwrap();
        metrics.push(metric);

        // 古いメトリクスを削除
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// CPU使用率の平均を取得
    pub async fn get_average_cpu_usage(&self) -> Result<f64> {
        let metrics = self.cpu_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// メモリ使用率の平均を取得
    pub async fn get_average_memory_usage(&self) -> Result<f64> {
        let metrics = self.memory_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// リクエスト数の平均を取得
    pub async fn get_average_request_rate(&self) -> Result<f64> {
        let metrics = self.request_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }

    /// 応答時間の平均を取得
    pub async fn get_average_response_time(&self) -> Result<f64> {
        let metrics = self.response_time_metrics.read().unwrap();
        if metrics.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics.iter().map(|m| m.value).sum();
        Ok(sum / metrics.len() as f64)
    }
}

impl LoadBalancer {
    /// 新しいロードバランサーを作成
    pub fn new(algorithm: LoadBalancingAlgorithm) -> Self {
        Self {
            instance_pool: Arc::new(RwLock::new(HashMap::new())),
            algorithm,
        }
    }

    /// インスタンスを追加
    pub async fn add_instance(&self, instance: InstanceInfo) -> Result<()> {
        let mut pool = self.instance_pool.write().unwrap();
        pool.insert(instance.id.clone(), instance);
        Ok(())
    }

    /// インスタンスを削除
    pub async fn remove_instance(&self, instance_id: &str) -> Result<()> {
        let mut pool = self.instance_pool.write().unwrap();
        pool.remove(instance_id);
        Ok(())
    }

    /// 最適なインスタンスを選択
    pub async fn select_instance(&self, client_ip: Option<&str>) -> Result<Option<InstanceInfo>> {
        let pool = self.instance_pool.read().unwrap();

        // 実行中のインスタンスのみをフィルタリング
        let running_instances: Vec<_> = pool.values()
            .filter(|instance| instance.status == InstanceStatus::Running)
            .collect();

        if running_instances.is_empty() {
            return Ok(None);
        }

        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // シンプルなラウンドロビン実装
                // TODO: 実際のラウンドロビンカウンターを実装
                Ok(Some(running_instances[0].clone()))
            }
            LoadBalancingAlgorithm::LeastConnections => {
                // 最小接続数アルゴリズム
                let min_connections = running_instances.iter()
                    .min_by_key(|instance| instance.active_connections)
                    .unwrap();
                Ok(Some((*min_connections).clone()))
            }
            LoadBalancingAlgorithm::LeastResponseTime => {
                // 最小応答時間アルゴリズム
                // TODO: 応答時間メトリクスに基づく実装
                Ok(Some(running_instances[0].clone()))
            }
            LoadBalancingAlgorithm::IpHash => {
                if let Some(ip) = client_ip {
                    // IPベースのハッシュ
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};

                    let mut hasher = DefaultHasher::new();
                    ip.hash(&mut hasher);
                    let hash = hasher.finish() as usize;

                    let index = hash % running_instances.len();
                    Ok(Some(running_instances[index].clone()))
                } else {
                    Ok(Some(running_instances[0].clone()))
                }
            }
        }
    }

    /// インスタンスプールを取得
    pub fn instance_pool(&self) -> Arc<RwLock<HashMap<String, InstanceInfo>>> {
        Arc::clone(&self.instance_pool)
    }
}

impl PredictiveScaler {
    /// 新しい予測スケーリングエンジンを作成
    pub fn new(prediction_window_minutes: u32, confidence_interval: f64) -> Self {
        Self {
            historical_data: Arc::new(RwLock::new(Vec::new())),
            prediction_model: None,
            prediction_window_minutes,
            confidence_interval,
        }
    }

    /// 時系列データを追加
    pub async fn add_data_point(&self, data: TimeSeriesData) -> Result<()> {
        let mut historical = self.historical_data.write().unwrap();
        historical.push(data);

        // 古いデータを削除（7日分保持）
        let seven_days_ago = Utc::now() - chrono::Duration::days(7);
        historical.retain(|d| d.timestamp > seven_days_ago);

        Ok(())
    }

    /// 予測モデルをトレーニング
    pub async fn train_model(&mut self) -> Result<()> {
        let historical = self.historical_data.read().unwrap();

        if historical.len() < 10 {
            return Err(KotobaError::InvalidArgument("Insufficient data for training".to_string()));
        }

        // 線形回帰モデルを使用した簡易実装
        {
            // CPU使用率の予測モデル
            let cpu_data: Vec<f64> = historical.iter().map(|d| d.cpu_usage).collect();
            let time_indices: Vec<f64> = (0..cpu_data.len()).map(|i| i as f64).collect();

            if let Ok(model) = Self::simple_linear_regression(&time_indices, &cpu_data) {
                self.prediction_model = Some(PredictionModel::LinearRegression(model));
            }
        }

        Ok(())
    }

    /// CPU使用率を予測
    pub async fn predict_cpu_usage(&self, minutes_ahead: u32) -> Result<PredictionResult> {
        let historical = self.historical_data.read().unwrap();

        if historical.is_empty() {
            return Err(KotobaError::InvalidArgument("No historical data available".to_string()));
        }

        match &self.prediction_model {
            Some(PredictionModel::LinearRegression(model)) => {
                let next_index = historical.len() as f64 + (minutes_ahead as f64 / 60.0);
                let predicted_value = model.slope * next_index + model.intercept;

                // 信頼区間を計算
                let confidence_range = self.confidence_interval * model.r_squared.sqrt();

                Ok(PredictionResult {
                    predicted_value: predicted_value.max(0.0).min(100.0),
                    lower_bound: (predicted_value - confidence_range).max(0.0),
                    upper_bound: (predicted_value + confidence_range).min(100.0),
                    confidence_level: self.confidence_interval,
                    timestamp: Utc::now() + chrono::Duration::minutes(minutes_ahead as i64),
                })
            }
            _ => {
                // モデルのない場合は最新の値を返す
                let latest = historical.last().unwrap();
                Ok(PredictionResult {
                    predicted_value: latest.cpu_usage,
                    lower_bound: latest.cpu_usage * 0.9,
                    upper_bound: latest.cpu_usage * 1.1,
                    confidence_level: 0.8,
                    timestamp: Utc::now() + chrono::Duration::minutes(minutes_ahead as i64),
                })
            }
        }
    }

    /// 必要なインスタンス数を予測
    pub async fn predict_required_instances(&self, current_instances: u32, target_cpu_threshold: f64) -> Result<u32> {
        let prediction = self.predict_cpu_usage(30).await?; // 30分先を予測

        // 予測されるCPU使用率に基づいて必要なインスタンス数を計算
        let predicted_instances = if prediction.predicted_value > target_cpu_threshold {
            let scale_factor = prediction.predicted_value / target_cpu_threshold;
            (current_instances as f64 * scale_factor).ceil() as u32
        } else {
            current_instances
        };

        Ok(predicted_instances.max(1))
    }

    /// 簡易線形回帰
    fn simple_linear_regression(time_indices: &[f64], values: &[f64]) -> Result<LinearRegressionModel> {
        Self::train_linear_regression_simple(time_indices, values)
    }

    fn train_linear_regression_simple(time_indices: &[f64], values: &[f64]) -> Result<LinearRegressionModel> {
        let n = time_indices.len() as f64;
        let sum_x: f64 = time_indices.iter().sum();
        let sum_y: f64 = values.iter().sum();
        let sum_xy: f64 = time_indices.iter().zip(values.iter()).map(|(x, y)| x * y).sum();
        let sum_x2: f64 = time_indices.iter().map(|x| x * x).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let r_squared = Self::calculate_r_squared(time_indices, values, slope, intercept);

        Ok(LinearRegressionModel {
            slope,
            intercept,
            r_squared,
        })
    }

    fn calculate_r_squared(x: &[f64], y: &[f64], slope: f64, intercept: f64) -> f64 {
        let y_mean = y.iter().sum::<f64>() / y.len() as f64;

        let ss_res: f64 = x.iter().zip(y.iter())
            .map(|(xi, yi)| {
                let predicted = slope * xi + intercept;
                (yi - predicted).powi(2)
            })
            .sum();

        let ss_tot: f64 = y.iter()
            .map(|yi| (yi - y_mean).powi(2))
            .sum();

        if ss_tot == 0.0 {
            1.0
        } else {
            1.0 - (ss_res / ss_tot)
        }
    }
}

/// 予測結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionResult {
    /// 予測値
    pub predicted_value: f64,
    /// 下限
    pub lower_bound: f64,
    /// 上限
    pub upper_bound: f64,
    /// 信頼度
    pub confidence_level: f64,
    /// 予測時刻
    pub timestamp: DateTime<Utc>,
}

impl AutoScaler {
    /// 新しいオートスケーラーを作成
    pub fn new(scaling_engine: Arc<ScalingEngine>) -> Self {
        Self {
            scaling_engine,
            metrics_interval: 30,  // 30秒ごとにメトリクス収集
            scaling_interval: 60,  // 60秒ごとにスケーリング判定
            scaling_task: None,
        }
    }

    /// オートスケーリングを開始
    pub async fn start(&mut self) -> Result<()> {
        let scaling_engine = Arc::clone(&self.scaling_engine);
        let metrics_interval = self.metrics_interval;
        let scaling_interval = self.scaling_interval;

        // メトリクス収集タスク
        let metrics_task: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(metrics_interval));
            loop {
                interval.tick().await;

                // モックメトリクスの収集
                let cpu_usage = 45.0 + (rand::random::<f64>() - 0.5) * 20.0;
                let memory_usage = 60.0 + (rand::random::<f64>() - 0.5) * 30.0;
                let request_rate = 50.0 + (rand::random::<f64>() - 0.5) * 25.0;
                let response_time = 100.0 + (rand::random::<f64>() - 0.5) * 50.0;

                if let Err(e) = scaling_engine.collect_metrics("instance-1", cpu_usage, memory_usage, request_rate, response_time).await {
                    eprintln!("Failed to collect metrics: {}", e);
                }
            }
        });

        // スケーリング判定タスク
        let scaling_engine_clone = Arc::clone(&self.scaling_engine);
        let scaling_task: tokio::task::JoinHandle<Result<()>> = tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(scaling_interval));
            loop {
                interval.tick().await;

                match scaling_engine_clone.should_scale().await {
                    Ok(Some(ScalingDecision::ScaleUp)) => {
                        if let Err(e) = scaling_engine_clone.scale_up().await {
                            eprintln!("Failed to scale up: {}", e);
                        } else {
                            println!("Scaled up to {} instances", scaling_engine_clone.get_current_instances());
                        }
                    }
                    Ok(Some(ScalingDecision::ScaleDown)) => {
                        if let Err(e) = scaling_engine_clone.scale_down().await {
                            eprintln!("Failed to scale down: {}", e);
                        } else {
                            println!("Scaled down to {} instances", scaling_engine_clone.get_current_instances());
                        }
                    }
                    Ok(None) => {
                        // スケーリング不要
                    }
                    Err(e) => {
                        eprintln!("Failed to check scaling: {}", e);
                    }
                }
            }
        });

        // 両方のタスクが完了するまで待機
        let (metrics_result, scaling_result) = match tokio::try_join!(metrics_task, scaling_task) {
            Ok(results) => results,
            Err(e) => return Err(KotobaError::Execution(format!("Task join failed: {}", e))),
        };

        // エラーハンドリング
        metrics_result.map_err(|e| KotobaError::Execution(format!("Metrics task failed: {}", e)))?;
        scaling_result.map_err(|e| KotobaError::Execution(format!("Scaling task failed: {}", e)))?;

        Ok(())
    }

    /// オートスケーリングを停止
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(task) = self.scaling_task.take() {
            task.abort();
        }
        Ok(())
    }
}

impl CostOptimizer {
    /// 新しいコスト最適化エンジンを作成
    pub fn new(region: String, optimization_config: CostOptimizationConfig) -> Self {
        let mut instance_costs = HashMap::new();

        // デフォルトのインスタンスタイプとコストを追加
        Self::initialize_default_instance_costs(&mut instance_costs, &region);

        Self {
            instance_costs,
            region,
            optimization_config,
            http_client: Client::new(),
        }
    }

    /// デフォルトのインスタンスコストを初期化
    fn initialize_default_instance_costs(instance_costs: &mut HashMap<String, InstanceCost>, region: &str) {
        let base_costs = match region {
            "us-east-1" => vec![
                ("t3.micro", 0.0104, 1, 1.0, "Low"),
                ("t3.small", 0.0208, 1, 2.0, "Low"),
                ("t3.medium", 0.0416, 2, 4.0, "Low"),
                ("c5.large", 0.085, 2, 4.0, "Moderate"),
                ("c5.xlarge", 0.17, 4, 8.0, "High"),
                ("m5.large", 0.096, 2, 8.0, "Moderate"),
                ("m5.xlarge", 0.192, 4, 16.0, "High"),
            ],
            "eu-west-1" => vec![
                ("t3.micro", 0.0112, 1, 1.0, "Low"),
                ("t3.small", 0.0224, 1, 2.0, "Low"),
                ("t3.medium", 0.0448, 2, 4.0, "Low"),
                ("c5.large", 0.091, 2, 4.0, "Moderate"),
                ("c5.xlarge", 0.182, 4, 8.0, "High"),
                ("m5.large", 0.103, 2, 8.0, "Moderate"),
                ("m5.xlarge", 0.206, 4, 16.0, "High"),
            ],
            _ => vec![
                ("t3.micro", 0.0104, 1, 1.0, "Low"),
                ("t3.small", 0.0208, 1, 2.0, "Low"),
                ("t3.medium", 0.0416, 2, 4.0, "Low"),
                ("c5.large", 0.085, 2, 4.0, "Moderate"),
                ("c5.xlarge", 0.17, 4, 8.0, "High"),
                ("m5.large", 0.096, 2, 8.0, "Moderate"),
                ("m5.xlarge", 0.192, 4, 16.0, "High"),
            ],
        };

        for (instance_type, hourly_cost, vcpu, memory, network) in base_costs {
            instance_costs.insert(
                instance_type.to_string(),
                InstanceCost {
                    instance_type: instance_type.to_string(),
                    hourly_cost,
                    vcpu_count: vcpu,
                    memory_gb: memory,
                    network_performance: network.to_string(),
                },
            );
        }
    }

    /// 最適なインスタンスタイプを推奨
    pub async fn recommend_instance_type(&self, required_vcpu: u32, required_memory_gb: f64, workload_type: &WorkloadType) -> Result<InstanceRecommendation> {
        let candidates: Vec<_> = self.instance_costs.values()
            .filter(|cost| cost.vcpu_count >= required_vcpu && cost.memory_gb >= required_memory_gb)
            .collect();

        if candidates.is_empty() {
            return Err(KotobaError::Execution("No suitable instance types found".to_string()));
        }

        let mut recommendations = Vec::new();

        for candidate in candidates {
            let score = self.calculate_instance_score(candidate, &workload_type);
            let monthly_cost = candidate.hourly_cost * 24.0 * 30.0;

            recommendations.push(InstanceRecommendation {
                instance_type: candidate.instance_type.clone(),
                hourly_cost: candidate.hourly_cost,
                monthly_cost,
                vcpu_count: candidate.vcpu_count,
                memory_gb: candidate.memory_gb,
                network_performance: candidate.network_performance.clone(),
                suitability_score: score,
            });
        }

        // スコアでソート（高い順）
        recommendations.sort_by(|a, b| b.suitability_score.partial_cmp(&a.suitability_score).unwrap_or(Ordering::Equal));

        Ok(recommendations.into_iter().next().unwrap())
    }

    /// インスタンスの適合性を計算
    fn calculate_instance_score(&self, instance: &InstanceCost, workload_type: &WorkloadType) -> f64 {
        let mut score = 0.0;

        match workload_type {
            WorkloadType::CpuIntensive => {
                // CPU性能を重視
                score += (instance.vcpu_count as f64) * 2.0;
                score += instance.memory_gb * 0.5;
            }
            WorkloadType::MemoryIntensive => {
                // メモリ性能を重視
                score += instance.vcpu_count as f64 * 0.5;
                score += instance.memory_gb * 2.0;
            }
            WorkloadType::Balanced => {
                // バランスを重視
                score += instance.vcpu_count as f64 * 1.0;
                score += instance.memory_gb * 1.0;
            }
            WorkloadType::NetworkIntensive => {
                // ネットワーク性能を重視
                score += instance.vcpu_count as f64 * 1.0;
                score += instance.memory_gb * 1.0;
                if instance.network_performance == "High" {
                    score += 1.0;
                }
            }
        }

        // コストペナルティ（コストが高いほどスコアを下げる）
        let cost_penalty = instance.hourly_cost * 0.1;
        score -= cost_penalty;

        score.max(0.0)
    }

    /// 現在のデプロイメントのコストを計算
    pub fn calculate_deployment_cost(&self, instance_types: &[String], instance_counts: &[u32]) -> Result<DeploymentCost> {
        if instance_types.len() != instance_counts.len() {
            return Err(KotobaError::InvalidArgument("Instance types and counts arrays must have the same length".to_string()));
        }

        let mut total_hourly_cost = 0.0;
        let mut total_monthly_cost = 0.0;
        let mut instance_breakdown = Vec::new();

        for (instance_type, &count) in instance_types.iter().zip(instance_counts.iter()) {
            if let Some(cost_info) = self.instance_costs.get(instance_type) {
                let hourly_cost = cost_info.hourly_cost * count as f64;
                let monthly_cost = hourly_cost * 24.0 * 30.0;

                total_hourly_cost += hourly_cost;
                total_monthly_cost += monthly_cost;

                instance_breakdown.push(InstanceCostBreakdown {
                    instance_type: instance_type.clone(),
                    count,
                    hourly_cost_per_instance: cost_info.hourly_cost,
                    total_hourly_cost: hourly_cost,
                    total_monthly_cost: monthly_cost,
                });
            } else {
                return Err(KotobaError::InvalidArgument(format!("Unknown instance type: {}", instance_type)));
            }
        }

        Ok(DeploymentCost {
            total_hourly_cost,
            total_monthly_cost,
            instance_breakdown,
            timestamp: Utc::now(),
        })
    }

    /// コスト最適化の提案を生成
    pub async fn optimize_cost(&self, current_instance_types: &[String], current_counts: &[u32], workload_type: &WorkloadType) -> Result<CostOptimizationResult> {
        let current_cost = self.calculate_deployment_cost(current_instance_types, current_counts)?;

        // 各インスタンスタイプの最適化を検討
        let mut optimizations = Vec::new();

        for (i, instance_type) in current_instance_types.iter().enumerate() {
            if let Some(current_cost_info) = self.instance_costs.get(instance_type) {
                let required_vcpu = current_cost_info.vcpu_count;
                let required_memory = current_cost_info.memory_gb;

                // よりコスト効率の良い代替案を探す
                let recommendation = self.recommend_instance_type(required_vcpu, required_memory, &workload_type).await?;

                if recommendation.instance_type != *instance_type {
                    let current_hourly = current_cost_info.hourly_cost * current_counts[i] as f64;
                    let recommended_hourly = recommendation.hourly_cost * current_counts[i] as f64;
                    let savings = current_hourly - recommended_hourly;

                    if savings > 0.0 {
                        optimizations.push(CostOptimization {
                            instance_index: i,
                            current_instance_type: instance_type.clone(),
                            recommended_instance_type: recommendation.instance_type,
                            current_hourly_cost: current_hourly,
                            recommended_hourly_cost: recommended_hourly,
                            monthly_savings: savings * 24.0 * 30.0,
                            reason: format!("More cost-effective alternative with similar performance"),
                        });
                    }
                }
            }
        }

        let total_monthly_savings: f64 = optimizations.iter().map(|opt| opt.monthly_savings).sum();

        Ok(CostOptimizationResult {
            current_cost,
            optimizations,
            total_monthly_savings,
            total_yearly_savings: total_monthly_savings * 12.0,
        })
    }

    /// コスト予算に基づいてスケーリングを提案
    pub fn suggest_scaling_for_budget(&self, current_cost: &DeploymentCost, budget_limit: f64, workload_type: &WorkloadType) -> Result<BudgetScalingSuggestion> {
        if current_cost.total_hourly_cost <= budget_limit {
            return Ok(BudgetScalingSuggestion::NoChange);
        }

        let excess_cost = current_cost.total_hourly_cost - budget_limit;

        // 最もコスト効率の良いインスタンスタイプを探す
        let cheapest_suitable = self.instance_costs.values()
            .filter(|cost| {
                match workload_type {
                    WorkloadType::CpuIntensive => cost.vcpu_count >= 2,
                    WorkloadType::MemoryIntensive => cost.memory_gb >= 4.0,
                    _ => cost.vcpu_count >= 1,
                }
            })
            .min_by(|a, b| a.hourly_cost.partial_cmp(&b.hourly_cost).unwrap());

        if let Some(cheapest) = cheapest_suitable {
            let max_instances = (budget_limit / cheapest.hourly_cost) as u32;

            Ok(BudgetScalingSuggestion::ScaleDown {
                suggested_instance_type: cheapest.instance_type.clone(),
                max_instances,
                estimated_hourly_cost: cheapest.hourly_cost * max_instances as f64,
                cost_savings: excess_cost,
            })
        } else {
            Ok(BudgetScalingSuggestion::NoChange)
        }
    }
}

/// ワークロードタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkloadType {
    /// CPU集約型
    CpuIntensive,
    /// メモリ集約型
    MemoryIntensive,
    /// バランス型
    Balanced,
    /// ネットワーク集約型
    NetworkIntensive,
}

/// インスタンス推奨
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceRecommendation {
    /// インスタンスタイプ
    pub instance_type: String,
    /// 時間あたりのコスト
    pub hourly_cost: f64,
    /// 月あたりのコスト
    pub monthly_cost: f64,
    /// vCPU数
    pub vcpu_count: u32,
    /// メモリ（GB）
    pub memory_gb: f64,
    /// ネットワーク性能
    pub network_performance: String,
    /// 適合性スコア
    pub suitability_score: f64,
}

/// デプロイメントコスト
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentCost {
    /// 合計時間あたりのコスト
    pub total_hourly_cost: f64,
    /// 合計月あたりのコスト
    pub total_monthly_cost: f64,
    /// インスタンス別内訳
    pub instance_breakdown: Vec<InstanceCostBreakdown>,
    /// 計算時刻
    pub timestamp: DateTime<Utc>,
}

/// インスタンスコスト内訳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceCostBreakdown {
    /// インスタンスタイプ
    pub instance_type: String,
    /// 台数
    pub count: u32,
    /// インスタンスあたりの時間コスト
    pub hourly_cost_per_instance: f64,
    /// 合計時間コスト
    pub total_hourly_cost: f64,
    /// 合計月コスト
    pub total_monthly_cost: f64,
}

/// コスト最適化提案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimization {
    /// インスタンスインデックス
    pub instance_index: usize,
    /// 現在のインスタンスタイプ
    pub current_instance_type: String,
    /// 推奨インスタンスタイプ
    pub recommended_instance_type: String,
    /// 現在の時間コスト
    pub current_hourly_cost: f64,
    /// 推奨時間コスト
    pub recommended_hourly_cost: f64,
    /// 月間節約額
    pub monthly_savings: f64,
    /// 提案理由
    pub reason: String,
}

/// コスト最適化結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostOptimizationResult {
    /// 現在のコスト
    pub current_cost: DeploymentCost,
    /// 最適化提案
    pub optimizations: Vec<CostOptimization>,
    /// 合計月間節約額
    pub total_monthly_savings: f64,
    /// 合計年間節約額
    pub total_yearly_savings: f64,
}

/// 予算ベースのスケーリング提案
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetScalingSuggestion {
    /// 変更不要
    NoChange,
    /// スケールダウン提案
    ScaleDown {
        /// 推奨インスタンスタイプ
        suggested_instance_type: String,
        /// 最大インスタンス数
        max_instances: u32,
        /// 推定時間コスト
        estimated_hourly_cost: f64,
        /// コスト節約額
        cost_savings: f64,
    },
}

impl AdvancedMetricsAnalyzer {
    /// 新しい高度なメトリクスアナライザーを作成
    pub fn new(anomaly_config: AnomalyDetectionConfig) -> Self {
        Self {
            metrics_storage: Arc::new(RwLock::new(HashMap::new())),
            anomaly_config,
            statistical_analyzer: StatisticalAnalyzer::new(),
        }
    }

    /// メトリクスを追加
    pub async fn add_metric(&self, metric_name: &str, value: f64) -> Result<()> {
        let mut storage = self.metrics_storage.write().unwrap();

        let metrics = storage.entry(metric_name.to_string()).or_insert_with(Vec::new);
        metrics.push(MetricPoint {
            timestamp: SystemTime::now(),
            value,
            instance_id: "default".to_string(),
        });

        // 古いデータを削除（1時間分保持）
        let one_hour_ago = SystemTime::now() - Duration::from_secs(3600);
        metrics.retain(|m| m.timestamp > one_hour_ago);

        Ok(())
    }

    /// 異常検知を実行
    pub async fn detect_anomalies(&self, metric_name: &str) -> Result<Vec<Anomaly>> {
        let storage = self.metrics_storage.read().unwrap();

        if let Some(metrics) = storage.get(metric_name) {
            if metrics.len() < self.anomaly_config.min_data_points {
                return Ok(Vec::new());
            }

            let values: Vec<f64> = metrics.iter().map(|m| m.value).collect();
            let anomalies = self.detect_z_score_anomalies(&values)?;

            Ok(anomalies.into_iter().enumerate()
                .filter_map(|(i, is_anomaly)| {
                    if is_anomaly {
                        Some(Anomaly {
                            timestamp: metrics[i].timestamp,
                            metric_name: metric_name.to_string(),
                            value: values[i],
                            z_score: self.calculate_z_score(&values, values[i]),
                            severity: if values[i].abs() > self.anomaly_config.z_score_threshold * 2.0 {
                                AnomalySeverity::High
                            } else {
                                AnomalySeverity::Medium
                            },
                        })
                    } else {
                        None
                    }
                })
                .collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Zスコアベースの異常検知
    fn detect_z_score_anomalies(&self, values: &[f64]) -> Result<Vec<bool>> {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std_dev = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();

        if std_dev == 0.0 {
            return Ok(vec![false; values.len()]);
        }

        Ok(values.iter()
            .map(|v| (v - mean).abs() / std_dev > self.anomaly_config.z_score_threshold)
            .collect())
    }

    /// Zスコアを計算
    fn calculate_z_score(&self, values: &[f64], value: f64) -> f64 {
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let std_dev = (values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();

        if std_dev == 0.0 {
            0.0
        } else {
            (value - mean) / std_dev
        }
    }

    /// 統計サマリーを取得
    pub async fn get_statistical_summary(&self, metric_name: &str) -> Result<StatisticalSummary> {
        self.statistical_analyzer.get_summary(metric_name).await
    }
}

/// 異常
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// タイムスタンプ
    pub timestamp: SystemTime,
    /// メトリクス名
    pub metric_name: String,
    /// 値
    pub value: f64,
    /// Zスコア
    pub z_score: f64,
    /// 重要度
    pub severity: AnomalySeverity,
}

/// 異常の重要度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnomalySeverity {
    /// 低
    Low,
    /// 中
    Medium,
    /// 高
    High,
}

impl StatisticalAnalyzer {
    /// 新しい統計分析器を作成
    pub fn new() -> Self {
        Self {
            statistics_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 統計サマリーを取得
    pub async fn get_summary(&self, _metric_name: &str) -> Result<StatisticalSummary> {
        // 実際の実装ではメトリクスストレージからデータを取得して計算
        // ここではモック実装
        Ok(StatisticalSummary {
            mean: 50.0,
            std_dev: 10.0,
            min: 20.0,
            max: 80.0,
            median: 50.0,
            percentiles: HashMap::from([
                ("p25".to_string(), 40.0),
                ("p75".to_string(), 60.0),
                ("p95".to_string(), 75.0),
            ]),
            count: 100,
            last_updated: Utc::now(),
        })
    }
}

impl IntegratedScalingManager {
    /// 新しい統合スケーリングマネージャーを作成
    pub fn new(scaling_config: ScalingConfig) -> Self {
        let scaling_engine = Arc::new(ScalingEngine::new(scaling_config));
        let predictive_scaler = PredictiveScaler::new(60, 0.95); // 1時間先予測、95%信頼区間
        let cost_optimizer = CostOptimizer::new(
            "us-east-1".to_string(),
            CostOptimizationConfig {
                max_hourly_budget: None,
                priority_metric: CostPriorityMetric::CostPerformanceBalance,
                optimization_interval_minutes: 30,
                auto_optimization_enabled: true,
            },
        );
        let metrics_analyzer = AdvancedMetricsAnalyzer::new(AnomalyDetectionConfig {
            z_score_threshold: 3.0,
            moving_average_window: 10,
            min_data_points: 20,
            enabled: true,
        });
        let auto_scaler = AutoScaler::new(Arc::clone(&scaling_engine));

        Self {
            scaling_engine,
            predictive_scaler,
            cost_optimizer,
            metrics_analyzer,
            auto_scaler,
            http_client: Client::new(),
        }
    }

    /// 統合スケーリングを実行
    pub async fn perform_integrated_scaling(&self) -> Result<IntegratedScalingResult> {
        // 現在のメトリクスを取得
        let current_instances = self.scaling_engine.get_current_instances();

        // 予測スケーリング
        let prediction = self.predictive_scaler.predict_required_instances(
            current_instances,
            self.scaling_engine.config().cpu_threshold
        ).await?;

        // コスト最適化
        let current_cost = self.cost_optimizer.calculate_deployment_cost(
            &vec!["t3.medium".to_string(); current_instances as usize],
            &vec![1; current_instances as usize],
        )?;

        let workload_type = WorkloadType::Balanced; // デフォルトのワークロードタイプを使用

        // 異常検知
        let cpu_anomalies = self.metrics_analyzer.detect_anomalies("cpu_usage").await?;
        let memory_anomalies = self.metrics_analyzer.detect_anomalies("memory_usage").await?;

        // 統合判定
        let scaling_decision = self.make_integrated_decision(
            current_instances,
            prediction,
            &current_cost,
            &cpu_anomalies,
            &memory_anomalies,
            &workload_type,
        ).await?;

        Ok(IntegratedScalingResult {
            current_instances,
            predicted_instances: prediction,
            current_cost,
            scaling_decision,
            cpu_anomalies,
            memory_anomalies,
            timestamp: Utc::now(),
        })
    }

    /// 統合スケーリング判定
    async fn make_integrated_decision(
        &self,
        current_instances: u32,
        prediction: u32,
        current_cost: &DeploymentCost,
        cpu_anomalies: &[Anomaly],
        memory_anomalies: &[Anomaly],
        workload_type: &WorkloadType,
    ) -> Result<ScalingDecision> {
        // 異常がある場合は慎重にスケーリング
        let has_high_severity_anomalies = cpu_anomalies.iter().any(|a| matches!(a.severity, AnomalySeverity::High)) ||
                                         memory_anomalies.iter().any(|a| matches!(a.severity, AnomalySeverity::High));

        if has_high_severity_anomalies {
            return Ok(ScalingDecision::ScaleUp); // 異常時はスケールアップで対応
        }

        // 予測に基づいて判定
        if prediction > current_instances {
            // コストチェック
            let estimated_new_cost = current_cost.total_hourly_cost * (prediction as f64 / current_instances as f64);

            if let Some(budget) = self.cost_optimizer.optimization_config.max_hourly_budget {
                if estimated_new_cost > budget {
                    // 予算オーバーの場合はコスト最適化を試みる
                    let suggestion = self.cost_optimizer.suggest_scaling_for_budget(
                        current_cost,
                        budget,
                        workload_type,
                    )?;
                    match suggestion {
                        BudgetScalingSuggestion::ScaleDown { .. } => {
                            return Ok(ScalingDecision::ScaleDown);
                        }
                        BudgetScalingSuggestion::NoChange => {
                            // 予算内で何もできない場合はスケールアップしない
                            return Ok(ScalingDecision::ScaleDown);
                        }
                    }
                }
            }

            Ok(ScalingDecision::ScaleUp)
        } else if prediction < current_instances {
            Ok(ScalingDecision::ScaleDown)
        } else {
            Ok(ScalingDecision::ScaleDown) // 変更なしの場合はScaleDownとして扱う
        }
    }
}

/// 統合スケーリング結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegratedScalingResult {
    /// 現在のインスタンス数
    pub current_instances: u32,
    /// 予測されるインスタンス数
    pub predicted_instances: u32,
    /// 現在のコスト
    pub current_cost: DeploymentCost,
    /// スケーリング判定
    pub scaling_decision: ScalingDecision,
    /// CPU異常
    pub cpu_anomalies: Vec<Anomaly>,
    /// メモリ異常
    pub memory_anomalies: Vec<Anomaly>,
    /// タイムスタンプ
    pub timestamp: DateTime<Utc>,
}

// Re-export commonly used types
pub use ScalingEngine as ScalingSvc;
pub use MetricsCollector as Metrics;
pub use LoadBalancer as LoadBalancerSvc;
pub use AutoScaler as AutoScaling;
pub use PredictiveScaler as PredictiveScaling;
pub use CostOptimizer as CostOptimizationSvc;
pub use AdvancedMetricsAnalyzer as AdvancedMetrics;
pub use IntegratedScalingManager as IntegratedScaling;

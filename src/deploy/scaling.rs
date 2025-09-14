//! 自動スケーリングエンジン
//!
//! このモジュールはデプロイされたアプリケーションの自動スケーリングを管理します。
//! CPU使用率、メモリ使用率、リクエスト数などのメトリクスに基づいて
//! インスタンス数を動的に調整します。

use crate::types::{Result, Value, VertexId, EdgeId, GraphRef};
use crate::graph::{Graph, VertexData, EdgeData};
use crate::deploy::config::{ScalingConfig, ScalingPolicy};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::time::{interval, Duration};
use std::time::{SystemTime, UNIX_EPOCH};

/// スケーリングエンジン
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
pub struct AutoScaler {
    /// スケーリングエンジン
    scaling_engine: Arc<ScalingEngine>,
    /// ロードバランサー
    load_balancer: Arc<LoadBalancer>,
    /// 予測スケーリング有効化
    predictive_scaling_enabled: bool,
    /// 予測モデル
    predictive_model: Option<PredictiveModel>,
}

/// 予測モデル
pub struct PredictiveModel {
    /// 履歴データ
    historical_data: Vec<TimeSeriesData>,
    /// 予測アルゴリズム
    algorithm: PredictionAlgorithm,
}

/// 時系列データ
#[derive(Debug, Clone)]
pub struct TimeSeriesData {
    /// タイムスタンプ
    pub timestamp: SystemTime,
    /// CPU使用率
    pub cpu_usage: f64,
    /// リクエスト数
    pub request_count: f64,
    /// インスタンス数
    pub instance_count: u32,
}

/// 予測アルゴリズム
#[derive(Debug, Clone)]
pub enum PredictionAlgorithm {
    /// 線形回帰
    LinearRegression,
    /// 指数平滑
    ExponentialSmoothing,
    /// ARIMA
    Arima,
}

impl ScalingEngine {
    /// 新しいスケーリングエンジンを作成
    pub fn new(config: ScalingConfig) -> Self {
        Self {
            config,
            current_instances: Arc::new(RwLock::new(config.min_instances)),
            metrics_collector: MetricsCollector::new(),
            scaling_history: Arc::new(RwLock::new(Vec::new())),
            last_scaling_time: Arc::new(RwLock::new(SystemTime::now())),
        }
    }

    /// スケーリングエンジンを開始
    pub async fn start(&self) -> Result<()> {
        let metrics_collector = self.metrics_collector.clone();
        let scaling_history = self.scaling_history.clone();
        let current_instances = self.current_instances.clone();
        let last_scaling_time = self.last_scaling_time.clone();
        let config = self.config.clone();

        // メトリクス収集タスク
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // 30秒ごとに収集
            loop {
                interval.tick().await;
                if let Err(e) = metrics_collector.collect_metrics().await {
                    eprintln!("Failed to collect metrics: {}", e);
                }
            }
        });

        // スケーリング判定タスク
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // 1分ごとに判定
            loop {
                interval.tick().await;

                // クールダウン期間チェック
                let last_time = *last_scaling_time.read().unwrap();
                if last_time.elapsed().unwrap().as_secs() < config.cooldown_period as u64 {
                    continue;
                }

                // スケーリング判定
                if let Ok(Some(action)) = Self::determine_scaling_action(
                    &config,
                    &metrics_collector,
                    *current_instances.read().unwrap(),
                ).await {
                    match action {
                        ScalingAction::ScaleUp(reason) => {
                            if let Err(e) = Self::execute_scale_up(
                                &current_instances,
                                &last_scaling_time,
                                &scaling_history,
                                reason,
                            ).await {
                                eprintln!("Failed to scale up: {}", e);
                            }
                        }
                        ScalingAction::ScaleDown(reason) => {
                            if let Err(e) = Self::execute_scale_down(
                                &current_instances,
                                &last_scaling_time,
                                &scaling_history,
                                reason,
                            ).await {
                                eprintln!("Failed to scale down: {}", e);
                            }
                        }
                        ScalingAction::NoAction => {}
                    }
                }
            }
        });

        Ok(())
    }

    /// スケーリングアクションを判定
    async fn determine_scaling_action(
        config: &ScalingConfig,
        metrics: &MetricsCollector,
        current_instances: u32,
    ) -> Result<Option<ScalingAction>> {
        let cpu_avg = metrics.get_average_cpu_usage().await?;
        let memory_avg = metrics.get_average_memory_usage().await?;
        let request_rate = metrics.get_average_request_rate().await?;

        match config.policy {
            ScalingPolicy::CpuBased => {
                if cpu_avg > config.cpu_threshold && current_instances < config.max_instances {
                    return Ok(Some(ScalingAction::ScaleUp(
                        format!("CPU usage {:.1}% exceeds threshold {:.1}%", cpu_avg, config.cpu_threshold)
                    )));
                } else if cpu_avg < config.cpu_threshold * 0.5 && current_instances > config.min_instances {
                    return Ok(Some(ScalingAction::ScaleDown(
                        format!("CPU usage {:.1}% is below threshold {:.1}%", cpu_avg, config.cpu_threshold)
                    )));
                }
            }
            ScalingPolicy::RequestBased => {
                let threshold_requests_per_second = 100.0; // 設定可能にするべき
                if request_rate > threshold_requests_per_second && current_instances < config.max_instances {
                    return Ok(Some(ScalingAction::ScaleUp(
                        format!("Request rate {:.1} req/s exceeds threshold", request_rate)
                    )));
                }
            }
            ScalingPolicy::Predictive => {
                // 予測スケーリングの実装（将来拡張）
            }
            ScalingPolicy::Manual => {
                // 手動スケーリングの場合は何もしない
            }
        }

        Ok(Some(ScalingAction::NoAction))
    }

    /// スケールアップを実行
    async fn execute_scale_up(
        current_instances: &Arc<RwLock<u32>>,
        last_scaling_time: &Arc<RwLock<SystemTime>>,
        scaling_history: &Arc<RwLock<Vec<ScalingEvent>>>,
        reason: String,
    ) -> Result<()> {
        let mut instances = current_instances.write().unwrap();
        let previous_instances = *instances;
        *instances += 1;

        let event = ScalingEvent {
            timestamp: SystemTime::now(),
            event_type: ScalingEventType::ScaleUp,
            previous_instances,
            new_instances: *instances,
            reason,
        };

        scaling_history.write().unwrap().push(event);
        *last_scaling_time.write().unwrap() = SystemTime::now();

        println!("Scaled up from {} to {} instances", previous_instances, *instances);
        Ok(())
    }

    /// スケールダウンを実行
    async fn execute_scale_down(
        current_instances: &Arc<RwLock<u32>>,
        last_scaling_time: &Arc<RwLock<SystemTime>>,
        scaling_history: &Arc<RwLock<Vec<ScalingEvent>>>,
        reason: String,
    ) -> Result<()> {
        let mut instances = current_instances.write().unwrap();
        let previous_instances = *instances;
        *instances -= 1;

        let event = ScalingEvent {
            timestamp: SystemTime::now(),
            event_type: ScalingEventType::ScaleDown,
            previous_instances,
            new_instances: *instances,
            reason,
        };

        scaling_history.write().unwrap().push(event);
        *last_scaling_time.write().unwrap() = SystemTime::now();

        println!("Scaled down from {} to {} instances", previous_instances, *instances);
        Ok(())
    }

    /// 現在のインスタンス数を取得
    pub fn get_current_instances(&self) -> u32 {
        *self.current_instances.read().unwrap()
    }

    /// スケーリング履歴を取得
    pub fn get_scaling_history(&self) -> Vec<ScalingEvent> {
        self.scaling_history.read().unwrap().clone()
    }
}

/// スケーリングアクション
enum ScalingAction {
    ScaleUp(String),
    ScaleDown(String),
    NoAction,
}

impl MetricsCollector {
    /// 新しいメトリクス収集器を作成
    pub fn new() -> Self {
        Self {
            cpu_metrics: Arc::new(RwLock::new(Vec::new())),
            memory_metrics: Arc::new(RwLock::new(Vec::new())),
            request_metrics: Arc::new(RwLock::new(Vec::new())),
            response_time_metrics: Arc::new(RwLock::new(Vec::new()))),
        }
    }

    /// メトリクスを収集
    pub async fn collect_metrics(&self) -> Result<()> {
        // 実際の実装では、各インスタンスからメトリクスを収集
        // ここではサンプルデータを生成

        let timestamp = SystemTime::now();

        // CPU使用率を収集（実際にはインスタンスから取得）
        let cpu_usage = 45.0 + (rand::random::<f64>() - 0.5) * 20.0;
        self.cpu_metrics.write().unwrap().push(MetricPoint {
            timestamp,
            value: cpu_usage.max(0.0).min(100.0),
            instance_id: "instance-1".to_string(),
        });

        // メモリ使用率を収集
        let memory_usage = 60.0 + (rand::random::<f64>() - 0.5) * 30.0;
        self.memory_metrics.write().unwrap().push(MetricPoint {
            timestamp,
            value: memory_usage.max(0.0).min(100.0),
            instance_id: "instance-1".to_string(),
        });

        // リクエスト数を収集
        let request_count = 50.0 + (rand::random::<f64>() - 0.5) * 25.0;
        self.request_metrics.write().unwrap().push(MetricPoint {
            timestamp,
            value: request_count.max(0.0),
            instance_id: "instance-1".to_string(),
        });

        // 古いメトリクスをクリーンアップ（5分以上前のデータを削除）
        self.cleanup_old_metrics();

        Ok(())
    }

    /// CPU使用率の平均を取得
    pub async fn get_average_cpu_usage(&self) -> Result<f64> {
        self.get_average_metric(&self.cpu_metrics).await
    }

    /// メモリ使用率の平均を取得
    pub async fn get_average_memory_usage(&self) -> Result<f64> {
        self.get_average_metric(&self.memory_metrics).await
    }

    /// リクエスト数の平均を取得
    pub async fn get_average_request_rate(&self) -> Result<f64> {
        self.get_average_metric(&self.request_metrics).await
    }

    /// メトリクスの平均を計算
    async fn get_average_metric(&self, metrics: &Arc<RwLock<Vec<MetricPoint>>>) -> Result<f64> {
        let metrics_data = metrics.read().unwrap();
        if metrics_data.is_empty() {
            return Ok(0.0);
        }

        let sum: f64 = metrics_data.iter().map(|m| m.value).sum();
        Ok(sum / metrics_data.len() as f64)
    }

    /// 古いメトリクスをクリーンアップ
    fn cleanup_old_metrics(&self) {
        let cutoff_time = SystemTime::now() - Duration::from_secs(300); // 5分前

        self.cpu_metrics.write().unwrap().retain(|m| m.timestamp > cutoff_time);
        self.memory_metrics.write().unwrap().retain(|m| m.timestamp > cutoff_time);
        self.request_metrics.write().unwrap().retain(|m| m.timestamp > cutoff_time);
        self.response_time_metrics.write().unwrap().retain(|m| m.timestamp > cutoff_time);
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
    pub fn add_instance(&self, instance: InstanceInfo) {
        self.instance_pool.write().unwrap().insert(instance.id.clone(), instance);
    }

    /// インスタンスを削除
    pub fn remove_instance(&self, instance_id: &str) {
        self.instance_pool.write().unwrap().remove(instance_id);
    }

    /// 最適なインスタンスを選択
    pub fn select_instance(&self, client_ip: Option<&str>) -> Option<InstanceInfo> {
        let pool = self.instance_pool.read().unwrap();

        if pool.is_empty() {
            return None;
        }

        // 実行中のインスタンスのみを対象
        let running_instances: Vec<_> = pool.values()
            .filter(|i| i.status == InstanceStatus::Running)
            .cloned()
            .collect();

        if running_instances.is_empty() {
            return None;
        }

        match self.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // 簡易的なラウンドロビン実装
                Some(running_instances[0].clone())
            }
            LoadBalancingAlgorithm::LeastConnections => {
                running_instances.into_iter()
                    .min_by_key(|i| i.active_connections)
                    .clone()
            }
            LoadBalancingAlgorithm::LeastResponseTime => {
                // 応答時間ベースの選択（将来拡張）
                Some(running_instances[0].clone())
            }
            LoadBalancingAlgorithm::IpHash => {
                if let Some(ip) = client_ip {
                    let hash = Self::hash_ip(ip);
                    let index = hash % running_instances.len();
                    Some(running_instances[index].clone())
                } else {
                    Some(running_instances[0].clone())
                }
            }
        }
    }

    /// IPアドレスをハッシュ化
    fn hash_ip(ip: &str) -> usize {
        let mut hash = 0usize;
        for byte in ip.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(byte as usize);
        }
        hash
    }

    /// ヘルスチェックを実行
    pub async fn perform_health_checks(&self) -> Result<()> {
        let mut pool = self.instance_pool.write().unwrap();

        for instance in pool.values_mut() {
            // 実際の実装ではHTTPヘルスチェックを実行
            // ここでは簡易的な実装

            let is_healthy = rand::random::<bool>(); // 実際にはヘルスチェックの結果

            instance.last_health_check = SystemTime::now();

            if is_healthy {
                if instance.status != InstanceStatus::Running {
                    instance.status = InstanceStatus::Running;
                    println!("Instance {} is now healthy", instance.id);
                }
            } else {
                if instance.status == InstanceStatus::Running {
                    instance.status = InstanceStatus::Error;
                    println!("Instance {} is unhealthy", instance.id);
                }
            }
        }

        Ok(())
    }
}

impl AutoScaler {
    /// 新しいオートスケーラーを作成
    pub fn new(
        scaling_engine: Arc<ScalingEngine>,
        load_balancer: Arc<LoadBalancer>,
        predictive_scaling_enabled: bool,
    ) -> Self {
        Self {
            scaling_engine,
            load_balancer,
            predictive_scaling_enabled,
            predictive_model: if predictive_scaling_enabled {
                Some(PredictiveModel::new())
            } else {
                None
            },
        }
    }

    /// オートスケーリングを開始
    pub async fn start(&self) -> Result<()> {
        // スケーリングエンジンを開始
        self.scaling_engine.start().await?;

        // ヘルスチェックを開始
        let load_balancer = self.load_balancer.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                if let Err(e) = load_balancer.perform_health_checks().await {
                    eprintln!("Health check failed: {}", e);
                }
            }
        });

        Ok(())
    }
}

impl PredictiveModel {
    /// 新しい予測モデルを作成
    pub fn new() -> Self {
        Self {
            historical_data: Vec::new(),
            algorithm: PredictionAlgorithm::LinearRegression,
        }
    }

    /// 予測を実行
    pub fn predict(&self, current_data: &TimeSeriesData) -> Result<u32> {
        // 簡易的な予測実装
        // 実際には機械学習アルゴリズムを使用

        if self.historical_data.is_empty() {
            return Ok(current_data.instance_count);
        }

        // 単純なトレンド分析
        let recent_trend = self.calculate_trend()?;
        let predicted_instances = (current_data.instance_count as f64 + recent_trend).round() as u32;

        Ok(predicted_instances.max(1))
    }

    /// トレンドを計算
    fn calculate_trend(&self) -> Result<f64> {
        if self.historical_data.len() < 2 {
            return Ok(0.0);
        }

        let len = self.historical_data.len();
        let recent = &self.historical_data[len - 1];
        let previous = &self.historical_data[len - 2];

        let cpu_diff = recent.cpu_usage - previous.cpu_usage;
        let request_diff = recent.request_count - previous.request_count;

        // CPU使用率とリクエスト数の変化に基づいてインスタンス数の変化を予測
        let trend = (cpu_diff * 0.6 + request_diff * 0.4) / 10.0;
        Ok(trend)
    }

    /// 履歴データを追加
    pub fn add_historical_data(&mut self, data: TimeSeriesData) {
        self.historical_data.push(data);

        // 古いデータを削除（直近100件のみ保持）
        if self.historical_data.len() > 100 {
            self.historical_data.remove(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deploy::config::{ScalingConfig, ScalingPolicy};

    #[test]
    fn test_scaling_engine_creation() {
        let config = ScalingConfig {
            min_instances: 1,
            max_instances: 10,
            cpu_threshold: 70.0,
            memory_threshold: 80.0,
            policy: ScalingPolicy::CpuBased,
            cooldown_period: 300,
        };

        let engine = ScalingEngine::new(config);
        assert_eq!(engine.get_current_instances(), 1);
    }

    #[test]
    fn test_load_balancer_creation() {
        let lb = LoadBalancer::new(LoadBalancingAlgorithm::RoundRobin);
        assert!(lb.select_instance(None).is_none());
    }

    #[test]
    fn test_instance_management() {
        let lb = LoadBalancer::new(LoadBalancingAlgorithm::RoundRobin);

        let instance = InstanceInfo {
            id: "test-instance".to_string(),
            address: "127.0.0.1".to_string(),
            port: 8080,
            status: InstanceStatus::Running,
            last_health_check: SystemTime::now(),
            cpu_usage: 50.0,
            memory_usage: 60.0,
            active_connections: 10,
        };

        lb.add_instance(instance.clone());
        let selected = lb.select_instance(None).unwrap();
        assert_eq!(selected.id, "test-instance");

        lb.remove_instance("test-instance");
        assert!(lb.select_instance(None).is_none());
    }

    #[test]
    fn test_metrics_collection() {
        let collector = MetricsCollector::new();

        // メトリクスが空の場合は0を返す
        assert_eq!(collector.get_average_cpu_usage(), Ok(0.0));
        assert_eq!(collector.get_average_memory_usage(), Ok(0.0));
        assert_eq!(collector.get_average_request_rate(), Ok(0.0));
    }

    #[test]
    fn test_predictive_model() {
        let mut model = PredictiveModel::new();

        let data = TimeSeriesData {
            timestamp: SystemTime::now(),
            cpu_usage: 70.0,
            request_count: 100.0,
            instance_count: 3,
        };

        model.add_historical_data(data.clone());
        let prediction = model.predict(&data).unwrap();
        assert!(prediction >= 1);
    }
}
